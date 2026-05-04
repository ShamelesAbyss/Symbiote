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
            Self::Life => '▒',
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
                        if neighbors == 3 || (neighbors == 2 && nutrient_neighbors > 0) {
                            next.kind = CellKind::Life;
                            next.energy = 36.0 + nutrient_neighbors as f32 * 8.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                        }
                    }
                    CellKind::Life => {
                        if neighbors < 2 || neighbors > 5 {
                            next.kind = CellKind::Dead;
                            next.energy = 22.0;
                        } else if neighbors == 3 && nutrient_neighbors > 1 {
                            next.kind = CellKind::Spore;
                            next.energy = (cell.energy + 5.0).min(100.0);
                        } else {
                            next.energy =
                                (cell.energy + nutrient_neighbors as f32 * 1.2 - 0.9).clamp(0.0, 100.0);
                        }
                    }
                    CellKind::Spore => {
                        if neighbors < 1 || neighbors > 6 {
                            next.kind = CellKind::Dead;
                            next.energy = 20.0;
                        } else if neighbors == 2 || neighbors == 3 {
                            next.energy = (cell.energy + 1.6).min(100.0);
                        } else {
                            next.energy = (cell.energy - 1.1).max(0.0);
                        }
                    }
                    CellKind::Nutrient => {
                        if neighbors >= 3 {
                            next.kind = CellKind::Life;
                            next.energy = 54.0;
                            next.age = 0;
                        } else {
                            next.energy = (cell.energy - 0.05).max(0.0);
                        }
                    }
                    CellKind::Dead => {
                        if nutrient_neighbors >= 2 && neighbors >= 2 {
                            next.kind = CellKind::Nutrient;
                            next.energy = 35.0;
                        } else {
                            next.energy = (cell.energy - 0.22).max(0.0);

                            if next.energy <= 0.0 || dead_neighbors > 5 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }
                    CellKind::Mutagen => {
                        if neighbors >= 2 && neighbors <= 4 {
                            next.kind = CellKind::Spore;
                            next.energy = 58.0;
                        } else {
                            next.energy = (cell.energy - 0.08).max(0.0);
                        }
                    }
                    CellKind::Nest => {
                        if neighbors > 6 {
                            next.kind = CellKind::Dead;
                            next.energy = 32.0;
                        } else {
                            next.energy = (cell.energy + 0.35).min(100.0);
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

        let desired = if particle.rare_trait != RareTrait::None {
            CellKind::Mutagen
        } else {
            match archetype {
                Some(Archetype::Grazer) => CellKind::Nutrient,
                Some(Archetype::Hunter) => CellKind::Dead,
                Some(Archetype::Architect) | Some(Archetype::Leviathan) => CellKind::Nest,
                Some(Archetype::Mycelial) => CellKind::Spore,
                Some(Archetype::Phantom) => CellKind::Mutagen,
                _ => CellKind::Life,
            }
        };

        if cell.kind == CellKind::Empty || particle.energy > 80.0 || particle.rare_trait != RareTrait::None {
            cell.kind = desired;
            cell.energy = (cell.energy + particle.energy * 0.18).clamp(0.0, 100.0);
            cell.tribe_hint = particle.tribe.index();
        }
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

    fn seed_initial_life(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                let n = hash(self.seed, x, y) % 1000;
                let idx = self.idx(x, y);

                self.cells[idx] = if n < 28 {
                    Cell {
                        kind: CellKind::Life,
                        energy: 42.0,
                        age: 0,
                        tribe_hint: n % 6,
                    }
                } else if n < 38 {
                    Cell {
                        kind: CellKind::Nutrient,
                        energy: 64.0,
                        age: 0,
                        tribe_hint: n % 6,
                    }
                } else if n < 42 {
                    Cell {
                        kind: CellKind::Mutagen,
                        energy: 70.0,
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
