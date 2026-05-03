use crate::{
    cluster::{ClusterEvents, ClusterTracker},
    memory::MemoryBank,
    particle::{Genome, Particle, Tribe},
    sim::{build_rule_matrix, child_from, mutate_rules, step_particles, RuleMatrix},
};
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::VecDeque;

pub const TRIBE_COUNT: usize = 6;
pub const PARTICLE_COUNT: usize = 460;
pub const MAX_PARTICLES: usize = 780;
pub const MIN_PARTICLES: usize = 230;

#[derive(Clone, Copy, PartialEq)]
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

pub struct App {
    pub particles: Vec<Particle>,
    pub rules: RuleMatrix,
    pub clusters: ClusterTracker,
    pub memory: MemoryBank,
    pub seed: u64,
    pub age: u64,
    pub generation: u64,
    pub paused: bool,
    pub evolution_enabled: bool,
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
        let seed = now_seed();

        let mut app = Self {
            particles: Vec::new(),
            rules: build_rule_matrix(seed),
            clusters: ClusterTracker::new(),
            memory: MemoryBank::load_or_new(seed),
            seed,
            age: 0,
            generation: 0,
            paused: false,
            evolution_enabled: true,
            tick_ms: 22,
            energy: 0.0,
            cohesion: 0.0,
            chaos: 0.0,
            drift: 0.0,
            population: 0.0,
            environment: Environment::Calm,
            events: VecDeque::new(),
        };

        app.reset_particles();
        app.push_event("symbiote organism awakened");
        app.push_event("predator pressure enabled");
        app.push_event("pulsing membrane layer active");
        app
    }

    pub fn step(&mut self) {
        step_particles(&mut self.particles, &self.rules, self.environment);
        self.age += 1;

        if self.age % 24 == 0 {
            let cluster_events = self.clusters.update(&mut self.particles, self.age);
            self.process_cluster_events(cluster_events);
        }

        if self.age % 360 == 0 {
            self.shift_environment();
        }

        if self.evolution_enabled && self.age % 90 == 0 {
            self.evolve_population();
        }

        if self.evolution_enabled && self.age % 280 == 0 {
            let intensity = match self.environment {
                Environment::Calm => 0.035,
                Environment::Bloom => 0.025,
                Environment::Hunger => 0.075,
                Environment::Storm => 0.1,
                Environment::Drift => 0.04,
            };

            mutate_rules(&mut self.rules, self.seed ^ self.age, intensity);
            self.generation += 1;
            self.push_event("symbiosis matrix adapted");
        }

        self.measure();
        self.update_memory();

        if self.age % 600 == 0 {
            self.save_memory();
        }
    }

    pub fn reset_particles(&mut self) {
        let mut rng = StdRng::seed_from_u64(self.seed ^ self.age);

        self.particles.clear();
        self.clusters = ClusterTracker::new();

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
                mass: rng.gen_range(0.65..1.5),
                cluster_id: None,
                genome: Genome {
                    perception: rng.gen_range(0.17..0.31),
                    hunger: rng.gen_range(0.009..0.023),
                    bonding: rng.gen_range(1.05..1.85),
                    volatility: rng.gen_range(0.64..1.35),
                    orbit: rng.gen_range(0.0..0.82),
                    membrane: rng.gen_range(0.0..0.98),
                },
            });
        }

        self.age = 0;
        self.generation = 0;
        self.environment = Environment::Calm;
        self.measure();
        self.push_event("particle field reseeded");
    }

    pub fn force_mutation(&mut self) {
        mutate_rules(&mut self.rules, self.seed ^ self.age ^ 0xB10B10, 0.24);
        self.evolve_population();
        self.generation += 1;
        self.push_event("manual mutation injected");
    }

    pub fn randomize_world(&mut self) {
        self.save_memory();
        self.seed = now_seed();
        self.rules = build_rule_matrix(self.seed);
        self.memory = MemoryBank::new(self.seed);
        self.events.clear();
        self.reset_particles();
        self.push_event("new symbiote seed generated");
    }

    pub fn toggle_evolution(&mut self) {
        self.evolution_enabled = !self.evolution_enabled;

        if self.evolution_enabled {
            self.push_event("evolution resumed");
        } else {
            self.push_event("evolution paused, motion continues");
        }
    }

    pub fn speed_up(&mut self) {
        self.tick_ms = self.tick_ms.saturating_sub(4).max(4);
    }

    pub fn slow_down(&mut self) {
        self.tick_ms = (self.tick_ms + 4).min(120);
    }

    pub fn save_memory(&self) {
        let _ = self.memory.save();
    }

    fn process_cluster_events(&mut self, events: ClusterEvents) {
        if events.births > 0 {
            self.memory.total_births += events.births as u64;
            self.push_event(&format!("{} organism cluster(s) formed", events.births));
        }

        if events.deaths > 0 {
            self.memory.total_deaths += events.deaths as u64;
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

    fn evolve_population(&mut self) {
        let mut rng = StdRng::seed_from_u64(self.seed ^ self.age ^ self.generation);
        let before = self.particles.len();

        self.particles.retain(|p| {
            let clustered_bonus = if p.cluster_id.is_some() { 0.22 } else { 0.0 };
            let survival = ((p.health / 118.0) + clustered_bonus).clamp(0.05, 0.985);
            p.health > 0.0 && p.age < 13000 && rng.gen_bool(survival as f64)
        });

        let survivors = self.particles.clone();

        for parent in survivors {
            if self.particles.len() >= MAX_PARTICLES {
                break;
            }

            let cluster_bonus = if parent.cluster_id.is_some() { 0.04 } else { 0.0 };

            let reproduction_chance = match self.environment {
                Environment::Bloom => 0.12,
                Environment::Calm => 0.06,
                Environment::Drift => 0.045,
                Environment::Hunger => 0.026,
                Environment::Storm => 0.018,
            } + cluster_bonus;

            let mature = parent.age > 140;
            let healthy = parent.health > 44.0;

            if mature && healthy && rng.gen_bool(reproduction_chance) {
                let child = child_from(parent, rng.gen());
                self.particles.push(child);
            }
        }

        while self.particles.len() < MIN_PARTICLES {
            self.particles.push(random_particle(&mut rng));
        }

        let after = self.particles.len();

        if after > before {
            self.push_event(&format!("population bloomed {} -> {}", before, after));
        } else if after < before {
            self.push_event(&format!("selection culled {} -> {}", before, after));
        }
    }

    fn measure(&mut self) {
        if self.particles.is_empty() {
            return;
        }

        let mut speed_sum = 0.0;
        let mut cx = 0.0;
        let mut cy = 0.0;

        for p in &self.particles {
            speed_sum += (p.vx * p.vx + p.vy * p.vy).sqrt();
            cx += p.x;
            cy += p.y;
        }

        cx /= self.particles.len() as f32;
        cy /= self.particles.len() as f32;

        let mut spread = 0.0;

        for p in &self.particles {
            let dx = p.x - cx;
            let dy = p.y - cy;
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
        self.memory.longest_age = self.memory.longest_age.max(self.age);
        self.memory.highest_generation = self.memory.highest_generation.max(self.generation);
        self.memory.peak_population = self.memory.peak_population.max(self.particles.len());
        self.memory.peak_clusters = self.memory.peak_clusters.max(self.clusters.clusters.len());

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
        mass: rng.gen_range(0.65..1.5),
        cluster_id: None,
        genome: Genome {
            perception: rng.gen_range(0.17..0.31),
            hunger: rng.gen_range(0.009..0.023),
            bonding: rng.gen_range(1.05..1.85),
            volatility: rng.gen_range(0.64..1.35),
            orbit: rng.gen_range(0.0..0.82),
            membrane: rng.gen_range(0.0..0.98),
        },
    }
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
