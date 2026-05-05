#!/bin/bash
set -e

FILE="src/automata.rs"

echo "[1] Reduce bottom seeding density..."

# reduce chance near bottom
sed -i 's/bottom_penalty *= *[^;]*/bottom_penalty *= 2/g' $FILE

echo "[2] Boost upward growth bias..."

# increase vertical bias influence
sed -i 's/vertical_bias *= *[^;]*/vertical_bias *= 2/g' $FILE

echo "[3] Allow lateral drift (prevent straight lines)..."

# slightly boost bend bias
sed -i 's/bend_bias *= *[^;]*/bend_bias *= 1.5/g' $FILE

echo "[4] Prevent root starvation..."

# ensure minimum chance exists
sed -i 's/let chance = \(.*\);/let chance = (\1).max(2);/' $FILE

echo "[DONE] Root growth loosened."
