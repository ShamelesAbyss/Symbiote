use crate::particle::{Genome, RareTrait, Tribe};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum Archetype {
    Swarmer,
    Hunter,
    Grazer,
    Orbiter,
    Parasite,
    Architect,
    Leviathan,
    Mycelial,
    Phantom,
    Harvester,
    Reaper,
}

impl Archetype {
    pub fn name(self) -> &'static str {
        match self {
            Self::Swarmer => "Swarmer",
            Self::Hunter => "Hunter",
            Self::Grazer => "Grazer",
            Self::Orbiter => "Orbiter",
            Self::Parasite => "Parasite",
            Self::Architect => "Architect",
            Self::Leviathan => "Leviathan",
            Self::Mycelial => "Mycelial",
            Self::Phantom => "Phantom",
            Self::Harvester => "Harvester",
            Self::Reaper => "Reaper",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Swarmer => "SWR",
            Self::Hunter => "HNT",
            Self::Grazer => "GRZ",
            Self::Orbiter => "ORB",
            Self::Parasite => "PAR",
            Self::Architect => "ARC",
            Self::Leviathan => "LEV",
            Self::Mycelial => "MYC",
            Self::Phantom => "PHM",
            Self::Harvester => "HRV",
            Self::Reaper => "RPR",
        }
    }

    pub fn index(self) -> usize {
        match self {
            Self::Swarmer => 0,
            Self::Hunter => 1,
            Self::Grazer => 2,
            Self::Orbiter => 3,
            Self::Parasite => 4,
            Self::Architect => 5,
            Self::Leviathan => 6,
            Self::Mycelial => 7,
            Self::Phantom => 8,
            Self::Harvester => 9,
            Self::Reaper => 10,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Species {
    pub id: u64,
    pub parent_id: Option<u64>,
    pub name: String,
    pub dominant_tribe: Tribe,
    pub archetype: Archetype,
    pub rare_trait: RareTrait,
    pub genome: Genome,
    pub created_at_age: u64,
    pub last_seen_age: u64,
    pub peak_size: usize,
    pub sightings: u64,
    pub descendants: u64,
    pub births: u64,
    pub extinct: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SpeciesBank {
    pub species: Vec<Species>,
    pub next_id: u64,
}

impl SpeciesBank {
    pub fn new() -> Self {
        Self {
            species: Vec::new(),
            next_id: 1,
        }
    }

    pub fn assign_or_create(
        &mut self,
        dominant: Tribe,
        genome: Genome,
        rare_trait: RareTrait,
        size: usize,
        age: u64,
        parent_hint: Option<u64>,
    ) -> u64 {
        let archetype = derive_archetype(genome, rare_trait, size);
        let mut best_index = None;
        let mut best_score = f32::MAX;
        let match_threshold = species_match_threshold(genome, rare_trait, size);

        for (idx, species) in self.species.iter().enumerate() {
            if species.extinct || species.dominant_tribe != dominant {
                continue;
            }

            let score = genome_distance(species.genome, genome);

            if score < match_threshold && score < best_score {
                best_score = score;
                best_index = Some(idx);
            }
        }

        if let Some(idx) = best_index {
            let species = &mut self.species[idx];

            species.last_seen_age = age;
            species.peak_size = species.peak_size.max(size);
            species.sightings += 1;
            species.genome = blend_genome(species.genome, genome);

            if species.rare_trait == RareTrait::None && rare_trait != RareTrait::None {
                species.rare_trait = rare_trait;
            }

            let derived = derive_archetype(species.genome, species.rare_trait, species.peak_size);
            species.archetype = stabilize_archetype(species.archetype, derived, species.sightings);
            species.name = rename_species(species.rare_trait, species.archetype, species.id);

            return species.id;
        }

        let id = self.next_id;
        self.next_id += 1;

        let name = rename_species(rare_trait, archetype, id);

        if let Some(parent_id) = parent_hint {
            if let Some(parent) = self
                .species
                .iter_mut()
                .find(|species| species.id == parent_id)
            {
                parent.descendants += 1;
            }
        }

        self.species.push(Species {
            id,
            parent_id: parent_hint,
            name,
            dominant_tribe: dominant,
            archetype,
            rare_trait,
            genome,
            created_at_age: age,
            last_seen_age: age,
            peak_size: size,
            sightings: 1,
            descendants: 0,
            births: 0,
            extinct: false,
        });

        id
    }

    pub fn record_birth(&mut self, species_id: Option<u64>) {
        if let Some(id) = species_id {
            if let Some(species) = self.species.iter_mut().find(|species| species.id == id) {
                species.births += 1;
            }
        }
    }

    pub fn mark_extinctions(&mut self, age: u64) -> usize {
        let mut count = 0;

        for species in &mut self.species {
            let stale_limit = match species.archetype {
                Archetype::Harvester | Archetype::Reaper => 4_200,
                Archetype::Leviathan | Archetype::Phantom => 3_800,
                _ => 3_200,
            };

            if !species.extinct && age.saturating_sub(species.last_seen_age) > stale_limit {
                species.extinct = true;
                count += 1;
            }
        }

        count
    }

    pub fn active_count(&self) -> usize {
        self.species
            .iter()
            .filter(|species| !species.extinct)
            .count()
    }
}

pub fn derive_archetype(genome: Genome, rare_trait: RareTrait, size: usize) -> Archetype {
    let pressure = conway_species_pressure(genome, rare_trait, size);

    if is_harvester_genome(genome, rare_trait) {
        Archetype::Harvester
    } else if is_reaper_genome(genome) {
        Archetype::Reaper
    } else if rare_trait == RareTrait::Voidborne {
        Archetype::Phantom
    } else if rare_trait == RareTrait::SporeKing {
        Archetype::Mycelial
    } else if rare_trait == RareTrait::ElderCore
        || size > 58
        || (genome.membrane > 1.24 && genome.bonding > 1.18 && genome.fertility > 0.88)
    {
        Archetype::Leviathan
    } else if genome.membrane > 0.82 && genome.bonding > 0.92 && size > 6 {
        Archetype::Architect
    } else if genome.membrane > 0.52 && genome.fertility > 0.78 && genome.metabolism < 0.040 {
        Archetype::Mycelial
    } else if pressure.birth && genome.membrane > 0.48 && genome.fertility > 0.76 {
        Archetype::Mycelial
    } else if pressure.birth && genome.orbit > 0.58 && genome.perception > 0.182 {
        Archetype::Orbiter
    } else if pressure.birth && genome.bonding > 0.84 {
        Archetype::Swarmer
    } else if genome.bonding > 1.02 && genome.volatility < 1.55 && genome.hunger < 0.026 {
        Archetype::Swarmer
    } else if pressure.overcrowded && genome.volatility > 1.22 && genome.hunger > 0.015 {
        Archetype::Hunter
    } else if pressure.overcrowded && genome.orbit > 0.80 {
        Archetype::Phantom
    } else if pressure.underpopulated && genome.perception > 0.245 && genome.metabolism < 0.020 {
        Archetype::Grazer
    } else if pressure.underpopulated {
        Archetype::Parasite
    } else if genome.membrane > 0.56 && genome.bonding > 0.62 {
        Archetype::Architect
    } else if genome.orbit > 0.58 && genome.perception > 0.12 {
        Archetype::Orbiter
    } else if genome.volatility > 1.34 && genome.metabolism > 0.018 {
        Archetype::Hunter
    } else if genome.bonding > 1.20 {
        Archetype::Swarmer
    } else if genome.perception > 0.238 && genome.metabolism < 0.021 {
        Archetype::Grazer
    } else if genome.membrane > 0.46 && genome.fertility > 0.74 {
        Archetype::Mycelial
    } else if genome.orbit > 0.98 || rare_trait == RareTrait::Radiant {
        Archetype::Phantom
    } else {
        Archetype::Parasite
    }
}

#[derive(Clone, Copy, Debug)]
struct SpeciesConwayPressure {
    active_neighbors: u8,
    underpopulated: bool,
    survives: bool,
    birth: bool,
    overcrowded: bool,
}

fn conway_species_pressure(
    genome: Genome,
    rare_trait: RareTrait,
    size: usize,
) -> SpeciesConwayPressure {
    let mut active = 0u8;

    if genome.perception > 0.245 {
        active += 1;
    }

    if genome.hunger > 0.017 {
        active += 1;
    }

    if genome.bonding > 1.08 {
        active += 1;
    }

    if genome.volatility > 1.18 {
        active += 1;
    }

    if genome.orbit > 0.72 {
        active += 1;
    }

    if genome.membrane > 0.66 {
        active += 1;
    }

    if genome.metabolism > 0.018 {
        active += 1;
    }

    if genome.fertility > 1.10 {
        active += 1;
    }

    if rare_trait != RareTrait::None {
        active = active.saturating_add(1).min(8);
    }

    if size > 34 {
        active = active.saturating_add(1).min(8);
    }

    SpeciesConwayPressure {
        active_neighbors: active,
        underpopulated: active < 2,
        survives: active == 2 || active == 3,
        birth: active == 3,
        overcrowded: active > 3,
    }
}

fn species_match_threshold(genome: Genome, rare_trait: RareTrait, size: usize) -> f32 {
    let pressure = conway_species_pressure(genome, rare_trait, size);

    if pressure.active_neighbors == 0 {
        0.385
    } else if pressure.birth {
        0.275
    } else if pressure.overcrowded {
        0.295
    } else if pressure.survives {
        0.365
    } else if pressure.underpopulated {
        0.325
    } else {
        0.34
    }
}

fn is_reaper_genome(genome: Genome) -> bool {
    let predator_drive = genome.volatility > 1.26 && genome.hunger > 0.0158;
    let perception_gate = genome.perception > 0.235;
    let fertility_gate = genome.fertility < 2.05;
    let metabolic_edge =
        genome.metabolism > 0.0145 || genome.volatility > 1.44 || genome.hunger > 0.022;

    predator_drive && perception_gate && fertility_gate && metabolic_edge
}

fn is_harvester_genome(genome: Genome, rare_trait: RareTrait) -> bool {
    if rare_trait == RareTrait::Devourer {
        genome.perception > 0.225 && genome.fertility > 0.90 && genome.hunger < 0.030
    } else {
        let forager_drive = genome.perception > 0.224 && genome.fertility > 0.92;
        let hunger_gate = genome.hunger < 0.030;
        let metabolic_gate = genome.metabolism < 0.042;
        let volatility_gate = genome.volatility < 1.92;

        forager_drive && hunger_gate && metabolic_gate && volatility_gate
    }
}

fn stabilize_archetype(current: Archetype, derived: Archetype, sightings: u64) -> Archetype {
    if current == derived {
        return current;
    }

    if sightings < 2 {
        return current;
    }

    if derived == Archetype::Harvester && sightings >= 3 {
        return derived;
    }

    if derived == Archetype::Reaper && sightings >= 3 {
        return derived;
    }

    if matches!(current, Archetype::Reaper | Archetype::Leviathan) && sightings < 5 {
        return current;
    }

    if current == Archetype::Harvester && sightings < 5 {
        return current;
    }

    if matches!(
        derived,
        Archetype::Architect | Archetype::Orbiter | Archetype::Mycelial | Archetype::Phantom
    ) && sightings >= 4
    {
        return derived;
    }

    derived
}

fn rename_species(rare_trait: RareTrait, archetype: Archetype, id: u64) -> String {
    if rare_trait == RareTrait::None {
        format!("{}-{}", archetype.short(), id)
    } else {
        format!("{}-{}-{}", rare_trait.short(), archetype.short(), id)
    }
}

fn genome_distance(a: Genome, b: Genome) -> f32 {
    let perception = (a.perception - b.perception).abs() * 3.0;
    let hunger = (a.hunger - b.hunger).abs() * 18.0;
    let bonding = (a.bonding - b.bonding).abs() * 0.55;
    let volatility = (a.volatility - b.volatility).abs() * 0.55;
    let orbit = (a.orbit - b.orbit).abs() * 0.45;
    let membrane = (a.membrane - b.membrane).abs() * 0.45;
    let metabolism = (a.metabolism - b.metabolism).abs() * 16.0;
    let fertility = (a.fertility - b.fertility).abs() * 2.0;

    perception + hunger + bonding + volatility + orbit + membrane + metabolism + fertility
}

fn blend_genome(a: Genome, b: Genome) -> Genome {
    Genome {
        perception: a.perception * 0.94 + b.perception * 0.06,
        hunger: a.hunger * 0.94 + b.hunger * 0.06,
        bonding: a.bonding * 0.94 + b.bonding * 0.06,
        volatility: a.volatility * 0.94 + b.volatility * 0.06,
        orbit: a.orbit * 0.94 + b.orbit * 0.06,
        membrane: a.membrane * 0.94 + b.membrane * 0.06,
        metabolism: a.metabolism * 0.94 + b.metabolism * 0.06,
        fertility: a.fertility * 0.94 + b.fertility * 0.06,
    }
}
