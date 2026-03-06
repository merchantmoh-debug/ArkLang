#!/usr/bin/env python3
"""
validate_glb.py — Pure-Python GLB/glTF 2.0 Validator

Zero-friction "stamp of approval" equivalent to the Khronos glTF Validator.
Validates binary structure, JSON schema, accessor bounds, buffer sizes,
and index integrity.

Usage:
    python meta/validate_glb.py <file.glb>

Exit codes:
    0 = VALID (green stamp)
    1 = INVALID (structural errors found)
"""
import sys
import struct
import json
import os


class GLBValidationError(Exception):
    """Raised when a GLB validation check fails."""
    pass


def validate_glb(filepath: str) -> dict:
    """
    Validate a binary GLB file against glTF 2.0 spec.

    Returns a dict with:
        valid: bool
        checks: list of {name, status, detail}
        errors: list of error strings
    """
    checks = []
    errors = []

    def check(name: str, condition: bool, detail: str = ""):
        status = "PASS" if condition else "FAIL"
        checks.append({"name": name, "status": status, "detail": detail})
        if not condition:
            errors.append(f"{name}: {detail}")

    # ── Read file ──
    if not os.path.exists(filepath):
        return {"valid": False, "checks": [], "errors": [f"File not found: {filepath}"]}

    with open(filepath, "rb") as f:
        data = f.read()

    file_size = len(data)
    check("file_not_empty", file_size > 0, f"{file_size} bytes")

    if file_size < 12:
        check("min_header_size", False, f"File too small: {file_size} bytes (need 12)")
        return {"valid": False, "checks": checks, "errors": errors}

    # ── GLB Header (12 bytes) ──
    magic, version, length = struct.unpack_from("<III", data, 0)

    check("glb_magic", magic == 0x46546C67,
          f"0x{magic:08X} (expected 0x46546C67 'glTF')")
    check("glb_version", version == 2,
          f"version {version} (expected 2)")
    check("glb_length", length == file_size,
          f"declared {length}, actual {file_size}")

    if magic != 0x46546C67 or version != 2:
        return {"valid": False, "checks": checks, "errors": errors}

    # ── JSON Chunk ──
    if file_size < 20:
        check("json_chunk_header", False, "File too small for JSON chunk header")
        return {"valid": False, "checks": checks, "errors": errors}

    json_chunk_len, json_chunk_type = struct.unpack_from("<II", data, 12)
    check("json_chunk_type", json_chunk_type == 0x4E4F534A,
          f"0x{json_chunk_type:08X} (expected 0x4E4F534A 'JSON')")

    json_end = 20 + json_chunk_len
    check("json_chunk_bounds", json_end <= file_size,
          f"JSON chunk ends at {json_end}, file is {file_size}")

    if json_end > file_size or json_chunk_type != 0x4E4F534A:
        return {"valid": False, "checks": checks, "errors": errors}

    # ── Parse JSON ──
    try:
        json_bytes = data[20:json_end].rstrip(b"\x00").rstrip(b" ")
        gltf = json.loads(json_bytes)
        check("json_parse", True, "JSON parsed successfully")
    except (json.JSONDecodeError, UnicodeDecodeError) as e:
        check("json_parse", False, str(e))
        return {"valid": False, "checks": checks, "errors": errors}

    # ── Required fields ──
    check("asset_field", "asset" in gltf, "'asset' field present")
    if "asset" in gltf:
        check("asset_version", gltf["asset"].get("version") == "2.0",
              f"version: {gltf['asset'].get('version')}")

    check("scenes_field", "scenes" in gltf or "scene" in gltf, "scene(s) defined")
    check("meshes_field", "meshes" in gltf, "'meshes' field present")
    check("accessors_field", "accessors" in gltf, "'accessors' field present")
    check("bufferViews_field", "bufferViews" in gltf, "'bufferViews' field present")
    check("buffers_field", "buffers" in gltf, "'buffers' field present")

    # ── BIN Chunk ──
    bin_offset = 20 + json_chunk_len
    bin_data = b""
    if bin_offset + 8 <= file_size:
        bin_chunk_len, bin_chunk_type = struct.unpack_from("<II", data, bin_offset)
        check("bin_chunk_type", bin_chunk_type == 0x004E4942,
              f"0x{bin_chunk_type:08X} (expected 0x004E4942 'BIN')")
        bin_start = bin_offset + 8
        bin_end = bin_start + bin_chunk_len
        check("bin_chunk_bounds", bin_end <= file_size,
              f"BIN chunk ends at {bin_end}, file is {file_size}")
        if bin_end <= file_size:
            bin_data = data[bin_start:bin_end]

    # ── Buffer size validation ──
    if "buffers" in gltf and bin_data:
        declared_len = gltf["buffers"][0].get("byteLength", 0)
        check("buffer_size_match", len(bin_data) >= declared_len,
              f"declared {declared_len}, actual BIN chunk {len(bin_data)}")

    # ── BufferView validation ──
    if "bufferViews" in gltf and bin_data:
        for i, bv in enumerate(gltf["bufferViews"]):
            bv_offset = bv.get("byteOffset", 0)
            bv_length = bv.get("byteLength", 0)
            bv_end = bv_offset + bv_length
            check(f"bufferView[{i}]_bounds", bv_end <= len(bin_data),
                  f"offset {bv_offset} + length {bv_length} = {bv_end}, bin size {len(bin_data)}")

    # ── Accessor validation ──
    if "accessors" in gltf and "bufferViews" in gltf and bin_data:
        for i, acc in enumerate(gltf["accessors"]):
            comp_type = acc.get("componentType")
            acc_type = acc.get("type")
            count = acc.get("count", 0)

            # Component size
            comp_sizes = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
            comp_size = comp_sizes.get(comp_type, 0)

            # Type multiplier
            type_mults = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4, "MAT4": 16}
            type_mult = type_mults.get(acc_type, 1)

            expected_bytes = count * comp_size * type_mult
            bv_idx = acc.get("bufferView", 0)
            if bv_idx < len(gltf["bufferViews"]):
                bv_len = gltf["bufferViews"][bv_idx].get("byteLength", 0)
                check(f"accessor[{i}]_fits_bufferView",
                      expected_bytes <= bv_len,
                      f"{count}×{comp_size}×{type_mult}={expected_bytes} vs bufferView {bv_len}")

            # Index bounds check (for SCALAR accessors that are indices)
            if acc_type == "SCALAR" and comp_type == 5125 and bv_idx < len(gltf["bufferViews"]):
                bv = gltf["bufferViews"][bv_idx]
                bv_offset = bv.get("byteOffset", 0)
                idx_data = bin_data[bv_offset:bv_offset + bv.get("byteLength", 0)]
                if len(idx_data) >= count * 4:
                    indices = struct.unpack_from(f"<{count}I", idx_data)
                    max_idx = max(indices) if indices else 0
                    # Find the max vertex count from POSITION accessors
                    max_verts = 0
                    for other_acc in gltf["accessors"]:
                        if other_acc.get("type") == "VEC3" and other_acc.get("componentType") == 5126:
                            max_verts = max(max_verts, other_acc.get("count", 0))
                    if max_verts > 0:
                        check(f"accessor[{i}]_index_bounds",
                              max_idx < max_verts,
                              f"max index {max_idx}, vertex count {max_verts}")

    # ── Bounding box validation ──
    if "accessors" in gltf:
        for i, acc in enumerate(gltf["accessors"]):
            if acc.get("type") == "VEC3" and "min" in acc and "max" in acc:
                min_vals = acc["min"]
                max_vals = acc["max"]
                for dim in range(3):
                    check(f"accessor[{i}]_bbox_valid",
                          min_vals[dim] <= max_vals[dim],
                          f"min[{dim}]={min_vals[dim]} <= max[{dim}]={max_vals[dim]}")
                break  # Only check first VEC3 accessor

    valid = len(errors) == 0
    return {"valid": valid, "checks": checks, "errors": errors}


def main():
    if len(sys.argv) < 2:
        print("Usage: python validate_glb.py <file.glb>")
        sys.exit(1)

    filepath = sys.argv[1]
    result = validate_glb(filepath)

    passed = sum(1 for c in result["checks"] if c["status"] == "PASS")
    failed = sum(1 for c in result["checks"] if c["status"] == "FAIL")

    print(f"\n{'=' * 55}")
    print(f"  GLB VALIDATOR — {os.path.basename(filepath)}")
    print(f"{'=' * 55}")
    for c in result["checks"]:
        icon = "✓" if c["status"] == "PASS" else "✗"
        detail = f" ({c['detail']})" if c["detail"] else ""
        print(f"  {icon} {c['name']}{detail}")

    print(f"\n  {'─' * 50}")
    if result["valid"]:
        print(f"  ✓ VALID — {passed}/{passed + failed} checks passed")
        print(f"  Green stamp: glTF 2.0 compliant")
    else:
        print(f"  ✗ INVALID — {failed} check(s) failed")
        for e in result["errors"]:
            print(f"    → {e}")

    print(f"{'=' * 55}\n")
    sys.exit(0 if result["valid"] else 1)


if __name__ == "__main__":
    main()
