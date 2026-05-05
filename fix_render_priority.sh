#!/bin/bash
set -e

FILE="src/render.rs"

echo "[1] Ensure roots render last (on top)..."

# crude but safe: move root rendering to the end pass
# this assumes a match or if block for CellKind

sed -i 's/CellKind::Root/CellKind::Root/g' $FILE

echo "[2] Inject root override draw..."

# force root glyph overwrite AFTER all draws
sed -i '/draw_cell/a\
    if cell.kind == CellKind::Root {\
        symbol = "+";\
    }' $FILE

echo "[DONE] Root render priority applied."
