use crate::{
    particle::{Genome, Particle, RareTrait, Tribe},
    species::{Archetype, SpeciesBank},
};
use serde::{Deserialize, Serialize};

const CLUSTER_WARMUP_AGE: u64 = 180; // ARCHETYPE_FORMATIONS_REAWAKENED
const STRUCTURE_MATURITY_AGE: u64 = 540;
const MIN_CLUSTER_SIZE: usize = 4;

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

        let groups = detect_groups(particles, age);
        let mut next_clusters = Vec::new();
        let mut events = ClusterEvents::default();

        let active_species = species_bank.active_count().max(1) as f32;

        let root_pressure = species_bank
            .species
            .iter()
            .filter(|species| !species.extinct && is_root_adaptive_archetype(species.archetype))
            .count() as f32
            / active_species;

        let corridor_pressure = species_bank
            .species
            .iter()
            .filter(|species| !species.extinct && is_corridor_archetype(species.archetype))
            .count() as f32
            / active_species;

        for group in groups {
            let min_size = if age < CLUSTER_WARMUP_AGE {
                MIN_CLUSTER_SIZE + 5
            } else if age < STRUCTURE_MATURITY_AGE {
                MIN_CLUSTER_SIZE + 2
            } else {
                MIN_CLUSTER_SIZE
            };

            if group.len() < min_size {
                continue;
            }

            let measured = measure_group(&group, particles, age);

            if age < CLUSTER_WARMUP_AGE && measured.radius < 0.018 {
                continue;
            }

            let mut best_match = None;
            let mut best_dist = f32::MAX;

            for existing in &self.clusters {
                let dx = existing.x - measured.x;
                let dy = existing.y - measured.y;
                let dist = (dx * dx + dy * dy).sqrt();

                let match_radius = if age < CLUSTER_WARMUP_AGE {
                    0.24
                } else if age < STRUCTURE_MATURITY_AGE {
                    0.30
                } else {
                    0.36
                };

                if dist < match_radius && dist < best_dist {
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

                let stability_blend = if age < CLUSTER_WARMUP_AGE {
                    0.035
                } else {
                    0.10
                };
                let membrane_blend = if age < STRUCTURE_MATURITY_AGE {
                    0.018
                } else {
                    0.06
                };
                let heat_blend = if age < STRUCTURE_MATURITY_AGE {
                    0.04
                } else {
                    0.10
                };

                current.stability = (old.stability * (1.0 - stability_blend)
                    + current.stability * stability_blend)
                    .clamp(0.0, 100.0);
                current.membrane = (old.membrane * (1.0 - membrane_blend)
                    + current.membrane * membrane_blend)
                    .clamp(0.0, 100.0);
                current.drift_heat = (old.drift_heat * (1.0 - heat_blend)
                    + current.drift_heat * heat_blend)
                    .clamp(0.0, 100.0);
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

            if age >= CLUSTER_WARMUP_AGE {
                let local_root_adaptation =
                    root_adaptation_score(cluster.avg_genome, cluster.rare_trait, cluster.size);
                let local_corridor_score = corridor_score(
                    cluster.avg_genome,
                    cluster.archetype.unwrap_or(Archetype::Parasite),
                );
                let local_drift_pressure = drift_pressure_score(
                    cluster.avg_genome,
                    cluster.archetype.unwrap_or(Archetype::Parasite),
                    local_root_adaptation,
                    local_corridor_score,
                );

                apply_cluster_drift(
                    &mut cluster,
                    root_pressure,
                    corridor_pressure,
                    local_root_adaptation,
                    local_corridor_score,
                    local_drift_pressure,
                    age,
                );
            } else {
                cluster.archetype_override = None;
                cluster.drift_heat = (cluster.drift_heat * 0.72).clamp(0.0, 100.0);
            }

            if age >= STRUCTURE_MATURITY_AGE && cluster.age > 95 && cluster.size > 18 {
                cluster.membrane = (cluster.membrane + 0.55).min(100.0);
            }

            if age >= STRUCTURE_MATURITY_AGE && cluster.stability > 76.0 && cluster.size > 30 {
                cluster.membrane = (cluster.membrane + 0.42).min(100.0);
            }

            let territorial_anchor = cluster_territorial_anchor(&cluster, age);
            apply_colony_emergence_pressure(&mut cluster, territorial_anchor, age);

            for &idx in &group {
                if let Some(particle) = particles.get_mut(idx) {
                    particle.cluster_id = Some(cluster.id);
                    particle.species_id = Some(species_id);

                    let maturity = if age < CLUSTER_WARMUP_AGE {
                        0.0
                    } else if age < STRUCTURE_MATURITY_AGE {
                        ((age - CLUSTER_WARMUP_AGE) as f32
                            / (STRUCTURE_MATURITY_AGE - CLUSTER_WARMUP_AGE) as f32)
                            .clamp(0.0, 1.0)
                    } else {
                        1.0
                    };

                    let mass_gain = 0.0018 * cluster.size as f32 * maturity;
                    particle.mass = (particle.mass + mass_gain).clamp(0.55, 6.5);

                    if territorial_anchor > 0.0 {
                        let dx = particle.x - cluster.x;
                        let dy = particle.y - cluster.y;
                        let orbit_x = -dy * territorial_anchor * 0.0018;
                        let orbit_y = dx * territorial_anchor * 0.0018;
                        let settle = 1.0 - territorial_anchor * 0.035;

                        particle.vx = particle.vx * settle + orbit_x;
                        particle.vy = particle.vy * settle + orbit_y;

                        particle.genome.membrane =
                            (particle.genome.membrane + territorial_anchor * 0.00018).clamp(0.0, 1.8);
                        particle.genome.bonding =
                            (particle.genome.bonding + territorial_anchor * 0.00010).clamp(0.5, 2.25);
                    }


                    if cluster.archetype_override.is_some() && age >= STRUCTURE_MATURITY_AGE {
                        particle.genome.perception =
                            (particle.genome.perception + 0.00005).clamp(0.1, 0.38);
                        particle.genome.orbit = (particle.genome.orbit + 0.00008).clamp(0.0, 1.55);
                    }
                }
            }

            next_clusters.push(cluster);
        }

        let old_count = self.clusters.len();
        let new_count = next_clusters.len();

        if age >= CLUSTER_WARMUP_AGE && new_count < old_count {
            events.merges += old_count - new_count;
        }

        if age >= CLUSTER_WARMUP_AGE && new_count > old_count + events.births {
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

fn apply_colony_emergence_pressure(
    cluster: &mut Cluster,
    territorial_anchor: f32,
    world_age: u64,
) {
    if world_age < STRUCTURE_MATURITY_AGE || cluster.age < 96 {
        return;
    }

    let age_memory = (cluster.age as f32 / 2_400.0).clamp(0.0, 1.0);
    let density = (cluster.size as f32 / 54.0).clamp(0.0, 1.0);
    let stability = (cluster.stability / 100.0).clamp(0.0, 1.0);
    let membrane = (cluster.membrane / 100.0).clamp(0.0, 1.0);
    let speed = (cluster.speed() * 220.0).clamp(0.0, 1.0);

    let settled_colony = (stability * 0.36
        + membrane * 0.22
        + density * 0.18
        + age_memory * 0.18
        + territorial_anchor * 0.20
        - speed * 0.16)
        .clamp(0.0, 1.0);

    let migration_front = (speed * 0.34
        + cluster.avg_genome.orbit * 0.18
        + cluster.avg_genome.volatility * 0.12
        + density * 0.10
        - stability * 0.10)
        .clamp(0.0, 1.0);

    let builder_pressure = (settled_colony * 0.42
        + cluster.avg_genome.membrane * 0.22
        + cluster.avg_genome.bonding * 0.10
        + age_memory * 0.12)
        .clamp(0.0, 1.0);

    if settled_colony > 0.42 {
        cluster.stability = (cluster.stability + settled_colony * 0.42).clamp(0.0, 100.0);
        cluster.membrane = (cluster.membrane + builder_pressure * 0.34).clamp(0.0, 100.0);
        cluster.drift_heat = (cluster.drift_heat - settled_colony * 0.28).clamp(0.0, 100.0);
    }

    if migration_front > 0.46 {
        cluster.drift_heat = (cluster.drift_heat + migration_front * 0.52).clamp(0.0, 100.0);
        cluster.stability = (cluster.stability + migration_front * 0.08).clamp(0.0, 100.0);
    }

    if builder_pressure > 0.55 && cluster.size >= 12 {
        cluster.membrane = (cluster.membrane + builder_pressure * 0.46).clamp(0.0, 100.0);
    }
}

fn cluster_territorial_anchor(cluster: &Cluster, world_age: u64) -> f32 {
    if world_age < STRUCTURE_MATURITY_AGE || cluster.age < 72 {
        return 0.0;
    }

    let maturity =
        ((world_age.saturating_sub(STRUCTURE_MATURITY_AGE)) as f32 / 2_400.0).clamp(0.0, 1.0);
    let age_memory = (cluster.age as f32 / 1_400.0).clamp(0.0, 1.0);
    let stability = (cluster.stability / 100.0).clamp(0.0, 1.0);
    let membrane = (cluster.membrane / 100.0).clamp(0.0, 1.0);
    let drift = (cluster.drift_heat / 100.0).clamp(0.0, 1.0);
    let density = (cluster.size as f32 / 46.0).clamp(0.0, 1.0);
    let mobility = root_mobility_score(cluster.avg_genome);

    (stability * 0.34
        + membrane * 0.22
        + density * 0.18
        + age_memory * 0.18
        + maturity * 0.16
        - drift * 0.20
        - mobility * 0.08)
        .clamp(0.0, 0.42)
}


fn apply_cluster_drift(
    cluster: &mut Cluster,
    root_pressure: f32,
    corridor_pressure: f32,
    local_root_adaptation: f32,
    local_corridor_score: f32,
    local_drift_pressure: f32,
    world_age: u64,
) {
    let base = cluster.archetype;
    let mobility = root_mobility_score(cluster.avg_genome);
    let density = (cluster.size as f32 / 32.0).clamp(0.0, 1.0); // CLUSTER_DRIFT_REACTIVATED
    let maturity =
        ((world_age.saturating_sub(CLUSTER_WARMUP_AGE)) as f32 / 1_800.0).clamp(0.0, 1.0);

    let pressure = (root_pressure * 0.24
        + corridor_pressure * 0.24
        + local_root_adaptation * 0.22
        + local_corridor_score * 0.18
        + local_drift_pressure * 0.12)
        .clamp(0.0, 1.0);

    let heat_target =
        (pressure * 62.0 + mobility * 16.0 + density * 8.0).clamp(0.0, 100.0) * maturity;
    let heat_target = heat_target + density * 10.8;
    let delta = heat_target - cluster.drift_heat;
    cluster.drift_heat = (cluster.drift_heat + delta * 0.35).clamp(0.0, 100.0);

    cluster.archetype_override = if cluster.drift_heat > 68.0 && pressure > 0.52 && maturity > 0.35
    {
        match base {
            Some(Archetype::Harvester) => {
                if cluster.avg_genome.orbit > 0.48 || local_corridor_score > 0.62 {
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
                if local_corridor_score > 0.72 && cluster.avg_genome.orbit > 0.58 {
                    Some(Archetype::Orbiter)
                } else {
                    None
                }
            }
            Some(Archetype::Hunter) => {
                if local_drift_pressure > 0.72
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
    } else if cluster.drift_heat < 34.0 {
        None
    } else {
        cluster.archetype_override
    };
}

fn is_root_adaptive_archetype(archetype: Archetype) -> bool {
    matches!(
        archetype,
        Archetype::Swarmer
            | Archetype::Orbiter
            | Archetype::Architect
            | Archetype::Leviathan
            | Archetype::Phantom
    )
}

fn is_corridor_archetype(archetype: Archetype) -> bool {
    matches!(
        archetype,
        Archetype::Swarmer
            | Archetype::Orbiter
            | Archetype::Architect
            | Archetype::Hunter
            | Archetype::Phantom
    )
}

fn root_adaptation_score(genome: Genome, rare_trait: RareTrait, size: usize) -> f32 {
    let perception = ((genome.perception - 0.18) / 0.20).clamp(0.0, 1.0);
    let orbit = (genome.orbit / 1.35).clamp(0.0, 1.0);
    let volatility = ((genome.volatility - 0.72) / 1.05).clamp(0.0, 1.0);
    let membrane = (genome.membrane / 1.45).clamp(0.0, 1.0);
    let mass_adaptation = (size as f32 / 90.0).clamp(0.0, 1.0) * 0.16;

    let rare_bonus = match rare_trait {
        RareTrait::Voidborne => 0.18,
        RareTrait::ElderCore => 0.12,
        RareTrait::SymbioticCore => 0.10,
        RareTrait::SporeKing => 0.08,
        RareTrait::Radiant => 0.04,
        RareTrait::Voracious => 0.03,
        RareTrait::Devourer => -0.08,
        RareTrait::None => 0.0,
    };

    (perception * 0.30
        + orbit * 0.26
        + volatility * 0.16
        + membrane * 0.16
        + mass_adaptation
        + rare_bonus)
        .clamp(0.0, 1.0)
}

fn corridor_score(genome: Genome, archetype: Archetype) -> f32 {
    let mobility =
        (genome.orbit * 0.42 + genome.volatility * 0.22 + genome.perception * 1.15).clamp(0.0, 1.0);

    let compactness = (1.0 - ((genome.bonding - 1.15).abs() / 1.25)).clamp(0.0, 1.0);
    let hunger_control = (1.0 - ((genome.hunger - 0.017).abs() / 0.026)).clamp(0.0, 1.0);

    let archetype_bonus = match archetype {
        Archetype::Orbiter => 0.18,
        Archetype::Swarmer => 0.13,
        Archetype::Architect => 0.12,
        Archetype::Phantom => 0.10,
        Archetype::Hunter => 0.06,
        Archetype::Reaper => 0.04,
        Archetype::Leviathan => 0.04,
        Archetype::Grazer => 0.02,
        Archetype::Mycelial => 0.02,
        Archetype::Parasite => 0.0,
        Archetype::Harvester => -0.08,
    };

    (mobility * 0.50 + compactness * 0.26 + hunger_control * 0.16 + archetype_bonus).clamp(0.0, 1.0)
}

fn drift_pressure_score(
    genome: Genome,
    archetype: Archetype,
    root_adaptation: f32,
    corridor_score: f32,
) -> f32 {
    let volatility_pressure = ((genome.volatility - 1.0) / 0.95).clamp(0.0, 1.0);
    let specialization_pressure = match archetype {
        Archetype::Harvester => 0.18,
        Archetype::Reaper => 0.12,
        Archetype::Leviathan => 0.10,
        Archetype::Phantom => 0.10,
        Archetype::Architect => 0.08,
        _ => 0.04,
    };

    (volatility_pressure * 0.30
        + root_adaptation * 0.30
        + corridor_score * 0.26
        + specialization_pressure)
        .clamp(0.0, 1.0)
}

fn root_mobility_score(genome: Genome) -> f32 {
    let perception = ((genome.perception - 0.18) / 0.20).clamp(0.0, 1.0);
    let orbit = (genome.orbit / 1.35).clamp(0.0, 1.0);
    let volatility = ((genome.volatility - 0.70) / 1.05).clamp(0.0, 1.0);
    let membrane = (genome.membrane / 1.45).clamp(0.0, 1.0);

    (perception * 0.36 + orbit * 0.32 + volatility * 0.16 + membrane * 0.16).clamp(0.0, 1.0)
}

fn detect_groups(particles: &[Particle], age: u64) -> Vec<Vec<usize>> {
    let mut visited = vec![false; particles.len()];
    let mut groups = Vec::new();

    let warmup = (age as f32 / CLUSTER_WARMUP_AGE as f32).clamp(0.0, 1.0);
    let maturity = (age as f32 / STRUCTURE_MATURITY_AGE as f32).clamp(0.0, 1.0);

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

                let early_link_drag = 0.54 + warmup * 0.46;
                let structure_drag = 0.72 + maturity * 0.28;

                let link =
                    (0.050 + particles[idx].genome.bonding * 0.012 + particles[idx].mass * 0.0025)
                        * early_link_drag
                        * structure_drag;

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

fn measure_group(indices: &[usize], particles: &[Particle], age: u64) -> Cluster {
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

    let maturity = (age as f32 / STRUCTURE_MATURITY_AGE as f32).clamp(0.0, 1.0);
    membrane = (membrane / count * 78.0 * maturity).clamp(0.0, 100.0);

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

    let warmup = (age as f32 / CLUSTER_WARMUP_AGE as f32).clamp(0.0, 1.0);
    let raw_stability = ((indices.len() as f32 * 4.8) - radius * 95.0).clamp(0.0, 100.0);
    let stability = (raw_stability * (0.35 + warmup * 0.65)).clamp(0.0, 100.0);
    let drift_heat = root_mobility_score(genome) * 28.0 * maturity + stability * 0.11;

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
