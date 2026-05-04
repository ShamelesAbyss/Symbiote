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

    #[serde(default)]
    pub root_adaptation: f32,
    #[serde(default)]
    pub corridor_score: f32,
    #[serde(default)]
    pub drift_pressure: f32,
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
        let root_adaptation = root_adaptation_score(genome, rare_trait, size);
        let corridor_score = corridor_score(genome, archetype);
        let drift_pressure =
            species_drift_pressure(genome, archetype, root_adaptation, corridor_score);

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

            species.root_adaptation =
                (species.root_adaptation * 0.92 + root_adaptation * 0.08).clamp(0.0, 1.0);
            species.corridor_score =
                (species.corridor_score * 0.92 + corridor_score * 0.08).clamp(0.0, 1.0);
            species.drift_pressure =
                (species.drift_pressure * 0.90 + drift_pressure * 0.10).clamp(0.0, 1.0);

            species.archetype = derive_adaptive_archetype(
                species.genome,
                species.rare_trait,
                species.peak_size,
                species.root_adaptation,
                species.corridor_score,
                species.drift_pressure,
            );

            return species.id;
        }

        let id = self.next_id;
        self.next_id += 1;

        let name = species_name(archetype, rare_trait, id, root_adaptation, corridor_score);

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
            root_adaptation,
            corridor_score,
            drift_pressure,
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
            let stale_limit = if species.root_adaptation > 0.62 || species.corridor_score > 0.62 {
                4200
            } else {
                3200
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
    let root_adaptation = root_adaptation_score(genome, rare_trait, size);
    let base = derive_base_archetype(genome, rare_trait, size);
    let corridor = corridor_score(genome, base);
    let drift = species_drift_pressure(genome, base, root_adaptation, corridor);

    derive_adaptive_archetype(genome, rare_trait, size, root_adaptation, corridor, drift)
}

fn derive_base_archetype(genome: Genome, rare_trait: RareTrait, size: usize) -> Archetype {
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

fn derive_adaptive_archetype(
    genome: Genome,
    rare_trait: RareTrait,
    size: usize,
    root_adaptation: f32,
    corridor_score: f32,
    drift_pressure: f32,
) -> Archetype {
    let base = derive_base_archetype(genome, rare_trait, size);

    if base == Archetype::Reaper {
        return Archetype::Reaper;
    }

    if rare_trait == RareTrait::Voidborne {
        return Archetype::Phantom;
    }

    if root_adaptation > 0.74 && corridor_score > 0.54 {
        if genome.orbit > 0.66 || genome.perception > 0.31 {
            return Archetype::Orbiter;
        }

        if genome.membrane > 0.86 && genome.bonding > 1.12 {
            return Archetype::Architect;
        }

        if genome.bonding > 1.35 {
            return Archetype::Swarmer;
        }
    }

    if drift_pressure > 0.68 && root_adaptation > 0.58 {
        if genome.volatility > 1.38 && genome.hunger > 0.018 {
            return Archetype::Hunter;
        }

        if genome.orbit > 0.55 {
            return Archetype::Orbiter;
        }

        if genome.membrane > 0.98 {
            return Archetype::Architect;
        }
    }

    if base == Archetype::Harvester && root_adaptation > 0.58 {
        if corridor_score > 0.62 && genome.orbit > 0.45 {
            return Archetype::Orbiter;
        }

        if genome.bonding > 1.45 && genome.fertility < 1.75 {
            return Archetype::Swarmer;
        }
    }

    if base == Archetype::Grazer && root_adaptation > 0.66 && genome.membrane > 0.92 {
        return Archetype::Architect;
    }

    base
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

fn species_drift_pressure(
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

fn species_name(
    archetype: Archetype,
    rare_trait: RareTrait,
    id: u64,
    root_adaptation: f32,
    corridor_score: f32,
) -> String {
    let prefix = if root_adaptation > 0.72 {
        "ROOT"
    } else if corridor_score > 0.68 {
        "PATH"
    } else if root_adaptation > 0.54 || corridor_score > 0.54 {
        "DRF"
    } else {
        ""
    };

    if rare_trait == RareTrait::None {
        if prefix.is_empty() {
            format!("{}-{}", archetype.short(), id)
        } else {
            format!("{}-{}-{}", prefix, archetype.short(), id)
        }
    } else if prefix.is_empty() {
        format!("{}-{}-{}", rare_trait.short(), archetype.short(), id)
    } else {
        format!(
            "{}-{}-{}-{}",
            prefix,
            rare_trait.short(),
            archetype.short(),
            id
        )
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
