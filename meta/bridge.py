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

import pyarrow as pa
import pyarrow.ipc as ipc
import os

IPC_FILE = "ark_state.arrow"

def read_state():
    if not os.path.exists(IPC_FILE):
        print(f"[Qi:Bridge] Error: '{IPC_FILE}' not found. Run Rust core first.")
        return

    print(f"[Qi:Bridge] Reading '{IPC_FILE}' via Zero-Copy...")
    
    with open(IPC_FILE, 'rb') as f:
        reader = ipc.RecordBatchFileReader(f)
        print(f"[Qi:Bridge] Schema: {reader.schema}")
        
        for i in range(reader.num_record_batches):
            batch = reader.get_batch(i)
            print(f"[Qi:Bridge] Batch {i}: {batch.num_rows} rows")
            print(batch.to_pandas())

if __name__ == "__main__":
    read_state()
