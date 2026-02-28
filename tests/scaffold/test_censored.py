"""
Test: GCD Censored Sentinel — Verifies that the Ark interpreter's
CensoredAccessError guard prevents arithmetic on Censored (infinity_rec) values.

This validates P2 of the GCD integration: typed return sentinels that
physically cannot be smoothed over by implicit coercion.
"""
import sys
import os
import unittest

# Import from `meta.*` package path to match how the interpreter loads them.
# Using `ark_types` via sys.path creates a DIFFERENT Python module identity
# than `meta.ark_types`, causing assertRaises to miss the exception.
from meta.ark_types import ArkValue, CENSORED_VALUE, CensoredAccessError, UNIT_VALUE
from meta.ark_interpreter import eval_binop


class TestCensoredSentinel(unittest.TestCase):
    """Verify that Censored values raise CensoredAccessError on arithmetic."""

    def test_censored_type_exists(self):
        """CENSORED_VALUE has type 'Censored' and val None."""
        self.assertEqual(CENSORED_VALUE.type, "Censored")
        self.assertIsNone(CENSORED_VALUE.val)

    def test_add_censored_left(self):
        """Censored + Integer raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("add", CENSORED_VALUE, ArkValue(100, "Integer"))

    def test_add_censored_right(self):
        """Integer + Censored raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("add", ArkValue(100, "Integer"), CENSORED_VALUE)

    def test_sub_censored(self):
        """Censored - Integer raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("sub", CENSORED_VALUE, ArkValue(50, "Integer"))

    def test_mul_censored(self):
        """Integer * Censored raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("mul", ArkValue(10, "Integer"), CENSORED_VALUE)

    def test_div_censored(self):
        """Censored / Integer raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("div", CENSORED_VALUE, ArkValue(5, "Integer"))

    def test_comparison_censored(self):
        """Censored < Integer raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("lt", CENSORED_VALUE, ArkValue(100, "Integer"))

    def test_both_censored(self):
        """Censored + Censored raises CensoredAccessError."""
        with self.assertRaises(CensoredAccessError):
            eval_binop("add", CENSORED_VALUE, CENSORED_VALUE)

    def test_normal_arithmetic_unaffected(self):
        """Normal Integer arithmetic still works (no false positives)."""
        result = eval_binop("add", ArkValue(3, "Integer"), ArkValue(4, "Integer"))
        self.assertEqual(result.val, 7)
        self.assertEqual(result.type, "Integer")

    def test_error_message_quality(self):
        """Error message includes operand types and guidance."""
        with self.assertRaises(CensoredAccessError) as ctx:
            eval_binop("mul", CENSORED_VALUE, ArkValue(10, "Integer"))
        msg = str(ctx.exception)
        self.assertIn("Censored", msg)
        self.assertIn("pattern matching", msg)


if __name__ == '__main__':
    print("\n[ARK CI] Executing Censored Sentinel Tests...")
    suite = unittest.TestLoader().loadTestsFromTestCase(TestCensoredSentinel)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.wasSuccessful():
        print("\n[ARK CI] STATUS: TITANIUM. CENSORED SENTINEL HOLDS.")
    else:
        print("\n[ARK CI] STATUS: COMPROMISED.")
        exit(1)
