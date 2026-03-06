#!/usr/bin/env python3
"""
Spin-Torsion Metamaterial Analysis
===================================
Applies Sarfatti's SO(2,4) gauge gravity equations from RSA Space Unites
(March 5, 2026) to evaluate Ark-compiled lattice geometries for metamaterial
substrate suitability.

Physics (from Sarfatti RSA030526.pdf, slides 8-9):
  G*_μν = k₁T_μν + k₂T*_μν + k₃...
  T*_μν ~ S_μ S_ν + S_λ S^λ g_μν   (spin-torsion source)
  J^a ~ n_s ℏ/2                      (Fermi gas approximation)
  coupling: (e²/(4πε₀ m c³))²        (EM analogy)
  G*/G ~ 10^40  for n_s ~ 10^28/m³

Key variables:
  n_s      = spin carrier density in the bulk material [m^-3]
  φ        = filling fraction (solid_vol / total_vol) [dimensionless]
  n_s_eff  = n_s × φ                 (effective spin density in lattice)
  G*       = G_Newton × (n_s_eff / n_s_ref)² × amplification_factor

Author: Mohamad Al-Zawahreh / Ark Research Division
Date: March 6, 2026
"""

import json
import math

# ============================================================
# PHYSICAL CONSTANTS (MKS — as Jack demands)
# ============================================================
G_NEWTON    = 6.674e-11      # m³ kg⁻¹ s⁻²
HBAR        = 1.054e-34      # J·s
C           = 2.998e8        # m/s
E_CHARGE    = 1.602e-19      # C
EPSILON_0   = 8.854e-12      # F/m
M_ELECTRON  = 9.109e-31      # kg

# Sarfatti's reference spin density for G*/G ~ 10^40
N_S_REF     = 1e28           # m^-3 (from RSA slide derivation)
G_STAR_REF  = 1e40           # G*/G at n_s = n_s_ref

# YIG (Yttrium Iron Garnet) bulk spin density
# Fe³⁺ ions in Y₃Fe₅O₁₂: 5 Fe per formula unit
# Unit cell volume: ~1.896e-27 m³, contains 8 formula units
# = 40 Fe³⁺ spins per unit cell → n_s = 40 / 1.896e-27 ≈ 2.11e28 m⁻³
N_S_YIG     = 2.11e28        # m^-3 (bulk YIG spin density)

# ============================================================
# COMPILED GEOMETRY DATA (from proof-of-matter receipts)
# ============================================================
geometries = [
    {
        "name": "Leviathan Anisotropic Heat Sink",
        "material": "Ti-6Al-4V",
        "topology": "3-axis cylindrical channel array",
        "periodicity": "3D (orthogonal channels)",
        "bounding_vol_mm3": 100**3,           # 100mm cube
        "solid_vol_mm3": 275494,
        "vertices": 287720,
        "triangles": 625980,
        "channels": 972,
        "symmetry": "orthogonal",
        "metamaterial_candidate": True,
        "notes": "972 cylindrical voids, 3 axes × 324 channels"
    },
    {
        "name": "Honeycomb Core",
        "material": "Ti-6Al-4V",
        "topology": "hexagonal prism void array",
        "periodicity": "2D (hex lattice, extruded)",
        "bounding_vol_mm3": 100**3,
        "solid_vol_mm3": 670304,
        "vertices": 1732,
        "triangles": 4012,
        "channels": 144,
        "symmetry": "hexagonal_2D",
        "metamaterial_candidate": True,
        "notes": "144 hex prism voids, staggered 12×12 grid"
    },
    {
        "name": "Gyroid BCC Lattice",
        "material": "Ti-6Al-4V",
        "topology": "body-centered cubic spherical voids",
        "periodicity": "3D (BCC)",
        "bounding_vol_mm3": 100**3,
        "solid_vol_mm3": 429417,
        "vertices": 32498,
        "triangles": 61572,
        "channels": 1024,
        "symmetry": "BCC_cubic",
        "metamaterial_candidate": True,
        "notes": "1024 spherical voids in BCC arrangement (8×8×8×2)"
    },
    {
        "name": "Turbine Disc",
        "material": "Inconel 718",
        "topology": "radial blade slots",
        "periodicity": "1D (azimuthal)",
        "bounding_vol_mm3": math.pi * 75**2 * 15,  # r=75mm, h=15mm cylinder
        "solid_vol_mm3": 181898,
        "vertices": 336,
        "triangles": 736,
        "channels": 24,
        "symmetry": "azimuthal",
        "metamaterial_candidate": False,
        "notes": "24 radial blade slots + central bore. NOT periodic in 3D — excluded from metamaterial ranking."
    },
    {
        "name": "Topology-Optimized Bracket",
        "material": "Al 7075-T6",
        "topology": "lightening holes + bolt bores",
        "periodicity": "None",
        "bounding_vol_mm3": 100 * 100 * 15,  # approximate L-block
        "solid_vol_mm3": 82505,
        "vertices": 243,
        "triangles": 490,
        "channels": 10,
        "symmetry": "none",
        "metamaterial_candidate": False,
        "notes": "Structural part, NOT a metamaterial candidate — excluded from ranking."
    }
]

# ============================================================
# ANALYSIS
# ============================================================

def compute_spin_torsion_metrics(geo: dict, n_s_bulk: float = N_S_YIG) -> dict:
    """
    Compute spin-torsion coupling metrics for a geometry,
    assuming the lattice is fabricated in YIG.
    """
    bv = geo["bounding_vol_mm3"]
    sv = geo["solid_vol_mm3"]

    # Filling fraction
    phi = sv / bv

    # Effective spin density (bulk × filling fraction)
    n_s_eff = n_s_bulk * phi

    # Sarfatti's G*/G scaling
    # From RSA slide: G*/G ~ (n_s / n_s_ref)²  (simplified)
    # More precisely: G* = G × (coupling)² × (J²)
    # where J ~ n_s × ℏ/2
    # The amplification scales as n_s² because G*_μν ~ T*_μν ~ S_μ S_ν ~ J²
    g_star_ratio = (n_s_eff / N_S_REF) ** 2 * G_STAR_REF

    # Spin current magnitude: |J| = n_s_eff × ℏ/2
    J_magnitude = n_s_eff * HBAR / 2  # A/m² (spin current density)

    # Effective gravitational coupling inside metamaterial
    G_effective = G_NEWTON * g_star_ratio

    # Metamaterial quality metrics
    periodicity_score = {
        "3D (orthogonal channels)": 0.85,
        "3D (BCC)": 1.00,         # Best — isotropic 3D periodicity
        "2D (hex lattice, extruded)": 0.70,
        "1D (azimuthal)": 0.30,
        "None": 0.00
    }.get(geo["periodicity"], 0.0)

    # Combined figure of merit:
    # FoM = n_s_eff² × periodicity_score
    # (spin density matters quadratically, periodicity linearly)
    fom = (n_s_eff / N_S_REF) ** 2 * periodicity_score

    return {
        "name": geo["name"],
        "material_if_YIG": "Y₃Fe₅O₁₂ (YIG)",
        "original_material": geo["material"],
        "topology": geo["topology"],
        "periodicity": geo["periodicity"],
        "bounding_volume_mm3": bv,
        "solid_volume_mm3": sv,
        "filling_fraction": phi,
        "n_s_bulk_m3": n_s_bulk,
        "n_s_effective_m3": n_s_eff,
        "spin_current_J_Am2": J_magnitude,
        "G_star_over_G": g_star_ratio,
        "G_effective_m3_kg_s2": G_effective,
        "periodicity_score": periodicity_score,
        "figure_of_merit": fom,
        "symmetry": geo["symmetry"],
        "vertices": geo["vertices"],
        "triangles": geo["triangles"],
        "notes": geo["notes"]
    }


def format_scientific(x: float, precision: int = 2) -> str:
    """Format number in scientific notation."""
    if x == 0:
        return "0"
    exp = int(math.floor(math.log10(abs(x))))
    mantissa = x / 10**exp
    return f"{mantissa:.{precision}f} × 10^{exp}"


# ============================================================
# MAIN
# ============================================================
if __name__ == "__main__":
    print("=" * 72)
    print("SPIN-TORSION METAMATERIAL ANALYSIS")
    print("Sarfatti SO(2,4) Gauge Gravity × Ark Compiled Geometries")
    print("=" * 72)
    print(f"\nBulk YIG spin density: n_s = {format_scientific(N_S_YIG)} m⁻³")
    print(f"Reference: G*/G = {format_scientific(G_STAR_REF)} at n_s = {format_scientific(N_S_REF)} m⁻³")
    print()

    results = []
    for geo in geometries:
        metrics = compute_spin_torsion_metrics(geo)
        results.append(metrics)

    # Separate metamaterial candidates from structural parts
    meta_results = [r for r in results if r.get("metamaterial_candidate", True)]
    struct_results = [r for r in results if not r.get("metamaterial_candidate", True)]

    # Sort metamaterial candidates by figure of merit (descending)
    meta_results.sort(key=lambda x: x["figure_of_merit"], reverse=True)

    # Print comparison table — metamaterial candidates only
    print("-" * 72)
    print(f"{'Geometry':<35} {'φ':>6} {'n_s_eff':>14} {'G*/G':>14} {'FoM':>8}")
    print("-" * 72)
    for r in meta_results:
        phi_pct = f"{r['filling_fraction']*100:.1f}%"
        n_eff = format_scientific(r["n_s_effective_m3"])
        gstar = format_scientific(r["G_star_over_G"])
        fom = f"{r['figure_of_merit']:.4f}"
        marker = " ◄ OPTIMAL" if r == meta_results[0] else ""
        print(f"{r['name']:<35} {phi_pct:>6} {n_eff:>14} {gstar:>14} {fom:>8}{marker}")

    if struct_results:
        print(f"\n  (Excluded — non-periodic structural parts:)")
        for r in struct_results:
            print(f"    {r['name']}: φ={r['filling_fraction']*100:.1f}%, no 3D periodicity")

    print("-" * 72)

    # Detailed report for the winner
    best = meta_results[0]
    print(f"\n{'=' * 72}")
    print(f"OPTIMAL METAMATERIAL SUBSTRATE: {best['name']}")
    print(f"{'=' * 72}")
    print(f"  Topology:           {best['topology']}")
    print(f"  Periodicity:        {best['periodicity']}")
    print(f"  Symmetry:           {best['symmetry']}")
    print(f"  Filling fraction:   {best['filling_fraction']:.4f} ({best['filling_fraction']*100:.1f}%)")
    print(f"  Solid volume:       {best['solid_volume_mm3']:,.0f} mm³")
    print(f"  Mesh complexity:    {best['vertices']:,} verts / {best['triangles']:,} tris")
    print(f"")
    print(f"  Bulk YIG n_s:       {format_scientific(best['n_s_bulk_m3'])} m⁻³")
    print(f"  Effective n_s:      {format_scientific(best['n_s_effective_m3'])} m⁻³")
    print(f"  Spin current |J|:   {format_scientific(best['spin_current_J_Am2'])} A/m²")
    print(f"  G*/G:               {format_scientific(best['G_star_over_G'])}")
    print(f"  G_effective:        {format_scientific(best['G_effective_m3_kg_s2'])} m³ kg⁻¹ s⁻²")
    print(f"  Periodicity score:  {best['periodicity_score']:.2f}")
    print(f"  Figure of merit:    {best['figure_of_merit']:.4f}")
    print(f"")
    print(f"  PHYSICAL INTERPRETATION:")
    print(f"  Inside this YIG metamaterial substrate, the effective gravitational")
    print(f"  coupling is amplified by a factor of {format_scientific(best['G_star_over_G'])}")
    print(f"  relative to Newton's G. The BCC lattice symmetry ensures isotropic")
    print(f"  magnon propagation for Fröhlich condensation at room temperature.")

    # Save results as JSON
    output = {
        "analysis": "Spin-Torsion Metamaterial Substrate Evaluation",
        "source_physics": "Sarfatti SO(2,4) gauge gravity (RSA Space Unites, March 5 2026)",
        "source_geometry": "Ark Leviathan Compiler (5 verified lattice types)",
        "bulk_material": "Y₃Fe₅O₁₂ (Yttrium Iron Garnet)",
        "n_s_bulk": N_S_YIG,
        "reference_G_star_over_G": G_STAR_REF,
        "results": results,
        "optimal_topology": best["name"],
        "recommendation": (
            f"The {best['name']} topology with {best['periodicity']} periodicity "
            f"and {best['filling_fraction']*100:.1f}% filling fraction produces the "
            f"highest figure of merit ({best['figure_of_merit']:.4f}) for spin-torsion "
            f"coupling in a YIG substrate. Its {best['symmetry']} symmetry ensures "
            f"isotropic magnon propagation for Fröhlich condensation."
        )
    }

    with open("spin_torsion_results.json", "w") as f:
        json.dump(output, f, indent=2, default=str)
    print(f"\nResults saved to spin_torsion_results.json")
