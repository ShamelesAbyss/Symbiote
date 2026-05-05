#!/bin/bash
set -e

FILE="src/app.rs"

echo "[1] Fixing missing struct initializers..."

# add tick + runtime_seconds to all App initializations
sed -i 's/Self {/Self {\n            tick: 0,\n            runtime_seconds: 0.0,/' $FILE

echo "[2] Fixing runtime increment (remove delta_time)..."

# replace invalid delta_time usage with constant tick step
sed -i 's/self.runtime_seconds += self.delta_time;/self.runtime_seconds += 1.0;/' $FILE

echo "[DONE] app.rs compile issues fixed."
