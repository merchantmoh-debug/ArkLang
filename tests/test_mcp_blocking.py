
import asyncio
import json
import os
import time
import unittest
from pathlib import Path

from src.config import settings
from src.mcp_client import MCPClientManager

# Temporary config path for testing
CONFIG_PATH = "test_mcp_config.json"


class TestMCPBlocking(unittest.TestCase):
    def setUp(self):
        # Create a dummy config file with many entries to simulate load
        self.data = {
            "servers": [
                {
                    "name": f"server_{i}",
                    "command": "echo",
                    "args": ["hello"],
                    "env": {},
                    "enabled": True,
                    "transport": "stdio",
                }
                for i in range(100)
            ]
        }
        with open(CONFIG_PATH, "w") as f:
            json.dump(self.data, f)

        # Backup settings
        self.original_config_path = settings.MCP_SERVERS_CONFIG
        self.original_enabled = settings.MCP_ENABLED
        settings.MCP_SERVERS_CONFIG = CONFIG_PATH
        settings.MCP_ENABLED = True

    def tearDown(self):
        # Restore settings
        settings.MCP_SERVERS_CONFIG = self.original_config_path
        settings.MCP_ENABLED = self.original_enabled
        # Remove dummy config
        if os.path.exists(CONFIG_PATH):
            os.remove(CONFIG_PATH)

    async def monitor_loop_lag(self, stop_event):
        """Monitors the event loop lag."""
        max_lag = 0
        while not stop_event.is_set():
            start = time.time()
            await asyncio.sleep(0.001)  # Yield to loop
            actual_duration = time.time() - start
            lag = actual_duration - 0.001
            if lag > max_lag:
                max_lag = lag
        return max_lag

    def test_load_server_configs_is_non_blocking(self):
        """
        Verify that _load_server_configs does not block the event loop.
        It should use run_in_executor (or asyncio.to_thread) to offload file I/O.
        """

        async def run_test():
            manager = MCPClientManager(config_path=CONFIG_PATH)
            stop_event = asyncio.Event()
            monitor_task = asyncio.create_task(self.monitor_loop_lag(stop_event))

            # Run multiple times to generate load
            for _ in range(50):
                # Directly call _load_server_configs to isolate its performance
                configs = await manager._load_server_configs()
                self.assertEqual(len(configs), 100)

            stop_event.set()
            max_lag = await monitor_task

            # Allow some jitter, but 50ms implies significant blocking for a simple read
            # A typical blocking read of a small file is < 1ms on SSD, but json.load
            # might take longer. If it blocked, we'd see spikes.
            # With threads, we expect near-zero lag (context switch only).
            print(f"Max loop lag observed: {max_lag:.6f}s")
            self.assertLess(max_lag, 0.05, "Event loop was blocked significantly!")

        loop = asyncio.new_event_loop()
        asyncio.set_event_loop(loop)
        try:
            loop.run_until_complete(run_test())
        finally:
            loop.close()


if __name__ == "__main__":
    unittest.main()
