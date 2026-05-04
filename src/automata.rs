use crate::{
    particle::{Particle, RareTrait},
    species::Archetype,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum CellKind {
    Empty,
    Life,
    Nutrient,
    Dead,
    Mutagen,
    Nest,
    Spore,
}

impl CellKind {
    pub fn glyph(self) -> char {
        match self {
            Self::Empty => ' ',
            Self::Life => '·',
            Self::Nutrient => '+',
            Self::Dead => '×',
            Self::Mutagen => '*',
            Self::Nest => '◎',
            Self::Spore => '░',
        }
    }

    pub fn is_alive(self) -> bool {
        matches!(self, Self::Life | Self::Spore | Self::Nest)
    }

    pub fn food_value(self) -> f32 {
        match self {
            Self::Life => 14.0,
            Self::Nutrient => 22.0,
            Self::Spore => 18.0,
            Self::Nest => 26.0,
            Self::Mutagen => 10.0,
            Self::Dead => 5.0,
            Self::Empty => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub kind: CellKind,
    pub energy: f32,
    pub age: u16,
    pub tribe_hint: usize,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            kind: CellKind::Empty,
            energy: 0.0,
            age: 0,
            tribe_hint: 0,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CellularAutomata {
    pub width: usize,
    pub height: usize,
    pub seed: u64,
    pub cycle: u64,
    pub cells: Vec<Cell>,
}

impl CellularAutomata {
    pub fn new(seed: u64, width: usize, height: usize) -> Self {
        let width = width.max(24);
        let height = height.max(12);

        let mut substrate = Self {
            width,
            height,
            seed,
            cycle: 0,
            cells: vec![Cell::default(); width * height],
        };

        substrate.seed_initial_life();
        substrate
    }

    pub fn tick(&mut self) {
        self.cycle += 1;
        let snapshot = self.cells.clone();

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                let cell = snapshot[idx];
                let neighbors = self.alive_neighbors(&snapshot, x, y);
                let nutrient_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Nutrient);
                let dead_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Dead);
                let mut next = cell;

                match cell.kind {
                    CellKind::Empty => {
                        if neighbors == 3 && nutrient_neighbors > 0 {
                            next.kind = CellKind::Life;
                            next.energy = 28.0 + nutrient_neighbors as f32 * 5.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                        }
                    }
                    CellKind::Life => {
                        if neighbors < 2 || neighbors > 3 {
                            next.kind = CellKind::Dead;
                            next.energy = 12.0;
                        } else if neighbors == 3 && nutrient_neighbors > 2 {
                            next.kind = CellKind::Spore;
                            next.energy = (cell.energy + 3.0).min(80.0);
                        } else {
                            next.energy =
                                (cell.energy + nutrient_neighbors as f32 * 0.45 - 1.25).clamp(0.0, 80.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                    CellKind::Spore => {
                        if neighbors < 2 || neighbors > 4 {
                            next.kind = CellKind::Dead;
                            next.energy = 14.0;
                        } else {
                            next.energy = (cell.energy - 1.35).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                    CellKind::Nutrient => {
                        if neighbors == 3 && cell.energy > 40.0 {
                            next.kind = CellKind::Life;
                            next.energy = 42.0;
                            next.age = 0;
                        } else {
                            next.energy = (cell.energy - 0.18).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                    CellKind::Dead => {
                        next.energy = (cell.energy - 0.45).max(0.0);

                        if nutrient_neighbors >= 3 && neighbors == 2 {
                            next.kind = CellKind::Nutrient;
                            next.energy = 28.0;
                        } else if next.energy <= 0.0 || dead_neighbors > 4 {
                            next.kind = CellKind::Empty;
                            next.age = 0;
                        }
                    }
                    CellKind::Mutagen => {
                        if neighbors == 3 {
                            next.kind = CellKind::Spore;
                            next.energy = 42.0;
                        } else {
                            next.energy = (cell.energy - 0.22).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                    CellKind::Nest => {
                        if neighbors > 4 {
                            next.kind = CellKind::Dead;
                            next.energy = 18.0;
                        } else {
                            next.energy = (cell.energy - 0.08).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                }

                if next.kind != CellKind::Empty {
                    next.age = next.age.saturating_add(1);
                }

                self.cells[idx] = next;
            }
        }
    }

    pub fn deposit_particle(&mut self, particle: &Particle, archetype: Option<Archetype>) {
        let Some((x, y)) = self.world_to_grid(particle.x, particle.y) else {
            return;
        };

        let idx = self.idx(x, y);
        let cell = &mut self.cells[idx];

        let deposit_allowed = match cell.kind {
            CellKind::Empty => true,
            CellKind::Dead => particle.energy > 95.0,
            CellKind::Nutrient => matches!(archetype, Some(Archetype::Grazer | Archetype::Mycelial)),
            CellKind::Mutagen => particle.rare_trait != RareTrait::None,
            CellKind::Life | CellKind::Spore | CellKind::Nest => particle.energy > 125.0,
        };

        if !deposit_allowed {
            return;
        }

        let desired = if particle.rare_trait != RareTrait::None && particle.energy > 110.0 {
            CellKind::Mutagen
        } else {
            match archetype {
                Some(Archetype::Grazer) => CellKind::Nutrient,
                Some(Archetype::Hunter) => CellKind::Dead,
                Some(Archetype::Architect) | Some(Archetype::Leviathan) => CellKind::Nest,
                Some(Archetype::Mycelial) => CellKind::Spore,
                Some(Archetype::Phantom) => CellKind::Mutagen,
                Some(Archetype::Harvester) => CellKind::Nutrient,
                _ => CellKind::Life,
            }
        };

        cell.kind = desired;
        cell.energy = (cell.energy + particle.energy * 0.055).clamp(0.0, 80.0);
        cell.tribe_hint = particle.tribe.index();
    }

    pub fn consume_at(&mut self, x: f32, y: f32, power: f32) -> Option<CellKind> {
        let (gx, gy) = self.world_to_grid(x, y)?;
        let idx = self.idx(gx, gy);
        let cell = &mut self.cells[idx];

        if cell.kind == CellKind::Empty {
            return None;
        }

        let eaten = cell.kind;
        cell.energy -= power;

        if cell.energy <= 0.0 || matches!(eaten, CellKind::Life | CellKind::Spore | CellKind::Nutrient) {
            cell.kind = CellKind::Empty;
            cell.energy = 0.0;
            cell.age = 0;
        }

        Some(eaten)
    }

    pub fn influence_at(&self, x: f32, y: f32) -> CellKind {
        if let Some((gx, gy)) = self.world_to_grid(x, y) {
            self.cells[self.idx(gx, gy)].kind
        } else {
            CellKind::Empty
        }
    }

    pub fn sample_screen(&self, sx: usize, sy: usize, screen_w: usize, screen_h: usize) -> CellKind {
        if screen_w == 0 || screen_h == 0 {
            return CellKind::Empty;
        }

        let x = (sx * self.width / screen_w).min(self.width.saturating_sub(1));
        let y = (sy * self.height / screen_h).min(self.height.saturating_sub(1));

        self.cells[self.idx(x, y)].kind
    }

    pub fn living_cells(&self) -> usize {
        self.cells.iter().filter(|cell| cell.kind.is_alive()).count()
    }

    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }

    fn seed_initial_life(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let n = hash(self.seed, x, y) % 10_000;
                let idx = self.idx(x, y);

                self.cells[idx] = if n < 70 {
                    Cell {
                        kind: CellKind::Life,
                        energy: 34.0,
                        age: 0,
                        tribe_hint: n % 6,
                    }
                } else if n < 95 {
                    Cell {
                        kind: CellKind::Nutrient,
                        energy: 48.0,
                        age: 0,
                        tribe_hint: n % 6,
                    }
                } else if n < 102 {
                    Cell {
                        kind: CellKind::Mutagen,
                        energy: 55.0,
                        age: 0,
                        tribe_hint: n % 6,
                    }
                } else {
                    Cell::default()
                };
            }
        }
    }

    fn alive_neighbors(&self, snapshot: &[Cell], x: usize, y: usize) -> usize {
        let mut count = 0;

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);

                if snapshot[self.idx(nx, ny)].kind.is_alive() {
                    count += 1;
                }
            }
        }

        count
    }

    fn kind_neighbors(&self, snapshot: &[Cell], x: usize, y: usize, kind: CellKind) -> usize {
        let mut count = 0;

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);

                if snapshot[self.idx(nx, ny)].kind == kind {
                    count += 1;
                }
            }
        }

        count
    }

    fn local_tribe_hint(&self, snapshot: &[Cell], x: usize, y: usize) -> usize {
        let mut counts = [0usize; 6];

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);
                let hint = snapshot[self.idx(nx, ny)].tribe_hint % 6;
                counts[hint] += 1;
            }
        }

        let mut best = 0;

        for i in 1..6 {
            if counts[i] > counts[best] {
                best = i;
            }
        }

        best
    }

    fn world_to_grid(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        if !(-1.25..=1.25).contains(&x) || !(-1.25..=1.25).contains(&y) {
            return None;
        }

        let gx = (((x + 1.2) / 2.4) * self.width as f32) as isize;
        let gy = (((y + 1.2) / 2.4) * self.height as f32) as isize;

        if gx < 0 || gy < 0 || gx >= self.width as isize || gy >= self.height as isize {
            None
        } else {
            Some((gx as usize, gy as usize))
        }
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

fn wrap(value: isize, max: usize) -> usize {
    value.rem_euclid(max as isize) as usize
}

fn hash(seed: u64, x: usize, y: usize) -> usize {
    let mut value = seed as usize;
    value ^= x.wrapping_mul(374_761_393);
    value ^= y.wrapping_mul(668_265_263);
    value = (value ^ (value >> 13)).wrapping_mul(1_274_126_177);
    value ^ (value >> 16)
}
