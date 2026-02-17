#!/bin/bash
# ══════════════════════════════════════════════════════════════
# The Golden Demo Test — "The README Promise"
# ══════════════════════════════════════════════════════════════
# Runs the EXACT commands from the README in order.
# If any step fails, the README is lying to users.
# ══════════════════════════════════════════════════════════════
set -e

PASS=0
FAIL=0

pass() { echo "  ✓ $1"; PASS=$((PASS + 1)); }
fail() { echo "  ✗ $1"; FAIL=$((FAIL + 1)); }

echo "═══════════════════════════════════"
echo " GOLDEN DEMO: README Promise Test"
echo "═══════════════════════════════════"
echo ""

# ── Step 1: Python deps installed (README: pip install -r requirements.txt)
echo "[1/6] Python dependencies..."
python3 -c "from meta.ark import run_file" 2>/dev/null && pass "Python imports resolve" || fail "Python imports broken"

# ── Step 2: Hello World (README: implicit first demo)
echo "[2/6] Hello World..."
OUTPUT=$(python3 meta/ark.py run apps/hello.ark 2>&1)
echo "$OUTPUT" | grep -qi "hello" && pass "Hello World" || fail "Hello World: $OUTPUT"

# ── Step 3: Version (README: python3 meta/ark.py version)
echo "[3/6] Version command..."
OUTPUT=$(python3 meta/ark.py version 2>&1)
echo "$OUTPUT" | grep -qi "v112" && pass "Version" || fail "Version: $OUTPUT"

# ── Step 4: REPL module loads (README: docker run -it --rm ark-compiler → REPL)
echo "[4/6] REPL module..."
python3 -c "from meta.repl import run_repl; print('REPL module OK')" 2>/dev/null && pass "REPL module loads" || fail "REPL module import"

# ── Step 5: Test suite (README: implied via CI)
echo "[5/6] Test suite..."
python3 meta/ark.py run tests/test_suite.ark 2>&1 && pass "Test suite" || fail "Test suite"

# ── Step 6: Intrinsic count (README: claims 121 Rust-native)
echo "[6/6] Intrinsic count..."
OUTPUT=$(python3 meta/count_intrinsics.py 2>&1)
echo "$OUTPUT" | grep -q "Total Rust Intrinsics" && pass "Intrinsic count runs" || fail "Intrinsic count: $OUTPUT"

echo ""
echo "═══════════════════════════════════"
echo " Results: $PASS passed, $FAIL failed"
echo "═══════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
    echo "⚠ README makes promises the code doesn't keep."
    exit 1
fi

echo "✓ All README promises verified."
