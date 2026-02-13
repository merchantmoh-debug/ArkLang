
import asyncio
import time
import json
import os
from pathlib import Path
from src.mcp_client import MCPClientManager, MCPServerConfig
from src.config import settings

# Setup dummy config
CONFIG_PATH = "bench_mcp_config.json"
DATA = {
    "servers": [
        {
            "name": f"server_{i}",
            "command": "echo",
            "args": ["hello"],
            "env": {},
            "enabled": True,
             "transport": "stdio"
        }
        for i in range(100) # 100 servers to make JSON slightly larger
    ]
}

def setup():
    with open(CONFIG_PATH, "w") as f:
        json.dump(DATA, f)
    # Set config path in settings if needed, or pass to init
    settings.MCP_SERVERS_CONFIG = CONFIG_PATH
    settings.MCP_ENABLED = True  # Enable MCP for benchmark

def teardown():
    if os.path.exists(CONFIG_PATH):
        os.remove(CONFIG_PATH)

async def monitor_loop_lag(stop_event):
    """Monitors the event loop lag."""
    max_lag = 0
    while not stop_event.is_set():
        start = time.time()
        await asyncio.sleep(0.001) # Yield to loop
        actual_duration = time.time() - start
        lag = actual_duration - 0.001
        if lag > max_lag:
            max_lag = lag
    return max_lag

async def mock_connect_server(self, config):
    # Simulate async work
    await asyncio.sleep(0.001)

async def benchmark():
    print(f"Starting benchmark with config: {CONFIG_PATH}")

    manager = MCPClientManager(config_path=CONFIG_PATH)
    # Monkey patch _connect_server on the INSTANCE
    manager._connect_server = lambda config: mock_connect_server(manager, config)

    stop_event = asyncio.Event()
    monitor_task = asyncio.create_task(monitor_loop_lag(stop_event))

    start_time = time.time()
    # Call initialize multiple times to stress it
    for i in range(50):
        manager._initialized = False
        await manager.initialize()

    duration = time.time() - start_time
    stop_event.set()
    max_lag = await monitor_task

    print(f"Total Duration: {duration:.4f}s")
    print(f"Max Loop Lag: {max_lag:.6f}s")

    if max_lag > 0.05:
        print("FAIL: Significant loop blocking detected!")
    else:
        print("PASS: Loop blocking is minimal.")

if __name__ == "__main__":
    setup()
    try:
        asyncio.run(benchmark())
    finally:
        teardown()
