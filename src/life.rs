#![allow(dead_code)]

use serde::{Deserialize, Serialize};

const HISTORY_LIMIT: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifeCell {
    Dead,
    Alive,
}

impl LifeCell {
    pub fn is_alive(self) -> bool {
        matches!(self, Self::Alive)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AxiomPatternState {
    Dormant,
    Static,
    Oscillating,
    Translating,
    Expanding,
    Collapsing,
    Chaotic,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct AxiomStats {
    pub generation: u64,
    pub alive: usize,
    pub births: usize,
    pub deaths: usize,
    pub stable_ticks: u16,
    pub oscillator_hits: u64,
    pub translation_hits: u64,
    pub peak_alive: usize,
    pub state: AxiomPatternState,
}

impl Default for AxiomStats {
    fn default() -> Self {
        Self {
            generation: 0,
            alive: 0,
            births: 0,
            deaths: 0,
            stable_ticks: 0,
            oscillator_hits: 0,
            translation_hits: 0,
            peak_alive: 0,
            state: AxiomPatternState::Dormant,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct LifeFingerprint {
    hash: u64,
    alive: usize,
    centroid_x: i32,
    centroid_y: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AxiomLattice {
    width: usize,
    height: usize,
    seed: u64,
    cells: Vec<LifeCell>,
    scratch: Vec<LifeCell>,
    history: Vec<LifeFingerprint>,
    stats: AxiomStats,
}

impl AxiomLattice {
    pub fn new(seed: u64, width: usize, height: usize) -> Self {
        let width = width.max(24);
        let height = height.max(12);
        let cells = vec![LifeCell::Dead; width * height];

        let mut lattice = Self {
            width,
            height,
            seed,
            scratch: cells.clone(),
            cells,
            history: Vec::with_capacity(HISTORY_LIMIT),
            stats: AxiomStats::default(),
        };

        lattice.seed_prime_soup();
        lattice
    }

    pub fn reset(&mut self, seed: u64) {
        self.seed = seed;
        self.cells.fill(LifeCell::Dead);
        self.scratch.fill(LifeCell::Dead);
        self.history.clear();
        self.stats = AxiomStats::default();
        self.seed_prime_soup();
    }

    pub fn tick_b3s23(&mut self) {
        let before_alive = self.alive_cells();
        let previous_hash = self.fingerprint().hash;

        let mut births = 0usize;
        let mut deaths = 0usize;

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                let alive_neighbors = self.alive_neighbors(x, y);
                let alive = self.cells[idx].is_alive();

                let next_alive = if alive {
                    alive_neighbors == 2 || alive_neighbors == 3
                } else {
                    alive_neighbors == 3
                };

                self.scratch[idx] = if next_alive {
                    LifeCell::Alive
                } else {
                    LifeCell::Dead
                };

                if !alive && next_alive {
                    births = births.saturating_add(1);
                } else if alive && !next_alive {
                    deaths = deaths.saturating_add(1);
                }
            }
        }

        std::mem::swap(&mut self.cells, &mut self.scratch);

        let fingerprint = self.fingerprint();
        let after_alive = fingerprint.alive;

        self.stats.generation = self.stats.generation.saturating_add(1);
        self.stats.alive = after_alive;
        self.stats.births = births;
        self.stats.deaths = deaths;
        self.stats.peak_alive = self.stats.peak_alive.max(after_alive);

        if fingerprint.hash == previous_hash {
            self.stats.stable_ticks = self.stats.stable_ticks.saturating_add(1);
        } else {
            self.stats.stable_ticks = 0;
        }

        self.classify_pattern(before_alive, fingerprint);
        self.push_history(fingerprint);
    }

    pub fn seed_prime_soup(&mut self) {
        self.cells.fill(LifeCell::Dead);

        let soup_cells = (self.width * self.height / 38).max(12);

        for i in 0..soup_cells {
            let x = hash(self.seed ^ 0xA111_0C1C, i, self.width) % self.width;
            let y = hash(self.seed ^ 0xB317_0523, i, self.height) % self.height;

            if hash(self.seed ^ 0x5EED_1AFE, x, y) % 100 < 42 {
                let idx = self.idx(x, y);
                self.cells[idx] = LifeCell::Alive;
            }
        }

        self.seed_known_pattern(AxiomSeed::RPentomino, self.width / 3, self.height / 3);
        self.seed_known_pattern(AxiomSeed::Acorn, self.width * 2 / 3, self.height / 2);
        self.seed_known_pattern(AxiomSeed::Glider, self.width / 2, self.height / 4);

        self.stats.alive = self.alive_cells();
        self.stats.peak_alive = self.stats.alive;
        self.push_history(self.fingerprint());
    }

    pub fn seed_known_pattern(&mut self, seed: AxiomSeed, origin_x: usize, origin_y: usize) {
        for (dx, dy) in seed.points() {
            let x = wrap(origin_x as isize + dx, self.width);
            let y = wrap(origin_y as isize + dy, self.height);
            let idx = self.idx(x, y);
            self.cells[idx] = LifeCell::Alive;
        }
    }

    pub fn living_pressure_at_screen(
        &self,
        sx: usize,
        sy: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> f32 {
        if screen_w == 0 || screen_h == 0 {
            return 0.0;
        }

        let x = (sx * self.width / screen_w).min(self.width.saturating_sub(1));
        let y = (sy * self.height / screen_h).min(self.height.saturating_sub(1));

        let center = if self.cells[self.idx(x, y)].is_alive() {
            1.0
        } else {
            0.0
        };

        let neighbors = self.alive_neighbors(x, y) as f32 / 8.0;
        (center * 0.72 + neighbors * 0.28).clamp(0.0, 1.0)
    }

    pub fn sample_screen(
        &self,
        sx: usize,
        sy: usize,
        screen_w: usize,
        screen_h: usize,
    ) -> LifeCell {
        if screen_w == 0 || screen_h == 0 {
            return LifeCell::Dead;
        }

        let x = (sx * self.width / screen_w).min(self.width.saturating_sub(1));
        let y = (sy * self.height / screen_h).min(self.height.saturating_sub(1));

        self.cells[self.idx(x, y)]
    }

    pub fn stats(&self) -> AxiomStats {
        self.stats
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn alive_cells(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_alive()).count()
    }

    pub fn axiom_status_line(&self) -> String {
        format!(
            "AXIOM {:?} gen:{} live:{} birth:{} death:{} osc:{} drift:{}",
            self.stats.state,
            self.stats.generation,
            self.stats.alive,
            self.stats.births,
            self.stats.deaths,
            self.stats.oscillator_hits,
            self.stats.translation_hits
        )
    }

    fn classify_pattern(&mut self, before_alive: usize, fingerprint: LifeFingerprint) {
        if fingerprint.alive == 0 {
            self.stats.state = AxiomPatternState::Dormant;
            return;
        }

        if self.stats.stable_ticks >= 3 {
            self.stats.state = AxiomPatternState::Static;
            return;
        }

        if self
            .history
            .iter()
            .rev()
            .take(12)
            .any(|past| past.hash == fingerprint.hash)
        {
            self.stats.oscillator_hits = self.stats.oscillator_hits.saturating_add(1);
            self.stats.state = AxiomPatternState::Oscillating;
            return;
        }

        if let Some(past) = self.history.iter().rev().find(|past| {
            past.alive.abs_diff(fingerprint.alive) <= 3
                && past.hash != fingerprint.hash
                && centroid_distance(**past, fingerprint) >= 2
                && centroid_distance(**past, fingerprint) <= 9
        }) {
            let _ = past;
            self.stats.translation_hits = self.stats.translation_hits.saturating_add(1);
            self.stats.state = AxiomPatternState::Translating;
            return;
        }

        if fingerprint.alive > before_alive.saturating_add(before_alive / 3).max(4) {
            self.stats.state = AxiomPatternState::Expanding;
        } else if before_alive
            > fingerprint
                .alive
                .saturating_add(fingerprint.alive / 3)
                .max(4)
        {
            self.stats.state = AxiomPatternState::Collapsing;
        } else {
            self.stats.state = AxiomPatternState::Chaotic;
        }
    }

    fn push_history(&mut self, fingerprint: LifeFingerprint) {
        self.history.push(fingerprint);

        if self.history.len() > HISTORY_LIMIT {
            self.history.remove(0);
        }
    }

    fn fingerprint(&self) -> LifeFingerprint {
        let mut h = self.seed ^ 0xA0C1_0F1F_EC07_0523;
        let mut alive = 0usize;
        let mut sum_x = 0usize;
        let mut sum_y = 0usize;

        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[self.idx(x, y)].is_alive() {
                    alive = alive.saturating_add(1);
                    sum_x = sum_x.saturating_add(x);
                    sum_y = sum_y.saturating_add(y);

                    let local = hash(h, x, y) as u64;
                    h ^= local.rotate_left(((x + y) % 63) as u32 + 1);
                    h = h.wrapping_mul(0x9E37_79B9_7F4A_7C15);
                }
            }
        }

        LifeFingerprint {
            hash: h,
            alive,
            centroid_x: if alive == 0 {
                0
            } else {
                (sum_x / alive) as i32
            },
            centroid_y: if alive == 0 {
                0
            } else {
                (sum_y / alive) as i32
            },
        }
    }

    fn alive_neighbors(&self, x: usize, y: usize) -> usize {
        let mut count = 0usize;

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);

                if self.cells[self.idx(nx, ny)].is_alive() {
                    count += 1;
                }
            }
        }

        count
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxiomSeed {
    Glider,
    Blinker,
    Toad,
    Beacon,
    RPentomino,
    Acorn,
    Diehard,
}

impl AxiomSeed {
    fn points(self) -> &'static [(isize, isize)] {
        match self {
            Self::Glider => &[(1, 0), (2, 1), (0, 2), (1, 2), (2, 2)],
            Self::Blinker => &[(0, 0), (1, 0), (2, 0)],
            Self::Toad => &[(1, 0), (2, 0), (3, 0), (0, 1), (1, 1), (2, 1)],
            Self::Beacon => &[(0, 0), (1, 0), (0, 1), (3, 2), (2, 3), (3, 3)],
            Self::RPentomino => &[(1, 0), (2, 0), (0, 1), (1, 1), (1, 2)],
            Self::Acorn => &[(1, 0), (3, 1), (0, 2), (1, 2), (4, 2), (5, 2), (6, 2)],
            Self::Diehard => &[(6, 0), (0, 1), (1, 1), (1, 2), (5, 2), (6, 2), (7, 2)],
        }
    }
}

fn centroid_distance(a: LifeFingerprint, b: LifeFingerprint) -> i32 {
    (a.centroid_x - b.centroid_x).abs() + (a.centroid_y - b.centroid_y).abs()
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
