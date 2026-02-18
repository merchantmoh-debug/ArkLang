import asyncio
import sys
import os
import unittest
import json
import logging

# Add project root to path
sys.path.insert(0, os.path.abspath(os.path.join(os.path.dirname(__file__), '..')))

from meta.p2p_node import P2PNode, NodeID

# Configure logging for test
logging.basicConfig(level=logging.DEBUG)

class TestP2PHandshake(unittest.IsolatedAsyncioTestCase):
    async def test_handshake_and_gossip(self):
        print("\n--- Starting P2P Handshake Test ---")

        # Create nodes
        # Node A is the bootstrap node
        node_a = P2PNode("127.0.0.1", 8001)
        # Node B knows about A
        node_b = P2PNode("127.0.0.1", 8002, bootstrap_nodes=[("127.0.0.1", 8001)])

        # Start nodes
        await node_a.start()
        await node_b.start()

        try:
            # Wait for bootstrap/discovery (UDP)
            print("Waiting for discovery...")
            # B sends FIND_NODE to A. A responds. A sees B.
            # B adds A. A adds B.
            await asyncio.sleep(1.0)

            # Check Routing Tables
            peers_a = node_a.routing_table.get_all_peers()
            peers_b = node_b.routing_table.get_all_peers()

            print(f"Node A peers: {peers_a}")
            print(f"Node B peers: {peers_b}")

            self.assertTrue(any(p.port == 8002 for p in peers_a), "Node A should have discovered Node B")
            self.assertTrue(any(p.port == 8001 for p in peers_b), "Node B should have discovered Node A")

            # Test Gossip (TCP)
            # Node A broadcasts a message. Since it knows B, it should connect and send.
            message = {
                "type": "GOSSIP",
                "data": "Hello Ark Network"
            }
            print("Node A gossiping message...")
            await node_a.gossip(message)

            # Wait for propagation
            try:
                print("Waiting for Node B to receive...")
                received = await asyncio.wait_for(node_b.message_queue.get(), timeout=2.0)
                print(f"Node B received: {received}")
                self.assertEqual(received['data'], "Hello Ark Network")
                self.assertEqual(received['type'], "GOSSIP")
            except asyncio.TimeoutError:
                self.fail("Node B did not receive gossip message in time")

        finally:
            print("Stopping nodes...")
            await node_a.stop()
            await node_b.stop()

if __name__ == "__main__":
    unittest.main()
