# Copyright (c) 2026 Mohamad Al-Zawahreh (dba Sovereign Systems).
#
# This file is part of the Ark Sovereign Compiler.
#
# LICENSE: DUAL-LICENSED (AGPLv3 or COMMERCIAL).
#
# 1. OPEN SOURCE: You may use this file under the terms of the GNU Affero
#    General Public License v3.0. If you link to this code, your ENTIRE
#    application must be open-sourced under AGPLv3.
#
# 2. COMMERCIAL: For proprietary use, you must obtain a Commercial License
#    from Sovereign Systems.
#
# PATENT NOTICE: Protected by US Patent App #63/935,467.
# NO IMPLIED LICENSE to rights of Mohamad Al-Zawahreh or Sovereign Systems.

from ark_parser import QiParser
import sys

def verify():
    print("[Verification] Spawning Qi Parser...")
    try:
        parser = QiParser("meta/ark.lark")
    except Exception as e:
        print(f"[Fatal] Could not load grammar: {e}")
        sys.exit(1)

    test_cases = [
        ("Assignment", "x := 10"),
        ("Arithmetic", "y := 10 + 20"),
        ("Flow Typing", "token :: Linear<Access>"),
        ("Hole", "solution := ???"),
        ("Neuro Block", "@train(gpt4) { loss := 0 }"),
    ]

    success_count = 0
    for name, code in test_cases:
        print(f"\n[Test: {name}] Code: '{code}'")
        try:
            ast = parser.parse(code)
            print(f"  > PASSED. AST: {ast}")
            success_count += 1
        except Exception as e:
            print(f"  > FAILED. Error: {e}")

    print(f"\n[Result] {success_count}/{len(test_cases)} Tests Passed.")
    if success_count == len(test_cases):
        sys.exit(0)
    else:
        sys.exit(1)

if __name__ == "__main__":
    verify()
