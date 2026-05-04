use crate::{
    app::{Environment, TRIBE_COUNT},
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

pub fn build_rule_matrix(seed: u64) -> RuleMatrix {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut matrix = [[0.0; TRIBE_COUNT]; TRIBE_COUNT];

    for a in 0..TRIBE_COUNT {
        for b in 0..TRIBE_COUNT {
            matrix[a][b] = rng.gen_range(-1.0..1.0);
        }
    }

    matrix
}

pub fn mutate_rules(rules: &mut RuleMatrix, seed: u64, intensity: f32) {
    let mut rng = StdRng::seed_from_u64(seed);

    for a in 0..TRIBE_COUNT {
        for b in 0..TRIBE_COUNT {
            if rng.gen_bool(0.18) {
                rules[a][b] = (rules[a][b] + rng.gen_range(-intensity..intensity)).clamp(-1.0, 1.0);
            }
        }
    }
}

pub fn step_particles(
    particles: &mut [Particle],
    rules: &RuleMatrix,
    env: Environment,
    ecology: &Ecology,
    archetypes: &[Option<Archetype>],
) {
    let snapshot = particles.to_vec();

    for p in particles.iter_mut() {
        let mut fx = 0.0;
        let mut fy = 0.0;

        let mut local_density = 0usize;
        let mut friendly_density = 0usize;
        let mut hostile_density = 0usize;

        let mut vx_avg = 0.0;
        let mut vy_avg = 0.0;

        let mut orbit_x = 0.0;
        let mut orbit_y = 0.0;

        let archetype = p
            .species_id
            .and_then(|id| archetypes.get(id as usize).copied().flatten());

        for other in &snapshot {
            let dx = other.x - p.x;
            let dy = other.y - p.y;
            let d2 = dx * dx + dy * dy;

            if d2 <= 0.000001 {
                continue;
            }

            let d = d2.sqrt();
            let attraction = rules[p.tribe.index()][other.tribe.index()];
            let predator_pressure = predator_factor(p.tribe, other.tribe, archetype);

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
                    Some(Archetype::Parasite) => 0.82,
                    _ => 1.0,
                };

                let bond = (1.0 - d / BOND_RADIUS) * p.genome.bonding * bond_mult;

                fx += dx * bond * 0.62;
                fy += dy * bond * 0.62;

                orbit_x += -dy / d;
                orbit_y += dx / d;
            }

            let mut perception = p.genome.perception * env.perception_mult();

            if matches!(archetype, Some(Archetype::Grazer | Archetype::Hunter)) {
                perception *= 1.15;
            }

            if d > perception {
                continue;
            }

            let mut force = if d < MIN_DISTANCE {
                -1.7
            } else {
                attraction * predator_pressure * (1.0 - d / perception)
            };

            force *= p.genome.volatility;
            force *= env.force_mult();

            if matches!(archetype, Some(Archetype::Hunter)) {
                force *= 1.18;
            }

            fx += (dx / d) * force;
            fy += (dy / d) * force;

            if matches!(archetype, Some(Archetype::Hunter)) && predator_pressure > 1.1 && d < perception * 0.45 {
                p.energy += 0.018;
                p.health += 0.012;
            }

            if matches!(archetype, Some(Archetype::Parasite)) && d < BOND_RADIUS && other.mass > p.mass {
                p.energy += 0.012;
            }
        }

        if local_density > 0 {
            vx_avg /= local_density as f32;
            vy_avg /= local_density as f32;

            p.vx += (vx_avg - p.vx) * 0.22;
            p.vy += (vy_avg - p.vy) * 0.22;

            let orbit_boost = if matches!(archetype, Some(Archetype::Orbiter)) {
                1.75
            } else {
                1.0
            };

            p.vx += orbit_x * p.genome.orbit * orbit_boost * 0.00042;
            p.vy += orbit_y * p.genome.orbit * orbit_boost * 0.00042;
        }

        apply_ecology(p, ecology);

        let mass_drag = (1.0 + p.mass * 0.13).clamp(1.0, 2.0);

        p.vx = (p.vx + fx * FORCE_SCALE) * FRICTION / mass_drag;
        p.vy = (p.vy + fy * FORCE_SCALE) * FRICTION / mass_drag;

        apply_environment_current(p, env);

        if p.x < -1.0 {
            p.vx += WALL_FORCE;
        }

        if p.x > 1.0 {
            p.vx -= WALL_FORCE;
        }

        if p.y < -1.0 {
            p.vy += WALL_FORCE;
        }

        if p.y > 1.0 {
            p.vy -= WALL_FORCE;
        }

        p.x = (p.x + p.vx).clamp(-1.2, 1.2);
        p.y = (p.y + p.vy).clamp(-1.2, 1.2);

        if friendly_density >= 4 {
            p.health += 0.14;
            p.energy += 0.028;
            p.mass += 0.014;
        }

        if hostile_density >= 3 {
            p.health -= 0.21;
            p.energy -= 0.026;
            p.mass -= 0.012;
        }

        if local_density == 0 {
            p.health -= 0.07;
            p.energy -= 0.04;
            p.mass -= 0.008;
        }

        if p.cluster_id.is_some() {
            p.health += 0.035;
            p.energy += 0.012;
        }

        if env == Environment::Bloom {
            p.health += 0.035;
            p.energy += 0.018;
        }

        if matches!(archetype, Some(Archetype::Architect | Archetype::Leviathan)) {
            p.mass += 0.002;
        }

        if p.rare_trait == RareTrait::Radiant {
            p.energy += 0.015;
        }

        if p.rare_trait == RareTrait::Voracious {
            p.energy -= 0.01;
            p.health += 0.008;
        }

        p.energy -= p.genome.metabolism * env.hunger_mult();
        p.health -= p.genome.hunger * env.hunger_mult();

        p.energy = p.energy.clamp(0.0, 160.0);
        p.health = p.health.clamp(0.0, 100.0);
        p.mass = p.mass.clamp(0.45, 7.0);
        p.age = p.age.saturating_add(1);
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

    if matches!(archetype, Some(Archetype::Hunter)) {
        base * 1.24
    } else if matches!(archetype, Some(Archetype::Grazer)) {
        base * 0.86
    } else {
        base
    }
}

fn apply_ecology(p: &mut Particle, ecology: &Ecology) {
    for zone in &ecology.zones {
        let dx = zone.x - p.x;
        let dy = zone.y - p.y;
        let dist = (dx * dx + dy * dy).sqrt();

        if dist > zone.radius {
            continue;
        }

        let effect = (1.0 - dist / zone.radius) * zone.strength;

        match zone.kind {
            ZoneKind::Nutrient => {
                p.health += 0.12 * effect;
                p.energy += 0.08 * effect;
                p.mass += 0.006 * effect;
            }
            ZoneKind::Dead => {
                p.health -= 0.18 * effect;
                p.energy -= 0.09 * effect;
                p.mass -= 0.006 * effect;
            }
            ZoneKind::Turbulent => {
                p.vx += (p.y * 33.0).sin() * 0.001 * effect;
                p.vy -= (p.x * 29.0).cos() * 0.001 * effect;
            }
            ZoneKind::Mutagen => {
                p.genome.volatility = (p.genome.volatility + 0.00045 * effect).clamp(0.36, 1.95);
                p.genome.orbit = (p.genome.orbit + 0.0003 * effect).clamp(0.0, 1.55);
            }
            ZoneKind::Nest => {
                p.energy += 0.04 * effect;
                p.genome.fertility = (p.genome.fertility + 0.00035 * effect).clamp(0.2, 2.4);
            }
        }
    }
}

fn apply_environment_current(p: &mut Particle, env: Environment) {
    match env {
        Environment::Calm => {}
        Environment::Bloom => {
            p.vx *= 0.998;
            p.vy *= 0.998;
        }
        Environment::Hunger => {
            p.vx *= 1.006;
            p.vy *= 1.006;
        }
        Environment::Storm => {
            let phase = ((p.x * 22.0 + p.y * 31.0).sin()) * 0.00105;
            p.vx += phase;
            p.vy -= phase;
        }
        Environment::Drift => {
            p.vx += 0.0002;
            p.vy += 0.00007;
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

    if rng.gen_bool(0.0015) {
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

    if rng.gen_bool(0.003) {
        child.rare_trait = roll_rare_trait(&mut rng, child.genome, child.mass);
    } else {
        child.rare_trait = if rng.gen_bool(0.5) { a.rare_trait } else { b.rare_trait };
    }

    child
}

pub fn mutate_genome(mut genome: Genome, rng: &mut StdRng) -> Genome {
    genome.perception = mutate_float(genome.perception, 0.012, 0.1, 0.38, rng);
    genome.hunger = mutate_float(genome.hunger, 0.002, 0.005, 0.04, rng);
    genome.bonding = mutate_float(genome.bonding, 0.045, 0.5, 2.25, rng);
    genome.volatility = mutate_float(genome.volatility, 0.04, 0.36, 1.95, rng);
    genome.orbit = mutate_float(genome.orbit, 0.04, 0.0, 1.55, rng);
    genome.membrane = mutate_float(genome.membrane, 0.04, 0.0, 1.8, rng);
    genome.metabolism = mutate_float(genome.metabolism, 0.002, 0.004, 0.05, rng);
    genome.fertility = mutate_float(genome.fertility, 0.04, 0.2, 2.4, rng);
    genome
}

fn roll_rare_trait(rng: &mut StdRng, genome: Genome, mass: f32) -> RareTrait {
    if mass > 5.6 && genome.membrane > 1.1 {
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
