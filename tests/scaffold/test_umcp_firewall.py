"""
UMCP/GCD Python Reference Implementation — Epistemic Firewall Test Suite.

Validates GCD kernel math (F, IC, Delta) in float-space as a cross-check
against the Ark fixed-point implementation in lib/std/gcd.ark.

Source: Clement Paulus, UMCP/GCD v2.1.3
"""
import hashlib
import math
import unittest
from enum import Enum
from typing import List, Dict, Union


class TraceState(Enum):
    VALID = 1
    CENSORED = 2  # tau_R = infinity_rec


class UMCPContract:
    def __init__(self, pipeline_name: str, epsilon: float, weights: List[float]):
        assert math.isclose(sum(weights), 1.0), "Weights must sum to 1.0"
        self.pipeline_name = pipeline_name
        self.epsilon = epsilon
        self.weights = weights

    def generate_run_id(self) -> str:
        payload = f"{self.pipeline_name}_{self.epsilon}_{self.weights}".encode('utf-8')
        return hashlib.sha256(payload).hexdigest()


def apply_adapter(raw_trace: List[Union[float, TraceState]], contract: UMCPContract) -> List[float]:
    clipped_trace = []
    for val in raw_trace:
        if val is TraceState.CENSORED:
            clipped_trace.append(contract.epsilon)
        else:
            lower_bound = max(contract.epsilon, val)
            safe_val = min(1.0 - contract.epsilon, lower_bound)
            clipped_trace.append(safe_val)
    return clipped_trace


def compute_kernel(clipped_trace: List[float], contract: UMCPContract) -> Dict[str, float]:
    fidelity = sum(val * w for val, w in zip(clipped_trace, contract.weights))
    log_sum = sum(w * math.log(val) for val, w in zip(clipped_trace, contract.weights))
    integrity_composite = math.exp(log_sum)

    return {
        "F": fidelity,
        "IC": integrity_composite,
        "Delta": fidelity - integrity_composite
    }


def audit_dataset(contract: UMCPContract, raw_trace: List[Union[float, TraceState]], max_delta: float):
    clipped = apply_adapter(raw_trace, contract)
    ledger = compute_kernel(clipped, contract)

    if ledger["Delta"] > max_delta:
        raise SystemError(f"UMCP VETO: Multiplicative collapse. Delta {ledger['Delta']:.4f} > {max_delta}")
    return ledger


class TestUMCPFirewall(unittest.TestCase):

    def setUp(self):
        self.weights = [0.5, 0.5]
        self.contract = UMCPContract("FUSION_CORE_LVK", 1e-3, self.weights)
        self.max_delta = 0.20

    def test_immutable_run_id_fracture(self):
        """Proof: Semantic smuggling breaks the cryptographic hash."""
        original_hash = self.contract.generate_run_id()
        compromised_contract = UMCPContract("FUSION_CORE_LVK", 1e-4, self.weights)
        compromised_hash = compromised_contract.generate_run_id()
        self.assertNotEqual(original_hash, compromised_hash,
                            "RunID failed to fracture on contract mutation.")

    def test_healthy_data_passes(self):
        """Proof: High fidelity and coherence pass the firewall."""
        trace = [0.95, 0.90]
        ledger = audit_dataset(self.contract, trace, self.max_delta)
        self.assertTrue(ledger["Delta"] < self.max_delta)
        self.assertGreater(ledger["IC"], 0.85)

    def test_typed_censoring_penalty(self):
        """Proof: Censored states cannot be smoothed; they drag IC to epsilon levels."""
        trace = [0.99, TraceState.CENSORED]
        clipped = apply_adapter(trace, self.contract)
        ledger = compute_kernel(clipped, self.contract)
        self.assertGreater(ledger["F"], 0.49)
        self.assertLess(ledger["IC"], 0.04)
        with self.assertRaises(SystemError) as context:
            audit_dataset(self.contract, trace, self.max_delta)
        self.assertIn("UMCP VETO: Multiplicative collapse", str(context.exception))

    def test_am_gm_bottleneck_veto(self):
        """Proof: The OS actively halts execution when Delta exceeds threshold."""
        poisoned_trace = [1.0, 0.01]
        with self.assertRaises(SystemError) as context:
            audit_dataset(self.contract, poisoned_trace, self.max_delta)
        self.assertIn("UMCP VETO", str(context.exception))


if __name__ == '__main__':
    print("\n[ARK CI] Executing UMCP Epistemic Firewall Tests...")
    suite = unittest.TestLoader().loadTestsFromTestCase(TestUMCPFirewall)
    result = unittest.TextTestRunner(verbosity=2).run(suite)
    if result.wasSuccessful():
        print("\n[ARK CI] STATUS: TITANIUM. ALL EPISTEMIC GATES HOLD.")
    else:
        print("\n[ARK CI] STATUS: COMPROMISED.")
        exit(1)
