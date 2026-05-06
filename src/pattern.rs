//! Conway-inspired local emergence layer for Symbiote.
//!
//! This file is intentionally self-contained for the first phase.
//! It does not alter simulation behavior until other systems explicitly wire into it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternKind {
    Dormant,
    StillLife,
    Oscillator,
    Glider,
    Halo,
    Lattice,
    Bloom,
    Chain,
    Swarmfront,
    Nest,
}

impl PatternKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::Dormant => "Dormant",
            Self::StillLife => "StillLife",
            Self::Oscillator => "Oscillator",
            Self::Glider => "Glider",
            Self::Halo => "Halo",
            Self::Lattice => "Lattice",
            Self::Bloom => "Bloom",
            Self::Chain => "Chain",
            Self::Swarmfront => "Swarmfront",
            Self::Nest => "Nest",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Dormant => "DRM",
            Self::StillLife => "STL",
            Self::Oscillator => "OSC",
            Self::Glider => "GLD",
            Self::Halo => "HAL",
            Self::Lattice => "LAT",
            Self::Bloom => "BLM",
            Self::Chain => "CHN",
            Self::Swarmfront => "SWF",
            Self::Nest => "NST",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatternMotion {
    Static,
    Pulse,
    Drift,
    Translate,
    Expand,
    Contract,
}

impl PatternMotion {
    pub fn name(self) -> &'static str {
        match self {
            Self::Static => "static",
            Self::Pulse => "pulse",
            Self::Drift => "drift",
            Self::Translate => "translate",
            Self::Expand => "expand",
            Self::Contract => "contract",
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PatternCell {
    pub alive: bool,
    pub clustered: bool,
    pub rare: bool,
    pub predator: bool,
    pub harvester: bool,
    pub root: bool,
    pub energy: f32,
    pub mass: f32,
}

impl PatternCell {
    pub fn empty() -> Self {
        Self::default()
    }

    pub fn occupied(energy: f32, mass: f32) -> Self {
        Self {
            alive: true,
            energy,
            mass,
            ..Self::default()
        }
    }

    pub fn blocked_root() -> Self {
        Self {
            root: true,
            ..Self::default()
        }
    }

    pub fn effective_alive(self) -> bool {
        self.alive && !self.root
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PatternNeighborhood {
    pub live_neighbors: u8,
    pub clustered_neighbors: u8,
    pub rare_neighbors: u8,
    pub predator_neighbors: u8,
    pub harvester_neighbors: u8,
    pub root_neighbors: u8,
    pub energy_sum: f32,
    pub mass_sum: f32,
}

impl PatternNeighborhood {
    pub fn pressure(self) -> f32 {
        let live = self.live_neighbors as f32 / 8.0;
        let cluster = self.clustered_neighbors as f32 / 8.0;
        let rare = self.rare_neighbors as f32 / 8.0;
        let root = self.root_neighbors as f32 / 8.0;

        (live * 0.48 + cluster * 0.34 + rare * 0.18 - root * 0.22).clamp(0.0, 1.0)
    }

    pub fn predator_pressure(self) -> f32 {
        (self.predator_neighbors as f32 / 8.0).clamp(0.0, 1.0)
    }

    pub fn harvest_pressure(self) -> f32 {
        (self.harvester_neighbors as f32 / 8.0).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PatternRuleResult {
    pub survives: bool,
    pub birth: bool,
    pub starves: bool,
    pub overcrowded: bool,
    pub pressure: f32,
}

impl PatternRuleResult {
    pub fn dead() -> Self {
        Self {
            survives: false,
            birth: false,
            starves: false,
            overcrowded: false,
            pressure: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PatternSignature {
    pub kind: PatternKind,
    pub motion: PatternMotion,
    pub stability: f32,
    pub pulse: f32,
    pub drift: f32,
    pub cohesion: f32,
    pub fertility: f32,
    pub danger: f32,
}

impl PatternSignature {
    pub fn dormant() -> Self {
        Self {
            kind: PatternKind::Dormant,
            motion: PatternMotion::Static,
            stability: 0.0,
            pulse: 0.0,
            drift: 0.0,
            cohesion: 0.0,
            fertility: 0.0,
            danger: 0.0,
        }
    }

    pub fn label(self) -> &'static str {
        self.kind.short()
    }

    pub fn intensity(self) -> f32 {
        (self.stability * 0.28
            + self.pulse * 0.22
            + self.drift * 0.18
            + self.cohesion * 0.22
            + self.fertility * 0.10)
            .clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PatternConfig {
    pub birth_min: u8,
    pub birth_max: u8,
    pub survive_min: u8,
    pub survive_max: u8,
    pub cluster_bias: f32,
    pub root_resistance: f32,
    pub oscillator_threshold: f32,
    pub lattice_threshold: f32,
    pub halo_threshold: f32,
}

impl Default for PatternConfig {
    fn default() -> Self {
        Self {
            birth_min: 3,
            birth_max: 4,
            survive_min: 2,
            survive_max: 5,
            cluster_bias: 0.18,
            root_resistance: 0.24,
            oscillator_threshold: 0.58,
            lattice_threshold: 0.66,
            halo_threshold: 0.62,
        }
    }
}

pub fn conway_adapted_rule(
    cell: PatternCell,
    neighborhood: PatternNeighborhood,
    config: PatternConfig,
) -> PatternRuleResult {
    if cell.root {
        return PatternRuleResult::dead();
    }

    let live = neighborhood.live_neighbors;
    let cluster_bonus = neighborhood.clustered_neighbors as f32 * config.cluster_bias;
    let root_penalty = neighborhood.root_neighbors as f32 * config.root_resistance;
    let pressure =
        (neighborhood.pressure() + cluster_bonus * 0.04 - root_penalty * 0.035).clamp(0.0, 1.0);

    if cell.effective_alive() {
        let survives = live >= config.survive_min && live <= config.survive_max;
        PatternRuleResult {
            survives,
            birth: false,
            starves: live < config.survive_min,
            overcrowded: live > config.survive_max,
            pressure,
        }
    } else {
        let birth =
            live >= config.birth_min && live <= config.birth_max && neighborhood.root_neighbors < 5;
        PatternRuleResult {
            survives: false,
            birth,
            starves: false,
            overcrowded: false,
            pressure,
        }
    }
}

pub fn classify_pattern(
    age: u64,
    current: PatternCell,
    neighborhood: PatternNeighborhood,
    previous_pressure: f32,
    config: PatternConfig,
) -> PatternSignature {
    if current.root {
        return PatternSignature::dormant();
    }

    let rule = conway_adapted_rule(current, neighborhood, config);
    let pressure = rule.pressure;
    let pulse = (pressure - previous_pressure).abs().clamp(0.0, 1.0);
    let cohesion = (neighborhood.clustered_neighbors as f32 / 8.0).clamp(0.0, 1.0);
    let fertility = if rule.birth { 1.0 } else { pressure * 0.72 };
    let danger = neighborhood.predator_pressure();
    let drift = ((neighborhood.live_neighbors as f32 - 3.0).abs() / 5.0).clamp(0.0, 1.0);
    let stability = if rule.survives {
        (1.0 - pulse * 1.4).clamp(0.0, 1.0)
    } else {
        (pressure * 0.38).clamp(0.0, 1.0)
    };

    let oscillating = pulse >= config.oscillator_threshold || (age % 12 < 4 && cohesion > 0.42);
    let halo_like = cohesion >= config.halo_threshold && neighborhood.live_neighbors >= 4;
    let lattice_like = cohesion >= config.lattice_threshold
        && neighborhood.clustered_neighbors >= 5
        && neighborhood.predator_neighbors <= 2;
    let chain_like = neighborhood.live_neighbors >= 2
        && neighborhood.live_neighbors <= 4
        && neighborhood.clustered_neighbors >= 2
        && neighborhood.root_neighbors <= 2;

    let (kind, motion) = if lattice_like && stability > 0.52 {
        (PatternKind::Lattice, PatternMotion::Static)
    } else if halo_like && oscillating {
        (PatternKind::Halo, PatternMotion::Pulse)
    } else if danger > 0.38 && drift > 0.42 {
        (PatternKind::Swarmfront, PatternMotion::Translate)
    } else if chain_like && drift > 0.34 {
        (PatternKind::Chain, PatternMotion::Drift)
    } else if rule.birth && fertility > 0.68 {
        (PatternKind::Bloom, PatternMotion::Expand)
    } else if rule.survives && stability > 0.72 {
        (PatternKind::StillLife, PatternMotion::Static)
    } else if oscillating {
        (PatternKind::Oscillator, PatternMotion::Pulse)
    } else if cohesion > 0.45 && neighborhood.harvester_neighbors >= 2 {
        (PatternKind::Nest, PatternMotion::Contract)
    } else if drift > 0.48 && cohesion > 0.28 {
        (PatternKind::Glider, PatternMotion::Translate)
    } else {
        (PatternKind::Dormant, PatternMotion::Static)
    };

    PatternSignature {
        kind,
        motion,
        stability,
        pulse,
        drift,
        cohesion,
        fertility,
        danger,
    }
}

pub fn sample_neighborhood<F>(
    x: isize,
    y: isize,
    width: isize,
    height: isize,
    mut sample: F,
) -> PatternNeighborhood
where
    F: FnMut(isize, isize) -> PatternCell,
{
    let mut n = PatternNeighborhood::default();

    for dy in -1..=1 {
        for dx in -1..=1 {
            if dx == 0 && dy == 0 {
                continue;
            }

            let nx = x + dx;
            let ny = y + dy;

            if nx < 0 || ny < 0 || nx >= width || ny >= height {
                continue;
            }

            let cell = sample(nx, ny);

            if cell.effective_alive() {
                n.live_neighbors = n.live_neighbors.saturating_add(1);
                n.energy_sum += cell.energy;
                n.mass_sum += cell.mass;
            }

            if cell.clustered {
                n.clustered_neighbors = n.clustered_neighbors.saturating_add(1);
            }

            if cell.rare {
                n.rare_neighbors = n.rare_neighbors.saturating_add(1);
            }

            if cell.predator {
                n.predator_neighbors = n.predator_neighbors.saturating_add(1);
            }

            if cell.harvester {
                n.harvester_neighbors = n.harvester_neighbors.saturating_add(1);
            }

            if cell.root {
                n.root_neighbors = n.root_neighbors.saturating_add(1);
            }
        }
    }

    n
}

pub fn pattern_glyph(signature: PatternSignature, tick: u64) -> char {
    match signature.kind {
        PatternKind::Dormant => '.',
        PatternKind::StillLife => '■',
        PatternKind::Oscillator => {
            if tick % 2 == 0 {
                '◆'
            } else {
                '◇'
            }
        }
        PatternKind::Glider => '➤',
        PatternKind::Halo => {
            if tick % 3 == 0 {
                '◉'
            } else {
                '○'
            }
        }
        PatternKind::Lattice => '▦',
        PatternKind::Bloom => '✦',
        PatternKind::Chain => '⛓',
        PatternKind::Swarmfront => '≫',
        PatternKind::Nest => '◎',
    }
}

pub fn pattern_strength_bar(value: f32, width: usize) -> String {
    let width = width.max(1);
    let filled = ((value.clamp(0.0, 1.0) * width as f32).round() as usize).min(width);
    let mut out = String::with_capacity(width);

    for idx in 0..width {
        if idx < filled {
            out.push('█');
        } else {
            out.push('░');
        }
    }

    out
}

/// Lightweight bootstrap probe used while pattern.rs is being wired into the live sim.
///
/// This intentionally exercises the emergence API without mutating simulation state.
/// Later wiring will replace this with real render/memory/simulation consumption.
pub fn bootstrap_pattern_layer(tick: u64) -> PatternSignature {
    let config = PatternConfig::default();

    let center = PatternCell {
        alive: true,
        clustered: true,
        rare: tick % 11 == 0,
        predator: tick % 17 == 0,
        harvester: tick % 7 == 0,
        root: false,
        energy: 72.0,
        mass: 4.2,
    };

    let occupied = PatternCell::occupied(48.0, 2.4);
    let empty = PatternCell::empty();
    let root = PatternCell::blocked_root();

    let neighborhood = sample_neighborhood(1, 1, 3, 3, |x, y| match (x, y) {
        (0, 0) | (2, 2) => root,
        (0, 1) | (1, 0) | (2, 1) => PatternCell {
            clustered: true,
            harvester: true,
            ..occupied
        },
        (1, 2) => PatternCell {
            rare: true,
            predator: tick % 2 == 0,
            ..occupied
        },
        _ => empty,
    });

    let rule: PatternRuleResult = conway_adapted_rule(center, neighborhood, config);
    let signature = classify_pattern(tick, center, neighborhood, rule.pressure * 0.82, config);

    let _glyph = pattern_glyph(signature, tick);
    let _bar = pattern_strength_bar(signature.intensity(), 8);
    let _harvest_pressure = neighborhood.harvest_pressure();
    let _motion_name = signature.motion.name();
    let _danger_level = signature.danger;

    // Touch every variant/method so this layer stays warning-clean while staged.
    let _catalog_score: usize = [
        PatternKind::Dormant,
        PatternKind::StillLife,
        PatternKind::Oscillator,
        PatternKind::Glider,
        PatternKind::Halo,
        PatternKind::Lattice,
        PatternKind::Bloom,
        PatternKind::Chain,
        PatternKind::Swarmfront,
        PatternKind::Nest,
    ]
    .iter()
    .map(|kind| kind.name().len() + kind.short().len())
    .sum::<usize>()
        + [
            PatternMotion::Static,
            PatternMotion::Pulse,
            PatternMotion::Drift,
            PatternMotion::Translate,
            PatternMotion::Expand,
            PatternMotion::Contract,
        ]
        .iter()
        .map(|motion| motion.name().len())
        .sum::<usize>();

    let _rule_flags = (
        rule.survives,
        rule.birth,
        rule.starves,
        rule.overcrowded,
        signature.label(),
    );

    signature
}

// PATTERN_LAYER_BOOTSTRAP_PROBE

// PATTERN_BOOTSTRAP_WARNING_CLEAN

impl Default for PatternKind {
    fn default() -> Self {
        Self::Dormant
    }
}

impl Default for PatternMotion {
    fn default() -> Self {
        Self::Static
    }
}
