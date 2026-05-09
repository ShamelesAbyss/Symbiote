use crate::{
    particle::{Particle, RareTrait},
    species::Archetype,
    tree::{self, TreePolicy},
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
    Root,
}

impl CellKind {
    pub fn is_alive(self) -> bool {
        matches!(self, Self::Life | Self::Spore | Self::Nest | Self::Root)
    }

    pub fn is_protected(self) -> bool {
        matches!(self, Self::Root)
    }

    pub fn is_regenerative(self) -> bool {
        matches!(self, Self::Dead | Self::Nutrient | Self::Spore | Self::Root)
    }

    pub fn food_value(self) -> f32 {
        match self {
            Self::Life => 11.0,
            Self::Nutrient => 0.0,
            Self::Spore => 0.0,
            Self::Nest => 18.0,
            Self::Mutagen => 7.0,
            Self::Dead => 0.0,
            Self::Root => 0.0,
            Self::Empty => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Signal {
    #[serde(default)]
    pub hunger: f32,
    #[serde(default)]
    pub fear: f32,
    #[serde(default)]
    pub growth: f32,
    #[serde(default)]
    pub danger: f32,
}

impl Signal {
    pub fn strongest(self) -> Option<(SignalKind, f32)> {
        let mut kind = SignalKind::Hunger;
        let mut value = self.hunger;

        if self.fear > value {
            kind = SignalKind::Fear;
            value = self.fear;
        }

        if self.growth > value {
            kind = SignalKind::Growth;
            value = self.growth;
        }

        if self.danger > value {
            kind = SignalKind::Danger;
            value = self.danger;
        }

        if value > 0.08 {
            Some((kind, value))
        } else {
            None
        }
    }

    fn decay(&mut self, recovery_mode: bool) {
        let slow = if recovery_mode { 0.972 } else { 0.955 };
        let fast = if recovery_mode { 0.94 } else { 0.925 };

        self.hunger = (self.hunger * slow - 0.002).max(0.0);
        self.fear = (self.fear * fast - 0.002).max(0.0);
        self.growth = (self.growth * slow - 0.0015).max(0.0);
        self.danger = (self.danger * fast - 0.0025).max(0.0);
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq)]
pub enum SignalKind {
    Hunger,
    Fear,
    Growth,
    Danger,
}

impl SignalKind {
    pub fn glyph(self) -> char {
        match self {
            Self::Hunger => '∿',
            Self::Fear => '!',
            Self::Growth => '∙',
            Self::Danger => '∷',
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Cell {
    pub kind: CellKind,
    pub energy: f32,
    pub age: u16,
    pub tribe_hint: usize,
    #[serde(default)]
    pub signal: Signal,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            kind: CellKind::Empty,
            energy: 0.0,
            age: 0,
            tribe_hint: 0,
            signal: Signal::default(),
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
        let living = snapshot.iter().filter(|cell| cell.kind.is_alive()).count();
        let total = snapshot.len().max(1);
        let density = living as f32 / total as f32;

        let recovery_mode = living < 1_400;
        let bloom_mode = living < 800;

        let root_count = snapshot
            .iter()
            .filter(|cell| cell.kind == CellKind::Root)
            .count();

        let tree_policy = TreePolicy::default();
        let root_cap = tree::root_cap(total, self.width, tree_policy);

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = self.idx(x, y);
                let cell = snapshot[idx];

                let neighbors = self.alive_neighbors(&snapshot, x, y);
                let conway_neighbors = self.life_neighbors(&snapshot, x, y);
                let nutrient_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Nutrient);
                let dead_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Dead);
                let spore_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Spore);
                let root_neighbors = self.kind_neighbors(&snapshot, x, y, CellKind::Root);

                let mut next = cell;
                next.signal.decay(recovery_mode);

                // --- ROOT SOFT INVASION LAYER ---
                // Build on top of the existing ecology instead of replacing it:
                // roots remain permanent terrain, but tendrils can claim soft substrate
                // when they are continuing a valid root path.
                let root_seed_roll = hash(self.seed ^ self.cycle ^ 0xA17E_5EED, x, y) % 10_000;
                let soft_root_target = matches!(
                    cell.kind,
                    CellKind::Empty
                        | CellKind::Life
                        | CellKind::Nutrient
                        | CellKind::Dead
                        | CellKind::Spore
                );

                if soft_root_target
                    && self.should_grow_trunk_root(
                        &snapshot,
                        x,
                        y,
                        root_count,
                        root_cap,
                        root_neighbors,
                        neighbors,
                        root_seed_roll,
                    )
                {
                    next.kind = CellKind::Root;
                    next.energy = 96.0;
                    next.age = 0;
                    next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                    next.signal.growth = (next.signal.growth + 0.24).clamp(0.0, 1.0);
                    self.cells[idx] = next;
                    continue;
                }

                match cell.kind {
                    CellKind::Empty => {
                        let seed_roll = hash(self.seed ^ self.cycle, x, y) % 10_000;

                        if self.should_grow_trunk_root(
                            &snapshot,
                            x,
                            y,
                            root_count,
                            root_cap,
                            root_neighbors,
                            neighbors,
                            seed_roll,
                        ) {
                            next.kind = CellKind::Root;
                            next.energy = 92.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                            next.signal.growth = (next.signal.growth + 0.18).clamp(0.0, 1.0);
                        } else if conway_neighbors == 3
                            || (neighbors == 3 && nutrient_neighbors >= 1)
                        {
                            next.kind = CellKind::Life;
                            next.energy = 64.0 + nutrient_neighbors as f32 * 7.5;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                        } else if recovery_mode && root_neighbors > 0 && nutrient_neighbors > 0 {
                            next.kind = CellKind::Life;
                            next.energy = 31.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                        } else if recovery_mode && nutrient_neighbors >= 1 && spore_neighbors >= 1 {
                            next.kind = CellKind::Life;
                            next.energy = 27.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                        } else if bloom_mode && seed_roll < 160 {
                            next.kind = CellKind::Nutrient;
                            next.energy = 38.0;
                            next.age = 0;
                            next.tribe_hint = seed_roll % 6;
                            next.signal.growth = (next.signal.growth + 0.06).clamp(0.0, 1.0);
                        } else if recovery_mode && seed_roll < 70 {
                            next.kind = CellKind::Spore;
                            next.energy = 30.0;
                            next.age = 0;
                            next.tribe_hint = seed_roll % 6;
                            next.signal.growth = (next.signal.growth + 0.05).clamp(0.0, 1.0);
                        }
                    }

                    CellKind::Life => {
                        if conway_neighbors < 2 {
                            next.kind = CellKind::Dead;
                            next.energy = 14.0;
                            next.signal.danger = (next.signal.danger + 0.16).clamp(0.0, 1.0);
                        } else if conway_neighbors > 3 {
                            next.kind = CellKind::Dead;
                            next.energy = 24.0;
                            next.signal.danger = (next.signal.danger + 0.22).clamp(0.0, 1.0);
                        } else if root_neighbors > 1 {
                            next.kind = CellKind::Dead;
                            next.energy = 24.0;
                            next.signal.danger = (next.signal.danger + 0.18).clamp(0.0, 1.0);
                        } else if root_neighbors > 0
                            && recovery_mode
                            && neighbors >= 1
                            && neighbors <= 4
                        {
                            next.energy = (cell.energy + 0.35).min(85.0);
                            next.signal.growth = (next.signal.growth + 0.03).clamp(0.0, 1.0);
                        } else if neighbors < 2 || neighbors > 3 {
                            next.kind = CellKind::Dead;
                            next.energy = 24.0;
                            next.signal.danger = (next.signal.danger + 0.22).clamp(0.0, 1.0);
                        } else if neighbors == 3 && nutrient_neighbors > 2 {
                            next.kind = CellKind::Spore;
                            next.energy = (cell.energy + 3.0).min(85.0);
                            next.signal.growth = (next.signal.growth + 0.08).clamp(0.0, 1.0);
                        } else {
                            let age_thin = if cell.age > 900 { 0.06 } else { 0.0 };
                            let crowd_thin = if density > 0.82 { 0.06 } else { 0.0 };

                            next.energy = (cell.energy + nutrient_neighbors as f32 * 1.20
                                - 1.22
                                - age_thin
                                - crowd_thin)
                                .clamp(0.0, 85.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Dead;
                                next.energy = 12.0;
                                next.signal.danger = (next.signal.danger + 0.16).clamp(0.0, 1.0);
                            }
                        }
                    }

                    CellKind::Spore => {
                        next.signal.growth = (next.signal.growth + 0.012).clamp(0.0, 1.0);

                        if conway_neighbors == 3 && root_neighbors == 0 {
                            next.kind = CellKind::Life;
                            next.energy = 48.0;
                            next.age = 0;
                        } else if root_neighbors > 1 {
                            next.kind = CellKind::Nutrient;
                            next.energy = 28.0;
                        } else if root_neighbors > 0 && recovery_mode && nutrient_neighbors > 0 {
                            next.kind = CellKind::Life;
                            next.energy = 52.0;
                        } else if neighbors < 2 || neighbors > 4 {
                            next.kind = CellKind::Dead;
                            next.energy = 14.0;
                            next.signal.danger = (next.signal.danger + 0.14).clamp(0.0, 1.0);
                        } else {
                            let age_thin = if cell.age > 860 { 0.03 } else { 0.0 };
                            let crowd_thin = if density > 0.82 { 0.04 } else { 0.0 };

                            next.energy = (cell.energy - 1.02 - age_thin - crowd_thin).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Dead;
                                next.energy = 8.0;
                                next.signal.danger = (next.signal.danger + 0.12).clamp(0.0, 1.0);
                            }
                        }
                    }

                    CellKind::Nutrient => {
                        next.signal.growth = (next.signal.growth + 0.006).clamp(0.0, 1.0);

                        if conway_neighbors == 3 && root_neighbors == 0 {
                            next.kind = CellKind::Life;
                            next.energy = 54.0;
                            next.age = 0;
                        } else if root_neighbors > 1 && density > 0.14 {
                            next.kind = CellKind::Empty;
                            next.energy = 0.0;
                            next.age = 0;
                        } else if root_neighbors > 0 && recovery_mode && neighbors >= 1 {
                            next.kind = CellKind::Spore;
                            next.energy = 52.0;
                        } else if ((neighbors == 3 || conway_neighbors == 3)
                            || (dead_neighbors >= 2 && nutrient_neighbors >= 2))
                            && cell.energy > 12.0
                        {
                            next.kind = CellKind::Life;
                            next.energy = 58.0;
                            next.age = 0;
                        } else if recovery_mode && neighbors >= 2 {
                            next.kind = CellKind::Spore;
                            next.energy = 35.0;
                        } else {
                            let decay = if recovery_mode { 0.004 } else { 0.018 };
                            next.energy = (cell.energy - decay).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }

                    CellKind::Dead => {
                        next.signal.danger = (next.signal.danger + 0.006).clamp(0.0, 1.0);

                        if conway_neighbors == 3 || (dead_neighbors >= 2 && nutrient_neighbors >= 2)
                        {
                            next.kind = CellKind::Life;
                            next.energy = 48.0;
                            next.age = 0;
                            next.tribe_hint = self.local_tribe_hint(&snapshot, x, y);
                            next.signal.growth = (next.signal.growth + 0.10).clamp(0.0, 1.0);
                        } else {
                            let decay = if recovery_mode { 0.02 } else { 0.08 };
                            next.energy = (cell.energy - decay).max(0.0);

                            if root_neighbors > 0 && recovery_mode {
                                next.kind = CellKind::Nutrient;
                                next.energy = 30.0;
                                next.signal.growth = (next.signal.growth + 0.12).clamp(0.0, 1.0);
                            } else if nutrient_neighbors >= 2 && neighbors >= 2 {
                                next.kind = CellKind::Nutrient;
                                next.energy = 32.0;
                                next.signal.growth = (next.signal.growth + 0.08).clamp(0.0, 1.0);
                            } else if recovery_mode && dead_neighbors >= 2 && density < 0.06 {
                                next.kind = CellKind::Nutrient;
                                next.energy = 24.0;
                                next.signal.growth = (next.signal.growth + 0.06).clamp(0.0, 1.0);
                            } else if next.energy <= 0.0 || dead_neighbors > 5 {
                                next.kind = CellKind::Empty;
                                next.age = 0;
                            }
                        }
                    }

                    CellKind::Mutagen => {
                        if root_neighbors > 0 && recovery_mode {
                            next.kind = CellKind::Spore;
                            next.energy = 52.0;
                            next.signal.growth = (next.signal.growth + 0.08).clamp(0.0, 1.0);
                        } else if neighbors == 3 {
                            next.kind = CellKind::Spore;
                            next.energy = 58.0;
                            next.signal.growth = (next.signal.growth + 0.08).clamp(0.0, 1.0);
                        } else {
                            next.energy = (cell.energy - 0.18).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Nutrient;
                                next.energy = 20.0;
                            }
                        }
                    }

                    CellKind::Nest => {
                        next.signal.growth = (next.signal.growth + 0.018).clamp(0.0, 1.0);

                        if root_neighbors > 1 || neighbors > 4 {
                            next.kind = CellKind::Dead;
                            next.energy = 20.0;
                            next.signal.danger = (next.signal.danger + 0.20).clamp(0.0, 1.0);
                        } else if recovery_mode && neighbors >= 1 {
                            next.energy = (cell.energy + 0.12).min(90.0);
                        } else {
                            next.energy = (cell.energy - 0.05).max(0.0);

                            if next.energy <= 0.0 {
                                next.kind = CellKind::Nutrient;
                                next.energy = 22.0;
                            }
                        }
                    }

                    CellKind::Root => {
                        let seed_roll = hash(self.seed ^ self.cycle ^ 0xBEEF, x, y) % 10_000;

                        next.signal.growth = (next.signal.growth + 0.02).clamp(0.0, 1.0);

                        if root_neighbors > 4 {
                            next.kind = CellKind::Nutrient;
                            next.energy = 44.0;
                            next.signal.danger = (next.signal.danger + 0.10).clamp(0.0, 1.0);
                        } else if root_count > root_cap && root_neighbors > 2 && seed_roll < 120 {
                            next.kind = CellKind::Dead;
                            next.energy = 22.0;
                            next.signal.danger = (next.signal.danger + 0.08).clamp(0.0, 1.0);
                        } else {
                            let trunk_bonus = if cell.age > 240 { 0.015 } else { 0.006 };
                            next.energy = (cell.energy + trunk_bonus).clamp(65.0, 100.0);
                        }
                    }
                }

                if next.kind != CellKind::Empty {
                    next.age = next.age.saturating_add(1);
                }

                if self.cells[idx].kind == CellKind::Root {
                    next.kind = CellKind::Root;
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

        if cell.kind.is_protected() {
            return;
        }

        let deposit_allowed = match cell.kind {
            CellKind::Empty => true,
            CellKind::Dead => particle.energy > 80.0,
            CellKind::Nutrient => {
                matches!(archetype, Some(Archetype::Grazer | Archetype::Mycelial))
            }
            CellKind::Mutagen => particle.rare_trait != RareTrait::None,
            CellKind::Life | CellKind::Spore | CellKind::Nest => particle.energy > 125.0,
            CellKind::Root => false,
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
                Some(Archetype::Reaper) => CellKind::Dead,
                _ => CellKind::Life,
            }
        };

        if cell.kind != CellKind::Root {
            cell.kind = desired;
        }
        cell.energy = (cell.energy + particle.energy * 0.055).clamp(0.0, 85.0);
        cell.tribe_hint = particle.tribe.index();

        match archetype {
            Some(Archetype::Harvester) => {
                cell.signal.hunger = (cell.signal.hunger + 0.16).clamp(0.0, 1.0);
            }
            Some(Archetype::Reaper) => {
                cell.signal.fear = (cell.signal.fear + 0.18).clamp(0.0, 1.0);
            }
            Some(
                Archetype::Grazer
                | Archetype::Mycelial
                | Archetype::Architect
                | Archetype::Leviathan,
            ) => {
                cell.signal.growth = (cell.signal.growth + 0.12).clamp(0.0, 1.0);
            }
            Some(Archetype::Hunter | Archetype::Parasite) => {
                cell.signal.danger = (cell.signal.danger + 0.10).clamp(0.0, 1.0);
            }
            _ => {}
        }
    }

    pub fn deposit_signal(&mut self, x: f32, y: f32, kind: SignalKind, amount: f32) {
        let Some((gx, gy)) = self.world_to_grid(x, y) else {
            return;
        };

        let idx = self.idx(gx, gy);
        let cell = &mut self.cells[idx];

        match kind {
            SignalKind::Hunger => {
                cell.signal.hunger = (cell.signal.hunger + amount).clamp(0.0, 1.0);
            }
            SignalKind::Fear => {
                cell.signal.fear = (cell.signal.fear + amount).clamp(0.0, 1.0);
            }
            SignalKind::Growth => {
                cell.signal.growth = (cell.signal.growth + amount).clamp(0.0, 1.0);
            }
            SignalKind::Danger => {
                cell.signal.danger = (cell.signal.danger + amount).clamp(0.0, 1.0);
            }
        }
    }

    pub fn consume_at(&mut self, x: f32, y: f32, power: f32, compost: bool) -> Option<CellKind> {
        let (gx, gy) = self.world_to_grid(x, y)?;
        let idx = self.idx(gx, gy);
        let cell = &mut self.cells[idx];

        if cell.kind == CellKind::Empty || cell.kind.is_protected() || cell.kind.is_regenerative() {
            return None;
        }

        let eaten = cell.kind;

        cell.energy -= power;
        cell.signal.hunger = (cell.signal.hunger + 0.22).clamp(0.0, 1.0);
        cell.signal.danger = (cell.signal.danger + 0.06).clamp(0.0, 1.0);

        if cell.energy <= 0.0
            || matches!(eaten, CellKind::Life | CellKind::Nest | CellKind::Mutagen)
        {
            if compost {
                cell.kind = match eaten {
                    CellKind::Life | CellKind::Nest => CellKind::Dead,
                    CellKind::Mutagen => CellKind::Nutrient,
                    CellKind::Spore => CellKind::Spore,
                    CellKind::Nutrient => CellKind::Nutrient,
                    CellKind::Dead => CellKind::Dead,
                    CellKind::Root => CellKind::Root,
                    CellKind::Empty => CellKind::Empty,
                };

                cell.energy = match cell.kind {
                    CellKind::Dead => 22.0,
                    CellKind::Spore => 22.0,
                    CellKind::Nutrient => 28.0,
                    CellKind::Root => 95.0,
                    _ => 0.0,
                };

                cell.age = 0;
            } else {
                if cell.kind != CellKind::Root {
                    cell.kind = CellKind::Empty;
                }
                cell.energy = 0.0;
                cell.age = 0;
            }
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

    pub fn signal_at(&self, x: f32, y: f32) -> Signal {
        if let Some((gx, gy)) = self.world_to_grid(x, y) {
            self.cells[self.idx(gx, gy)].signal
        } else {
            Signal::default()
        }
    }

    pub fn living_cells(&self) -> usize {
        self.cells
            .iter()
            .filter(|cell| cell.kind.is_alive())
            .count()
    }

    pub fn protected_cells(&self) -> usize {
        self.cells
            .iter()
            .filter(|cell| cell.kind.is_protected())
            .count()
    }

    pub fn total_cells(&self) -> usize {
        self.cells.len()
    }

    fn seed_initial_life(&mut self) {
        let ground_start = self.height.saturating_sub(4);

        for y in 0..self.height {
            for x in 0..self.width {
                let n = hash(self.seed, x, y) % 10_000;
                let idx = self.idx(x, y);

                self.cells[idx] = if y >= ground_start && n < 210 {
                    Cell {
                        kind: CellKind::Root,
                        energy: 96.0,
                        age: 0,
                        tribe_hint: n % 6,
                        signal: Signal {
                            growth: 0.18,
                            ..Signal::default()
                        },
                    }
                } else if n < 48 {
                    Cell {
                        kind: CellKind::Life,
                        energy: 34.0,
                        age: 0,
                        tribe_hint: n % 6,
                        signal: Signal::default(),
                    }
                } else if n < 88 {
                    Cell {
                        kind: CellKind::Nutrient,
                        energy: 48.0,
                        age: 0,
                        tribe_hint: n % 6,
                        signal: Signal {
                            growth: 0.08,
                            ..Signal::default()
                        },
                    }
                } else if n < 108 {
                    Cell {
                        kind: CellKind::Spore,
                        energy: 36.0,
                        age: 0,
                        tribe_hint: n % 6,
                        signal: Signal {
                            growth: 0.06,
                            ..Signal::default()
                        },
                    }
                } else if n < 114 {
                    Cell {
                        kind: CellKind::Mutagen,
                        energy: 55.0,
                        age: 0,
                        tribe_hint: n % 6,
                        signal: Signal::default(),
                    }
                } else {
                    Cell::default()
                };
            }
        }
    }

    fn should_grow_trunk_root(
        &self,
        snapshot: &[Cell],
        x: usize,
        y: usize,
        root_count: usize,
        root_cap: usize,
        root_neighbors: usize,
        alive_neighbors: usize,
        seed_roll: usize,
    ) -> bool {
        if root_count >= root_cap {
            return false;
        }

        if root_neighbors == 0 || root_neighbors > 3 {
            return false;
        }

        if alive_neighbors > 8 {
            return false;
        }

        let local_roots = self.kind_radius_neighbors(snapshot, x, y, CellKind::Root, 2);
        if local_roots > 5 {
            return false;
        }

        let near_wall = x <= 1 || x + 2 >= self.width;

        let vertical_parent =
            y + 1 < self.height && snapshot[self.idx(x, y + 1)].kind == CellKind::Root;

        let diagonal_parent = (x > 0
            && y + 1 < self.height
            && snapshot[self.idx(x - 1, y + 1)].kind == CellKind::Root)
            || (x + 1 < self.width
                && y + 1 < self.height
                && snapshot[self.idx(x + 1, y + 1)].kind == CellKind::Root);

        let lateral_parent = (x > 0 && snapshot[self.idx(x - 1, y)].kind == CellKind::Root)
            || (x + 1 < self.width && snapshot[self.idx(x + 1, y)].kind == CellKind::Root);

        if !tree::allow_root_direction(near_wall, vertical_parent, diagonal_parent, lateral_parent)
        {
            return false;
        }

        let parent_age = self.oldest_neighbor_age(snapshot, x, y, CellKind::Root);
        if parent_age < 18 {
            return false;
        }

        let height_ratio = y as f32 / self.height.max(1) as f32;

        let growth_cadence = if self.cycle < 1_500 && height_ratio > 0.48 {
            4
        } else if vertical_parent {
            6
        } else {
            11
        };

        let phase = hash(self.seed ^ 0x51A7_EEAF ^ self.cycle, x, y) as u64;
        if (self.cycle + phase) % growth_cadence != 0 {
            return false;
        }

        let wiggle_roll = hash(self.seed ^ 0xC0FF_EE11 ^ self.cycle, x, y) % 10_000;
        if !tree::accept_wiggle(diagonal_parent, near_wall && lateral_parent, wiggle_roll) {
            return false;
        }

        let pressure = tree::growth_pressure(
            self.cycle,
            height_ratio,
            vertical_parent,
            diagonal_parent,
            near_wall && lateral_parent,
            u32::from(parent_age),
            root_count,
            root_cap,
        );

        seed_roll < pressure
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

    fn life_neighbors(&self, snapshot: &[Cell], x: usize, y: usize) -> usize {
        let mut count = 0;

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);

                if snapshot[self.idx(nx, ny)].kind == CellKind::Life {
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

    fn kind_radius_neighbors(
        &self,
        snapshot: &[Cell],
        x: usize,
        y: usize,
        kind: CellKind,
        radius: isize,
    ) -> usize {
        let mut count = 0;

        for dy in -radius..=radius {
            for dx in -radius..=radius {
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

    fn oldest_neighbor_age(&self, snapshot: &[Cell], x: usize, y: usize, kind: CellKind) -> u16 {
        let mut oldest = 0;

        for dy in [-1isize, 0, 1] {
            for dx in [-1isize, 0, 1] {
                if dx == 0 && dy == 0 {
                    continue;
                }

                let nx = wrap(x as isize + dx, self.width);
                let ny = wrap(y as isize + dy, self.height);
                let cell = snapshot[self.idx(nx, ny)];

                if cell.kind == kind {
                    oldest = oldest.max(cell.age);
                }
            }
        }

        oldest
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
