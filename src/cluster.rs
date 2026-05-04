use crate::{
    particle::{Genome, Particle, RareTrait, Tribe},
    species::{drift_candidate, Archetype, ArchetypePressure, SpeciesBank},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Cluster {
    pub id: u64,
    pub species_id: Option<u64>,
    pub archetype: Option<Archetype>,
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
    pub last_seen: u64,
    #[serde(default)]
    pub archetype_override: Option<Archetype>,
    #[serde(default)]
    pub drift_heat: f32,
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
        for particle in particles.iter_mut() {
            particle.cluster_id = None;
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

            let parent_species = best_match.as_ref().and_then(|cluster| cluster.species_id);

            let mut cluster = if let Some(old) = best_match {
                let mut current = measured;

                current.id = old.id;
                current.species_id = old.species_id;
                current.archetype = old.archetype;
                current.archetype_override = old.archetype_override;
                current.drift_heat = (old.drift_heat * 0.88).clamp(0.0, 100.0);
                current.age = old.age + 1;
                current.stability =
                    (old.stability * 0.9 + current.stability * 0.1).clamp(0.0, 100.0);
                current.membrane =
                    (old.membrane * 0.94 + current.membrane * 0.06).clamp(0.0, 100.0);
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
                current.archetype_override = None;
                current.drift_heat = 0.0;
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

    pub fn drift_cluster_archetypes(
        &mut self,
        particles: &mut [Particle],
        species_bank: &SpeciesBank,
        pressure: ArchetypePressure,
    ) -> usize {
        let mut drifted = 0;

        for cluster in &mut self.clusters {
            let base = cluster
                .species_id
                .and_then(|id| species_bank.species.iter().find(|species| species.id == id))
                .map(|species| species.archetype)
                .or(cluster.archetype);

            let Some(base_archetype) = base else {
                continue;
            };

            let candidate = cluster_drift_candidate(cluster, base_archetype, pressure);

            if candidate == base_archetype {
                cluster.drift_heat = (cluster.drift_heat * 0.82).clamp(0.0, 100.0);

                if cluster.drift_heat < 18.0 {
                    cluster.archetype_override = None;
                }

                continue;
            }

            let age_factor = (cluster.age as f32 / 850.0).clamp(0.0, 1.0);
            let size_factor = (cluster.size as f32 / 52.0).clamp(0.0, 1.0);
            let stability_factor = (cluster.stability / 100.0).clamp(0.0, 1.0);
            let membrane_factor = (cluster.membrane / 100.0).clamp(0.0, 1.0);
            let pressure_factor = (pressure.matrix_pressure * 0.35
                + pressure.mutation_pressure * 0.24
                + pressure.reaper_urgency * 0.18
                + pressure.recovery_bias * 0.14
                + pressure.harvester_pressure * 0.09)
                .clamp(0.0, 1.0);

            let heat_gain = age_factor * 8.0
                + size_factor * 10.0
                + stability_factor * 7.0
                + membrane_factor * 5.0
                + pressure_factor * 18.0;

            cluster.drift_heat = (cluster.drift_heat + heat_gain).clamp(0.0, 100.0);

            if cluster.drift_heat > 62.0 {
                cluster.archetype_override = Some(candidate);
                drifted += 1;

                for particle in particles
                    .iter_mut()
                    .filter(|particle| particle.cluster_id == Some(cluster.id))
                {
                    particle.mass = match candidate {
                        Archetype::Leviathan => (particle.mass + 0.014).clamp(0.55, 7.0),
                        Archetype::Architect => (particle.mass + 0.008).clamp(0.55, 7.0),
                        Archetype::Reaper => (particle.mass + 0.006).clamp(0.55, 7.0),
                        Archetype::Harvester => (particle.mass + 0.004).clamp(0.55, 7.0),
                        Archetype::Phantom => (particle.mass - 0.004).clamp(0.45, 7.0),
                        _ => particle.mass,
                    };
                }
            }
        }

        drifted
    }
}

#[derive(Default)]
pub struct ClusterEvents {
    pub births: usize,
    pub merges: usize,
    pub splits: usize,
    pub extinctions: usize,
}

fn cluster_drift_candidate(
    cluster: &Cluster,
    base_archetype: Archetype,
    pressure: ArchetypePressure,
) -> Archetype {
    if cluster.rare_trait == RareTrait::ElderCore || cluster.size > 86 {
        return Archetype::Leviathan;
    }

    if cluster.rare_trait == RareTrait::SporeKing {
        return Archetype::Mycelial;
    }

    if cluster.rare_trait == RareTrait::Voidborne {
        return Archetype::Phantom;
    }

    let candidate = drift_candidate(
        base_archetype,
        cluster.avg_genome,
        cluster.rare_trait,
        cluster.size,
        pressure,
    );

    if cluster.stability > 78.0
        && cluster.membrane > 48.0
        && pressure.matrix_attraction > pressure.matrix_repulsion
        && cluster.avg_genome.bonding > 1.35
    {
        return Archetype::Architect;
    }

    if pressure.matrix_repulsion > 0.42
        && pressure.matrix_pressure > 0.55
        && cluster.avg_genome.volatility > 1.28
        && cluster.speed() > 0.00045
    {
        return Archetype::Hunter;
    }

    if pressure.reaper_urgency > 0.55
        && pressure.harvester_pressure > 0.42
        && cluster.avg_genome.perception > 0.28
        && cluster.avg_genome.volatility > 1.36
    {
        return Archetype::Reaper;
    }

    if pressure.substrate_density > 0.12
        && pressure.matrix_attraction > 0.34
        && pressure.harvester_pressure < 0.55
        && cluster.avg_genome.fertility > 1.42
        && cluster.avg_genome.hunger < 0.018
    {
        return Archetype::Harvester;
    }

    if pressure.recovery_bias > 0.58
        && cluster.avg_genome.fertility > 1.24
        && cluster.avg_genome.bonding > 1.22
    {
        return Archetype::Mycelial;
    }

    candidate
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

    Cluster {
        id: 0,
        species_id: None,
        archetype: None,
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
        last_seen: 0,
        archetype_override: None,
        drift_heat: 0.0,
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
