use crate::density::{DensityConfig, DensityPressure, DensitySnapshot};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

const MEMORY_PATH: &str = "memory/session_memory.json";

#[derive(Serialize, Deserialize, Clone)]
pub struct MemoryBank {
    pub seed: u64,

    pub longest_age: u64,
    pub highest_generation: u64,

    pub peak_population: usize,
    pub peak_clusters: usize,
    pub peak_species: usize,
    pub peak_rare_lifeforms: usize,
    pub peak_living_cells: usize,
    pub peak_harvesters: usize,
    pub peak_reapers: usize,

    pub total_cells_consumed: u64,
    pub total_harvesters_consumed: u64,
    pub total_species_created: u64,
    pub total_extinctions: u64,
    pub total_births: u64,
    pub total_deaths: u64,
    pub total_reproductions: u64,
    pub total_fusions: u64,
    pub total_merges: u64,
    pub total_splits: u64,

    pub strongest_cluster_size: usize,
    pub strongest_cluster_age: u64,

    pub dominant_archetype: String,
    pub richest_zone: String,

    #[serde(default)]
    pub peak_root_cells: usize,
    #[serde(default)]
    pub peak_tree_trunk_cells: usize,
    #[serde(default)]
    pub peak_tree_branch_cells: usize,
    #[serde(default)]
    pub peak_tree_canopy_cells: usize,
    #[serde(default)]
    pub tree_growth_events: u64,
    #[serde(default)]
    pub tree_wall_events: u64,
    #[serde(default)]
    pub tree_surface_flow_events: u64,
    #[serde(default)]
    pub peak_root_pressure: f32,
    #[serde(default)]
    pub root_pressure_average: f32,
    #[serde(default)]
    pub root_pressure_samples: u64,
    #[serde(default)]
    pub root_collision_events: u64,
    #[serde(default)]
    pub root_corridor_events: u64,
    #[serde(default)]
    pub root_choked_ticks: u64,
    #[serde(default)]
    pub substrate_starved_ticks: u64,
    #[serde(default)]
    pub substrate_overgrown_ticks: u64,
    #[serde(default)]
    pub substrate_balance_average: f32,
    #[serde(default)]
    pub substrate_balance_samples: u64,
    #[serde(default)]
    pub adaptive_root_bias: f32,
    #[serde(default)]
    pub adaptive_corridor_bias: f32,
    #[serde(default)]
    pub adaptive_substrate_throttle: f32,

    #[serde(default)]
    pub density_band: String,
    #[serde(default)]
    pub density_cell_spawn_pressure: u16,
    #[serde(default)]
    pub density_particle_spawn_pressure: u16,
    #[serde(default)]
    pub density_root_growth_pressure: u16,
    #[serde(default)]
    pub density_refill_pressure: u16,
    #[serde(default)]
    pub density_crowding_pressure: u16,
    #[serde(default)]
    pub density_empty_ratio: f32,
    #[serde(default)]
    pub density_occupied_ratio: f32,
    #[serde(default)]
    pub density_peak_occupied_ratio: f32,
    #[serde(default)]
    pub density_samples: u64,

    #[serde(default)]
    pub archetype_live_counts: [usize; 11],
    #[serde(default)]
    pub archetype_peak_counts: [usize; 11],
    #[serde(default)]
    pub archetype_seen_counts: [u64; 11],
    #[serde(default)]
    pub archetype_population_samples: u64,
    #[serde(default)]
    pub trophic_balance_label: String,
    pub primitive_population: usize,
    pub evolved_population: usize,
    pub mature_population: usize,
    pub mature_evolved_population: usize,
    pub evolved_ratio: f32,
    pub mature_evolved_ratio: f32,

    pub notes: Vec<String>,
}

impl MemoryBank {
    pub fn new(seed: u64) -> Self {
        Self {
            seed,

            longest_age: 0,
            highest_generation: 0,

            peak_population: 0,
            peak_clusters: 0,
            peak_species: 0,
            peak_rare_lifeforms: 0,
            peak_living_cells: 0,
            peak_harvesters: 0,
            peak_reapers: 0,

            total_cells_consumed: 0,
            total_harvesters_consumed: 0,
            total_species_created: 0,
            total_extinctions: 0,
            total_births: 0,
            total_deaths: 0,
            total_reproductions: 0,
            total_fusions: 0,
            total_merges: 0,
            total_splits: 0,

            strongest_cluster_size: 0,
            strongest_cluster_age: 0,

            dominant_archetype: "unknown".to_string(),
            richest_zone: "unknown".to_string(),

            peak_root_cells: 0,
            peak_tree_trunk_cells: 0,
            peak_tree_branch_cells: 0,
            peak_tree_canopy_cells: 0,
            tree_growth_events: 0,
            tree_wall_events: 0,
            tree_surface_flow_events: 0,
            peak_root_pressure: 0.0,
            root_pressure_average: 0.0,
            root_pressure_samples: 0,
            root_collision_events: 0,
            root_corridor_events: 0,
            root_choked_ticks: 0,
            substrate_starved_ticks: 0,
            substrate_overgrown_ticks: 0,
            substrate_balance_average: 0.0,
            substrate_balance_samples: 0,
            adaptive_root_bias: 0.0,
            adaptive_corridor_bias: 0.0,
            adaptive_substrate_throttle: 0.0,

            density_band: "unknown".to_string(),
            density_cell_spawn_pressure: 0,
            density_particle_spawn_pressure: 0,
            density_root_growth_pressure: 0,
            density_refill_pressure: 0,
            density_crowding_pressure: 0,
            density_empty_ratio: 0.0,
            density_occupied_ratio: 0.0,
            density_peak_occupied_ratio: 0.0,
            density_samples: 0,

            archetype_live_counts: [0; 11],
            archetype_peak_counts: [0; 11],
            archetype_seen_counts: [0; 11],
            archetype_population_samples: 0,
            trophic_balance_label: "unknown".to_string(),
            primitive_population: 0,
            evolved_population: 0,
            mature_population: 0,
            mature_evolved_population: 0,
            evolved_ratio: 0.0,
            mature_evolved_ratio: 0.0,

            notes: Vec::new(),
        }
    }

    pub fn load_or_new(seed: u64) -> Self {
        if Path::new(MEMORY_PATH).exists() {
            if let Ok(data) = fs::read_to_string(MEMORY_PATH) {
                if let Ok(mut memory) = serde_json::from_str::<Self>(&data) {
                    memory.seed = seed;
                    memory.recalculate_adaptive_pressures();
                    return memory;
                }
            }
        }

        Self::new(seed)
    }

    pub fn save(&self) -> Result<()> {
        fs::create_dir_all("memory")?;
        fs::write(MEMORY_PATH, serde_json::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn note(&mut self, text: String) {
        self.notes.push(text);

        if self.notes.len() > 32 {
            self.notes.remove(0);
        }
    }

    #[allow(dead_code)]
    pub fn observe_tree(
        &mut self,
        root_cells: usize,
        trunk_cells: usize,
        branch_cells: usize,
        canopy_cells: usize,
    ) {
        self.peak_root_cells = self.peak_root_cells.max(root_cells);
        self.peak_tree_trunk_cells = self.peak_tree_trunk_cells.max(trunk_cells);
        self.peak_tree_branch_cells = self.peak_tree_branch_cells.max(branch_cells);
        self.peak_tree_canopy_cells = self.peak_tree_canopy_cells.max(canopy_cells);

        if root_cells + trunk_cells + branch_cells + canopy_cells > 0 {
            self.tree_growth_events = self.tree_growth_events.saturating_add(1);
        }

        self.recalculate_adaptive_pressures();
    }

    #[allow(dead_code)]
    pub fn observe_tree_wall_event(&mut self) {
        self.tree_wall_events = self.tree_wall_events.saturating_add(1);
        self.root_collision_events = self.root_collision_events.saturating_add(1);
        self.recalculate_adaptive_pressures();
    }

    #[allow(dead_code)]
    pub fn observe_tree_surface_flow_event(&mut self) {
        self.tree_surface_flow_events = self.tree_surface_flow_events.saturating_add(1);
        self.root_corridor_events = self.root_corridor_events.saturating_add(1);
        self.recalculate_adaptive_pressures();
    }

    pub fn observe_density(&mut self, snapshot: DensitySnapshot) {
        let pressure = DensityPressure::analyze(snapshot, DensityConfig::default());

        self.density_band = format!("{:?}", pressure.band);
        self.density_cell_spawn_pressure = pressure.cell_spawn_pressure;
        self.density_particle_spawn_pressure = pressure.particle_spawn_pressure;
        self.density_root_growth_pressure = pressure.root_growth_pressure;
        self.density_refill_pressure = pressure.refill_pressure;
        self.density_crowding_pressure = pressure.crowding_pressure;
        self.density_empty_ratio = snapshot.empty_ratio();
        self.density_occupied_ratio = snapshot.occupied_ratio();
        self.density_peak_occupied_ratio = self
            .density_peak_occupied_ratio
            .max(self.density_occupied_ratio);
        self.density_samples = self.density_samples.saturating_add(1);
    }

    pub fn density_status_line(&self) -> String {
        format!(
            "{} occ:{:>2}% empty:{:>2}% cell:{} part:{} root:{} crowd:{}",
            self.density_band,
            percent(self.density_occupied_ratio),
            percent(self.density_empty_ratio),
            self.density_cell_spawn_pressure,
            self.density_particle_spawn_pressure,
            self.density_root_growth_pressure,
            self.density_crowding_pressure
        )
    }

    pub fn observe_archetypes(&mut self, live_counts: [usize; 11]) {
        self.archetype_live_counts = live_counts;
        self.archetype_population_samples = self.archetype_population_samples.saturating_add(1);

        for idx in 0..11 {
            self.archetype_peak_counts[idx] = self.archetype_peak_counts[idx].max(live_counts[idx]);

            if live_counts[idx] > 0 {
                self.archetype_seen_counts[idx] =
                    self.archetype_seen_counts[idx].saturating_add(live_counts[idx] as u64);
            }
        }

        self.trophic_balance_label = trophic_balance_label(&live_counts).to_string();
    }

    pub fn trophic_status_line(&self) -> String {
        format!(
            "{} prey:{} herb:{} apex:{} peak_hrv:{} peak_rpr:{}",
            self.trophic_balance_label,
            prey_count(&self.archetype_live_counts),
            herbivore_count(&self.archetype_live_counts),
            apex_count(&self.archetype_live_counts),
            self.archetype_peak_counts[9],
            self.archetype_peak_counts[10]
        )
    }

    pub fn observe_evolution_stage(
        &mut self,
        primitive_population: usize,
        evolved_population: usize,
        mature_population: usize,
        mature_evolved_population: usize,
    ) {
        let total_population = primitive_population + evolved_population;

        self.primitive_population = primitive_population;
        self.evolved_population = evolved_population;
        self.mature_population = mature_population;
        self.mature_evolved_population = mature_evolved_population;

        self.evolved_ratio = if total_population > 0 {
            evolved_population as f32 / total_population as f32
        } else {
            0.0
        };

        self.mature_evolved_ratio = if mature_population > 0 {
            mature_evolved_population as f32 / mature_population as f32
        } else {
            0.0
        };
    }

    pub fn evolution_status_line(&self) -> String {
        format!(
            "evo primitive:{} evolved:{} mature:{} lineage:{} evo:{:.0}% mature:{:.0}%",
            self.primitive_population,
            self.evolved_population,
            self.mature_population,
            self.mature_evolved_population,
            self.evolved_ratio * 100.0,
            self.mature_evolved_ratio * 100.0
        )
    }

    pub fn observe_substrate(
        &mut self,
        living_cells: usize,
        total_cells: usize,
        root_cells: usize,
        consumed_cells: usize,
    ) {
        let total = total_cells.max(1);
        let living_ratio = living_cells as f32 / total as f32;
        let root_pressure = root_cells as f32 / total as f32;

        self.peak_living_cells = self.peak_living_cells.max(living_cells);
        self.peak_root_cells = self.peak_root_cells.max(root_cells);
        self.peak_root_pressure = self.peak_root_pressure.max(root_pressure);

        self.total_cells_consumed = self
            .total_cells_consumed
            .saturating_add(consumed_cells as u64);

        self.root_pressure_samples = self.root_pressure_samples.saturating_add(1);
        self.root_pressure_average = rolling_average(
            self.root_pressure_average,
            root_pressure,
            self.root_pressure_samples,
        );

        self.substrate_balance_samples = self.substrate_balance_samples.saturating_add(1);
        self.substrate_balance_average = rolling_average(
            self.substrate_balance_average,
            living_ratio,
            self.substrate_balance_samples,
        );

        if living_ratio < 0.035 {
            self.substrate_starved_ticks = self.substrate_starved_ticks.saturating_add(1);
        }

        if living_ratio > 0.18 {
            self.substrate_overgrown_ticks = self.substrate_overgrown_ticks.saturating_add(1);
        }

        if root_pressure > 0.07 && living_ratio < 0.07 {
            self.root_choked_ticks = self.root_choked_ticks.saturating_add(1);
        }

        self.recalculate_adaptive_pressures();
    }

    pub fn harvester_resistance(&self) -> f32 {
        let reaped_pressure = self.total_harvesters_consumed as f32 / 250.0;
        let substrate_pressure = self.substrate_starved_ticks as f32 / 900.0;
        let root_pressure = self.root_choked_ticks as f32 / 600.0;

        (reaped_pressure + substrate_pressure + root_pressure + self.adaptive_substrate_throttle)
            .clamp(0.0, 1.0)
    }

    #[allow(dead_code)]
    pub fn reaper_urgency(&self) -> f32 {
        let harvester_peak = self.peak_harvesters as f32 / 80.0;
        let consumed_pressure = self.total_cells_consumed as f32 / 15_000.0;
        let substrate_loss = self.substrate_starved_ticks as f32 / 700.0;

        (harvester_peak + consumed_pressure + substrate_loss).clamp(0.0, 1.0)
    }

    pub fn substrate_recovery_bias(&self) -> f32 {
        let starvation = self.substrate_starved_ticks as f32 / 900.0;
        let overgrowth_penalty = self.substrate_overgrown_ticks as f32 / 1_400.0;
        let root_choke = self.root_choked_ticks as f32 / 1_000.0;

        (starvation + root_choke - overgrowth_penalty).clamp(0.0, 1.0)
    }

    pub fn mutation_pressure(&self) -> f32 {
        let extinction_pressure = self.total_extinctions as f32 / 80.0;
        let death_pressure = self.total_deaths as f32 / 12_000.0;
        let root_navigation_pressure = self.root_avoidance_pressure() * 0.35;
        let substrate_instability =
            (self.substrate_starved_ticks + self.substrate_overgrown_ticks) as f32 / 2_400.0;

        (extinction_pressure + death_pressure + root_navigation_pressure + substrate_instability)
            .clamp(0.0, 1.0)
    }

    pub fn root_avoidance_pressure(&self) -> f32 {
        let collision_pressure = self.root_collision_events as f32 / 5_000.0;
        let choke_pressure = self.root_choked_ticks as f32 / 800.0;
        let density_pressure = self.root_pressure_average * 8.0;

        (collision_pressure + choke_pressure + density_pressure + self.adaptive_root_bias)
            .clamp(0.0, 1.0)
    }

    pub fn corridor_pressure(&self) -> f32 {
        let corridor_learning = self.root_corridor_events as f32 / 4_000.0;
        let root_density = self.root_pressure_average * 6.0;
        let choke_pressure = self.root_choked_ticks as f32 / 1_000.0;

        (corridor_learning + root_density + choke_pressure + self.adaptive_corridor_bias)
            .clamp(0.0, 1.0)
    }

    pub fn substrate_throttle_pressure(&self) -> f32 {
        let overgrowth = self.substrate_overgrown_ticks as f32 / 900.0;
        let high_average = ((self.substrate_balance_average - 0.15) * 7.0).max(0.0);
        let root_density = (self.root_pressure_average - 0.065).max(0.0) * 5.0;

        (overgrowth + high_average + root_density + self.adaptive_substrate_throttle)
            .clamp(0.0, 1.0)
    }

    pub fn pathfinder_bias(&self) -> f32 {
        let root_pressure = self.root_avoidance_pressure();
        let corridor_pressure = self.corridor_pressure();
        let mutation_pressure = self.mutation_pressure() * 0.25;

        (root_pressure * 0.45 + corridor_pressure * 0.45 + mutation_pressure).clamp(0.0, 1.0)
    }

    fn recalculate_adaptive_pressures(&mut self) {
        let root_density = self.root_pressure_average;
        let choke = self.root_choked_ticks as f32 / 1_000.0;
        let corridors = self.root_corridor_events as f32 / 5_000.0;
        let collisions = self.root_collision_events as f32 / 5_000.0;
        let starved = self.substrate_starved_ticks as f32 / 1_000.0;
        let overgrown = self.substrate_overgrown_ticks as f32 / 1_000.0;

        self.adaptive_root_bias = (root_density * 4.5 + collisions + choke).clamp(0.0, 1.0);
        self.adaptive_corridor_bias =
            (root_density * 3.5 + corridors + choke * 0.5).clamp(0.0, 1.0);
        self.adaptive_substrate_throttle =
            (overgrown + root_density * 1.8 - starved * 0.35).clamp(0.0, 1.0);
    }
}

fn prey_count(counts: &[usize; 11]) -> usize {
    counts[0] + counts[2] + counts[3] + counts[5] + counts[6] + counts[7] + counts[8]
}

fn herbivore_count(counts: &[usize; 11]) -> usize {
    counts[2] + counts[7] + counts[9]
}

fn apex_count(counts: &[usize; 11]) -> usize {
    counts[1] + counts[4] + counts[10]
}

fn trophic_balance_label(counts: &[usize; 11]) -> &'static str {
    let herb = herbivore_count(counts);
    let apex = apex_count(counts);
    let prey = prey_count(counts);
    let harvesters = counts[9];
    let reapers = counts[10];

    if reapers > 0 && harvesters > 0 {
        "cycling"
    } else if harvesters >= 8 && reapers == 0 {
        "grazing"
    } else if reapers > 0 && apex > herb.saturating_add(6) {
        "predatory"
    } else if prey >= herb.saturating_add(apex).saturating_mul(3).max(1) {
        "prey bloom"
    } else if herb == 0 && apex == 0 {
        "basal"
    } else {
        "balanced"
    }
}

fn rolling_average(current: f32, next: f32, samples: u64) -> f32 {
    if samples <= 1 {
        next
    } else {
        let samples = samples.min(10_000) as f32;
        current + (next - current) / samples
    }
}

fn percent(value: f32) -> u16 {
    (value.clamp(0.0, 1.0) * 100.0).round() as u16
}
