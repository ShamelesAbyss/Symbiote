#![allow(dead_code)]
//! Smarticle Morphogenesis
//!
//! Phase 1 foundation file.
//!
//! This module is intentionally standalone right now.
//! It is NOT plugged into the build yet.
//!
//! Purpose:
//! - seeded cell-kind interaction rules
//! - invisible push/pull morphogenesis math
//! - no rendering
//! - no UI
//! - no dependencies
//! - no automata behavior changes
//!
//! Later, automata.rs can map CellKind -> SmarticleRole and sample these
//! rules to make substrate cells form more organic living structures.

pub const SMARTICLE_ROLE_COUNT: usize = 7;

const POWER_MIN: f32 = -1.0;
const POWER_MAX: f32 = 1.0;
const RADIUS_MIN: f32 = 1.0;
const RADIUS_MAX: f32 = 5.0;
const POWER_CURVE: f32 = 1.25;
const RADIUS_CURVE: f32 = 1.10;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SmarticleRole {
    Life,
    Nutrient,
    Dead,
    Mutagen,
    Nest,
    Spore,
    Root,
}

impl SmarticleRole {
    pub fn index(self) -> usize {
        match self {
            Self::Life => 0,
            Self::Nutrient => 1,
            Self::Dead => 2,
            Self::Mutagen => 3,
            Self::Nest => 4,
            Self::Spore => 5,
            Self::Root => 6,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct SmarticleRule {
    pub power: f32,
    pub radius: f32,
}

impl Default for SmarticleRule {
    fn default() -> Self {
        Self {
            power: 0.0,
            radius: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SmarticleSample {
    pub pull: f32,
    pub push: f32,
    pub growth_bias: f32,
    pub decay_bias: f32,
    pub mutation_bias: f32,
    pub nest_bias: f32,
    pub drift_x: f32,
    pub drift_y: f32,
}

#[derive(Clone, Debug)]
pub struct SmarticleField {
    seed: u64,
    rules: [[SmarticleRule; SMARTICLE_ROLE_COUNT]; SMARTICLE_ROLE_COUNT],
}

impl SmarticleField {
    pub fn from_seed(seed: u64) -> Self {
        let mut rules = [[SmarticleRule::default(); SMARTICLE_ROLE_COUNT]; SMARTICLE_ROLE_COUNT];

        for source in 0..SMARTICLE_ROLE_COUNT {
            for target in 0..SMARTICLE_ROLE_COUNT {
                rules[source][target] = seeded_rule(seed, source, target);
            }
        }

        stabilize_roots(&mut rules);

        Self { seed, rules }
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn rule(&self, source: SmarticleRole, target: SmarticleRole) -> SmarticleRule {
        self.rules[source.index()][target.index()]
    }

    pub fn rules(&self) -> &[[SmarticleRule; SMARTICLE_ROLE_COUNT]; SMARTICLE_ROLE_COUNT] {
        &self.rules
    }
}

fn seeded_rule(seed: u64, source: usize, target: usize) -> SmarticleRule {
    let power_roll = unit_hash(seed, source, target, 0);
    let radius_roll = unit_hash(seed, source, target, 1);

    let raw_power = POWER_MIN + (POWER_MAX - POWER_MIN) * power_roll;
    let power = curved_signed(raw_power, POWER_CURVE) * 0.18;

    let raw_radius = RADIUS_MIN + (RADIUS_MAX - RADIUS_MIN) * radius_roll;
    let radius = raw_radius
        .powf(1.0 / RADIUS_CURVE)
        .clamp(RADIUS_MIN, RADIUS_MAX);

    SmarticleRule { power, radius }
}

fn stabilize_roots(rules: &mut [[SmarticleRule; SMARTICLE_ROLE_COUNT]; SMARTICLE_ROLE_COUNT]) {
    let root = SmarticleRole::Root.index();

    for i in 0..SMARTICLE_ROLE_COUNT {
        rules[root][i].power = 0.0;
        rules[root][i].radius = 1.0;

        rules[i][root].power = (rules[i][root].power * 0.18).clamp(-0.035, 0.035);
        rules[i][root].radius = rules[i][root].radius.min(2.0);
    }
}

fn curved_signed(value: f32, curve: f32) -> f32 {
    if value >= 0.0 {
        value.powf(1.0 / curve)
    } else {
        -value.abs().powf(1.0 / curve)
    }
}

fn unit_hash(seed: u64, a: usize, b: usize, salt: usize) -> f32 {
    let mut value = seed as usize;
    value ^= a.wrapping_mul(374_761_393);
    value ^= b.wrapping_mul(668_265_263);
    value ^= salt.wrapping_mul(2_246_822_519usize);
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    value ^= value >> 16;

    (value % 10_000) as f32 / 10_000.0
}
