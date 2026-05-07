//! Persistent spatial pattern field for Symbiote.
//!
//! This layer stores low-resolution memory across the organism field.
//! It is intentionally passive until app/sim/render wire into it.

use crate::pattern::{PatternKind, PatternMotion, PatternSignature};

#[derive(Clone, Copy, Debug)]
pub struct FieldConfig {
    pub cell_size: f32,
    pub decay: f32,
    pub diffusion: f32,
    pub max_intensity: f32,
}

impl Default for FieldConfig {
    fn default() -> Self {
        Self {
            cell_size: 0.045,
            decay: 0.985,
            diffusion: 0.055,
            max_intensity: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct FieldCell {
    pub intensity: f32,
    pub stability: f32,
    pub pulse: f32,
    pub drift: f32,
    pub cohesion: f32,
    pub danger: f32,
    pub vx: f32,
    pub vy: f32,
    pub kind: PatternKind,
    pub motion: PatternMotion,
    pub age: u16,
}

impl Default for FieldCell {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            stability: 0.0,
            pulse: 0.0,
            drift: 0.0,
            cohesion: 0.0,
            danger: 0.0,
            vx: 0.0,
            vy: 0.0,
            kind: PatternKind::Dormant,
            motion: PatternMotion::Static,
            age: 0,
        }
    }
}

impl FieldCell {
    pub fn is_active(self) -> bool {
        self.intensity > 0.035 || self.age > 0
    }

    pub fn reinforce(
        &mut self,
        signature: PatternSignature,
        vx: f32,
        vy: f32,
        config: FieldConfig,
    ) {
        let incoming = signature.intensity().clamp(0.0, config.max_intensity);
        let blend = (0.18 + incoming * 0.34).clamp(0.18, 0.52);

        self.intensity = lerp(self.intensity, incoming, blend).clamp(0.0, config.max_intensity);
        self.stability = lerp(self.stability, signature.stability, blend).clamp(0.0, 1.0);
        self.pulse = lerp(self.pulse, signature.pulse, blend).clamp(0.0, 1.0);
        self.drift = lerp(self.drift, signature.drift, blend).clamp(0.0, 1.0);
        self.cohesion = lerp(self.cohesion, signature.cohesion, blend).clamp(0.0, 1.0);
        self.danger = lerp(self.danger, signature.danger, blend).clamp(0.0, 1.0);
        self.vx = lerp(self.vx, vx, blend * 0.62).clamp(-1.0, 1.0);
        self.vy = lerp(self.vy, vy, blend * 0.62).clamp(-1.0, 1.0);

        if incoming >= self.intensity * 0.72 || signature.kind != PatternKind::Dormant {
            self.kind = signature.kind;
            self.motion = signature.motion;
        }

        self.age = self.age.saturating_add(1);
    }

    pub fn decay(&mut self, config: FieldConfig) {
        self.intensity *= config.decay;
        self.stability *= config.decay;
        self.pulse *= config.decay;
        self.drift *= config.decay;
        self.cohesion *= config.decay;
        self.danger *= config.decay;
        self.vx *= config.decay;
        self.vy *= config.decay;

        if self.intensity < 0.012 {
            self.kind = PatternKind::Dormant;
            self.motion = PatternMotion::Static;
        }

        if self.intensity < 0.003 {
            *self = Self::default();
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Default)]
pub struct FieldSample {
    pub intensity: f32,
    pub stability: f32,
    pub pulse: f32,
    pub drift: f32,
    pub cohesion: f32,
    pub danger: f32,
    pub vx: f32,
    pub vy: f32,
    pub kind: PatternKind,
    pub motion: PatternMotion,
}

#[allow(dead_code)]
impl FieldSample {
    pub fn influence_strength(self) -> f32 {
        (self.intensity * 0.42
            + self.cohesion * 0.22
            + self.drift * 0.16
            + self.pulse * 0.12
            + self.stability * 0.08)
            .clamp(0.0, 1.0)
    }

    pub fn is_dangerous(self) -> bool {
        self.danger > 0.48 && self.intensity > 0.18
    }
}

pub struct PatternField {
    width: usize,
    height: usize,
    config: FieldConfig,
    cells: Vec<FieldCell>,
    tick: u64,
}

impl PatternField {
    pub fn new(world_width: usize, world_height: usize, config: FieldConfig) -> Self {
        let width = ((world_width as f32 * config.cell_size).ceil() as usize).max(8);
        let height = ((world_height as f32 * config.cell_size).ceil() as usize).max(6);

        Self {
            width,
            height,
            config,
            cells: vec![FieldCell::default(); width.saturating_mul(height)],
            tick: 0,
        }
    }

    pub fn dimensions(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn tick(&self) -> u64 {
        self.tick
    }

    pub fn active_cells(&self) -> usize {
        self.cells.iter().filter(|cell| cell.is_active()).count()
    }

    pub fn average_intensity(&self) -> f32 {
        if self.cells.is_empty() {
            return 0.0;
        }

        self.cells.iter().map(|cell| cell.intensity).sum::<f32>() / self.cells.len() as f32
    }

    pub fn strongest_kind(&self) -> PatternKind {
        self.cells
            .iter()
            .max_by(|a, b| a.intensity.total_cmp(&b.intensity))
            .map(|cell| cell.kind)
            .unwrap_or(PatternKind::Dormant)
    }

    pub fn step(&mut self) {
        self.tick = self.tick.saturating_add(1);

        for cell in &mut self.cells {
            cell.decay(self.config);
        }

        if self.config.diffusion > 0.0 && self.tick % 3 == 0 {
            self.diffuse();
        }
    }

    pub fn reinforce_world(
        &mut self,
        x: f32,
        y: f32,
        signature: PatternSignature,
        vx: f32,
        vy: f32,
    ) {
        if let Some(idx) = self.world_index(x, y) {
            self.cells[idx].reinforce(signature, vx, vy, self.config);
        }
    }

    #[allow(dead_code)]
    pub fn sample_world(&self, x: f32, y: f32) -> FieldSample {
        let Some((gx, gy)) = self.world_to_grid(x, y) else {
            return FieldSample::default();
        };

        self.sample_grid(gx, gy)
    }

    #[allow(dead_code)]
    pub fn sample_grid(&self, gx: usize, gy: usize) -> FieldSample {
        let mut sample = FieldSample::default();
        let mut total = 0.0_f32;

        for oy in -1..=1 {
            for ox in -1..=1 {
                let nx = gx as isize + ox;
                let ny = gy as isize + oy;

                if nx < 0 || ny < 0 || nx >= self.width as isize || ny >= self.height as isize {
                    continue;
                }

                let idx = self.idx(nx as usize, ny as usize);
                let cell = self.cells[idx];

                let weight = if ox == 0 && oy == 0 { 1.0 } else { 0.42 };
                let weighted = cell.intensity * weight;

                sample.intensity += weighted;
                sample.stability += cell.stability * weighted;
                sample.pulse += cell.pulse * weighted;
                sample.drift += cell.drift * weighted;
                sample.cohesion += cell.cohesion * weighted;
                sample.danger += cell.danger * weighted;
                sample.vx += cell.vx * weighted;
                sample.vy += cell.vy * weighted;

                if weighted > total {
                    sample.kind = cell.kind;
                    sample.motion = cell.motion;
                }

                total += weighted;
            }
        }

        if total > 0.0 {
            sample.intensity = (sample.intensity / total).clamp(0.0, 1.0);
            sample.stability = (sample.stability / total).clamp(0.0, 1.0);
            sample.pulse = (sample.pulse / total).clamp(0.0, 1.0);
            sample.drift = (sample.drift / total).clamp(0.0, 1.0);
            sample.cohesion = (sample.cohesion / total).clamp(0.0, 1.0);
            sample.danger = (sample.danger / total).clamp(0.0, 1.0);
            sample.vx = (sample.vx / total).clamp(-1.0, 1.0);
            sample.vy = (sample.vy / total).clamp(-1.0, 1.0);
        }

        sample
    }

    pub fn cells(&self) -> &[FieldCell] {
        &self.cells
    }

    fn diffuse(&mut self) {
        let mut next = self.cells.clone();
        let diffusion = self.config.diffusion.clamp(0.0, 0.25);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                let current = self.cells[idx];

                if !current.is_active() {
                    continue;
                }

                let mut neighbor_count = 0.0_f32;
                let mut intensity = 0.0_f32;
                let mut stability = 0.0_f32;
                let mut pulse = 0.0_f32;
                let mut drift = 0.0_f32;
                let mut cohesion = 0.0_f32;
                let mut danger = 0.0_f32;
                let mut vx = 0.0_f32;
                let mut vy = 0.0_f32;

                for (nx, ny) in self.neighbors4(x, y) {
                    let n = self.cells[self.idx(nx, ny)];
                    intensity += n.intensity;
                    stability += n.stability;
                    pulse += n.pulse;
                    drift += n.drift;
                    cohesion += n.cohesion;
                    danger += n.danger;
                    vx += n.vx;
                    vy += n.vy;
                    neighbor_count += 1.0;
                }

                if neighbor_count > 0.0 {
                    let cell = &mut next[idx];
                    cell.intensity = lerp(cell.intensity, intensity / neighbor_count, diffusion);
                    cell.stability = lerp(cell.stability, stability / neighbor_count, diffusion);
                    cell.pulse = lerp(cell.pulse, pulse / neighbor_count, diffusion);
                    cell.drift = lerp(cell.drift, drift / neighbor_count, diffusion);
                    cell.cohesion = lerp(cell.cohesion, cohesion / neighbor_count, diffusion);
                    cell.danger = lerp(cell.danger, danger / neighbor_count, diffusion);
                    cell.vx = lerp(cell.vx, vx / neighbor_count, diffusion);
                    cell.vy = lerp(cell.vy, vy / neighbor_count, diffusion);
                }
            }
        }

        self.cells = next;
    }

    fn world_index(&self, x: f32, y: f32) -> Option<usize> {
        let (gx, gy) = self.world_to_grid(x, y)?;
        Some(self.idx(gx, gy))
    }

    fn world_to_grid(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }

        let gx = (((x + 1.0) * 0.5) * self.width as f32).floor() as isize;
        let gy = (((y + 1.0) * 0.5) * self.height as f32).floor() as isize;

        if gx < 0 || gy < 0 || gx >= self.width as isize || gy >= self.height as isize {
            return None;
        }

        Some((gx as usize, gy as usize))
    }

    fn neighbors4(&self, x: usize, y: usize) -> impl Iterator<Item = (usize, usize)> + '_ {
        let mut out = [(usize::MAX, usize::MAX); 4];
        let mut len = 0usize;

        if x > 0 {
            out[len] = (x - 1, y);
            len += 1;
        }

        if x + 1 < self.width {
            out[len] = (x + 1, y);
            len += 1;
        }

        if y > 0 {
            out[len] = (x, y - 1);
            len += 1;
        }

        if y + 1 < self.height {
            out[len] = (x, y + 1);
            len += 1;
        }

        out.into_iter().take(len)
    }

    fn idx(&self, x: usize, y: usize) -> usize {
        y.saturating_mul(self.width).saturating_add(x)
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0.0, 1.0)
}
