use crate::{
    app::{Environment, TRIBE_COUNT},
    automata::{CellKind, CellularAutomata, SignalKind},
    ecology::{Ecology, ZoneKind},
    field::PatternField,
    particle::{Genome, Particle, RareTrait, Tribe},
    species::Archetype,
    tree::TreeForces,
};

use rand::{rngs::StdRng, Rng, SeedableRng};

pub type RuleMatrix = [[f32; TRIBE_COUNT]; TRIBE_COUNT];

const MIN_DISTANCE: f32 = 0.011;
const FRICTION: f32 = 0.912;
const FORCE_SCALE: f32 = 0.00122;
const WALL_FORCE: f32 = 0.017;
const BOND_RADIUS: f32 = 0.105;

const LOW_SUBSTRATE_RATIO: f32 = 0.035;
const HARVESTER_BODY_PRESSURE_RATIO: f32 = 0.095;
const HARVESTER_OVERGROWTH_RATIO: f32 = 0.155;

const SIGNAL_FORCE_SCALE: f32 = 0.42;
const ROOT_AVOIDANCE_RADIUS: f32 = TreeForces::DEFAULT.avoidance_radius;
const ROOT_FORCE_SCALE: f32 = TreeForces::DEFAULT.force_scale;
const ROOT_CHANNEL_FORCE: f32 = TreeForces::DEFAULT.channel_force;

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
    pattern_field: &PatternField,
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
    let reaper_pressure_needed = harvester_ratio > HARVESTER_BODY_PRESSURE_RATIO
        || harvester_particles >= 10
        || (harvester_particles >= 6 && substrate_ratio < 0.075);

    let matrix_pressure = matrix_pressure(rules);
    let matrix_attraction = matrix_attraction(rules);
    let matrix_repulsion = matrix_repulsion(rules);

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
        let is_harvester = matches!(archetype, Some(Archetype::Harvester))
            || particle.rare_trait == RareTrait::Devourer;

        apply_signal_field(
            particle,
            substrate,
            pattern_field,
            archetype,
            low_substrate,
            reaper_pressure_needed,
            &mut fx,
            &mut fy,
        );

        apply_root_field(
            particle,
            substrate,
            archetype,
            low_substrate,
            harvester_overgrowth,
            &mut fx,
            &mut fy,
        );

        for (other_idx, other) in snapshot.iter().enumerate() {
            if idx == other_idx {
                continue;
            }

            let other_archetype = snapshot_archetypes[other_idx];
            let other_is_harvester = matches!(other_archetype, Some(Archetype::Harvester))
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
            let pair_pressure = matrix_pair_pressure(
                attraction,
                particle.tribe,
                other.tribe,
                matrix_pressure,
                matrix_attraction,
                matrix_repulsion,
            );

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
                    Some(Archetype::Swarmer) => 1.20,
                    Some(Archetype::Architect) => 1.34,
                    Some(Archetype::Leviathan) => 1.48,
                    Some(Archetype::Harvester) => 0.72,
                    Some(Archetype::Reaper) => 0.46,
                    Some(Archetype::Parasite) => 0.78,
                    _ => 0.92,
                };

                let matrix_bond = if attraction > 0.0 {
                    1.0 + attraction.abs() * matrix_attraction * 0.18
                } else {
                    1.0 - attraction.abs() * matrix_repulsion * 0.12
                }
                .clamp(0.68_f32, 1.30_f32);

                let bond =
                    (1.0 - d / BOND_RADIUS) * particle.genome.bonding * bond_mult * matrix_bond;

                fx += dx * bond * 0.58;
                fy += dy * bond * 0.58;

                orbit_x += -dy / d;
                orbit_y += dx / d;
            }

            let mut perception = particle.genome.perception * env.perception_mult();

            if matches!(
                archetype,
                Some(
                    Archetype::Grazer
                        | Archetype::Hunter
                        | Archetype::Harvester
                        | Archetype::Reaper
                )
            ) {
                perception *= 1.16;
            }

            if is_reaper && reaper_pressure_needed {
                perception *= 1.24;
            }

            if is_harvester && low_substrate {
                perception *= 0.92;
            }

            perception *= matrix_perception_factor(attraction, matrix_pressure, archetype);

            if d > perception {
                continue;
            }

            if is_reaper && other_is_harvester {
                let pressure_boost = if reaper_pressure_needed { 1.52 } else { 1.08 };
                let matrix_hunt_boost = 1.0 + matrix_repulsion * 0.20 + matrix_pressure * 0.14;
                let chase =
                    (1.0 - d / perception).max(0.0) * 3.55 * pressure_boost * matrix_hunt_boost;

                fx += (dx / d) * chase;
                fy += (dy / d) * chase;

                if d < 0.058 {
                    let bite =
                        (3.45 + particle.genome.volatility * 1.45) * (1.0 + matrix_pressure * 0.10);

                    damage[other_idx] += bite;
                    particle.energy += bite * 2.10;
                    particle.health += bite * 0.52;
                    particle.mass += 0.0048;

                    substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.22);
                    substrate.deposit_signal(other.x, other.y, SignalKind::Danger, 0.30);

                    if other.health <= bite + 1.0 {
                        report.harvesters_consumed += 1;
                    }
                }

                continue;
            }

            if is_harvester && other_is_reaper {
                let fear = (1.0 - d / perception).max(0.0) * 2.75 * (1.0 + matrix_repulsion * 0.24);

                fx -= (dx / d) * fear;
                fy -= (dy / d) * fear;

                particle.energy -= if low_substrate { 0.012 } else { 0.006 };
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Fear, 0.08);

                continue;
            }

            let mut force = if d < MIN_DISTANCE {
                -1.7
            } else {
                attraction * predator_pressure * pair_pressure * (1.0 - d / perception)
            };

            force *= particle.genome.volatility;
            force *= env.force_mult();

            if matches!(archetype, Some(Archetype::Hunter)) {
                force *= 1.18 + matrix_repulsion * 0.10;
            }

            if matches!(archetype, Some(Archetype::Reaper)) && !other_is_harvester {
                force *= 0.35;
            }

            if is_harvester && low_substrate {
                force *= 0.92;
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

            if matches!(archetype, Some(Archetype::Parasite))
                && d < BOND_RADIUS
                && other.mass > particle.mass
            {
                particle.energy += 0.012;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.025);
            }
        }

        if local_density > 0 {
            vx_avg /= local_density as f32;
            vy_avg /= local_density as f32;

            let matrix_alignment =
                (1.0 + matrix_attraction * 0.14 - matrix_repulsion * 0.08).clamp(0.82, 1.20);

            particle.vx += (vx_avg - particle.vx) * 0.18 * matrix_alignment;
            particle.vy += (vy_avg - particle.vy) * 0.18 * matrix_alignment;

            let orbit_boost = if matches!(archetype, Some(Archetype::Orbiter)) {
                1.75
            } else {
                1.0
            };

            particle.vx += orbit_x * particle.genome.orbit * orbit_boost * 0.00022; // PATTERN_CALM_PASS_ACTIVE
            particle.vy += orbit_y * particle.genome.orbit * orbit_boost * 0.00022;
            // PATTERN_CALM_PASS_ACTIVE
        }

        apply_ecology(particle, ecology);

        report.cells_consumed += apply_substrate(
            particle,
            substrate,
            pattern_field,
            archetype,
            low_substrate,
            harvester_overgrowth,
        );
        deposit_behavior_signal(
            particle,
            substrate,
            pattern_field,
            archetype,
            harvester_overgrowth,
            reaper_pressure_needed,
            low_substrate,
        );
        let mass_drag = (1.0 + particle.mass * 0.13).clamp(1.0, 2.0);

        particle.vx = (particle.vx + fx * FORCE_SCALE) * FRICTION / mass_drag;
        particle.vy = (particle.vy + fy * FORCE_SCALE) * FRICTION / mass_drag;

        apply_environment_current(particle, env);

        let field_here = pattern_field.sample_world(particle.x, particle.y);
        let field_east =
            pattern_field.sample_world((particle.x + 0.045).clamp(-1.2, 1.2), particle.y);
        let field_west =
            pattern_field.sample_world((particle.x - 0.045).clamp(-1.2, 1.2), particle.y);
        let field_north =
            pattern_field.sample_world(particle.x, (particle.y - 0.045).clamp(-1.2, 1.2));
        let field_south =
            pattern_field.sample_world(particle.x, (particle.y + 0.045).clamp(-1.2, 1.2));

        let field_strength = field_here.influence_strength().clamp(0.0, 1.0);
        let field_dx = field_east.influence_strength() - field_west.influence_strength();
        let field_dy = field_south.influence_strength() - field_north.influence_strength();

        let (field_pull, field_calm, field_hunger) = field_polarity_response(
            archetype,
            particle.rare_trait,
            field_here.is_dangerous(),
            field_strength,
            particle.energy,
            particle.health,
        );

        if field_pull.abs() > 0.0 && field_strength > 0.08 {
            particle.vx += field_dx * field_pull;
            particle.vy += field_dy * field_pull;
        }

        if field_calm > 0.0 {
            let calm = (1.0 - field_calm).clamp(0.992, 1.0);
            particle.vx *= calm;
            particle.vy *= calm;
        }

        if field_hunger > 0.0 {
            particle.energy -= field_hunger;
            particle.health -= field_hunger * 0.42;
        }

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

        if substrate.influence_at(particle.x, particle.y) == CellKind::Root {
            nudge_from_root(particle, substrate);
        }

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
            particle.health -= 0.055;
            particle.energy -= 0.032;
            particle.mass -= 0.007;
        }

        if particle.cluster_id.is_some() {
            particle.health += 0.025;
            particle.energy += 0.009;
            substrate.deposit_signal(particle.x, particle.y, SignalKind::Growth, 0.010);
        }

        if env == Environment::Bloom {
            particle.health += 0.035;
            particle.energy += 0.018;
        }

        if matches!(archetype, Some(Archetype::Architect | Archetype::Leviathan)) {
            particle.mass += 0.002;
        }

        if matches!(archetype, Some(Archetype::Harvester)) {
            particle.genome.perception = (particle.genome.perception + 0.000012).clamp(0.1, 0.38);

            if low_substrate {
                particle.energy -= 0.038;
                particle.health -= 0.024;
                particle.mass -= 0.010;
                particle.genome.fertility = (particle.genome.fertility - 0.00016).clamp(0.2, 2.4);

                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.08);
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.025);
            } else {
                particle.energy -= 0.006;
                particle.health += 0.002;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.035);
            }

            if harvester_overgrowth {
                particle.energy -= 0.020;
                particle.health -= 0.014;
                particle.genome.hunger = (particle.genome.hunger + 0.00007).clamp(0.005, 0.04);

                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.06);
            }
        }

        if matches!(archetype, Some(Archetype::Reaper)) {
            let starvation_relief = if reaper_pressure_needed { 0.48 } else { 1.0 };

            particle.energy -= 0.017 * starvation_relief;
            particle.health -= 0.005 * starvation_relief;
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
            particle.energy -= if low_substrate { 0.030 } else { 0.010 };
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

fn field_polarity_response(
    archetype: Option<Archetype>,
    rare_trait: RareTrait,
    dangerous: bool,
    strength: f32,
    energy: f32,
    health: f32,
) -> (f32, f32, f32) {
    let strength = strength.clamp(0.0, 1.0);
    let vulnerable = energy < 34.0 || health < 36.0;

    let mut pull = if dangerous {
        -0.0036 * strength
    } else {
        0.0024 * strength
    };

    let mut calm = if dangerous { 0.0 } else { 0.0014 * strength };

    let mut hunger = if dangerous { 0.0042 * strength } else { 0.0 };

    match archetype {
        Some(Archetype::Harvester) => {
            pull *= if dangerous { 1.34 } else { 1.18 };
            calm *= 0.82;
            hunger *= 1.12;
        }
        Some(Archetype::Reaper) => {
            pull *= if dangerous { -0.48 } else { 0.42 };
            calm *= 0.42;
            hunger *= 0.55;
        }
        Some(Archetype::Architect | Archetype::Leviathan | Archetype::Mycelial) => {
            pull *= if dangerous { 0.92 } else { 1.46 };
            calm *= 1.62;
            hunger *= 0.72;
        }
        Some(Archetype::Orbiter | Archetype::Phantom) => {
            pull *= if dangerous { 0.72 } else { 1.08 };
            calm *= 0.58;
            hunger *= 0.62;
        }
        Some(Archetype::Hunter | Archetype::Parasite) => {
            pull *= if dangerous { -0.28 } else { 0.74 };
            calm *= 0.72;
            hunger *= 0.86;
        }
        Some(Archetype::Swarmer | Archetype::Grazer) | None => {
            pull *= if dangerous { 1.08 } else { 1.0 };
        }
    }

    if rare_trait == RareTrait::Voidborne {
        pull *= 0.62;
        calm *= 0.48;
        hunger *= 0.52;
    } else if rare_trait == RareTrait::ElderCore || rare_trait == RareTrait::SymbioticCore {
        pull *= if dangerous { 0.86 } else { 1.22 };
        calm *= 1.22;
    } else if rare_trait == RareTrait::Devourer {
        pull *= if dangerous { 1.20 } else { 0.78 };
        hunger *= 1.18;
    }

    if vulnerable && dangerous {
        pull *= 1.38;
        hunger *= 1.22;
    } else if vulnerable && !dangerous {
        pull *= 1.18;
        calm *= 1.16;
    }

    (
        pull.clamp(-0.0058, 0.0052),
        calm.clamp(0.0, 0.0048),
        hunger.clamp(0.0, 0.012),
    )
}

fn matrix_pressure(rules: &RuleMatrix) -> f32 {
    let mut total = 0.0;

    for row in rules {
        for value in row {
            total += value.abs();
        }
    }

    (total / (TRIBE_COUNT * TRIBE_COUNT) as f32).clamp(0.0_f32, 1.0_f32)
}

fn matrix_attraction(rules: &RuleMatrix) -> f32 {
    let mut total = 0.0;

    for row in rules {
        for value in row {
            if *value > 0.0 {
                total += *value;
            }
        }
    }

    (total / (TRIBE_COUNT * TRIBE_COUNT) as f32).clamp(0.0_f32, 1.0_f32)
}

fn matrix_repulsion(rules: &RuleMatrix) -> f32 {
    let mut total = 0.0;

    for row in rules {
        for value in row {
            if *value < 0.0 {
                total += value.abs();
            }
        }
    }

    (total / (TRIBE_COUNT * TRIBE_COUNT) as f32).clamp(0.0_f32, 1.0_f32)
}

fn matrix_pair_pressure(
    attraction: f32,
    a: Tribe,
    b: Tribe,
    pressure: f32,
    attraction_total: f32,
    repulsion_total: f32,
) -> f32 {
    let same_tribe = a.index() == b.index();
    let predator_lane = (a.index() + 1) % TRIBE_COUNT == b.index();

    let base = if attraction >= 0.0 {
        1.0 + attraction.abs() * attraction_total * 0.34
    } else {
        1.0 + attraction.abs() * repulsion_total * 0.42
    };

    let identity = if same_tribe {
        1.0 + pressure * 0.12
    } else if predator_lane {
        1.0 + repulsion_total * 0.18
    } else {
        1.0
    };

    (base * identity).clamp(0.72, 1.58)
}

fn matrix_perception_factor(attraction: f32, pressure: f32, archetype: Option<Archetype>) -> f32 {
    let base = if attraction.abs() > 0.62 {
        1.0 + pressure * 0.10
    } else if attraction.abs() < 0.16 {
        1.0 - pressure * 0.05
    } else {
        1.0
    };

    let archetype_mult = match archetype {
        Some(Archetype::Hunter | Archetype::Reaper | Archetype::Parasite) => 1.0 + pressure * 0.05,
        Some(Archetype::Architect | Archetype::Leviathan | Archetype::Mycelial) => {
            1.0 - pressure * 0.025
        }
        _ => 1.0,
    };

    (base * archetype_mult).clamp(0.84, 1.18)
}

fn apply_signal_field(
    particle: &Particle,
    substrate: &mut CellularAutomata,
    pattern_field: &PatternField,
    archetype: Option<Archetype>,
    low_substrate: bool,
    reaper_pressure_needed: bool,
    fx: &mut f32,
    fy: &mut f32,
) {
    let signal = substrate.signal_at(particle.x, particle.y);

    let mut seek = 0.0;
    let mut avoid = 0.0;

    let field_sample = pattern_field.sample_world(particle.x, particle.y);
    let field_strength = field_sample.influence_strength();

    if field_sample.is_dangerous() {
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Danger,
            0.012 * field_strength.max(0.25),
        );
    } else if field_strength > 0.35 {
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Growth,
            0.010 * field_strength,
        );
    }

    match archetype {
        Some(Archetype::Harvester) => {
            seek += signal.hunger * if low_substrate { 0.62 } else { 0.95 };
            seek += signal.growth * 0.48;

            avoid += signal.fear * 1.10;
            avoid += signal.danger * 0.50;
        }
        Some(Archetype::Reaper) => {
            seek += signal.hunger * if reaper_pressure_needed { 1.38 } else { 0.72 };
            seek += signal.fear * 0.30;

            avoid += signal.growth * 0.12;
            avoid += signal.danger * 0.16;
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

fn apply_root_field(
    particle: &Particle,
    substrate: &CellularAutomata,
    archetype: Option<Archetype>,
    low_substrate: bool,
    harvester_overgrowth: bool,
    fx: &mut f32,
    fy: &mut f32,
) {
    let root_here = substrate.influence_at(particle.x, particle.y) == CellKind::Root;
    let mut push_x = 0.0;
    let mut push_y = 0.0;
    let mut root_pressure = 0.0;
    let mut clear_x = 0.0;
    let mut clear_y = 0.0;
    let mut clear_count = 0.0;

    let probes = [
        (-ROOT_AVOIDANCE_RADIUS, 0.0),
        (ROOT_AVOIDANCE_RADIUS, 0.0),
        (0.0, -ROOT_AVOIDANCE_RADIUS),
        (0.0, ROOT_AVOIDANCE_RADIUS),
        (-ROOT_AVOIDANCE_RADIUS * 0.72, -ROOT_AVOIDANCE_RADIUS * 0.72),
        (ROOT_AVOIDANCE_RADIUS * 0.72, -ROOT_AVOIDANCE_RADIUS * 0.72),
        (-ROOT_AVOIDANCE_RADIUS * 0.72, ROOT_AVOIDANCE_RADIUS * 0.72),
        (ROOT_AVOIDANCE_RADIUS * 0.72, ROOT_AVOIDANCE_RADIUS * 0.72),
    ];

    for (dx, dy) in probes {
        let kind = substrate.influence_at(particle.x + dx, particle.y + dy);

        if kind == CellKind::Root {
            let dist = (dx * dx + dy * dy).sqrt().max(0.001);
            let pressure = (1.0 - dist / (ROOT_AVOIDANCE_RADIUS * 1.25)).max(0.0);

            push_x -= (dx / dist) * pressure;
            push_y -= (dy / dist) * pressure;
            root_pressure += pressure;
        } else if kind == CellKind::Empty || kind == CellKind::Nutrient || kind == CellKind::Life {
            clear_x += dx;
            clear_y += dy;
            clear_count += 1.0;
        }
    }

    if root_here {
        root_pressure += 1.65;
        push_x += (particle.x * 19.0 + particle.y * 7.0).sin() * 0.7;
        push_y += (particle.y * 17.0 - particle.x * 5.0).cos() * 0.7;
    }

    if root_pressure <= 0.0 {
        return;
    }

    let archetype_respect = match archetype {
        Some(Archetype::Architect | Archetype::Leviathan) => 0.82,
        Some(Archetype::Mycelial) => 0.74,
        Some(Archetype::Harvester) => {
            if low_substrate || harvester_overgrowth {
                1.34
            } else {
                1.10
            }
        }
        Some(Archetype::Reaper) => 1.02,
        Some(Archetype::Hunter | Archetype::Parasite) => 1.10,
        Some(Archetype::Phantom) => 0.68,
        Some(Archetype::Grazer) => 1.18,
        Some(Archetype::Swarmer | Archetype::Orbiter) | None => 1.0,
    };

    let mass_resistance = (1.0 / (1.0 + particle.mass * 0.11)).clamp(0.58, 1.0);
    let panic = if particle.health < 28.0 || particle.energy < 22.0 {
        1.22
    } else {
        1.0
    };

    let mut root_force =
        root_pressure * ROOT_FORCE_SCALE * archetype_respect * mass_resistance * panic;

    if particle.rare_trait == RareTrait::Devourer {
        root_force *= 1.16;
    }

    // Root terrain repulsion
    *fx += push_x * root_force;
    *fy += push_y * root_force;

    // Root surface flow (NEW)
    let tangent_x = -push_y;
    let tangent_y = push_x;
    let flow_strength = root_pressure * TreeForces::DEFAULT.surface_flow;

    *fx += tangent_x * flow_strength;
    *fy += tangent_y * flow_strength;

    if clear_count > 0.0 {
        let channel_x = clear_x / clear_count;
        let channel_y = clear_y / clear_count;
        let channel_len = (channel_x * channel_x + channel_y * channel_y).sqrt();

        if channel_len > 0.001 {
            *fx += (channel_x / channel_len) * ROOT_CHANNEL_FORCE * root_pressure;
            *fy += (channel_y / channel_len) * ROOT_CHANNEL_FORCE * root_pressure;
        }
    }
}

fn nudge_from_root(particle: &mut Particle, substrate: &CellularAutomata) {
    let probes = [
        (-0.055, 0.0),
        (0.055, 0.0),
        (0.0, -0.055),
        (0.0, 0.055),
        (-0.04, -0.04),
        (0.04, -0.04),
        (-0.04, 0.04),
        (0.04, 0.04),
    ];

    let mut best = None;

    for (dx, dy) in probes {
        let kind = substrate.influence_at(particle.x + dx, particle.y + dy);

        if kind != CellKind::Root {
            let score = if kind == CellKind::Empty { 2.0 } else { 1.0 };
            best = Some((dx, dy, score));
            break;
        }
    }

    if let Some((dx, dy, score)) = best {
        particle.x = (particle.x + dx * score).clamp(-1.2, 1.2);
        particle.y = (particle.y + dy * score).clamp(-1.2, 1.2);
        particle.vx = (particle.vx + dx * 0.04).clamp(-0.04, 0.04);
        particle.vy = (particle.vy + dy * 0.04).clamp(-0.04, 0.04);
    } else {
        particle.vx = -particle.vx * TreeForces::DEFAULT.bounce;
        particle.vy = -particle.vy * TreeForces::DEFAULT.bounce;
    }

    particle.energy -= 0.012;
}

fn deposit_behavior_signal(
    particle: &Particle,
    substrate: &mut CellularAutomata,
    pattern_field: &PatternField,
    archetype: Option<Archetype>,
    low_substrate: bool,
    harvester_overgrowth: bool,
    reaper_pressure_needed: bool,
) {
    let field_sample = pattern_field.sample_world(particle.x, particle.y);
    let field_strength = field_sample.influence_strength();

    if field_sample.is_dangerous() {
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Danger,
            0.012 * field_strength.max(0.25),
        );
    } else if field_strength > 0.35 {
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Growth,
            0.010 * field_strength,
        );
    }

    match archetype {
        Some(Archetype::Harvester) => {
            substrate.deposit_signal(
                particle.x,
                particle.y,
                SignalKind::Hunger,
                if low_substrate || harvester_overgrowth {
                    0.045
                } else {
                    0.022
                },
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
    pattern_field: &PatternField,
    archetype: Option<Archetype>,
    low_substrate: bool,
    harvester_overgrowth: bool,
) -> usize {
    let kind = substrate.influence_at(particle.x, particle.y);
    let mut consumed = 0usize;

    let field_sample = pattern_field.sample_world(particle.x, particle.y);
    let field_strength = field_sample.influence_strength();

    if field_sample.is_dangerous() {
        particle.energy -= 0.006 * field_strength.max(0.25);
        particle.health -= 0.004 * field_strength.max(0.25);
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Danger,
            0.010 * field_strength.max(0.25),
        );
    } else if field_strength > 0.40 {
        particle.energy += 0.004 * field_strength;
        substrate.deposit_signal(
            particle.x,
            particle.y,
            SignalKind::Growth,
            0.008 * field_strength,
        );
    }

    let is_harvester = matches!(archetype, Some(Archetype::Harvester))
        || particle.rare_trait == RareTrait::Devourer;

    if kind == CellKind::Root {
        let penalty = if is_harvester {
            if low_substrate || harvester_overgrowth {
                0.044
            } else {
                0.026
            }
        } else {
            0.014
        };

        particle.energy -= penalty;
        particle.health -= penalty * 0.35;
        particle.vx *= -0.42;
        particle.vy *= -0.42;

        substrate.deposit_signal(particle.x, particle.y, SignalKind::Danger, 0.038);

        return consumed;
    }

    if is_harvester && kind != CellKind::Empty {
        let protected_regeneration =
            matches!(kind, CellKind::Dead | CellKind::Nutrient | CellKind::Spore);

        if protected_regeneration {
            particle.energy -= if low_substrate { 0.022 } else { 0.010 };

            if low_substrate {
                particle.health -= 0.010;
                substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.045);
            }

            return consumed;
        }

        let power = if particle.rare_trait == RareTrait::Devourer {
            if low_substrate {
                44.0
            } else {
                64.0
            }
        } else if low_substrate {
            30.0
        } else {
            40.0
        };

        let compost = true;

        if let Some(eaten) = substrate.consume_at(particle.x, particle.y, power, compost) {
            let gain = eaten.food_value();

            let gain_mult = if particle.rare_trait == RareTrait::Devourer {
                if low_substrate {
                    0.86
                } else {
                    1.12
                }
            } else if low_substrate {
                0.62
            } else {
                0.84
            };

            particle.energy += gain * gain_mult;
            particle.health += gain * 0.125;
            particle.mass += gain * 0.00175;

            substrate.deposit_signal(particle.x, particle.y, SignalKind::Hunger, 0.18);

            if harvester_overgrowth {
                particle.energy -= 0.016;
                particle.health -= 0.010;
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
        CellKind::Root => {}
        CellKind::Empty => {}
    }

    consumed
}

fn apply_pattern_micro_rules(particle: &mut Particle) {
    let clustered = particle.cluster_id.is_some();
    let rare = !particle.rare_trait.short().is_empty();

    let center = crate::pattern::PatternCell {
        alive: true,
        clustered,
        rare,
        predator: particle.genome.hunger > 0.020 || particle.genome.volatility > 1.46,
        harvester: particle.genome.fertility > 1.34 && particle.genome.hunger < 0.018,
        root: false,
        energy: particle.energy,
        mass: particle.mass,
    };

    let mut neighborhood = crate::pattern::PatternNeighborhood::default();

    let orbit_pressure = particle.genome.orbit.clamp(0.0, 1.55);
    let bonding_pressure = particle.genome.bonding.clamp(0.0, 1.65);
    let membrane_pressure = (particle.genome.membrane / 1.35).clamp(0.0, 1.45);
    let volatility_pressure = particle.genome.volatility.clamp(0.0, 1.95);
    let fertility_pressure = particle.genome.fertility.clamp(0.0, 1.75);

    neighborhood.live_neighbors =
        ((bonding_pressure * 3.0 + membrane_pressure * 2.0 + fertility_pressure) as u8).clamp(0, 8);
    neighborhood.clustered_neighbors = if clustered {
        ((bonding_pressure * 4.0 + membrane_pressure * 2.0) as u8).clamp(1, 8)
    } else {
        ((bonding_pressure * 2.0) as u8).clamp(0, 4)
    };
    neighborhood.rare_neighbors = if rare { 2 } else { 0 };
    neighborhood.predator_neighbors =
        ((volatility_pressure * 2.4 + particle.genome.hunger * 80.0) as u8).clamp(0, 8);
    neighborhood.harvester_neighbors = ((fertility_pressure * 2.6) as u8).clamp(0, 8);
    neighborhood.root_neighbors = 0;
    neighborhood.energy_sum = particle.energy;
    neighborhood.mass_sum = particle.mass;

    let config = crate::pattern::PatternConfig::default();
    let previous_pressure =
        ((orbit_pressure + bonding_pressure + membrane_pressure) / 4.8).clamp(0.0, 1.0);
    let age_seed = particle
        .species_id
        .or(particle.cluster_id)
        .unwrap_or(0)
        .wrapping_add((particle.x.abs() * 1000.0) as u64)
        .wrapping_add((particle.y.abs() * 1000.0) as u64);

    let signature =
        crate::pattern::classify_pattern(age_seed, center, neighborhood, previous_pressure, config);

    let intensity = signature.intensity();
    let role = signature.morphology_role();
    let morphology_pressure = signature.morphology_pressure();
    let pulse = signature.pulse;
    let drift = signature.drift;
    let cohesion = signature.cohesion;

    match signature.kind {
        crate::pattern::PatternKind::StillLife => {
            particle.vx *= 0.996;
            particle.vy *= 0.996;
            particle.health += 0.006 * intensity;
            particle.energy += 0.004 * cohesion;
        }
        crate::pattern::PatternKind::Oscillator => {
            let wave =
                (particle.x * 7.0 + particle.y * 11.0 + age_seed as f32 * 0.013).sin() * 0.00022; // PATTERN_CALM_PASS_ACTIVE
            particle.vx += wave * (0.45 + pulse);
            particle.vy -= wave * (0.35 + pulse);
            particle.energy += 0.002 * intensity;
        }
        crate::pattern::PatternKind::Glider => {
            let angle = particle.genome.orbit * 6.28318 + particle.genome.volatility;
            particle.vx += angle.cos() * 0.00024 * (0.28 + drift * 0.72);
            particle.vy += angle.sin() * 0.00024 * (0.28 + drift * 0.72);
        }
        crate::pattern::PatternKind::Halo => {
            let turn = (particle.x * particle.y * 9.0 + particle.genome.orbit).sin() * 0.00018;
            particle.vx += -particle.y.signum() * turn * (0.6 + cohesion);
            particle.vy += particle.x.signum() * turn * (0.6 + cohesion);
            particle.genome.orbit = (particle.genome.orbit + 0.000012 * intensity).clamp(0.0, 1.55);
        }
        crate::pattern::PatternKind::Lattice => {
            particle.vx *= 0.998;
            particle.vy *= 0.998;
            particle.genome.bonding =
                (particle.genome.bonding + 0.000014 * intensity).clamp(0.0, 1.65);
            particle.genome.membrane =
                (particle.genome.membrane + 0.000018 * intensity).clamp(0.0, 1.55);
        }
        crate::pattern::PatternKind::Bloom => {
            particle.energy += 0.003 * signature.fertility;
            particle.health += 0.0018 * signature.fertility;
            particle.genome.fertility =
                (particle.genome.fertility + 0.000018 * intensity).clamp(0.0, 1.75);
        }
        crate::pattern::PatternKind::Chain => {
            particle.vx += particle.genome.bonding.sin() * 0.00028 * intensity;
            particle.vy += particle.genome.membrane.cos() * 0.00028 * intensity;
            particle.genome.bonding =
                (particle.genome.bonding + 0.00001 * cohesion).clamp(0.0, 1.65);
        }
        crate::pattern::PatternKind::Swarmfront => {
            particle.vx += particle.genome.volatility.cos() * 0.00028 * (0.42 + drift * 0.68);
            particle.vy += particle.genome.volatility.sin() * 0.00028 * (0.42 + drift * 0.68);
            particle.genome.volatility =
                (particle.genome.volatility + 0.000012 * signature.danger).clamp(0.0, 1.95);
        }
        crate::pattern::PatternKind::Nest => {
            particle.vx *= 0.997;
            particle.vy *= 0.997;
            particle.energy += 0.005 * cohesion;
            particle.mass += 0.0009 * intensity;
        }
        crate::pattern::PatternKind::Dormant => {}
    }

    apply_morphology_role_pressure(
        particle,
        role,
        morphology_pressure,
        intensity,
        cohesion,
        drift,
    );

    particle.health = particle.health.clamp(0.0, 140.0);
    particle.energy = particle.energy.clamp(0.0, 160.0);
    particle.mass = particle.mass.clamp(0.12, 18.0);
}
fn apply_morphology_role_pressure(
    particle: &mut Particle,
    role: crate::pattern::MorphologyRole,
    pressure: f32,
    intensity: f32,
    cohesion: f32,
    drift: f32,
) {
    let pressure = pressure.clamp(0.0, 1.0);
    let role_phase = (particle.x * 11.0 + particle.y * 17.0 + particle.genome.orbit * 3.0).sin();

    match role {
        crate::pattern::MorphologyRole::Dormant => {}

        crate::pattern::MorphologyRole::Anchor => {
            let settle = (0.9965 - pressure * 0.0022).clamp(0.992, 0.999);
            particle.vx *= settle;
            particle.vy *= settle;
            particle.health += 0.004 * pressure;
            particle.genome.bonding =
                (particle.genome.bonding + 0.000018 * cohesion).clamp(0.5, 2.25);
        }

        crate::pattern::MorphologyRole::Oscillator => {
            let pulse = role_phase * 0.00032 * (0.35 + pressure);
            particle.vx += pulse;
            particle.vy -= pulse * 0.72;
            particle.energy += 0.0025 * intensity;
            particle.genome.orbit = (particle.genome.orbit + 0.000018 * pressure).clamp(0.0, 1.55);
        }

        crate::pattern::MorphologyRole::Migrator => {
            let angle = particle.genome.orbit * std::f32::consts::TAU + role_phase;
            let migration = 0.00036 * (0.28 + pressure + drift * 0.42);
            particle.vx += angle.cos() * migration;
            particle.vy += angle.sin() * migration;
            particle.genome.volatility =
                (particle.genome.volatility + 0.000016 * pressure).clamp(0.36, 1.95);
        }

        crate::pattern::MorphologyRole::Seeder => {
            particle.energy += 0.0028 * pressure;
            particle.health += 0.0014 * pressure;
            particle.genome.fertility =
                (particle.genome.fertility + 0.000022 * pressure).clamp(0.2, 2.4);
            particle.genome.metabolism =
                (particle.genome.metabolism + 0.000004 * pressure).clamp(0.004, 0.05);
        }

        crate::pattern::MorphologyRole::Membrane => {
            let settle = (0.9975 - cohesion * 0.0018).clamp(0.993, 0.999);
            particle.vx *= settle;
            particle.vy *= settle;
            particle.mass += 0.0008 * pressure;
            particle.genome.membrane =
                (particle.genome.membrane + 0.000026 * pressure).clamp(0.0, 1.8);
            particle.genome.bonding =
                (particle.genome.bonding + 0.000012 * cohesion).clamp(0.5, 2.25);
        }

        crate::pattern::MorphologyRole::PredatorFront => {
            let angle = particle.genome.volatility * std::f32::consts::TAU + role_phase;
            let strike = 0.00042 * (0.35 + pressure + drift * 0.55);
            particle.vx += angle.cos() * strike;
            particle.vy += angle.sin() * strike;
            particle.energy -= 0.0018 * pressure;
            particle.genome.hunger =
                (particle.genome.hunger + 0.000012 * pressure).clamp(0.005, 0.04);
        }
    }
}

// PATTERN_MICRO_RULES_ACTIVE

fn apply_ecology(particle: &mut Particle, ecology: &Ecology) {
    apply_pattern_micro_rules(particle);

    // --- PATTERN MEMORY ---
    // lightweight persistence using genome channels (no struct changes)

    let memory_decay = 0.985;

    particle.genome.orbit *= memory_decay;
    particle.genome.bonding *= memory_decay;
    particle.genome.membrane *= memory_decay;

    // reinforce based on current motion (carry structure forward)
    let motion_strength = (particle.vx.abs() + particle.vy.abs()).clamp(0.0, 1.0);

    particle.genome.orbit += motion_strength * 0.0008;
    particle.genome.bonding += motion_strength * 0.0006;
    particle.genome.membrane += motion_strength * 0.0005;

    // clamp back into safe ranges
    particle.genome.orbit = particle.genome.orbit.clamp(0.0, 1.55);
    particle.genome.bonding = particle.genome.bonding.clamp(0.0, 1.65);
    particle.genome.membrane = particle.genome.membrane.clamp(0.0, 1.55);
    // --- PATTERN MEMORY ACTIVE ---

    for zone in &ecology.zones {
        let dx = zone.x - particle.x;
        let dy = zone.y - particle.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > zone.radius {
            continue;
        }

        let _effect = (1.0 - dist / zone.radius) * zone.strength;

        match zone.kind {
            ZoneKind::Nutrient => {
                particle.health += 0.12 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.energy += 0.08 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.mass += 0.006 * 1.0 /* ROOT_BIAS_DISABLED */;
            }
            ZoneKind::Dead => {
                particle.health -= 0.18 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.energy -= 0.09 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.mass -= 0.006 * 1.0 /* ROOT_BIAS_DISABLED */;
            }
            ZoneKind::Turbulent => {
                particle.vx += (particle.y * 33.0).sin() * 0.001 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.vy -= (particle.x * 29.0).cos() * 0.001 * 1.0 /* ROOT_BIAS_DISABLED */;
            }
            ZoneKind::Mutagen => {
                particle.genome.volatility =
                    (particle.genome.volatility + 0.00045 * 1.0/* ROOT_BIAS_DISABLED */)
                        .clamp(0.36, 1.95);
                particle.genome.orbit =
                    (particle.genome.orbit + 0.0003 * 1.0/* ROOT_BIAS_DISABLED */).clamp(0.0, 1.55);
            }
            ZoneKind::Nest => {
                particle.energy += 0.04 * 1.0 /* ROOT_BIAS_DISABLED */;
                particle.genome.fertility =
                    (particle.genome.fertility + 0.00035 * 1.0/* ROOT_BIAS_DISABLED */)
                        .clamp(0.2, 2.4);
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
    genome.perception = mutate_float(genome.perception, 0.011, 0.1, 0.38, rng);
    genome.hunger = mutate_float(genome.hunger, 0.0022, 0.005, 0.04, rng);
    genome.bonding = mutate_float(genome.bonding, 0.045, 0.5, 2.25, rng);
    genome.volatility = mutate_float(genome.volatility, 0.042, 0.36, 1.95, rng);
    genome.orbit = mutate_float(genome.orbit, 0.04, 0.0, 1.55, rng);
    genome.membrane = mutate_float(genome.membrane, 0.04, 0.0, 1.8, rng);
    genome.metabolism = mutate_float(genome.metabolism, 0.002, 0.004, 0.05, rng);
    genome.fertility = mutate_float(genome.fertility, 0.036, 0.2, 2.4, rng);

    if genome.perception > 0.302 && genome.fertility > 1.36 && genome.hunger < 0.019 {
        genome.hunger = (genome.hunger + rng.gen_range(0.00014..0.00085)).clamp(0.005, 0.04);
        genome.fertility = (genome.fertility - rng.gen_range(0.002..0.011)).clamp(0.2, 2.4);
    }

    genome
}

fn roll_rare_trait(rng: &mut StdRng, genome: Genome, mass: f32) -> RareTrait {
    if genome.perception > 0.33
        && genome.fertility > 1.68
        && genome.hunger < 0.016
        && genome.metabolism < 0.021
        && rng.gen_bool(0.18)
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
