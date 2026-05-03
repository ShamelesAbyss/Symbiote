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
    pub total_births: u64,
    pub total_deaths: u64,
    pub total_merges: u64,
    pub total_splits: u64,
    pub strongest_cluster_size: usize,
    pub strongest_cluster_age: u64,
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
            total_births: 0,
            total_deaths: 0,
            total_merges: 0,
            total_splits: 0,
            strongest_cluster_size: 0,
            strongest_cluster_age: 0,
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

        if self.notes.len() > 24 {
            self.notes.remove(0);
        }
    }
}
