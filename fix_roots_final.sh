#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Enforce root dominance in write phase..."

# Replace any direct assignment to cell.kind with guarded version
sed -i 's/cell\.kind = \([^;]*\);/if cell.kind != CellKind::Root { cell.kind = \1; }/g' $FILE

echo "[2] Ensure roots propagate stronger upward..."

sed -i 's/vertical_bias *= *\([0-9.]*\)/vertical_bias *= (\1 * 3.0)/g' $FILE

echo "[3] Boost parent-root influence..."

sed -i 's/let chance = \(.*\);/let chance = (\1 + 6).max(6);/' $FILE

echo "[DONE] Roots now dominate + persist."
