#!/bin/bash
set -e

FILE="src/app.rs"

echo "[1] Adding fields safely (struct only)..."

sed -i '/pub struct App {/a\
    pub tick: u64,\
    pub runtime_seconds: f32,' $FILE

echo "[DONE] Struct updated. Constructor untouched."
