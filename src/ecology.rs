use crate::app::Environment;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum ZoneKind {
    Nutrient,
    Dead,
    Turbulent,
    Mutagen,
}

impl ZoneKind {
    pub fn glyph(self) -> char {
        match self {
            Self::Nutrient => '+',
            Self::Dead => '×',
            Self::Turbulent => '∴',
            Self::Mutagen => '*',
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Nutrient => "nutrient",
            Self::Dead => "dead",
            Self::Turbulent => "turbulent",
            Self::Mutagen => "mutagen",
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

        for i in 0..8 {
            ecology.spawn_zone(seed, i);
        }

        ecology
    }

    pub fn tick(&mut self, seed: u64, age: u64, env: Environment) {
        for zone in &mut self.zones {
            zone.age += 1;

            let pulse = ((age as f32 / 180.0) + zone.id as f32).sin() * 0.00045;
            zone.radius = (zone.radius + pulse).clamp(0.12, 0.42);
        }

        self.zones.retain(|z| z.age < 3600);

        let spawn_rate = match env {
            Environment::Bloom => 180,
            Environment::Storm => 240,
            Environment::Hunger => 320,
            Environment::Drift => 300,
            Environment::Calm => 420,
        };

        if age % spawn_rate == 0 && self.zones.len() < 14 {
            self.spawn_zone(seed ^ age, age as usize);
        }
    }

    fn spawn_zone(&mut self, seed: u64, salt: usize) {
        let roll = hash(seed, salt, 99) % 100;

        let kind = match roll {
            0..=34 => ZoneKind::Nutrient,
            35..=54 => ZoneKind::Turbulent,
            55..=74 => ZoneKind::Mutagen,
            _ => ZoneKind::Dead,
        };

        let x = normalized(hash(seed, salt, 1));
        let y = normalized(hash(seed, salt, 2));
        let radius = 0.14 + (hash(seed, salt, 3) % 22) as f32 / 100.0;
        let strength = 0.45 + (hash(seed, salt, 4) % 55) as f32 / 100.0;

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
