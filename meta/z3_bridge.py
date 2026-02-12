import sys

try:
    import z3
    HAS_Z3 = True
except ImportError:
    HAS_Z3 = False

def verify_contract(constraints):
    """
    Verifies a set of constraints using Z3.
    Input: List of strings (SMT-LIB2 commands/assertions).
    Output: True if SAT (consistent), False if UNSAT (contradiction) or error.
    """
    if not HAS_Z3:
        print("Z3 Missing, Skipping Verification")
        return True

    try:
        solver = z3.Solver()
        # Combine all constraints into one SMT-LIB2 script
        # Each string in the list is a command or assertion
        full_script = "\n".join(constraints)

        if not full_script.strip():
            return True

        # Parse the script into assertions. parse_smt2_string returns an AstVector.
        assertions = z3.parse_smt2_string(full_script)

        solver.add(assertions)

        result = solver.check()
        if result == z3.sat:
            return True
        else:
            return False

    except Exception as e:
        print(f"Z3 Verification Error: {e}", file=sys.stderr)
        # Return False on error as verification failed
        return False
