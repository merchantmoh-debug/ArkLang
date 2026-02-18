import unittest
import asyncio
from unittest.mock import AsyncMock, MagicMock, patch
from src.mcp_client import MCPClient, Transport, MCPConnectionError, JSONRPCResponse, JSONRPCRequest, MCPTimeoutError

class MockTransport(Transport):
    def __init__(self):
        self.connect_side_effect = None
        self.send_side_effect = None
        self.receive_side_effect = None
        self.connected = False
        self._close_called = 0
        self.close_mock = AsyncMock() # Spy

    async def connect(self):
        if self.connect_side_effect:
            if isinstance(self.connect_side_effect, Exception):
                raise self.connect_side_effect
            await self.connect_side_effect()
        self.connected = True

    async def send(self, request: JSONRPCRequest):
        if not self.connected:
            raise MCPConnectionError("Not connected")
        if self.send_side_effect:
            if isinstance(self.send_side_effect, Exception):
                raise self.send_side_effect
            await self.send_side_effect(request)

    async def receive(self):
        if self.receive_side_effect:
             if isinstance(self.receive_side_effect, Exception):
                raise self.receive_side_effect
             res = await self.receive_side_effect()
             return res
        # Return nothing (simulate idle connection)
        await asyncio.sleep(0.01)
        return None

    async def close(self):
        self.connected = False
        self._close_called += 1
        await self.close_mock()

    def is_connected(self):
        return self.connected

class TestMCPConnectFailure(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        # Create a fast sleep mock that sleeps only for short durations
        # This prevents infinite loops in receive() while skipping long backoffs
        self.original_sleep = asyncio.sleep

        async def fast_sleep(delay, result=None):
            if delay >= 0.5: # Skip backoff sleeps (1.0, 2.0)
                return
            await self.original_sleep(delay, result=result)

        self.sleep_patcher = patch("asyncio.sleep", side_effect=fast_sleep)
        self.mock_sleep = self.sleep_patcher.start()

    async def asyncTearDown(self):
        self.sleep_patcher.stop()

    async def test_connect_transport_failure(self):
        """Test that connect fails after retries if transport.connect fails."""
        transport = MockTransport()
        transport.connect_side_effect = Exception("Connection refused")

        client = MCPClient(transport)

        with self.assertRaises(MCPConnectionError) as cm:
            await client.connect()

        self.assertIn("Failed to connect after 3 attempts", str(cm.exception))

    async def test_connect_handshake_failure(self):
        """Test that connect fails after retries if initialization (send) fails."""
        transport = MockTransport()
        transport.connect_side_effect = None
        transport.send_side_effect = Exception("Send failed")

        client = MCPClient(transport)

        with self.assertRaises(MCPConnectionError) as cm:
            await client.connect()

        self.assertIn("Failed to connect after 3 attempts", str(cm.exception))

    async def test_connect_handshake_timeout(self):
        """Test that connect fails if initialization times out."""
        transport = MockTransport()
        transport.connect_side_effect = None

        client = MCPClient(transport)

        # Patch settings to have a very short timeout
        with patch("src.config.settings.MCP_CONNECTION_TIMEOUT", 0.05):
             with self.assertRaises(MCPConnectionError) as cm:
                await client.connect()

             self.assertIn("Failed to connect after 3 attempts", str(cm.exception))

    async def test_cleanup_on_failure(self):
        """Test that transport is closed if initialization fails."""
        transport = MockTransport()
        transport.connect_side_effect = None
        # Send fails -> initialize fails
        transport.send_side_effect = Exception("Send failed during init")

        client = MCPClient(transport)

        with self.assertRaises(MCPConnectionError):
            await client.connect()

        # We expect 3 failed attempts.
        # Each failure should trigger a close() to release resources.
        # If connect fails, we might not close (nothing to close?), but if initialize fails, we MUST close.
        # Since connect succeeded, transport is connected.
        # If we don't close, the connection stays open.

        # We expect close called 3 times (once per attempt).
        self.assertGreaterEqual(transport._close_called, 3, "Transport was not closed on initialization failure")

if __name__ == "__main__":
    unittest.main()
