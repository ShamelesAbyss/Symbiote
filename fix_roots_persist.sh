#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Guard: never overwrite existing Root cells..."

# Insert a guard at the start of the per-cell update loop:
# If your file has a loop like: for (i, cell) in self.cells.iter_mut().enumerate() {
# we add a continue when it's already Root.

sed -i '/for .*cell.*iter_mut()/a\
        if cell.kind == CellKind::Root { continue; }' $FILE

echo "[DONE] Root persistence guard added."
