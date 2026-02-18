#!/bin/bash
# ══════════════════════════════════════════════════════════════
# P0 SMOKE TESTS — The "Clone → Demo Works" Guarantee
# ══════════════════════════════════════════════════════════════
# Run: bash tests/smoke.sh (from repo root)
# Exit: 0 = all pass, 1 = something is broken
# ══════════════════════════════════════════════════════════════
set -e

PASS=0
FAIL=0
RED='\033[91m'
GREEN='\033[92m'
YELLOW='\033[93m'
RESET='\033[0m'

run_test() {
    local name="$1"
    local cmd="$2"
    printf "%-40s" "$name"
    if eval "$cmd" > /dev/null 2>&1; then
        echo -e "${GREEN}[PASS]${RESET}"
        PASS=$((PASS + 1))
    else
        echo -e "${RED}[FAIL]${RESET}"
        FAIL=$((FAIL + 1))
    fi
}

echo -e "${YELLOW}══════════════════════════════════════════${RESET}"
echo -e "${YELLOW}   P0 SMOKE TESTS                         ${RESET}"
echo -e "${YELLOW}══════════════════════════════════════════${RESET}"
echo ""

# 1. Python imports resolve
run_test "Python imports" \
    "python -c 'from meta.ark_types import ArkValue, Scope'"

# 2. Version command
run_test "ark.py version" \
    "python meta/ark.py version"

# 3. Hello World
run_test "Hello World (apps/hello.ark)" \
    "python meta/ark.py run apps/hello.ark"

# 4. REPL recognizes command (doesn't crash with 'Unknown command')
run_test "REPL starts (then exits)" \
    "echo '!exit' | timeout 5 python meta/ark.py repl 2>&1 | grep -vi 'unknown command'"

# 5. Test suite runs
run_test "Test suite (tests/test_suite.ark)" \
    "python meta/ark.py run tests/test_suite.ark"

# 6. Rust binary builds
run_test "Rust build (check only)" \
    "cargo check --manifest-path core/Cargo.toml"

# 7. Index.html exists and doesn't reference nonexistent WASM API
run_test "index.html no broken WASM refs" \
    "! grep -q 'ark_init\|ark_alloc\|ark_dealloc' index.html"

echo ""
echo -e "${YELLOW}══════════════════════════════════════════${RESET}"
TOTAL=$((PASS + FAIL))
echo "Results: ${PASS}/${TOTAL} passed"

if [ "$FAIL" -gt 0 ]; then
    echo -e "${RED}SMOKE TESTS FAILED${RESET}"
    exit 1
else
    echo -e "${GREEN}ALL SMOKE TESTS PASSED${RESET}"
    exit 0
fi
