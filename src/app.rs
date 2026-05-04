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
    pub environment: Environment,
    pub events: VecDeque<String>,
}

impl App {
    pub fn new() -> Self {
        if let Some(mut restored) = Self::load_ecosystem() {
            restored.paused = false;
            restored.measure();
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
            environment: Environment::Calm,
            events: VecDeque::new(),
        };

        app.reset_particles();
        app.push_event("native reproduction engine online");
        app.push_event("cellular automata substrate online");
        app
    }

    pub fn step(&mut self) {
        let archetype_lookup = self.build_archetype_lookup();

        step_particles(
            &mut self.particles,
            &self.rules,
            self.environment,
            &self.ecology,
            &self.substrate,
            &archetype_lookup,
        );

        self.age += 1;

        self.ecology.tick(self.seed, self.age, self.environment);

        if self.age % 2 == 0 {
            self.deposit_to_substrate();
        }

        if self.age % 3 == 0 {
            self.substrate.tick();
        }

        if self.age % 32 == 0 {
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
                self.push_event(&format!("{} new species emerged", after_species - before_species));
            }

            self.process_cluster_events(cluster_events);
        }

        if self.age % 360 == 0 {
            self.shift_environment();
        }

        if self.age % 280 == 0 {
            let intensity = match self.environment {
                Environment::Calm => 0.018,
                Environment::Bloom => 0.014,
                Environment::Hunger => 0.035,
                Environment::Storm => 0.052,
                Environment::Drift => 0.022,
            };

            mutate_rules(&mut self.rules, self.seed ^ self.age, intensity);
            self.generation += 1;
            self.push_event("matrix drifted through native mutation");
        }

        self.measure();
        self.update_memory();

        if self.age % 600 == 0 {
            self.save_all();
        }
    }

    fn deposit_to_substrate(&mut self) {
        let lookup = self.build_archetype_lookup();

        for particle in self.particles.iter().step_by(3) {
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
        let mut children = Vec::new();

        for parent in snapshot.iter() {
            if self.particles.len() + children.len() >= MAX_PARTICLES {
                break;
            }

            let clustered_bonus = if parent.cluster_id.is_some() { 0.18 } else { 0.0 };
            let rare_bonus = if parent.rare_trait != RareTrait::None { 0.12 } else { 0.0 };
            let threshold = 115.0 - parent.genome.fertility * 13.0 - clustered_bonus * 20.0;

            if parent.energy < threshold || parent.health < 48.0 || parent.age < 180 {
                continue;
            }

            let chance =
                (0.018 + parent.genome.fertility * 0.018 + clustered_bonus + rare_bonus).clamp(0.01, 0.38);

            if !rng.gen_bool(chance as f64) {
                continue;
            }

            let maybe_partner = snapshot
                .iter()
                .find(|other| {
                    other.species_id != parent.species_id
                        && other.energy > 95.0
                        && dist(parent.x, parent.y, other.x, other.y) < 0.14
                })
                .copied();

            let child = if let Some(partner) = maybe_partner {
                self.memory.total_fusions += 1;
                fused_child(*parent, partner, rng.gen())
            } else {
                child_from(*parent, rng.gen())
            };

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

        self.particles.retain(|particle| {
            let old_age_pressure = if particle.age > 18_000 { 0.08 } else { 0.0 };
            let energy_score = (particle.energy / 130.0).clamp(0.0, 1.0);
            let health_score = (particle.health / 100.0).clamp(0.0, 1.0);
            let clustered_bonus = if particle.cluster_id.is_some() { 0.18 } else { 0.0 };
            let rare_bonus = if particle.rare_trait != RareTrait::None { 0.04 } else { 0.0 };

            let survival =
                (energy_score * 0.44 + health_score * 0.44 + clustered_bonus + rare_bonus - old_age_pressure)
                    .clamp(0.02, 0.995);

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
                x: rng.gen_range(-1.0..1.0),
                y: rng.gen_range(-1.0..1.0),
                vx: rng.gen_range(-0.012..0.012),
                vy: rng.gen_range(-0.012..0.012),
                tribe,
                age: 0,
                health: rng.gen_range(58.0..100.0),
                energy: rng.gen_range(60.0..110.0),
                mass: rng.gen_range(0.65..1.5),
                cluster_id: None,
                species_id: None,
                rare_trait: RareTrait::None,
                genome: Genome {
                    perception: rng.gen_range(0.17..0.31),
                    hunger: rng.gen_range(0.009..0.023),
                    bonding: rng.gen_range(1.05..1.85),
                    volatility: rng.gen_range(0.64..1.35),
                    orbit: rng.gen_range(0.0..0.82),
                    membrane: rng.gen_range(0.0..0.98),
                    metabolism: rng.gen_range(0.008..0.024),
                    fertility: rng.gen_range(0.65..1.35),
                },
            });
        }

        self.age = 0;
        self.generation = 0;
        self.environment = Environment::Calm;
        self.measure();
        self.push_event("particle field reseeded");
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
            self.memory.note(format!("[{}] merge event detected", self.age));
        }

        if events.splits > 0 {
            self.memory.total_splits += events.splits as u64;
            self.push_event("cluster split into daughter forms");
            self.memory.note(format!("[{}] split event detected", self.age));
        }

        if events.extinctions > 0 {
            self.memory.total_extinctions += events.extinctions as u64;
            self.push_event(&format!("{} species went extinct", events.extinctions));
        }
    }

    fn shift_environment(&mut self) {
        let roll = hash(self.seed, self.age as usize, self.generation as usize) % 100;

        self.environment = match roll {
            0..=32 => Environment::Calm,
            33..=55 => Environment::Bloom,
            56..=70 => Environment::Hunger,
            71..=86 => Environment::Drift,
            _ => Environment::Storm,
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
        self.cohesion = ((1.4 - spread) * 80.0).clamp(0.0, 100.0);
        self.chaos = (self.energy * 0.72 + spread * 22.0).clamp(0.0, 100.0);
        self.drift = ((cx.abs() + cy.abs()) * 55.0).clamp(0.0, 100.0);
        self.population = ((self.particles.len() as f32 / MAX_PARTICLES as f32) * 100.0).clamp(0.0, 100.0);
    }

    fn update_memory(&mut self) {
        self.memory.seed = self.seed;
        self.memory.longest_age = self.memory.longest_age.max(self.age);
        self.memory.highest_generation = self.memory.highest_generation.max(self.generation);
        self.memory.peak_population = self.memory.peak_population.max(self.particles.len());
        self.memory.peak_clusters = self.memory.peak_clusters.max(self.clusters.clusters.len());
        self.memory.peak_species = self.memory.peak_species.max(self.species_bank.active_count());
        self.memory.peak_living_cells = self.memory.peak_living_cells.max(self.substrate.living_cells());

        let rare_count = self
            .particles
            .iter()
            .filter(|particle| particle.rare_trait != RareTrait::None)
            .count();

        self.memory.peak_rare_lifeforms = self.memory.peak_rare_lifeforms.max(rare_count);

        let mut counts = [0usize; 9];

        for species in self.species_bank.species.iter().filter(|species| !species.extinct) {
            counts[species.archetype.index()] += 1;
        }

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
        ];

        let mut best = 0;

        for i in 1..9 {
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
            self.memory.strongest_cluster_size = self.memory.strongest_cluster_size.max(cluster.size);
            self.memory.strongest_cluster_age = self.memory.strongest_cluster_age.max(cluster.age);
        }
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
        x: rng.gen_range(-1.0..1.0),
        y: rng.gen_range(-1.0..1.0),
        vx: rng.gen_range(-0.012..0.012),
        vy: rng.gen_range(-0.012..0.012),
        tribe: Tribe::from_index(rng.gen_range(0..TRIBE_COUNT)),
        age: 0,
        health: rng.gen_range(60.0..100.0),
        energy: rng.gen_range(60.0..110.0),
        mass: rng.gen_range(0.65..1.5),
        cluster_id: None,
        species_id: None,
        rare_trait: RareTrait::None,
        genome: Genome {
            perception: rng.gen_range(0.17..0.31),
            hunger: rng.gen_range(0.009..0.023),
            bonding: rng.gen_range(1.05..1.85),
            volatility: rng.gen_range(0.64..1.35),
            orbit: rng.gen_range(0.0..0.82),
            membrane: rng.gen_range(0.0..0.98),
            metabolism: rng.gen_range(0.008..0.024),
            fertility: rng.gen_range(0.65..1.35),
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
