#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DensityBand {
    Starved,
    Sparse,
    Balanced,
    Crowded,
    Saturated,
}

#[derive(Debug, Clone, Copy)]
pub struct DensityConfig {
    pub min_cell_ratio: f32,
    pub target_cell_ratio: f32,
    pub max_cell_ratio: f32,

    pub min_particle_ratio: f32,
    pub target_particle_ratio: f32,
    pub max_particle_ratio: f32,

    pub target_root_ratio: f32,
    pub reserved_empty_ratio: f32,
}

impl Default for DensityConfig {
    fn default() -> Self {
        Self {
            min_cell_ratio: 0.035,
            target_cell_ratio: 0.105,
            max_cell_ratio: 0.185,

            min_particle_ratio: 0.045,
            target_particle_ratio: 0.145,
            max_particle_ratio: 0.245,

            target_root_ratio: 0.075,
            reserved_empty_ratio: 0.185,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DensitySnapshot {
    pub width: usize,
    pub height: usize,
    pub cells: usize,
    pub particles: usize,
    pub roots: usize,
}

impl DensitySnapshot {
    pub fn new(width: usize, height: usize, cells: usize, particles: usize, roots: usize) -> Self {
        Self {
            width,
            height,
            cells,
            particles,
            roots,
        }
    }

    pub fn area(&self) -> usize {
        self.width.saturating_mul(self.height)
    }

    pub fn occupied(&self) -> usize {
        self.cells
            .saturating_add(self.particles)
            .saturating_add(self.roots)
            .min(self.area())
    }

    pub fn empty(&self) -> usize {
        self.area().saturating_sub(self.occupied())
    }

    pub fn cell_ratio(&self) -> f32 {
        ratio(self.cells, self.area())
    }

    pub fn particle_ratio(&self) -> f32 {
        ratio(self.particles, self.area())
    }

    pub fn root_ratio(&self) -> f32 {
        ratio(self.roots, self.area())
    }

    pub fn occupied_ratio(&self) -> f32 {
        ratio(self.occupied(), self.area())
    }

    pub fn empty_ratio(&self) -> f32 {
        ratio(self.empty(), self.area())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DensityTargets {
    pub min_cells: usize,
    pub target_cells: usize,
    pub max_cells: usize,

    pub min_particles: usize,
    pub target_particles: usize,
    pub max_particles: usize,

    pub target_roots: usize,
    pub reserved_empty: usize,
}

impl DensityTargets {
    pub fn from_snapshot(snapshot: DensitySnapshot, config: DensityConfig) -> Self {
        let area = snapshot.area();

        Self {
            min_cells: scaled(area, config.min_cell_ratio),
            target_cells: scaled(area, config.target_cell_ratio),
            max_cells: scaled(area, config.max_cell_ratio),

            min_particles: scaled(area, config.min_particle_ratio),
            target_particles: scaled(area, config.target_particle_ratio),
            max_particles: scaled(area, config.max_particle_ratio),

            target_roots: scaled(area, config.target_root_ratio),
            reserved_empty: scaled(area, config.reserved_empty_ratio),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DensityPressure {
    pub band: DensityBand,
    pub cell_spawn_pressure: u16,
    pub particle_spawn_pressure: u16,
    pub root_growth_pressure: u16,
    pub refill_pressure: u16,
    pub crowding_pressure: u16,
}

impl DensityPressure {
    pub fn analyze(snapshot: DensitySnapshot, config: DensityConfig) -> Self {
        let targets = DensityTargets::from_snapshot(snapshot, config);

        let occupied = snapshot.occupied_ratio();
        let empty = snapshot.empty_ratio();

        let band = if empty <= config.reserved_empty_ratio * 0.45 {
            DensityBand::Saturated
        } else if occupied >= 0.72 {
            DensityBand::Crowded
        } else if snapshot.cells < targets.min_cells || snapshot.particles < targets.min_particles {
            DensityBand::Starved
        } else if occupied < 0.28 {
            DensityBand::Sparse
        } else {
            DensityBand::Balanced
        };

        Self {
            band,
            cell_spawn_pressure: spawn_pressure(
                snapshot.cells,
                targets.min_cells,
                targets.target_cells,
                targets.max_cells,
            ),
            particle_spawn_pressure: spawn_pressure(
                snapshot.particles,
                targets.min_particles,
                targets.target_particles,
                targets.max_particles,
            ),
            root_growth_pressure: root_pressure(snapshot, targets, config),
            refill_pressure: refill_pressure(snapshot, targets),
            crowding_pressure: crowding_pressure(snapshot, config),
        }
    }

    pub fn is_crowded(&self) -> bool {
        matches!(self.band, DensityBand::Crowded | DensityBand::Saturated)
    }

    pub fn wants_refill(&self) -> bool {
        matches!(self.band, DensityBand::Starved | DensityBand::Sparse)
    }
}

fn ratio(value: usize, total: usize) -> f32 {
    if total == 0 {
        0.0
    } else {
        value as f32 / total as f32
    }
}

fn scaled(area: usize, ratio: f32) -> usize {
    ((area as f32) * ratio).round().max(1.0) as usize
}

fn spawn_pressure(current: usize, min: usize, target: usize, max: usize) -> u16 {
    if current >= max {
        return 0;
    }

    if current <= min {
        return 1_000;
    }

    if current <= target {
        let span = target.saturating_sub(min).max(1);
        let remaining = target.saturating_sub(current);
        return 520 + ((remaining.saturating_mul(480) / span) as u16);
    }

    let span = max.saturating_sub(target).max(1);
    let remaining = max.saturating_sub(current);
    (remaining.saturating_mul(520) / span) as u16
}

fn root_pressure(
    snapshot: DensitySnapshot,
    targets: DensityTargets,
    config: DensityConfig,
) -> u16 {
    if snapshot.empty() <= targets.reserved_empty {
        return 80;
    }

    if snapshot.root_ratio() >= config.target_root_ratio * 1.6 {
        return 180;
    }

    if snapshot.roots < targets.target_roots {
        return 760;
    }

    420
}

fn refill_pressure(snapshot: DensitySnapshot, targets: DensityTargets) -> u16 {
    let cell_deficit = targets.target_cells.saturating_sub(snapshot.cells);
    let particle_deficit = targets.target_particles.saturating_sub(snapshot.particles);
    let total_deficit = cell_deficit.saturating_add(particle_deficit);

    if total_deficit == 0 {
        0
    } else {
        total_deficit.min(1_000) as u16
    }
}

fn crowding_pressure(snapshot: DensitySnapshot, config: DensityConfig) -> u16 {
    let empty = snapshot.empty_ratio();

    if empty >= config.reserved_empty_ratio {
        return 0;
    }

    let deficit = config.reserved_empty_ratio - empty;
    ((deficit / config.reserved_empty_ratio).clamp(0.0, 1.0) * 1_000.0).round() as u16
}
