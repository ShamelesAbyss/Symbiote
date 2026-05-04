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

        for (idx, species) in self.species.iter().enumerate() {
            if species.extinct || species.dominant_tribe != dominant {
                continue;
            }

            let score = genome_distance(species.genome, genome);

            if score < 0.34 && score < best_score {
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

            species.archetype =
                derive_archetype(species.genome, species.rare_trait, species.peak_size);

            return species.id;
        }

        let id = self.next_id;
        self.next_id += 1;

        let name = if rare_trait == RareTrait::None {
            format!("{}-{}", archetype.short(), id)
        } else {
            format!("{}-{}-{}", rare_trait.short(), archetype.short(), id)
        };

        if let Some(parent_id) = parent_hint {
            if let Some(parent) = self.species.iter_mut().find(|s| s.id == parent_id) {
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
            if let Some(species) = self.species.iter_mut().find(|s| s.id == id) {
                species.births += 1;
            }
        }
    }

    pub fn mark_extinctions(&mut self, age: u64) -> usize {
        let mut count = 0;

        for species in &mut self.species {
            if !species.extinct && age.saturating_sub(species.last_seen_age) > 3200 {
                species.extinct = true;
                count += 1;
            }
        }

        count
    }

    pub fn active_count(&self) -> usize {
        self.species.iter().filter(|s| !s.extinct).count()
    }
}

pub fn derive_archetype(genome: Genome, rare_trait: RareTrait, size: usize) -> Archetype {
    // Reaper checked first so the predator counter can appear before Harvesters dominate.
    if genome.volatility > 1.54
        && genome.perception > 0.295
        && genome.hunger > 0.019
        && genome.fertility < 1.45
    {
        Archetype::Reaper
    } else if rare_trait == RareTrait::Voidborne {
        Archetype::Phantom
    } else if rare_trait == RareTrait::SporeKing {
        Archetype::Mycelial
    } else if rare_trait == RareTrait::ElderCore || size > 78 {
        Archetype::Leviathan
    } else if rare_trait == RareTrait::Devourer
        && genome.perception > 0.315
        && genome.fertility > 1.45
        && genome.hunger < 0.016
    {
        Archetype::Harvester
    } else if genome.perception > 0.345
        && genome.fertility > 1.68
        && genome.hunger < 0.012
        && genome.metabolism < 0.023
    {
        Archetype::Harvester
    } else if genome.orbit > 0.95 {
        Archetype::Orbiter
    } else if genome.membrane > 1.1 && genome.bonding > 1.35 {
        Archetype::Architect
    } else if genome.volatility > 1.45 && genome.metabolism > 0.022 {
        Archetype::Hunter
    } else if genome.bonding > 1.65 {
        Archetype::Swarmer
    } else if genome.perception > 0.28 && genome.metabolism < 0.016 {
        Archetype::Grazer
    } else {
        Archetype::Parasite
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
