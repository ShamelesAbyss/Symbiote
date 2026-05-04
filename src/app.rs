use crate::{
    automata::CellularAutomata,
    cluster::{ClusterEvents, ClusterTracker},
    ecology::Ecology,
    memory::MemoryBank,
    particle::{Genome, Particle, RareTrait, Tribe},
    sim::{build_rule_matrix, child_from, fused_child, mutate_rules, step_particles, RuleMatrix},
    species::{Archetype, SpeciesBank},
};

use rand::{rngs::StdRng, Rng, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{collections::VecDeque, fs, path::Path};

const ECOSYSTEM_PATH: &str = "memory/ecosystem_state.json";

pub const TRIBE_COUNT: usize = 6;
pub const PARTICLE_COUNT: usize = 1200;
pub const MAX_PARTICLES: usize = 2500;
pub const MIN_PARTICLES: usize = 600;

const DISPERSAL_WARMUP_TICKS: u64 = 720;
const REPRODUCTION_WARMUP_TICKS: u64 = 420;
const STRUCTURE_WARMUP_TICKS: u64 = 900;

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Environment {
    Calm,
    Bloom,
    Hunger,
    Storm,
    Drift,
}

impl Environment {
    pub fn name(self) -> &'static str {
        match self {
            Self::Calm => "calm",
            Self::Bloom => "bloom",
            Self::Hunger => "hunger",
            Self::Storm => "storm",
            Self::Drift => "drift",
        }
    }

    pub fn force_mult(self) -> f32 {
        match self {
            Self::Calm => 1.0,
            Self::Bloom => 0.92,
            Self::Hunger => 1.25,
            Self::Storm => 1.45,
            Self::Drift => 0.86,
        }
    }

    pub fn hunger_mult(self) -> f32 {
        match self {
            Self::Calm => 1.0,
            Self::Bloom => 0.5,
            Self::Hunger => 1.9,
            Self::Storm => 1.18,
            Self::Drift => 0.92,
        }
    }

    pub fn perception_mult(self) -> f32 {
        match self {
            Self::Calm => 1.0,
            Self::Bloom => 1.18,
            Self::Hunger => 0.9,
            Self::Storm => 0.82,
            Self::Drift => 1.28,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct EcosystemState {
    particles: Vec<Particle>,
    rules: RuleMatrix,
    clusters: ClusterTracker,
    species_bank: SpeciesBank,
    ecology: Ecology,
    substrate: CellularAutomata,
    memory: MemoryBank,
    seed: u64,
    age: u64,
    generation: u64,
    tick_ms: u64,
    environment: Environment,
    events: Vec<String>,
}

pub struct App {
    pub particles: Vec<Particle>,
    pub rules: RuleMatrix,
    pub clusters: ClusterTracker,
    pub species_bank: SpeciesBank,
    pub ecology: Ecology,
    pub substrate: CellularAutomata,
    pub memory: MemoryBank,
    pub seed: u64,
    pub age: u64,
    pub generation: u64,
    pub paused: bool,
    pub tick_ms: u64,
    pub energy: f32,
    pub cohesion: f32,
    pub chaos: f32,
    pub drift: f32,
    pub population: f32,
    pub matrix_attraction: f32,
    pub matrix_repulsion: f32,
    pub matrix_pressure: f32,
    pub environment: Environment,
    pub events: VecDeque<String>,
}

impl App {
    pub fn new() -> Self {
        if let Some(mut restored) = Self::load_ecosystem() {
            restored.paused = false;
            restored.measure();
            restored.measure_matrix();
            restored
                .memory
                .note("restored ecosystem with root memory online".to_string());
            restored.push_event("ecosystem restored from persistent state");
            return restored;
        }

        let seed = now_seed();

        let mut app = Self {
            particles: Vec::new(),
            rules: build_rule_matrix(seed),
            clusters: ClusterTracker::new(),
            species_bank: SpeciesBank::new(),
            ecology: Ecology::new(seed),
            substrate: CellularAutomata::new(seed ^ 0xC011, 96, 48),
            memory: MemoryBank::load_or_new(seed),
            seed,
            age: 0,
            generation: 0,
            paused: false,
            tick_ms: 90,
            energy: 0.0,
            cohesion: 0.0,
            chaos: 0.0,
            drift: 0.0,
            population: 0.0,
            matrix_attraction: 0.0,
            matrix_repulsion: 0.0,
            matrix_pressure: 0.0,
            environment: Environment::Calm,
            events: VecDeque::new(),
        };

        app.reset_particles();
        app.push_event("regenerative substrate online");
        app.push_event("reaper ecology online");
        app.push_event("adaptive ecosystem memory online");
        app.push_event("adaptive attraction matrix online");
        app.push_event("root navigation memory online");
        app.push_event("early dispersal warmup online");

        app
    }

    pub fn step(&mut self) {
        let archetype_lookup = self.build_archetype_lookup();

        let report = step_particles(
            &mut self.particles,
            &self.rules,
            self.environment,
            &self.ecology,
            &mut self.substrate,
            &archetype_lookup,
        );

        if self.age < DISPERSAL_WARMUP_TICKS {
            self.apply_early_dispersal();
        }

        if report.cells_consumed > 0 {
            self.memory.total_cells_consumed += report.cells_consumed as u64;

            if report.cells_consumed >= 12 && self.age % 120 == 0 {
                self.push_event(&format!(
                    "harvesters consumed {} substrate cells",
                    report.cells_consumed
                ));
            }
        }

        if report.harvesters_consumed > 0 {
            self.memory.total_harvesters_consumed += report.harvesters_consumed as u64;

            self.push_event(&format!(
                "reaper consumed {} harvester(s)",
                report.harvesters_consumed
            ));
        }

        if report.harvester_particles >= 15 && self.age % 180 == 0 {
            self.push_event(&format!(
                "harvester pressure rising: {} bodies",
                report.harvester_particles
            ));
        }

        if report.reaper_particles > 0 && self.age % 240 == 0 {
            self.push_event(&format!(
                "reaper pressure active: {}",
                report.reaper_particles
            ));
        }

        if self.age > 0 && self.age % 720 == 0 {
            self.push_event(&self.adaptive_summary());
        }

        self.age += 1;

        self.ecology.tick(self.seed, self.age, self.environment);

        if self.age % 8 == 0 {
            self.deposit_to_substrate();
        }

        if self.age % 12 == 0 {
            self.substrate.tick();
        }

        if self.memory.substrate_recovery_bias() > 0.52 && self.age % 48 == 0 {
            self.substrate.tick();
        }

        if self.memory.substrate_throttle_pressure() > 0.62 && self.age % 36 == 0 {
            self.thin_overgrown_substrate_pressure();
        }

        if self.age >= REPRODUCTION_WARMUP_TICKS && self.age % 32 == 0 {
            self.native_reproduction();
        }

        if self.age % 24 == 0 {
            let before_species = self.species_bank.species.len();

            let cluster_events =
                self.clusters
                    .update(&mut self.particles, &mut self.species_bank, self.age);

            let after_species = self.species_bank.species.len();

            if after_species > before_species {
                self.memory.total_species_created += (after_species - before_species) as u64;

                self.push_event(&format!(
                    "{} new species emerged",
                    after_species - before_species
                ));
            }

            self.process_cluster_events(cluster_events);
        }

        if self.age % 60 == 0 && self.age < STRUCTURE_WARMUP_TICKS {
            self.apply_migration_pressure();
        } else if self.age % 90 == 0 {
            self.apply_migration_pressure();
        }

        if self.age >= STRUCTURE_WARMUP_TICKS && self.age % 120 == 0 {
            self.reinforce_matrix_from_clusters();
        }

        if self.age % 360 == 0 {
            self.shift_environment();
        }

        if self.age % 280 == 0 {
            let base_intensity = match self.environment {
                Environment::Calm => 0.018,
                Environment::Bloom => 0.014,
                Environment::Hunger => 0.035,
                Environment::Storm => 0.052,
                Environment::Drift => 0.022,
            };

            let adaptive_mutation = 1.0 + self.memory.mutation_pressure() * 0.42;
            let pressure_mutation = 1.0 + (self.matrix_pressure / 100.0) * 0.28;
            let pathfinder_mutation = 1.0 + self.memory.pathfinder_bias() * 0.22;

            let intensity =
                (base_intensity * adaptive_mutation * pressure_mutation * pathfinder_mutation)
                    .clamp(0.008, 0.082);

            mutate_rules(&mut self.rules, self.seed ^ self.age, intensity);
            self.evolve_attraction_matrix(intensity);

            self.generation += 1;

            self.push_event(&format!(
                "matrix evolved pressure:{:.0} attr:{:.0} rep:{:.0}",
                self.matrix_pressure, self.matrix_attraction, self.matrix_repulsion
            ));
        }

        self.measure();
        self.measure_matrix();
        self.update_memory();

        if self.age % 600 == 0 {
            self.save_all();
        }
    }

    fn deposit_to_substrate(&mut self) {
        let lookup = self.build_archetype_lookup();

        let throttle = self.memory.substrate_throttle_pressure();
        let step = if throttle > 0.72 {
            17
        } else if throttle > 0.48 {
            13
        } else {
            9
        };

        for particle in self.particles.iter().step_by(step) {
            let archetype = particle
                .species_id
                .and_then(|id| lookup.get(id as usize).copied().flatten());

            self.substrate.deposit_particle(particle, archetype);
        }
    }

    fn native_reproduction(&mut self) {
        if self.particles.len() >= MAX_PARTICLES {
            return;
        }

        let mut rng = StdRng::seed_from_u64(self.seed ^ self.age ^ self.generation);
        let snapshot = self.particles.clone();
        let archetype_lookup = self.build_archetype_lookup();

        let mut children = Vec::new();

        let substrate_density = if self.substrate.total_cells() == 0 {
            0.0
        } else {
            self.substrate.living_cells() as f32 / self.substrate.total_cells() as f32
        };

        let root_density = if self.substrate.total_cells() == 0 {
            0.0
        } else {
            self.substrate.protected_cells() as f32 / self.substrate.total_cells() as f32
        };

        let active_harvester_particles = snapshot
            .iter()
            .filter(|particle| {
                particle.rare_trait == RareTrait::Devourer
                    || particle
                        .species_id
                        .and_then(|id| archetype_lookup.get(id as usize).copied().flatten())
                        == Some(Archetype::Harvester)
            })
            .count();

        let harvester_body_ratio = active_harvester_particles as f32 / snapshot.len().max(1) as f32;
        let harvester_resistance = self.memory.harvester_resistance();
        let reaper_urgency = self.memory.reaper_urgency();
        let recovery_bias = self.memory.substrate_recovery_bias();
        let mutation_pressure = self.memory.mutation_pressure();
        let pathfinder_bias = self.memory.pathfinder_bias();
        let corridor_pressure = self.memory.corridor_pressure();

        let structure_maturity = ((self.age.saturating_sub(REPRODUCTION_WARMUP_TICKS)) as f32
            / (STRUCTURE_WARMUP_TICKS - REPRODUCTION_WARMUP_TICKS).max(1) as f32)
            .clamp(0.0, 1.0);

        for parent in snapshot.iter() {
            if self.particles.len() + children.len() >= MAX_PARTICLES {
                break;
            }

            let clustered_bonus = if parent.cluster_id.is_some() {
                0.18 * structure_maturity
            } else {
                0.0
            };
            let rare_bonus = if parent.rare_trait != RareTrait::None {
                0.12
            } else {
                0.0
            };
            let adaptive_fertility_drag = harvester_resistance * 4.5;

            let threshold = 118.0 - parent.genome.fertility * 11.5 - clustered_bonus * 14.0
                + adaptive_fertility_drag;

            if parent.energy < threshold || parent.health < 48.0 || parent.age < 220 {
                continue;
            }

            let chance = (0.012
                + parent.genome.fertility * 0.014
                + clustered_bonus
                + rare_bonus
                + mutation_pressure * 0.012
                + pathfinder_bias * 0.006)
                .clamp(0.006, 0.30);

            if !rng.gen_bool(chance as f64) {
                continue;
            }

            let maybe_partner = snapshot
                .iter()
                .find(|other| {
                    other.species_id != parent.species_id
                        && other.energy > 95.0
                        && dist(parent.x, parent.y, other.x, other.y)
                            < 0.11 + structure_maturity * 0.03
                })
                .copied();

            let mut child = if let Some(partner) = maybe_partner {
                self.memory.total_fusions += 1;
                fused_child(*parent, partner, rng.gen())
            } else {
                child_from(*parent, rng.gen())
            };

            if substrate_density > 0.09
                && rng.gen_bool(
                    (substrate_density * (0.95 - harvester_resistance * 0.45)).clamp(0.01, 0.12)
                        as f64,
                )
            {
                child.genome.perception =
                    (child.genome.perception + rng.gen_range(0.004..0.024)).clamp(0.1, 0.38);
                child.genome.fertility =
                    (child.genome.fertility + rng.gen_range(0.018..0.095)).clamp(0.2, 2.4);
                child.genome.hunger =
                    (child.genome.hunger + rng.gen_range(-0.0002..0.0012)).clamp(0.005, 0.04);
                child.genome.metabolism =
                    (child.genome.metabolism + rng.gen_range(-0.0002..0.0011)).clamp(0.004, 0.05);
                child.species_id = None;
            }

            if root_density > 0.035
                && rng.gen_bool((pathfinder_bias * 0.08).clamp(0.01, 0.18) as f64)
            {
                child.genome.perception =
                    (child.genome.perception + rng.gen_range(0.012..0.04)).clamp(0.1, 0.38);
                child.genome.orbit =
                    (child.genome.orbit + rng.gen_range(0.04..0.20)).clamp(0.0, 1.55);
                child.genome.volatility =
                    (child.genome.volatility + rng.gen_range(0.015..0.11)).clamp(0.36, 1.95);
                child.genome.bonding =
                    (child.genome.bonding - rng.gen_range(0.01..0.12)).clamp(0.5, 2.25);
                child.species_id = None;
            }

            if corridor_pressure > 0.38 && rng.gen_bool((corridor_pressure * 0.05) as f64) {
                child.x = (child.x + rng.gen_range(-0.18..0.18)).clamp(-1.16, 1.16);
                child.y = (child.y + rng.gen_range(-0.18..0.18)).clamp(-1.16, 1.16);
                child.vx += rng.gen_range(-0.01..0.01);
                child.vy += rng.gen_range(-0.01..0.01);
            }

            if child.genome.perception > 0.295
                && child.genome.fertility > 1.35
                && child.genome.hunger < 0.018
            {
                child.genome.hunger =
                    (child.genome.hunger + rng.gen_range(0.0008..0.0022)).clamp(0.005, 0.04);
                child.genome.fertility =
                    (child.genome.fertility - rng.gen_range(0.018..0.05)).clamp(0.2, 2.4);
                child.genome.metabolism =
                    (child.genome.metabolism + rng.gen_range(0.0003..0.0012)).clamp(0.004, 0.05);
                child.species_id = None;
            }

            let reaper_trigger = active_harvester_particles >= 15
                || harvester_body_ratio > 0.11
                || (active_harvester_particles >= 10 && reaper_urgency > 0.55);

            let reaper_chance =
                (0.15 + reaper_urgency * 0.18 + recovery_bias * 0.06).clamp(0.08, 0.38);

            if reaper_trigger
                && substrate_density < (0.08 + recovery_bias * 0.025)
                && rng.gen_bool(reaper_chance as f64)
            {
                child.genome.volatility = rng.gen_range(1.70..1.92);
                child.genome.perception = rng.gen_range(0.305..0.38);
                child.genome.hunger = rng.gen_range(0.021..0.036);
                child.genome.fertility = rng.gen_range(0.50..1.18);
                child.genome.bonding = rng.gen_range(0.55..1.15);
                child.mass = (child.mass + rng.gen_range(0.35..1.2)).clamp(0.45, 7.0);
                child.rare_trait = RareTrait::None;
                child.species_id = None;
            }

            if mutation_pressure > 0.45 && rng.gen_bool((mutation_pressure * 0.035) as f64) {
                child.genome.volatility =
                    (child.genome.volatility + rng.gen_range(0.04..0.16)).clamp(0.36, 1.95);
                child.genome.orbit =
                    (child.genome.orbit + rng.gen_range(-0.06..0.12)).clamp(0.0, 1.55);
                child.genome.membrane =
                    (child.genome.membrane + rng.gen_range(0.02..0.14)).clamp(0.0, 1.8);
                child.species_id = None;
            }

            self.species_bank.record_birth(parent.species_id);
            children.push(child);
        }

        if !children.is_empty() {
            let amount = children.len();

            for child in children {
                self.particles.push(child);
            }

            self.memory.total_reproductions += amount as u64;

            if amount > 4 {
                self.push_event(&format!("native reproduction bloom: {}", amount));
            }
        }

        self.apply_selection_pressure(&mut rng);
    }

    fn apply_selection_pressure(&mut self, rng: &mut StdRng) {
        let before = self.particles.len();
        let adaptive_stability = 1.0 - self.memory.mutation_pressure() * 0.08;
        let pathfinder_relief = self.memory.pathfinder_bias() * 0.035;

        self.particles.retain(|particle| {
            let old_age_pressure = if particle.age > 18_000 { 0.08 } else { 0.0 };
            let energy_score = (particle.energy / 130.0).clamp(0.0, 1.0);
            let health_score = (particle.health / 100.0).clamp(0.0, 1.0);
            let clustered_bonus = if particle.cluster_id.is_some() {
                if u64::from(particle.age) < STRUCTURE_WARMUP_TICKS {
                    0.03
                } else {
                    0.18
                }
            } else {
                0.0
            };
            let rare_bonus = if particle.rare_trait != RareTrait::None {
                0.04
            } else {
                0.0
            };
            let edge_explorer_bonus = if particle.x.abs() > 0.82 || particle.y.abs() > 0.82 {
                pathfinder_relief
            } else {
                0.0
            };

            let survival = (energy_score * 0.44
                + health_score * 0.44
                + clustered_bonus
                + rare_bonus
                + edge_explorer_bonus
                - old_age_pressure)
                .clamp(0.02, 0.995)
                * adaptive_stability.clamp(0.92, 1.0);

            particle.energy > 0.0 && particle.health > 0.0 && rng.gen_bool(survival as f64)
        });

        while self.particles.len() < MIN_PARTICLES {
            self.particles.push(random_particle(rng));
        }

        let after = self.particles.len();

        if after < before {
            self.memory.total_deaths += (before - after) as u64;
        }
    }

    fn apply_early_dispersal(&mut self) {
        if self.particles.is_empty() {
            return;
        }

        let warmup_left = 1.0 - (self.age as f32 / DISPERSAL_WARMUP_TICKS as f32).clamp(0.0, 1.0);

        let mut cx = 0.0;
        let mut cy = 0.0;

        for particle in &self.particles {
            cx += particle.x;
            cy += particle.y;
        }

        cx /= self.particles.len() as f32;
        cy /= self.particles.len() as f32;

        for particle in &mut self.particles {
            let dx = particle.x - cx;
            let dy = particle.y - cy;
            let len = (dx * dx + dy * dy).sqrt().max(0.001);

            let center_pullout = if particle.x.abs() < 0.72 && particle.y.abs() < 0.72 {
                1.0
            } else {
                0.35
            };

            let jitter_x =
                (self.seed as f32 * 0.000001 + particle.y * 19.0 + self.age as f32 * 0.021).sin();
            let jitter_y =
                (self.seed as f32 * 0.000002 + particle.x * 23.0 - self.age as f32 * 0.017).cos();

            let force = warmup_left * center_pullout * 0.0065;

            particle.vx += (dx / len) * force + jitter_x * force * 0.55;
            particle.vy += (dy / len) * force + jitter_y * force * 0.55;

            particle.genome.bonding =
                (particle.genome.bonding - warmup_left * 0.00022).clamp(0.5, 2.25);
            particle.genome.orbit =
                (particle.genome.orbit + warmup_left * 0.00010).clamp(0.0, 1.55);
        }
    }

    fn apply_migration_pressure(&mut self) {
        if self.particles.is_empty() {
            return;
        }

        let mut cx = 0.0;
        let mut cy = 0.0;

        for particle in &self.particles {
            cx += particle.x;
            cy += particle.y;
        }

        cx /= self.particles.len() as f32;
        cy /= self.particles.len() as f32;

        let mut avg_radius = 0.0;

        for particle in &self.particles {
            avg_radius += (particle.x * particle.x + particle.y * particle.y).sqrt();
        }

        avg_radius /= self.particles.len() as f32;

        let root_bias = self.memory.pathfinder_bias();
        let crowding_bias = if avg_radius < 0.56 { 1.0 } else { 0.0 };
        let early_bias = if self.age < STRUCTURE_WARMUP_TICKS {
            1.0
        } else {
            0.0
        };
        let migration_strength =
            (0.0025 + root_bias * 0.0075 + crowding_bias * 0.006 + early_bias * 0.006)
                .clamp(0.0, 0.020);

        if migration_strength <= 0.0 {
            return;
        }

        for particle in &mut self.particles {
            let away_x = particle.x - cx;
            let away_y = particle.y - cy;
            let len = (away_x * away_x + away_y * away_y).sqrt().max(0.001);

            let edge_room = (1.18 - particle.x.abs().max(particle.y.abs())).clamp(0.0, 1.0);
            let local_push = migration_strength * edge_room;

            particle.vx += (away_x / len) * local_push;
            particle.vy += (away_y / len) * local_push;

            if particle.x.abs() < 0.34 && particle.y.abs() < 0.34 {
                particle.vx += (particle.x * 13.0 + self.age as f32 * 0.003).sin() * local_push;
                particle.vy += (particle.y * 17.0 - self.age as f32 * 0.004).cos() * local_push;
            }
        }

        if self.age % 450 == 0 {
            self.push_event("migration pressure loosened center swarm");
        }
    }

    fn thin_overgrown_substrate_pressure(&mut self) {
        self.memory.note(format!(
            "[{}] substrate throttle pressure engaged",
            self.age
        ));
    }

    fn evolve_attraction_matrix(&mut self, intensity: f32) {
        let harvester_resistance = self.memory.harvester_resistance();
        let reaper_urgency = self.memory.reaper_urgency();
        let recovery_bias = self.memory.substrate_recovery_bias();
        let mutation_pressure = self.memory.mutation_pressure();
        let pathfinder_bias = self.memory.pathfinder_bias();
        let corridor_pressure = self.memory.corridor_pressure();

        for a in 0..TRIBE_COUNT {
            for b in 0..TRIBE_COUNT {
                let value = self.rules[a][b];

                let stabilizer = if a == b {
                    0.006 + recovery_bias * 0.01
                } else {
                    0.0
                };

                let predator_target = (a + 1) % TRIBE_COUNT == b;
                let prey_target = (b + 1) % TRIBE_COUNT == a;

                let predator_bias = if predator_target {
                    -(0.006 + reaper_urgency * 0.018 + mutation_pressure * 0.008)
                } else if prey_target {
                    0.004 + recovery_bias * 0.01
                } else {
                    0.0
                };

                let overgrowth_dampener = if harvester_resistance > 0.42 && value > 0.45 {
                    -harvester_resistance * 0.018
                } else {
                    0.0
                };

                let corridor_split = if a != b && value > 0.28 {
                    -corridor_pressure * 0.006
                } else if a == b {
                    pathfinder_bias * 0.003
                } else {
                    0.0
                };

                let chaos_drift = ((self.age as f32 * 0.013 + a as f32 * 1.7 + b as f32 * 2.3)
                    .sin())
                    * intensity
                    * 0.12;

                self.rules[a][b] = (value
                    + stabilizer
                    + predator_bias
                    + overgrowth_dampener
                    + corridor_split
                    + chaos_drift)
                    .clamp(-1.0, 1.0);
            }
        }

        self.measure_matrix();
    }

    fn reinforce_matrix_from_clusters(&mut self) {
        if self.clusters.clusters.is_empty() {
            return;
        }

        let mut tribe_counts = [0usize; TRIBE_COUNT];

        for cluster in &self.clusters.clusters {
            if cluster.size >= 8 {
                tribe_counts[cluster.dominant.index()] += cluster.size;
            }
        }

        let total: usize = tribe_counts.iter().sum();

        if total == 0 {
            return;
        }

        let corridor_pressure = self.memory.corridor_pressure();

        for tribe in 0..TRIBE_COUNT {
            let share = tribe_counts[tribe] as f32 / total as f32;

            if share <= 0.08 {
                continue;
            }

            let self_reinforce = 0.004 + share * 0.018 - corridor_pressure * 0.004;
            let neighbor = (tribe + 1) % TRIBE_COUNT;
            let counter = (tribe + TRIBE_COUNT - 1) % TRIBE_COUNT;

            self.rules[tribe][tribe] = (self.rules[tribe][tribe] + self_reinforce).clamp(-1.0, 1.0);
            self.rules[tribe][neighbor] =
                (self.rules[tribe][neighbor] + share * 0.006).clamp(-1.0, 1.0);
            self.rules[tribe][counter] =
                (self.rules[tribe][counter] - share * 0.005).clamp(-1.0, 1.0);
        }

        self.measure_matrix();
    }

    pub fn reset_particles(&mut self) {
        let mut rng = StdRng::seed_from_u64(self.seed ^ self.age);

        self.particles.clear();
        self.clusters = ClusterTracker::new();
        self.species_bank = SpeciesBank::new();
        self.ecology = Ecology::new(self.seed ^ self.age);
        self.substrate = CellularAutomata::new(self.seed ^ self.age ^ 0xC011, 96, 48);

        for i in 0..PARTICLE_COUNT {
            let tribe = Tribe::from_index(i % TRIBE_COUNT);

            self.particles.push(Particle {
                x: rng.gen_range(-1.17..1.17),
                y: rng.gen_range(-1.17..1.17),
                vx: rng.gen_range(-0.020..0.020),
                vy: rng.gen_range(-0.020..0.020),
                tribe,
                age: 0,
                health: rng.gen_range(58.0..100.0),
                energy: rng.gen_range(60.0..110.0),
                mass: rng.gen_range(0.58..1.32),
                cluster_id: None,
                species_id: None,
                rare_trait: RareTrait::None,
                genome: Genome {
                    perception: rng.gen_range(0.145..0.285),
                    hunger: rng.gen_range(0.009..0.023),
                    bonding: rng.gen_range(0.58..1.22),
                    volatility: rng.gen_range(0.82..1.55),
                    orbit: rng.gen_range(0.10..1.02),
                    membrane: rng.gen_range(0.0..0.52),
                    metabolism: rng.gen_range(0.008..0.024),
                    fertility: rng.gen_range(0.55..1.18),
                },
            });
        }

        self.age = 0;
        self.generation = 0;
        self.environment = Environment::Calm;

        self.measure();
        self.measure_matrix();
        self.push_event("particle field reseeded across full dish");
        self.save_all();
    }

    pub fn randomize_world(&mut self) {
        self.save_all();

        self.seed = now_seed();
        self.rules = build_rule_matrix(self.seed);
        self.memory = MemoryBank::new(self.seed);
        self.events.clear();

        self.reset_particles();
        self.push_event("new symbiote seed generated");
    }

    pub fn speed_up(&mut self) {
        self.tick_ms = self.tick_ms.saturating_sub(4).max(4);
    }

    pub fn slow_down(&mut self) {
        self.tick_ms = (self.tick_ms + 4).min(220);
    }

    pub fn save_all(&mut self) {
        self.update_memory();

        let _ = self.save_ecosystem();
        let _ = self.memory.save();
    }

    fn build_archetype_lookup(&self) -> Vec<Option<Archetype>> {
        let max_id = self.species_bank.next_id as usize + 1;
        let mut lookup = vec![None; max_id];

        for species in &self.species_bank.species {
            if let Some(slot) = lookup.get_mut(species.id as usize) {
                *slot = Some(species.archetype);
            }
        }

        for cluster in &self.clusters.clusters {
            if let (Some(species_id), Some(override_archetype)) =
                (cluster.species_id, cluster.archetype_override)
            {
                if let Some(slot) = lookup.get_mut(species_id as usize) {
                    *slot = Some(override_archetype);
                }
            }
        }

        lookup
    }

    fn load_ecosystem() -> Option<Self> {
        if !Path::new(ECOSYSTEM_PATH).exists() {
            return None;
        }

        let data = fs::read_to_string(ECOSYSTEM_PATH).ok()?;
        let state = serde_json::from_str::<EcosystemState>(&data).ok()?;

        Some(Self {
            particles: state.particles,
            rules: state.rules,
            clusters: state.clusters,
            species_bank: state.species_bank,
            ecology: state.ecology,
            substrate: state.substrate,
            memory: state.memory,
            seed: state.seed,
            age: state.age,
            generation: state.generation,
            paused: false,
            tick_ms: state.tick_ms,
            energy: 0.0,
            cohesion: 0.0,
            chaos: 0.0,
            drift: 0.0,
            population: 0.0,
            matrix_attraction: 0.0,
            matrix_repulsion: 0.0,
            matrix_pressure: 0.0,
            environment: state.environment,
            events: VecDeque::from(state.events),
        })
    }

    fn save_ecosystem(&self) -> anyhow::Result<()> {
        fs::create_dir_all("memory")?;

        let state = EcosystemState {
            particles: self.particles.clone(),
            rules: self.rules,
            clusters: ClusterTracker {
                clusters: self.clusters.clusters.clone(),
                next_id: self.clusters.next_id,
            },
            species_bank: self.species_bank.clone(),
            ecology: self.ecology.clone(),
            substrate: self.substrate.clone(),
            memory: self.memory.clone(),
            seed: self.seed,
            age: self.age,
            generation: self.generation,
            tick_ms: self.tick_ms,
            environment: self.environment,
            events: self.events.iter().cloned().collect(),
        };

        fs::write(ECOSYSTEM_PATH, serde_json::to_string_pretty(&state)?)?;

        Ok(())
    }

    fn process_cluster_events(&mut self, events: ClusterEvents) {
        if events.births > 0 {
            self.memory.total_births += events.births as u64;
            self.push_event(&format!("{} organism cluster(s) formed", events.births));
        }

        if events.merges > 0 {
            self.memory.total_merges += events.merges as u64;
            self.push_event("clusters merged into a larger body");
            self.memory
                .note(format!("[{}] merge event detected", self.age));
        }

        if events.splits > 0 {
            self.memory.total_splits += events.splits as u64;
            self.push_event("cluster split into daughter forms");
            self.memory
                .note(format!("[{}] split event detected", self.age));
        }

        if events.extinctions > 0 {
            self.memory.total_extinctions += events.extinctions as u64;
            self.push_event(&format!("{} species went extinct", events.extinctions));
        }
    }

    fn shift_environment(&mut self) {
        let roll = hash(self.seed, self.age as usize, self.generation as usize) % 100;
        let recovery_bias = self.memory.substrate_recovery_bias();
        let reaper_urgency = self.memory.reaper_urgency();
        let pathfinder_bias = self.memory.pathfinder_bias();

        self.environment = if recovery_bias > 0.62 && roll < 48 {
            Environment::Bloom
        } else if reaper_urgency > 0.68 && roll > 58 {
            Environment::Hunger
        } else if pathfinder_bias > 0.55 && roll > 40 {
            Environment::Drift
        } else {
            match roll {
                0..=32 => Environment::Calm,
                33..=55 => Environment::Bloom,
                56..=70 => Environment::Hunger,
                71..=86 => Environment::Drift,
                _ => Environment::Storm,
            }
        };

        self.push_event(&format!("environment shifted: {}", self.environment.name()));
    }

    fn measure(&mut self) {
        if self.particles.is_empty() {
            return;
        }

        let mut speed_sum = 0.0;
        let mut cx = 0.0;
        let mut cy = 0.0;

        for particle in &self.particles {
            speed_sum += (particle.vx * particle.vx + particle.vy * particle.vy).sqrt();
            cx += particle.x;
            cy += particle.y;
        }

        cx /= self.particles.len() as f32;
        cy /= self.particles.len() as f32;

        let mut spread = 0.0;

        for particle in &self.particles {
            let dx = particle.x - cx;
            let dy = particle.y - cy;

            spread += (dx * dx + dy * dy).sqrt();
        }

        spread /= self.particles.len() as f32;

        self.energy = (speed_sum / self.particles.len() as f32 * 1750.0).clamp(0.0, 100.0);
        self.cohesion = ((1.52 - spread) * 70.0).clamp(0.0, 100.0);
        self.chaos = (self.energy * 0.72 + spread * 24.0).clamp(0.0, 100.0);
        self.drift = ((cx.abs() + cy.abs()) * 55.0).clamp(0.0, 100.0);
        self.population =
            ((self.particles.len() as f32 / MAX_PARTICLES as f32) * 100.0).clamp(0.0, 100.0);
    }

    fn measure_matrix(&mut self) {
        let mut attraction = 0.0;
        let mut repulsion = 0.0;
        let mut tension = 0.0;

        for a in 0..TRIBE_COUNT {
            for b in 0..TRIBE_COUNT {
                let value = self.rules[a][b];

                if value >= 0.0 {
                    attraction += value;
                } else {
                    repulsion += value.abs();
                }

                tension += value.abs();
            }
        }

        let max = (TRIBE_COUNT * TRIBE_COUNT) as f32;

        self.matrix_attraction = ((attraction / max) * 100.0).clamp(0.0, 100.0);
        self.matrix_repulsion = ((repulsion / max) * 100.0).clamp(0.0, 100.0);
        self.matrix_pressure = ((tension / max) * 100.0).clamp(0.0, 100.0);
    }

    fn update_memory(&mut self) {
        self.memory.seed = self.seed;

        self.memory.longest_age = self.memory.longest_age.max(self.age);
        self.memory.highest_generation = self.memory.highest_generation.max(self.generation);
        self.memory.peak_population = self.memory.peak_population.max(self.particles.len());
        self.memory.peak_clusters = self.memory.peak_clusters.max(self.clusters.clusters.len());
        self.memory.peak_species = self
            .memory
            .peak_species
            .max(self.species_bank.active_count());
        self.memory.peak_living_cells = self
            .memory
            .peak_living_cells
            .max(self.substrate.living_cells());

        self.memory.observe_substrate(
            self.substrate.living_cells(),
            self.substrate.total_cells(),
            self.substrate.protected_cells(),
            0,
        );

        let rare_count = self
            .particles
            .iter()
            .filter(|particle| particle.rare_trait != RareTrait::None)
            .count();

        self.memory.peak_rare_lifeforms = self.memory.peak_rare_lifeforms.max(rare_count);

        let mut counts = [0usize; 11];

        for species in self
            .species_bank
            .species
            .iter()
            .filter(|species| !species.extinct)
        {
            counts[species.archetype.index()] += 1;
        }

        self.memory.peak_harvesters = self
            .memory
            .peak_harvesters
            .max(counts[Archetype::Harvester.index()]);

        self.memory.peak_reapers = self
            .memory
            .peak_reapers
            .max(counts[Archetype::Reaper.index()]);

        let archetypes = [
            Archetype::Swarmer,
            Archetype::Hunter,
            Archetype::Grazer,
            Archetype::Orbiter,
            Archetype::Parasite,
            Archetype::Architect,
            Archetype::Leviathan,
            Archetype::Mycelial,
            Archetype::Phantom,
            Archetype::Harvester,
            Archetype::Reaper,
        ];

        let mut best = 0;

        for i in 1..11 {
            if counts[i] > counts[best] {
                best = i;
            }
        }

        self.memory.dominant_archetype = archetypes[best].name().to_string();

        if let Some(zone) = self.ecology.zones.iter().max_by(|a, b| {
            a.strength
                .partial_cmp(&b.strength)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            self.memory.richest_zone = zone.kind.name().to_string();
        }

        for cluster in &self.clusters.clusters {
            self.memory.strongest_cluster_size =
                self.memory.strongest_cluster_size.max(cluster.size);
            self.memory.strongest_cluster_age = self.memory.strongest_cluster_age.max(cluster.age);
        }
    }

    fn adaptive_summary(&self) -> String {
        format!(
            "memory root:{:.2} corridor:{:.2} substrate:{:.2} mutation:{:.2}",
            self.memory.root_avoidance_pressure(),
            self.memory.corridor_pressure(),
            self.memory.substrate_throttle_pressure(),
            self.memory.mutation_pressure()
        )
    }

    fn push_event(&mut self, msg: &str) {
        self.events.push_back(format!("[{}] {}", self.age, msg));

        if self.events.len() > 9 {
            self.events.pop_front();
        }
    }
}

fn random_particle(rng: &mut StdRng) -> Particle {
    Particle {
        x: rng.gen_range(-1.17..1.17),
        y: rng.gen_range(-1.17..1.17),
        vx: rng.gen_range(-0.020..0.020),
        vy: rng.gen_range(-0.020..0.020),
        tribe: Tribe::from_index(rng.gen_range(0..TRIBE_COUNT)),
        age: 0,
        health: rng.gen_range(60.0..100.0),
        energy: rng.gen_range(60.0..110.0),
        mass: rng.gen_range(0.58..1.32),
        cluster_id: None,
        species_id: None,
        rare_trait: RareTrait::None,
        genome: Genome {
            perception: rng.gen_range(0.145..0.285),
            hunger: rng.gen_range(0.009..0.023),
            bonding: rng.gen_range(0.58..1.22),
            volatility: rng.gen_range(0.82..1.55),
            orbit: rng.gen_range(0.10..1.02),
            membrane: rng.gen_range(0.0..0.52),
            metabolism: rng.gen_range(0.008..0.024),
            fertility: rng.gen_range(0.55..1.18),
        },
    }
}

fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    let dx = ax - bx;
    let dy = ay - by;

    (dx * dx + dy * dy).sqrt()
}

fn hash(seed: u64, x: usize, y: usize) -> usize {
    let mut value = seed as usize;

    value ^= x.wrapping_mul(374_761_393);
    value ^= y.wrapping_mul(668_265_263);
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);

    value ^ (value >> 16)
}

fn now_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64
}
