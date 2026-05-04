use crate::{
    particle::{Genome, Particle, RareTrait, Tribe},
    species::{Archetype, SpeciesBank},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: u64,
    pub species_id: Option<u64>,
    pub archetype: Option<Archetype>,
    pub archetype_override: Option<Archetype>,
    pub rare_trait: RareTrait,
    pub age: u64,
    pub size: usize,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub radius: f32,
    pub dominant: Tribe,
    pub avg_genome: Genome,
    pub stability: f32,
    pub membrane: f32,
    pub drift_heat: f32,
    pub last_seen: u64,
}

impl Cluster {
    pub fn speed(&self) -> f32 {
        (self.vx * self.vx + self.vy * self.vy).sqrt()
    }

    pub fn direction_glyph(&self) -> char {
        if self.speed() < 0.0002 {
            return '•';
        }

        if self.vx.abs() > self.vy.abs() {
            if self.vx > 0.0 {
                '→'
            } else {
                '←'
            }
        } else if self.vy > 0.0 {
            '↓'
        } else {
            '↑'
        }
    }

    pub fn effective_archetype(&self) -> Option<Archetype> {
        self.archetype_override.or(self.archetype)
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ClusterTracker {
    pub clusters: Vec<Cluster>,
    pub next_id: u64,
}

impl ClusterTracker {
    pub fn new() -> Self {
        Self {
            clusters: Vec::new(),
            next_id: 1,
        }
    }

    pub fn update(
        &mut self,
        particles: &mut [Particle],
        species_bank: &mut SpeciesBank,
        age: u64,
    ) -> ClusterEvents {
        for particle in particles.iter_mut() {
            particle.cluster_id = None;
        }

        let groups = detect_groups(particles);
        let mut next_clusters = Vec::new();
        let mut events = ClusterEvents::default();

        let root_pressure = species_bank
            .species
            .iter()
            .filter(|species| !species.extinct && species.root_adaptation > 0.55)
            .count() as f32
            / species_bank.active_count().max(1) as f32;

        let corridor_pressure = species_bank
            .species
            .iter()
            .filter(|species| !species.extinct && species.corridor_score > 0.55)
            .count() as f32
            / species_bank.active_count().max(1) as f32;

        for group in groups {
            if group.len() < 5 {
                continue;
            }

            let measured = measure_group(&group, particles);

            let mut best_match = None;
            let mut best_dist = f32::MAX;

            for existing in &self.clusters {
                let dx = existing.x - measured.x;
                let dy = existing.y - measured.y;
                let dist = (dx * dx + dy * dy).sqrt();

                if dist < 0.28 && dist < best_dist {
                    best_match = Some(existing.clone());
                    best_dist = dist;
                }
            }

            let parent_species = best_match.as_ref().and_then(|cluster| cluster.species_id);

            let mut cluster = if let Some(old) = best_match {
                let mut current = measured;

                current.id = old.id;
                current.species_id = old.species_id;
                current.archetype = old.archetype;
                current.archetype_override = old.archetype_override;
                current.age = old.age + 1;
                current.stability =
                    (old.stability * 0.9 + current.stability * 0.1).clamp(0.0, 100.0);
                current.membrane =
                    (old.membrane * 0.94 + current.membrane * 0.06).clamp(0.0, 100.0);
                current.drift_heat =
                    (old.drift_heat * 0.90 + current.drift_heat * 0.10).clamp(0.0, 100.0);
                current.last_seen = age;

                if current.rare_trait == RareTrait::None && old.rare_trait != RareTrait::None {
                    current.rare_trait = old.rare_trait;
                }

                current
            } else {
                let mut current = measured;

                current.id = self.next_id;
                self.next_id += 1;
                current.age = 1;
                current.last_seen = age;
                events.births += 1;

                current
            };

            let species_id = species_bank.assign_or_create(
                cluster.dominant,
                cluster.avg_genome,
                cluster.rare_trait,
                cluster.size,
                age,
                parent_species,
            );

            let species = species_bank
                .species
                .iter()
                .find(|species| species.id == species_id);

            cluster.species_id = Some(species_id);
            cluster.archetype = species.map(|species| species.archetype);

            apply_cluster_drift(
                &mut cluster,
                root_pressure,
                corridor_pressure,
                species
                    .map(|species| species.root_adaptation)
                    .unwrap_or_default(),
                species
                    .map(|species| species.corridor_score)
                    .unwrap_or_default(),
                species
                    .map(|species| species.drift_pressure)
                    .unwrap_or_default(),
            );

            if cluster.age > 50 && cluster.size > 14 {
                cluster.membrane = (cluster.membrane + 1.2).min(100.0);
            }

            if cluster.stability > 65.0 && cluster.size > 22 {
                cluster.membrane = (cluster.membrane + 0.9).min(100.0);
            }

            for &idx in &group {
                if let Some(particle) = particles.get_mut(idx) {
                    particle.cluster_id = Some(cluster.id);
                    particle.species_id = Some(species_id);
                    particle.mass = (particle.mass + 0.004 * cluster.size as f32).clamp(0.55, 6.5);

                    if cluster.archetype_override.is_some() {
                        particle.genome.perception =
                            (particle.genome.perception + 0.00008).clamp(0.1, 0.38);
                        particle.genome.orbit = (particle.genome.orbit + 0.00012).clamp(0.0, 1.55);
                    }
                }
            }

            next_clusters.push(cluster);
        }

        let old_count = self.clusters.len();
        let new_count = next_clusters.len();

        if new_count < old_count {
            events.merges += old_count - new_count;
        }

        if new_count > old_count + events.births {
            events.splits += new_count.saturating_sub(old_count);
        }

        self.clusters = next_clusters;
        events.extinctions += species_bank.mark_extinctions(age);

        events
    }
}

#[derive(Default)]
pub struct ClusterEvents {
    pub births: usize,
    pub merges: usize,
    pub splits: usize,
    pub extinctions: usize,
}

fn apply_cluster_drift(
    cluster: &mut Cluster,
    root_pressure: f32,
    corridor_pressure: f32,
    species_root_adaptation: f32,
    species_corridor_score: f32,
    species_drift_pressure: f32,
) {
    let base = cluster.archetype;
    let mobility = root_mobility_score(cluster.avg_genome);
    let density = (cluster.size as f32 / 90.0).clamp(0.0, 1.0);
    let pressure = (root_pressure * 0.24
        + corridor_pressure * 0.24
        + species_root_adaptation * 0.22
        + species_corridor_score * 0.18
        + species_drift_pressure * 0.12)
        .clamp(0.0, 1.0);

    let heat_target = (pressure * 72.0 + mobility * 18.0 + density * 10.0).clamp(0.0, 100.0);
    cluster.drift_heat = (cluster.drift_heat * 0.86 + heat_target * 0.14).clamp(0.0, 100.0);

    cluster.archetype_override = if cluster.drift_heat > 64.0 && pressure > 0.48 {
        match base {
            Some(Archetype::Harvester) => {
                if cluster.avg_genome.orbit > 0.48 || species_corridor_score > 0.62 {
                    Some(Archetype::Orbiter)
                } else if cluster.avg_genome.bonding > 1.36 {
                    Some(Archetype::Swarmer)
                } else {
                    Some(Archetype::Grazer)
                }
            }
            Some(Archetype::Grazer | Archetype::Mycelial) => {
                if cluster.avg_genome.membrane > 0.88 {
                    Some(Archetype::Architect)
                } else if cluster.avg_genome.orbit > 0.62 {
                    Some(Archetype::Orbiter)
                } else {
                    None
                }
            }
            Some(Archetype::Parasite) => {
                if cluster.avg_genome.volatility > 1.36 && cluster.avg_genome.perception > 0.27 {
                    Some(Archetype::Hunter)
                } else if cluster.avg_genome.orbit > 0.68 {
                    Some(Archetype::Orbiter)
                } else {
                    None
                }
            }
            Some(Archetype::Swarmer) => {
                if species_corridor_score > 0.72 && cluster.avg_genome.orbit > 0.58 {
                    Some(Archetype::Orbiter)
                } else {
                    None
                }
            }
            Some(Archetype::Hunter) => {
                if species_drift_pressure > 0.72
                    && cluster.avg_genome.volatility > 1.54
                    && cluster.avg_genome.hunger > 0.02
                {
                    Some(Archetype::Reaper)
                } else {
                    None
                }
            }
            Some(Archetype::Leviathan) => {
                if cluster.avg_genome.membrane > 1.05 {
                    Some(Archetype::Architect)
                } else {
                    None
                }
            }
            Some(
                Archetype::Reaper | Archetype::Orbiter | Archetype::Architect | Archetype::Phantom,
            ) => None,
            None => None,
        }
    } else if cluster.drift_heat < 38.0 {
        None
    } else {
        cluster.archetype_override
    };
}

fn root_mobility_score(genome: Genome) -> f32 {
    let perception = ((genome.perception - 0.18) / 0.20).clamp(0.0, 1.0);
    let orbit = (genome.orbit / 1.35).clamp(0.0, 1.0);
    let volatility = ((genome.volatility - 0.70) / 1.05).clamp(0.0, 1.0);
    let membrane = (genome.membrane / 1.45).clamp(0.0, 1.0);

    (perception * 0.36 + orbit * 0.32 + volatility * 0.16 + membrane * 0.16).clamp(0.0, 1.0)
}

fn detect_groups(particles: &[Particle]) -> Vec<Vec<usize>> {
    let mut visited = vec![false; particles.len()];
    let mut groups = Vec::new();

    for i in 0..particles.len() {
        if visited[i] {
            continue;
        }

        let mut stack = vec![i];
        let mut group = Vec::new();
        visited[i] = true;

        while let Some(idx) = stack.pop() {
            group.push(idx);

            for j in 0..particles.len() {
                if visited[j] {
                    continue;
                }

                let dx = particles[idx].x - particles[j].x;
                let dy = particles[idx].y - particles[j].y;
                let dist = (dx * dx + dy * dy).sqrt();
                let link =
                    0.082 + particles[idx].genome.bonding * 0.019 + particles[idx].mass * 0.004;

                if dist < link {
                    visited[j] = true;
                    stack.push(j);
                }
            }
        }

        groups.push(group);
    }

    groups
}

fn measure_group(indices: &[usize], particles: &[Particle]) -> Cluster {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut vx = 0.0;
    let mut vy = 0.0;
    let mut tribe_counts = [0usize; 6];
    let mut rare_counts = [0usize; 8];
    let mut membrane = 0.0;

    let mut genome = Genome {
        perception: 0.0,
        hunger: 0.0,
        bonding: 0.0,
        volatility: 0.0,
        orbit: 0.0,
        membrane: 0.0,
        metabolism: 0.0,
        fertility: 0.0,
    };

    for &idx in indices {
        let particle = particles[idx];

        x += particle.x;
        y += particle.y;
        vx += particle.vx;
        vy += particle.vy;
        membrane += particle.genome.membrane;

        tribe_counts[particle.tribe.index()] += 1;
        rare_counts[rare_index(particle.rare_trait)] += 1;

        genome.perception += particle.genome.perception;
        genome.hunger += particle.genome.hunger;
        genome.bonding += particle.genome.bonding;
        genome.volatility += particle.genome.volatility;
        genome.orbit += particle.genome.orbit;
        genome.membrane += particle.genome.membrane;
        genome.metabolism += particle.genome.metabolism;
        genome.fertility += particle.genome.fertility;
    }

    let count = indices.len() as f32;

    x /= count;
    y /= count;
    vx /= count;
    vy /= count;
    membrane = (membrane / count * 74.0).clamp(0.0, 100.0);

    genome.perception /= count;
    genome.hunger /= count;
    genome.bonding /= count;
    genome.volatility /= count;
    genome.orbit /= count;
    genome.membrane /= count;
    genome.metabolism /= count;
    genome.fertility /= count;

    let mut radius = 0.0;

    for &idx in indices {
        let particle = particles[idx];
        let dx = particle.x - x;
        let dy = particle.y - y;

        radius += (dx * dx + dy * dy).sqrt();
    }

    radius /= count;

    let mut best_tribe = 0;

    for i in 1..6 {
        if tribe_counts[i] > tribe_counts[best_tribe] {
            best_tribe = i;
        }
    }

    let mut best_rare = 0;

    for i in 1..8 {
        if rare_counts[i] > rare_counts[best_rare] {
            best_rare = i;
        }
    }

    let stability = ((indices.len() as f32 * 4.5) - radius * 125.0).clamp(0.0, 100.0);
    let drift_heat = root_mobility_score(genome) * 28.0 + stability * 0.10;

    Cluster {
        id: 0,
        species_id: None,
        archetype: None,
        archetype_override: None,
        rare_trait: rare_from_index(best_rare),
        age: 0,
        size: indices.len(),
        x,
        y,
        vx,
        vy,
        radius,
        dominant: Tribe::from_index(best_tribe),
        avg_genome: genome,
        stability,
        membrane,
        drift_heat,
        last_seen: 0,
    }
}

fn rare_index(rare_trait: RareTrait) -> usize {
    match rare_trait {
        RareTrait::None => 0,
        RareTrait::ElderCore => 1,
        RareTrait::Radiant => 2,
        RareTrait::Voracious => 3,
        RareTrait::Voidborne => 4,
        RareTrait::SymbioticCore => 5,
        RareTrait::SporeKing => 6,
        RareTrait::Devourer => 7,
    }
}

fn rare_from_index(index: usize) -> RareTrait {
    match index {
        1 => RareTrait::ElderCore,
        2 => RareTrait::Radiant,
        3 => RareTrait::Voracious,
        4 => RareTrait::Voidborne,
        5 => RareTrait::SymbioticCore,
        6 => RareTrait::SporeKing,
        7 => RareTrait::Devourer,
        _ => RareTrait::None,
    }
}
