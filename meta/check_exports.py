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

import sys
import struct

def read_leb128(data, offset):
    result = 0
    shift = 0
    while True:
        byte = data[offset]
        offset += 1
        result |= (byte & 0x7f) << shift
        if not (byte & 0x80):
            break
        shift += 7
    return result, offset

def check_exports(file_path):
    with open(file_path, 'rb') as f:
        data = f.read()

    if data[0:4] != b'\x00asm':
        print("Not a WASM file")
        return

    offset = 8
    while offset < len(data):
        section_id = data[offset]
        offset += 1
        payload_len, offset = read_leb128(data, offset)
        next_section = offset + payload_len

        if section_id == 7: # Export Section
            num_exports, offset = read_leb128(data, offset)
            print(f"Found {num_exports} exports:")
            for _ in range(num_exports):
                name_len, offset = read_leb128(data, offset)
                name = data[offset:offset+name_len].decode('utf-8')
                offset += name_len
                kind = data[offset]
                offset += 1
                index, offset = read_leb128(data, offset)
                print(f" - {name} (kind={kind}, index={index})")
            return

        offset = next_section
    print("No Export Section found")

if __name__ == "__main__":
    if len(sys.argv) < 2:
        print("Usage: python check_exports.py <file.wasm>")
    else:
        check_exports(sys.argv[1])
