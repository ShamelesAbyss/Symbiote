#!/bin/bash
set -e

FILE="src/app.rs"

echo "[1] Fixing App::new constructor safely..."

# insert inside ONLY the first occurrence of `Self {`
awk '
BEGIN { done=0 }
/Self \{/ && done==0 {
    print $0
    print "            tick: 0,"
    print "            runtime_seconds: 0.0,"
    done=1
    next
}
{ print }
' $FILE > tmp.rs && mv tmp.rs $FILE

echo "[DONE] Constructor fixed safely."
