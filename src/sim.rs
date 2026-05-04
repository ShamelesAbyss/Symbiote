use crate::{
    app::{Environment, TRIBE_COUNT},
    ecology::{Ecology, ZoneKind},
    particle::{Genome, Particle, Tribe},
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
            if rng.gen_bool(0.24) {
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

            if matches!(archetype, Some(Archetype::Grazer)) {
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
            p.mass += 0.014;
        }

        if hostile_density >= 3 {
            p.health -= 0.21;
            p.mass -= 0.012;
        }

        if local_density == 0 {
            p.health -= 0.09;
            p.mass -= 0.009;
        }

        if p.cluster_id.is_some() {
            p.health += 0.035;
        }

        if env == Environment::Bloom {
            p.health += 0.035;
        }

        if matches!(archetype, Some(Archetype::Architect)) {
            p.mass += 0.002;
        }

        p.health -= p.genome.hunger * env.hunger_mult();
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
                p.mass += 0.006 * effect;
            }
            ZoneKind::Dead => {
                p.health -= 0.18 * effect;
                p.mass -= 0.006 * effect;
            }
            ZoneKind::Turbulent => {
                p.vx += (p.y * 33.0).sin() * 0.001 * effect;
                p.vy -= (p.x * 29.0).cos() * 0.001 * effect;
            }
            ZoneKind::Mutagen => {
                p.genome.volatility = (p.genome.volatility + 0.0009 * effect).clamp(0.36, 1.95);
                p.genome.orbit = (p.genome.orbit + 0.0006 * effect).clamp(0.0, 1.55);
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
    child.x += rng.gen_range(-0.038..0.038);
    child.y += rng.gen_range(-0.038..0.038);
    child.vx = rng.gen_range(-0.006..0.006);
    child.vy = rng.gen_range(-0.006..0.006);
    child.age = 0;
    child.health = 72.0;
    child.mass = (parent.mass * 0.68).clamp(0.45, 3.2);
    child.cluster_id = None;
    child.genome = mutate_genome(parent.genome, &mut rng);

    if rng.gen_bool(0.022) {
        child.tribe = Tribe::from_index(rng.gen_range(0..TRIBE_COUNT));
        child.species_id = None;
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
    genome
}

fn mutate_float(value: f32, amount: f32, min: f32, max: f32, rng: &mut StdRng) -> f32 {
    (value + rng.gen_range(-amount..amount)).clamp(min, max)
}
