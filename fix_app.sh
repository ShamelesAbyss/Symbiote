#!/bin/bash

set -e

FILE="src/app.rs"

echo "[1] Ensuring clean source..."
git fetch origin main
git checkout origin/main -- $FILE

echo "[2] Adding tick + runtime fields..."

sed -i '/pub struct App {/a\    pub tick: u64,\n    pub runtime_seconds: f32,' $FILE

echo "[3] Fixing tick progression..."

sed -i 's/self.age += 1;/self.age += 1;\n        self.tick += 1;\n        self.runtime_seconds += self.delta_time;/' $FILE

echo "[4] Fixing particle count..."

sed -i 's/let particle_count.*/let particle_count = self.particles.len();/' $FILE

echo "[5] Fixing root count..."

sed -i 's/let root_count.*/let root_count = self.substrate.cells.iter().filter(|c| c.kind == crate::automata::CellKind::Root).count();/' $FILE

echo "[DONE] app.rs patched safely."
