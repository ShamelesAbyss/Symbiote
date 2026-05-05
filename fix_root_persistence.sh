#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Enforcing root persistence at write phase..."

sed -i 's/self\.cells\[idx\] = next;/if self.cells[idx].kind == CellKind::Root {\
    next.kind = CellKind::Root;\
}\
self.cells[idx] = next;/g' $FILE

echo "[DONE] Roots can no longer be overwritten."
