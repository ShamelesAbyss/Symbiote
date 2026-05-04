use crate::{
    app::{Environment, TRIBE_COUNT},
    automata::{CellKind, CellularAutomata, SignalKind},
    ecology::{Ecology, ZoneKind},
    particle::{Genome, Particle, RareTrait, Tribe},
    species::Archetype,
};

use rand::{rngs::StdRng, Rng, SeedableRng};

pub type RuleMatrix = [[f32; TRIBE_COUNT]; TRIBE_COUNT];

const MIN_DISTANCE: f32 = 0.011;
const FRICTION: f32 = 0.912;
const FORCE_SCALE: f32 = 0.00122;
const WALL_FORCE: f32 = 0.017;
const BOND_RADIUS: f32 = 0.105;

const LOW_SUBSTRATE_RATIO: f32 = 0.035;
const HARVESTER_BODY_PRESSURE_RATIO: f32 = 0.115;
const HARVESTER_OVERGROWTH_RATIO: f32 = 0.18;
const SIGNAL_FORCE_SCALE: f32 = 0.42;

#[allow(dead_code)]
#[derive(Default, Clone, Copy, Debug)]
pub struct StepReport {
    pub cells_consumed: usize,
    pub harvesters_consumed: usize,
    pub harvester_particles: usize,
    pub reaper_particles: usize,
    pub living_substrate: usize,
    pub total_substrate: usize,
}

pub fn build_rule_matrix(seed: u64) -> RuleMatrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut matrix = [[0.0; TRIBE_COUNT]; TRIBE_COUNT];

    for row in matrix.iter_mut() {
        for value in row.iter_mut() {
            *value = rng.gen_range(-1.0..1.0);
        }
    }

    matrix
}

pub fn mutate_rules(rules: &mut RuleMatrix, seed: u64, intensity: f32) {
    let mut rng = StdRng::seed_from_u64(seed);

    for row in rules.iter_mut() {
        for value in row.iter_mut() {
            if rng.gen_bool(0.18) {
                *value = (*value + rng.gen_range(-intensity..intensity)).clamp(-1.0, 1.0);
            }
        }
    }
}

pub fn step_particles(
    particles: &mut [Particle],
    rules: &RuleMatrix,
    env: Environment,
    ecology: &Ecology,
    substrate: &mut CellularAutomata,
    archetypes: &[Option<Archetype>],
) -> StepReport {
    let snapshot = particles.to_vec();

    let snapshot_archetypes = snapshot
        .iter()
        .map(|particle| {
            particle
                .species_id
                .and_then(|id| archetypes.get(id as usize).copied().flatten())
        })
        .collect::<Vec<_>>();

    let harvester_particles = snapshot
        .iter()
        .enumerate()
        .filter(|(idx, particle)| {
            matches!(snapshot_archetypes[*idx], Some(Archetype::Harvester))
                || particle.rare_trait == RareTrait::Devourer
        })
        .count();

    let reaper_particles = snapshot
        .iter()
        .enumerate()
        .filter(|(idx, _)| matches!(snapshot_archetypes[*idx], Some(Archetype::Reaper)))
        .count();

    let total_substrate = substrate.total_cells();
    let living_substrate = substrate.living_cells();
    let substrate_ratio = living_substrate as f32 / total_substrate.max(1) as f32;
    let harvester_ratio = harvester_particles as f32 / snapshot.len().max(1) as f32;

    let low_substrate = substrate_ratio < LOW_SUBSTRATE_RATIO;
    let harvester_overgrowth = harvester_ratio > HARVESTER_OVERGROWTH_RATIO;
    let reaper_pressure_needed =
        harvester_ratio > HARVESTER_BODY_PRESSURE_RATIO || harvester_particles >= 15;

    let mut report = StepReport {
        cells_consumed: 0,
        harvesters_consumed: 0,
        harvester_particles,
        reaper_particles,
        living_substrate,
        total_substrate,
    };

    let mut damage = vec![0.0f32; particles.len()];

    for (idx, particle) in particles.iter_mut().enumerate() {
        let mut fx = 0.0;
        let mut fy = 0.0;

        let mut local_density = 0usize;
        let mut friendly_density = 0usize;
        let mut hostile_density = 0usize;

        let mut vx_avg = 0.0;
        let mut vy_avg = 0.0;
        let mut orbit_x = 0.0;
        let mut orbit_y = 0.0;

        let archetype = snapshot_archetypes[idx];

        let is_reaper = matches!(archetype, Some(Archetype::Reaper));
        let is_harvester =
            matches!(archetype, Some(Archetype::Harvester)) || particle.rare_trait == RareTrait::Devourer;

        apply_signal_field(
            particle,
            substrate,
            archetype,
            low_substrate,
            reaper_pressure_needed,
            &mut fx,
            &mut fy,
        );

        for (other_idx, other) in snapshot.iter().enumerate() {
            if idx == other_idx {
                continue;
            }

            let other_archetype = snapshot_archetypes[other_idx];

            let other_is_harvester =
                matches!(other_archetype, Some(Archetype::Harvester))
                    || other.rare_trait == RareTrait::Devourer;

            let other_is_reaper = matches!(other_archetype, Some(Archetype::Reaper));

            let dx = other.x - particle.x;
            let dy = other.y - particle.y;
            let d2 = dx * dx + dy * dy;

            if d2 <= 0.000001 {
                continue;
            }

            let d = d2.sqrt();
            let attraction = rules[particle.tribe.index()][other.tribe.index()];
            let predator_pressure = predator_factor(particle.tribe, other.tribe, archetype);

            if d < BOND_RADIUS {
                local_density += 1;
                vx_avg += other.vx;
                vy_avg += other.vy;

                if attraction >= 0.0 {
                    friendly_density += 1;
                } else {
                    hostile_density += 1;
                }

                let bond_mult = match archetype {
                    Some(Archetype::Swarmer) => 1.35,
                    Some(Archetype::Architect) => 1.55,
                    Some(Archetype::Leviathan) => 1.7,
                    Some(Archetype::Harvester) => 0.84,
                    Some(Archetype::Reaper) => 0.55,
                    Some(Archetype::Parasite) => 0.82,
                    _ => 1.0,
                };

                let bond = (1.0 - d / BOND_RADIUS) * particle.genome.bonding * bond_mult;

                fx += dx * bond * 0.62;
                fy += dy * bond * 0.62;

                orbit_x += -dy / d;
                orbit_y += dx / d;
            }

            let mut perception = particle.genome.perception * env.perception_mult();

            if matches!(
                archetype,
                Some(Archetype::Grazer | Archetype::Hunter | Archetype::Harvester | Archetype::Reaper)
            ) {
                perception *= 1.18;
            }

            if is_reaper && reaper_pressure_needed {
                perception *= 1.18;
            }

            if is_harvester && low_substrate {
                perception *= 0.86;
            }

            if d > perception {
                continue;
            }

            if is_reaper && other_is_harvester {
                let pressure_boost = if reaper_pressure_needed { 1.35 } else { 1.0 };
                let chase = (1.0 - d / perception).max(0.0) * 3.1 * pressure_boost;

                fx += (dx / d) * chase;
                fy += (dy / d) * chase;

                if d < 0.052 {
                    let bite = 3.25 + particle.genome.volatility * 1.35;

                    damage[other_idx] += bite;

                    particle.energy += bite * 2.05;
                    particle.health += bite * 0.48;
                    particle.mass += 0.0045;

                    substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.20);
                    substrate.deposit_signal(other.x, other.y, SignalKind::Danger, 0.26);

                    if other.health <= bite + 1.0 {
                        report.harvesters_consumed += 1;
                    }
                }

                continue;
            }

            if is_harvester && other_is_reaper {
                let fear = (1.0 - d / perception).max(0.0) * 2.55;

                fx -= (dx / d) * fear;
                fy -= (dy / d) * fear;

                particle.energy -= if low_substrate { 0.014 } else { 0.007 };
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.08);

                continue;
            }

            let mut force = if d < MIN_DISTANCE {
                -1.7
            } else {
                attraction * predator_pressure * (1.0 - d / perception)
            };

            force *= particle.genome.volatility;
            force *= env.force_mult();

            if matches!(archetype, Some(Archetype::Hunter)) {
                force *= 1.18;
            }

            if matches!(archetype, Some(Archetype::Reaper)) && !other_is_harvester {
                force *= 0.35;
            }

            if is_harvester && low_substrate {
                force *= 0.88;
            }

            fx += (dx / d) * force;
            fy += (dy / d) * force;

            if matches!(archetype, Some(Archetype::Hunter))
                && predator_pressure > 1.1
                && d < perception * 0.45
            {
                particle.energy += 0.018;
                particle.health += 0.012;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.035);
            }

            if matches!(archetype, Some(Archetype::Parasite)) && d < BOND_RADIUS && other.mass > particle.mass {
                particle.energy += 0.012;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.025);
            }
        }

        if local_density > 0 {
            vx_avg /= local_density as f32;
            vy_avg /= local_density as f32;

            particle.vx += (vx_avg - particle.vx) * 0.22;
            particle.vy += (vy_avg - particle.vy) * 0.22;

            let orbit_boost = if matches!(archetype, Some(Archetype::Orbiter)) {
                1.75
            } else {
                1.0
            };

            particle.vx += orbit_x * particle.genome.orbit * orbit_boost * 0.00042;
            particle.vy += orbit_y * particle.genome.orbit * orbit_boost * 0.00042;
        }

        apply_ecology(particle, ecology);

        report.cells_consumed += apply_substrate(
            particle,
            substrate,
            archetype,
            low_substrate,
            harvester_overgrowth,
        );

        deposit_behavior_signal(
            particle,
            substrate,
            archetype,
            low_substrate,
            harvester_overgrowth,
            reaper_pressure_needed,
        );

        let mass_drag = (1.0 + particle.mass * 0.13).clamp(1.0, 2.0);

        particle.vx = (particle.vx + fx * FORCE_SCALE) * FRICTION / mass_drag;
        particle.vy = (particle.vy + fy * FORCE_SCALE) * FRICTION / mass_drag;

        apply_environment_current(particle, env);

        if particle.x < -1.0 {
            particle.vx += WALL_FORCE;
        }

        if particle.x > 1.0 {
            particle.vx -= WALL_FORCE;
        }

        if particle.y < -1.0 {
            particle.vy += WALL_FORCE;
        }

        if particle.y > 1.0 {
            particle.vy -= WALL_FORCE;
        }

        particle.x = (particle.x + particle.vx).clamp(-1.2, 1.2);
        particle.y = (particle.y + particle.vy).clamp(-1.2, 1.2);

        if friendly_density >= 4 {
            particle.health += 0.14;
            particle.energy += 0.028;
            particle.mass += 0.014;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.018);
        }

        if hostile_density >= 3 {
            particle.health -= 0.21;
            particle.energy -= 0.026;
            particle.mass -= 0.012;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.035);
        }

        if local_density == 0 {
            particle.health -= 0.07;
            particle.energy -= 0.04;
            particle.mass -= 0.008;
        }

        if particle.cluster_id.is_some() {
            particle.health += 0.035;
            particle.energy += 0.012;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.012);
        }

        if env == Environment::Bloom {
            particle.health += 0.035;
            particle.energy += 0.018;
        }

        if matches!(archetype, Some(Archetype::Architect | Archetype::Leviathan)) {
            particle.mass += 0.002;
        }

        if matches!(archetype, Some(Archetype::Harvester)) {
            particle.genome.perception = (particle.genome.perception + 0.00001).clamp(0.1, 0.38);

            if low_substrate {
                particle.energy -= 0.052;
                particle.health -= 0.034;
                particle.mass -= 0.012;
                particle.genome.fertility = (particle.genome.fertility - 0.00025).clamp(0.2, 2.4);
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.08);
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.025);
            } else {
                particle.energy -= 0.008;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.035);
            }

            if harvester_overgrowth {
                particle.energy -= 0.026;
                particle.health -= 0.018;
                particle.genome.hunger = (particle.genome.hunger + 0.00008).clamp(0.005, 0.04);
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.06);
            }
        }

        if matches!(archetype, Some(Archetype::Reaper)) {
            let starvation_relief = if reaper_pressure_needed { 0.55 } else { 1.0 };

            particle.energy -= 0.018 * starvation_relief;
            particle.health -= 0.006 * starvation_relief;
            particle.mass = (particle.mass + 0.002).clamp(0.45, 7.0);
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.052);
        }

        if particle.rare_trait == RareTrait::Radiant {
            particle.energy += 0.015;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.012);
        }

        if particle.rare_trait == RareTrait::Voracious {
            particle.energy -= 0.01;
            particle.health += 0.008;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.03);
        }

        if particle.rare_trait == RareTrait::Devourer {
            particle.energy -= if low_substrate { 0.035 } else { 0.012 };
            particle.health += if low_substrate { 0.0 } else { 0.004 };
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.06);
        }

        particle.energy -= particle.genome.metabolism * env.hunger_mult();
        particle.health -= particle.genome.hunger * env.hunger_mult();

        particle.energy = particle.energy.clamp(0.0, 160.0);
        particle.health = particle.health.clamp(0.0, 100.0);
        particle.mass = particle.mass.clamp(0.45, 7.0);
        particle.age = particle.age.saturating_add(1);
    }

    for (idx, amount) in damage.into_iter().enumerate() {
        if amount > 0.0 {
            if let Some(particle) = particles.get_mut(idx) {
                particle.health -= amount;
                particle.energy -= amount * 1.4;
                particle.mass = (particle.mass - amount * 0.002).clamp(0.45, 7.0);
            }
        }
    }

    report
}

fn apply_signal_field(
    particle: &Particle,
    substrate: &CellularAutomata,
    archetype: Option<Archetype>,
    low_substrate: bool,
    reaper_pressure_needed: bool,
    fx: &mut f32,
    fy: &mut f32,
) {
    let signal = substrate.signal_at(particle.x, particle.y);

    let mut seek = 0.0;
    let mut avoid = 0.0;

    match archetype {
        Some(Archetype::Harvester) => {
            seek += signal.hunger * if low_substrate { 0.55 } else { 0.92 };
            seek += signal.growth * 0.42;
            avoid += signal.fear * 1.15;
            avoid += signal.danger * 0.55;
        }
        Some(Archetype::Reaper) => {
            seek += signal.hunger * if reaper_pressure_needed { 1.28 } else { 0.68 };
            seek += signal.fear * 0.25;
            avoid += signal.growth * 0.15;
            avoid += signal.danger * 0.18;
        }
        Some(Archetype::Grazer | Archetype::Mycelial) => {
            seek += signal.growth * 0.86;
            avoid += signal.danger * 0.82;
            avoid += signal.fear * 0.42;
        }
        Some(Archetype::Hunter | Archetype::Parasite) => {
            seek += signal.danger * 0.34;
            seek += signal.hunger * 0.26;
            avoid += signal.fear * 0.22;
        }
        Some(Archetype::Architect | Archetype::Leviathan) => {
            seek += signal.growth * 0.54;
            avoid += signal.danger * 0.38;
        }
        Some(Archetype::Phantom) => {
            seek += signal.fear * 0.34;
            seek += signal.danger * 0.28;
            avoid += signal.growth * 0.18;
        }
        Some(Archetype::Swarmer | Archetype::Orbiter) | None => {
            seek += signal.growth * 0.32;
            avoid += signal.danger * 0.36;
            avoid += signal.fear * 0.24;
        }
    }

    if particle.rare_trait == RareTrait::Devourer {
        seek += signal.hunger * 0.42;
        avoid += signal.fear * 0.35;
    }

    if particle.health < 32.0 || particle.energy < 25.0 {
        avoid += signal.danger * 0.7;
        avoid += signal.fear * 0.25;
        seek += signal.growth * 0.28;
    }

    let field = (seek - avoid).clamp(-1.0, 1.0);
    let curl_x = ((particle.y * 17.0 + particle.x * 9.0).sin()) * field * SIGNAL_FORCE_SCALE;
    let curl_y = ((particle.x * 19.0 - particle.y * 7.0).cos()) * field * SIGNAL_FORCE_SCALE;

    *fx += curl_x;
    *fy += curl_y;
}

fn deposit_behavior_signal(
    particle: &Particle,
    substrate: &mut CellularAutomata,
    archetype: Option<Archetype>,
    low_substrate: bool,
    harvester_overgrowth: bool,
    reaper_pressure_needed: bool,
) {
    match archetype {
        Some(Archetype::Harvester) => {
            substrate.deposit_signal(
                particle.x,
                particle.y,
                SignalKind::Hunger,
                if low_substrate || harvester_overgrowth { 0.045 } else { 0.022 },
            );
        }
        Some(Archetype::Reaper) => {
            substrate.deposit_signal(
                particle.x,
                particle.y,
                SignalKind::Fear,
                if reaper_pressure_needed { 0.042 } else { 0.026 },
            );
        }
        Some(Archetype::Grazer | Archetype::Mycelial) => {
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.018);
        }
        Some(Archetype::Hunter | Archetype::Parasite) => {
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.018);
        }
        Some(Archetype::Architect | Archetype::Leviathan) => {
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.025);
        }
        Some(Archetype::Phantom) => {
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.012);
        }
        _ => {}
    }

    if particle.health < 24.0 {
        substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.04);
    }

    if particle.energy > 110.0 && particle.cluster_id.is_some() {
        substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.025);
    }
}

fn predator_factor(a: Tribe, b: Tribe, archetype: Option<Archetype>) -> f32 {
    let ai = a.index();
    let bi = b.index();

    let base = if (ai + 1) % TRIBE_COUNT == bi {
        -1.35
    } else if (bi + 1) % TRIBE_COUNT == ai {
        1.22
    } else {
        1.0
    };

    if matches!(archetype, Some(Archetype::Hunter | Archetype::Reaper)) {
        base * 1.24
    } else if matches!(archetype, Some(Archetype::Grazer | Archetype::Harvester)) {
        base * 0.86
    } else {
        base
    }
}

fn apply_substrate(
    particle: &mut Particle,
    substrate: &mut CellularAutomata,
    archetype: Option<Archetype>,
    low_substrate: bool,
    harvester_overgrowth: bool,
) -> usize {
    let kind = substrate.influence_at(particle.x, particle.y);
    let mut consumed = 0usize;

    let is_harvester =
        matches!(archetype, Some(Archetype::Harvester)) || particle.rare_trait == RareTrait::Devourer;

    if is_harvester && kind != CellKind::Empty {
        let protected_regeneration = matches!(
            kind,
            CellKind::Dead | CellKind::Nutrient | CellKind::Spore | CellKind::Root
        );

        if protected_regeneration {
            particle.energy -= if low_substrate { 0.028 } else { 0.014 };

            if low_substrate {
                particle.health -= 0.014;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.045);
            }

            return consumed;
        }

        let power = if particle.rare_trait == RareTrait::Devourer {
            if low_substrate { 42.0 } else { 62.0 }
        } else if low_substrate {
            28.0
        } else {
            38.0
        };

        let compost = true;

        if let Some(eaten) = substrate.consume_at(particle.x, particle.y, power, compost) {
            let gain = eaten.food_value();

            let gain_mult = if particle.rare_trait == RareTrait::Devourer {
                if low_substrate { 0.82 } else { 1.08 }
            } else if low_substrate {
                0.54
            } else {
                0.74
            };

            particle.energy += gain * gain_mult;
            particle.health += gain * 0.115;
            particle.mass += gain * 0.00165;

            substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.18);

            if harvester_overgrowth {
                particle.energy -= 0.018;
                particle.health -= 0.012;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.035);
            }

            consumed += 1;
        }

        return consumed;
    }

    match kind {
        CellKind::Life => {
            particle.energy += 0.018;
            particle.health += 0.012;
        }
        CellKind::Spore => {
            particle.energy += 0.025;
            particle.genome.fertility = (particle.genome.fertility + 0.00018).clamp(0.2, 2.4);
        }
        CellKind::Nutrient => {
            particle.energy += 0.04;
            particle.health += 0.02;
        }
        CellKind::Dead => {
            particle.energy -= 0.02;
            particle.health -= 0.015;
        }
        CellKind::Mutagen => {
            particle.genome.volatility = (particle.genome.volatility + 0.00055).clamp(0.36, 1.95);
            particle.genome.orbit = (particle.genome.orbit + 0.00035).clamp(0.0, 1.55);
        }
        CellKind::Nest => {
            particle.energy += 0.032;
            particle.mass += 0.002;
        }
        CellKind::Root => {
            particle.energy += 0.006;
            particle.health += 0.004;
        }
        CellKind::Empty => {}
    }

    consumed
}

fn apply_ecology(particle: &mut Particle, ecology: &Ecology) {
    for zone in &ecology.zones {
        let dx = zone.x - particle.x;
        let dy = zone.y - particle.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > zone.radius {
            continue;
        }

        let effect = (1.0 - dist / zone.radius) * zone.strength;

        match zone.kind {
            ZoneKind::Nutrient => {
                particle.health += 0.12 * effect;
                particle.energy += 0.08 * effect;
                particle.mass += 0.006 * effect;
            }
            ZoneKind::Dead => {
                particle.health -= 0.18 * effect;
                particle.energy -= 0.09 * effect;
                particle.mass -= 0.006 * effect;
            }
            ZoneKind::Turbulent => {
                particle.vx += (particle.y * 33.0).sin() * 0.001 * effect;
                particle.vy -= (particle.x * 29.0).cos() * 0.001 * effect;
            }
            ZoneKind::Mutagen => {
                particle.genome.volatility =
                    (particle.genome.volatility + 0.00045 * effect).clamp(0.36, 1.95);
                particle.genome.orbit =
                    (particle.genome.orbit + 0.0003 * effect).clamp(0.0, 1.55);
            }
            ZoneKind::Nest => {
                particle.energy += 0.04 * effect;
                particle.genome.fertility =
                    (particle.genome.fertility + 0.00035 * effect).clamp(0.2, 2.4);
            }
        }
    }
}

fn apply_environment_current(particle: &mut Particle, env: Environment) {
    match env {
        Environment::Calm => {}
        Environment::Bloom => {
            particle.vx *= 0.998;
            particle.vy *= 0.998;
        }
        Environment::Hunger => {
            particle.vx *= 1.006;
            particle.vy *= 1.006;
        }
        Environment::Storm => {
            let phase = ((particle.x * 22.0 + particle.y * 31.0).sin()) * 0.00105;
            particle.vx += phase;
            particle.vy -= phase;
        }
        Environment::Drift => {
            particle.vx += 0.0002;
            particle.vy += 0.00007;
        }
    }
}

pub fn child_from(parent: Particle, seed: u64) -> Particle {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut child = parent;

    child.x += rng.gen_range(-0.04..0.04);
    child.y += rng.gen_range(-0.04..0.04);
    child.vx = rng.gen_range(-0.006..0.006);
    child.vy = rng.gen_range(-0.006..0.006);
    child.age = 0;
    child.health = 72.0;
    child.energy = 70.0;
    child.mass = (parent.mass * 0.62).clamp(0.45, 3.2);
    child.cluster_id = None;
    child.genome = mutate_genome(parent.genome, &mut rng);

    if rng.gen_bool(0.025) {
        child.tribe = Tribe::from_index(rng.gen_range(0..TRIBE_COUNT));
        child.species_id = None;
    }

    if rng.gen_bool(0.0011) {
        child.rare_trait = roll_rare_trait(&mut rng, child.genome, parent.mass);
        child.species_id = None;
    }

    child
}

pub fn fused_child(a: Particle, b: Particle, seed: u64) -> Particle {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut child = a;

    child.x = (a.x + b.x) / 2.0 + rng.gen_range(-0.025..0.025);
    child.y = (a.y + b.y) / 2.0 + rng.gen_range(-0.025..0.025);
    child.vx = rng.gen_range(-0.005..0.005);
    child.vy = rng.gen_range(-0.005..0.005);
    child.age = 0;
    child.health = 78.0;
    child.energy = 82.0;
    child.mass = ((a.mass + b.mass) * 0.34).clamp(0.55, 4.0);
    child.cluster_id = None;
    child.species_id = None;

    child.genome = Genome {
        perception: (a.genome.perception + b.genome.perception) / 2.0,
        hunger: (a.genome.hunger + b.genome.hunger) / 2.0,
        bonding: (a.genome.bonding + b.genome.bonding) / 2.0,
        volatility: (a.genome.volatility + b.genome.volatility) / 2.0,
        orbit: (a.genome.orbit + b.genome.orbit) / 2.0,
        membrane: (a.genome.membrane + b.genome.membrane) / 2.0,
        metabolism: (a.genome.metabolism + b.genome.metabolism) / 2.0,
        fertility: (a.genome.fertility + b.genome.fertility) / 2.0,
    };

    child.genome = mutate_genome(child.genome, &mut rng);

    if rng.gen_bool(0.5) {
        child.tribe = b.tribe;
    }

    if rng.gen_bool(0.0022) {
        child.rare_trait = roll_rare_trait(&mut rng, child.genome, child.mass);
    } else {
        child.rare_trait = if rng.gen_bool(0.5) {
            a.rare_trait
        } else {
            b.rare_trait
        };
    }

    child
}

pub fn mutate_genome(mut genome: Genome, rng: &mut StdRng) -> Genome {
    genome.perception = mutate_float(genome.perception, 0.01, 0.1, 0.38, rng);
    genome.hunger = mutate_float(genome.hunger, 0.0022, 0.005, 0.04, rng);
    genome.bonding = mutate_float(genome.bonding, 0.045, 0.5, 2.25, rng);
    genome.volatility = mutate_float(genome.volatility, 0.04, 0.36, 1.95, rng);
    genome.orbit = mutate_float(genome.orbit, 0.04, 0.0, 1.55, rng);
    genome.membrane = mutate_float(genome.membrane, 0.04, 0.0, 1.8, rng);
    genome.metabolism = mutate_float(genome.metabolism, 0.002, 0.004, 0.05, rng);
    genome.fertility = mutate_float(genome.fertility, 0.032, 0.2, 2.4, rng);

    if genome.perception > 0.295 && genome.fertility > 1.35 && genome.hunger < 0.018 {
        genome.hunger = (genome.hunger + rng.gen_range(0.0002..0.0012)).clamp(0.005, 0.04);
        genome.fertility = (genome.fertility - rng.gen_range(0.004..0.018)).clamp(0.2, 2.4);
    }

    genome
}

fn roll_rare_trait(rng: &mut StdRng, genome: Genome, mass: f32) -> RareTrait {
    if genome.perception > 0.34
        && genome.fertility > 1.82
        && genome.hunger < 0.014
        && genome.metabolism < 0.018
        && rng.gen_bool(0.16)
    {
        RareTrait::Devourer
    } else if mass > 5.6 && genome.membrane > 1.1 {
        RareTrait::ElderCore
    } else if genome.fertility > 1.9 && genome.bonding > 1.7 {
        RareTrait::SporeKing
    } else if genome.volatility > 1.65 && genome.metabolism > 0.026 {
        RareTrait::Voracious
    } else if genome.orbit > 1.25 {
        RareTrait::Voidborne
    } else if genome.bonding > 1.8 {
        RareTrait::SymbioticCore
    } else if rng.gen_bool(0.45) {
        RareTrait::Radiant
    } else {
        RareTrait::None
    }
}

fn mutate_float(value: f32, amount: f32, min: f32, max: f32, rng: &mut StdRng) -> f32 {
    (value + rng.gen_range(-amount..amount)).clamp(min, max)
}
