#!/bin/bash
set -e

FILE="src/app.rs"

echo "[1] Adding tick + runtime fields (safe)..."

# ONLY add fields, do not touch impl blocks
sed -i '/pub struct App {/a\
    pub tick: u64,\
    pub runtime_seconds: f32,' $FILE

echo "[2] Initialize fields inside App::new() ONLY..."

# insert initialization inside constructor block
sed -i '/pub fn new()/,/Self {/ s/Self {/Self {\n            tick: 0,\n            runtime_seconds: 0.0,/' $FILE

echo "[3] Fix tick progression safely..."

# only modify age increment line
sed -i 's/self.age += 1;/self.age += 1;\n        self.tick += 1;\n        self.runtime_seconds += 1.0;/' $FILE

echo "[DONE] Clean patch applied."
