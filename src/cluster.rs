use crate::{
    particle::{Genome, Particle, Tribe},
    species::{Archetype, SpeciesBank},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: u64,
    pub species_id: Option<u64>,
    pub archetype: Option<Archetype>,
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
            if self.vx > 0.0 { '→' } else { '←' }
        } else if self.vy > 0.0 {
            '↓'
        } else {
            '↑'
        }
    }
}

#[derive(Serialize, Deserialize)]
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
        for p in particles.iter_mut() {
            p.cluster_id = None;
        }

        let groups = detect_groups(particles);
        let mut next_clusters = Vec::new();
        let mut events = ClusterEvents::default();

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

            let parent_species = best_match.as_ref().and_then(|c| c.species_id);

            let mut cluster = if let Some(old) = best_match {
                let mut c = measured;
                c.id = old.id;
                c.species_id = old.species_id;
                c.archetype = old.archetype;
                c.age = old.age + 1;
                c.stability = (old.stability * 0.9 + c.stability * 0.1).clamp(0.0, 100.0);
                c.membrane = (old.membrane * 0.94 + c.membrane * 0.06).clamp(0.0, 100.0);
                c.last_seen = age;
                c
            } else {
                let mut c = measured;
                c.id = self.next_id;
                self.next_id += 1;
                c.age = 1;
                c.last_seen = age;
                events.births += 1;
                c
            };

            let species_id = species_bank.assign_or_create(
                cluster.dominant,
                cluster.avg_genome,
                cluster.size,
                age,
                parent_species,
            );

            let species = species_bank.species.iter().find(|s| s.id == species_id);
            cluster.species_id = Some(species_id);
            cluster.archetype = species.map(|s| s.archetype);

            if cluster.age > 50 && cluster.size > 14 {
                cluster.membrane = (cluster.membrane + 1.2).min(100.0);
            }

            if cluster.stability > 65.0 && cluster.size > 22 {
                cluster.membrane = (cluster.membrane + 0.9).min(100.0);
            }

            for &idx in &group {
                if let Some(p) = particles.get_mut(idx) {
                    p.cluster_id = Some(cluster.id);
                    p.species_id = Some(species_id);
                    p.mass = (p.mass + 0.004 * cluster.size as f32).clamp(0.55, 6.5);
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
    pub deaths: usize,
    pub merges: usize,
    pub splits: usize,
    pub extinctions: usize,
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
                let link = 0.082 + particles[idx].genome.bonding * 0.019 + particles[idx].mass * 0.004;

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
    let mut membrane = 0.0;

    let mut genome = Genome {
        perception: 0.0,
        hunger: 0.0,
        bonding: 0.0,
        volatility: 0.0,
        orbit: 0.0,
        membrane: 0.0,
    };

    for &idx in indices {
        let p = particles[idx];
        x += p.x;
        y += p.y;
        vx += p.vx;
        vy += p.vy;
        membrane += p.genome.membrane;
        tribe_counts[p.tribe.index()] += 1;

        genome.perception += p.genome.perception;
        genome.hunger += p.genome.hunger;
        genome.bonding += p.genome.bonding;
        genome.volatility += p.genome.volatility;
        genome.orbit += p.genome.orbit;
        genome.membrane += p.genome.membrane;
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

    let mut radius = 0.0;

    for &idx in indices {
        let p = particles[idx];
        let dx = p.x - x;
        let dy = p.y - y;
        radius += (dx * dx + dy * dy).sqrt();
    }

    radius /= count;

    let mut best = 0;

    for i in 1..6 {
        if tribe_counts[i] > tribe_counts[best] {
            best = i;
        }
    }

    let stability = ((indices.len() as f32 * 4.5) - radius * 125.0).clamp(0.0, 100.0);

    Cluster {
        id: 0,
        species_id: None,
        archetype: None,
        age: 0,
        size: indices.len(),
        x,
        y,
        vx,
        vy,
        radius,
        dominant: Tribe::from_index(best),
        avg_genome: genome,
        stability,
        membrane,
        last_seen: 0,
    }
}
