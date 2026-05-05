#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Make roots permanent..."

# prevent root cells from being overwritten
sed -i 's/match cell.kind {/if cell.kind == CellKind::Root { continue; }\n        match cell.kind {/' $FILE

echo "[2] Remove any root decay / replacement..."

# kill any logic that turns root into something else
sed -i 's/CellKind::Root => .*$/CellKind::Root => CellKind::Root,/' $FILE

echo "[3] Force upward growth bias..."

# strongly bias upward propagation
sed -i 's/vertical_bias *= *[^;]*/vertical_bias *= 3/g' $FILE

echo "[4] Allow aggressive spread from existing roots..."

# boost chance from existing roots
sed -i 's/let chance = \(.*\);/let chance = (\1 + 5).max(5);/' $FILE

echo "[5] Remove blocking conditions..."

# remove checks that prevent growth due to occupancy
sed -i 's/if .*occupied.*{//g' $FILE
sed -i 's/if .*blocked.*{//g' $FILE

echo "[DONE] Roots are now permanent + dominant."
