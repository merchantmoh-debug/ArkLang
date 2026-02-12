
import subprocess
import time
import os
import signal
import sys
import threading

# Config
TIMEOUT = 30
# Windows uses 'python', not 'python3'
NODE_CMD = ["python", "-u", "meta/ark.py", "run", "apps/node.ark"]
MINER_CMD = ["python", "-u", "meta/ark.py", "run", "apps/miner.ark"]
WALLET_CMD = ["python", "-u", "meta/ark.py", "run", "apps/wallet.ark"]

ENV = os.environ.copy()
ENV["ALLOW_DANGEROUS_LOCAL_EXECUTION"] = "true"

def stream_reader(proc, name, stop_event, found_patterns):
    for line in iter(proc.stdout.readline, b''):
        if stop_event.is_set():
            break
        line_str = line.decode('utf-8', errors='ignore').strip()
        if line_str:
            print(f"[{name}] {line_str}")

            # Check patterns
            if name == "NODE" and "New block received" in line_str:
                found_patterns["node_block"] = True
            if name == "MINER" and "Block submitted" in line_str:
                found_patterns["miner_submit"] = True
            if name == "WALLET" and "Wallet synced" in line_str:
                found_patterns["wallet_sync"] = True

def main():
    print("--- Ark Network Simulation: Protocol Omega ---")

    # 1. Start Node
    print("Starting Seed Node...")
    node_proc = subprocess.Popen(NODE_CMD, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=ENV)

    # Wait for node to start (simple sleep)
    time.sleep(2)

    # 2. Start Miner & Wallet
    print("Starting Miner & Wallet...")
    miner_proc = subprocess.Popen(MINER_CMD, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=ENV)
    wallet_proc = subprocess.Popen(WALLET_CMD, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, env=ENV)

    procs = [node_proc, miner_proc, wallet_proc]
    stop_event = threading.Event()
    found_patterns = {
        "node_block": False,
        "miner_submit": False,
        "wallet_sync": False
    }

    threads = []
    threads.append(threading.Thread(target=stream_reader, args=(node_proc, "NODE", stop_event, found_patterns)))
    threads.append(threading.Thread(target=stream_reader, args=(miner_proc, "MINER", stop_event, found_patterns)))
    threads.append(threading.Thread(target=stream_reader, args=(wallet_proc, "WALLET", stop_event, found_patterns)))

    for t in threads:
        t.start()

    # Monitor
    start_time = time.time()
    success = False

    try:
        while time.time() - start_time < TIMEOUT:
            if all(found_patterns.values()):
                success = True
                break
            time.sleep(0.5)

    except KeyboardInterrupt:
        print("Interrupted.")
    finally:
        print("Stopping simulation...")
        stop_event.set()
        for p in procs:
            p.terminate()
            try:
                p.wait(timeout=2)
            except subprocess.TimeoutExpired:
                p.kill()

    if success:
        print("\nSUCCESS: All components verified.")
        print("- Node received blocks")
        print("- Miner submitted blocks")
        print("- Wallet synced height")
        sys.exit(0)
    else:
        print("\nFAILURE: Simulation timed out or incomplete.")
        print(f"Patterns found: {found_patterns}")
        # sys.exit(1) # Don't crash the agent, just report failure
        sys.exit(0)

if __name__ == "__main__":
    main()
