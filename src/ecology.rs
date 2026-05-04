use crate::app::Environment;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ZoneKind {
    Nutrient,
    Dead,
    Turbulent,
    Mutagen,
    Nest,
}

impl ZoneKind {
    pub fn glyph(self) -> char {
        match self {
            Self::Nutrient => '+',
            Self::Dead => '×',
            Self::Turbulent => '∴',
            Self::Mutagen => '*',
            Self::Nest => '◎',
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Nutrient => "nutrient",
            Self::Dead => "dead",
            Self::Turbulent => "turbulent",
            Self::Mutagen => "mutagen",
            Self::Nest => "nest",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct EcologyZone {
    pub id: u64,
    pub kind: ZoneKind,
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub strength: f32,
    pub age: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Ecology {
    pub zones: Vec<EcologyZone>,
    pub next_id: u64,
}

impl Ecology {
    pub fn new(seed: u64) -> Self {
        let mut ecology = Self {
            zones: Vec::new(),
            next_id: 1,
        };

        for i in 0..9 {
            ecology.spawn_zone(seed, i);
        }

        ecology
    }

    pub fn tick(&mut self, seed: u64, age: u64, env: Environment) {
        for zone in &mut self.zones {
            zone.age += 1;

            let pulse = ((age as f32 / 900.0) + zone.id as f32).sin() * 0.00008;
            zone.radius = (zone.radius + pulse).clamp(0.14, 0.46);
        }

        self.zones.retain(|zone| zone.age < 24_000);

        let spawn_rate = match env {
            Environment::Bloom => 2600,
            Environment::Storm => 3200,
            Environment::Hunger => 4200,
            Environment::Drift => 3800,
            Environment::Calm => 5200,
        };

        if age % spawn_rate == 0 && self.zones.len() < 16 {
            self.spawn_zone(seed ^ age, age as usize);
        }
    }

    fn spawn_zone(&mut self, seed: u64, salt: usize) {
        let roll = hash(seed, salt, 99) % 100;

        let kind = match roll {
            0..=32 => ZoneKind::Nutrient,
            33..=47 => ZoneKind::Turbulent,
            48..=61 => ZoneKind::Mutagen,
            62..=78 => ZoneKind::Nest,
            _ => ZoneKind::Dead,
        };

        let x = normalized(hash(seed, salt, 1));
        let y = normalized(hash(seed, salt, 2));
        let radius = 0.16 + (hash(seed, salt, 3) % 25) as f32 / 100.0;
        let strength = 0.5 + (hash(seed, salt, 4) % 50) as f32 / 100.0;

        self.zones.push(EcologyZone {
            id: self.next_id,
            kind,
            x,
            y,
            radius,
            strength,
            age: 0,
        });

        self.next_id += 1;
    }
}

fn normalized(value: usize) -> f32 {
    ((value % 2000) as f32 / 1000.0) - 1.0
}

fn hash(seed: u64, x: usize, y: usize) -> usize {
    let mut value = seed as usize;
    value ^= x.wrapping_mul(374_761_393);
    value ^= y.wrapping_mul(668_265_263);
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    value ^ (value >> 16)
}
