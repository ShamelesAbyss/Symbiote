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
            notes: Vec::new(),
        }
    }

    pub fn load_or_new(seed: u64) -> Self {
        if Path::new(MEMORY_PATH).exists() {
            if let Ok(data) = fs::read_to_string(MEMORY_PATH) {
                if let Ok(memory) = serde_json::from_str::<Self>(&data) {
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

    pub fn harvester_resistance(&self) -> f32 {
        let harvester_pressure =
            self.peak_harvesters as f32 / self.peak_species.max(1) as f32;

        let consumption_pressure =
            self.total_cells_consumed as f32 / self.total_reproductions.max(1) as f32;

        (harvester_pressure * 0.72 + (consumption_pressure / 40.0) * 0.28)
            .clamp(0.0, 1.0)
    }

    pub fn reaper_urgency(&self) -> f32 {
        let harvester_peak = self.peak_harvesters as f32;
        let reaper_peak = self.peak_reapers as f32;

        if harvester_peak <= 0.0 {
            return 0.0;
        }

        let imbalance = ((harvester_peak - reaper_peak * 1.8) / harvester_peak).clamp(0.0, 1.0);
        let consumption = (self.total_cells_consumed as f32 / 900.0).clamp(0.0, 1.0);

        (imbalance * 0.75 + consumption * 0.25).clamp(0.0, 1.0)
    }

    pub fn substrate_recovery_bias(&self) -> f32 {
        let population_peak = self.peak_population.max(1) as f32;
        let living_peak = self.peak_living_cells as f32;

        let low_cell_history = (1.0 - (living_peak / population_peak).clamp(0.0, 1.0)).clamp(0.0, 1.0);
        let consumption = (self.total_cells_consumed as f32 / 1_200.0).clamp(0.0, 1.0);

        (low_cell_history * 0.55 + consumption * 0.45).clamp(0.0, 1.0)
    }

    pub fn mutation_pressure(&self) -> f32 {
        let extinction_pressure =
            self.total_extinctions as f32 / self.total_species_created.max(1) as f32;

        let death_pressure =
            self.total_deaths as f32 / self.total_births.max(1) as f32;

        (extinction_pressure * 0.68 + death_pressure.min(3.0) / 3.0 * 0.32).clamp(0.0, 1.0)
    }

    pub fn adaptive_summary(&self) -> String {
        format!(
            "memory pressure hrv:{:.2} rpr:{:.2} regen:{:.2} mut:{:.2}",
            self.harvester_resistance(),
            self.reaper_urgency(),
            self.substrate_recovery_bias(),
            self.mutation_pressure(),
        )
    }
}
