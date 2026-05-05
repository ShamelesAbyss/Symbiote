#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Enforce root persistence across buffer swap..."

# This targets the final write phase where next state is applied
# We inject a rule BEFORE assignment

sed -i 's/self\.cells\[i\] = next;/if self.cells[i].kind == CellKind::Root { continue; } self.cells[i] = next;/g' $FILE

echo "[DONE] Root buffer persistence enforced."
