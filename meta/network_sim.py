import random
import time
import sys

# Simulation Config
NUM_NODES = 100
TARGET_PROPAGATION = 0.90  # 90% of ALL nodes (or active nodes? Let's say total network saturation)
MAX_TICKS = 100
CHURN_PROBABILITY = 0.05   # 5% chance to toggle state per tick
CONNECTION_DEGREE = 5      # Average connections per node

class Node:
    def __init__(self, node_id):
        self.id = node_id
        self.peers = []
        self.height = 0
        self.online = True
        self.receive_tick = -1

    def connect(self, other):
        if other not in self.peers:
            self.peers.append(other)
        if self not in other.peers:
            other.peers.append(self)

class NetworkSimulation:
    def __init__(self):
        self.nodes = [Node(i) for i in range(NUM_NODES)]
        self.tick = 0
        self._build_graph()

    def _build_graph(self):
        # Random graph generation (approximate random regular graph)
        for node in self.nodes:
            while len(node.peers) < CONNECTION_DEGREE:
                peer = random.choice(self.nodes)
                if peer != node and peer not in node.peers:
                    node.connect(peer)

    def churn(self):
        """Randomly toggle node online/offline status."""
        for node in self.nodes:
            if random.random() < CHURN_PROBABILITY:
                node.online = not node.online

    def propagate(self):
        """
        Simulate gossip.
        Nodes that have the block (height=1) and are ONLINE
        broadcast to their ONLINE peers.
        """
        # We use a set to track who receives it THIS tick to avoid infinite instant propagation
        newly_received = []

        # Find all nodes that can broadcast (Online and have block)
        broadcasters = [n for n in self.nodes if n.online and n.height == 1]

        for sender in broadcasters:
            for peer in sender.peers:
                if peer.online and peer.height == 0:
                    # Peer receives the block
                    # In a real network, this takes time (latency).
                    # We simulate this by only allowing them to broadcast NEXT tick.
                    # But we mark them as having it now.
                    # To prevent them from broadcasting in THIS same tick loop (if we iterated differently),
                    # we usually separate read/write, but here 'broadcasters' is a snapshotted list.
                    # So we can update peer safely.
                    if peer.receive_tick == -1: # Not already marked for this tick
                        newly_received.append(peer)

        # Apply updates
        for node in newly_received:
            node.height = 1
            node.receive_tick = self.tick

        return len(newly_received)

    def run(self):
        print(f"--- Ark Network Simulation: {NUM_NODES} Nodes ---")
        print(f"Parameters: Churn={CHURN_PROBABILITY*100}%, Degree={CONNECTION_DEGREE}")

        start_time = time.time()

        # Genesis: Node 0 gets the block
        self.nodes[0].height = 1
        self.nodes[0].receive_tick = 0
        self.nodes[0].online = True # Ensure seed is online

        print(f"[Tick 0] Node 0 mined block 1.")

        for t in range(1, MAX_TICKS + 1):
            self.tick = t
            self.churn()

            new_cnt = self.propagate()

            # Stats
            active_count = sum(1 for n in self.nodes if n.online)
            reached_count = sum(1 for n in self.nodes if n.height == 1)

            # print(f"[Tick {t}] Active: {active_count}, Reached: {reached_count} (+{new_cnt})")

            if reached_count >= NUM_NODES * TARGET_PROPAGATION:
                elapsed = time.time() - start_time
                print(f"\nSUCCESS: Block propagated to {reached_count}/{NUM_NODES} nodes.")
                print(f"Total Ticks: {t}")
                print(f"Real Time: {elapsed:.4f}s")
                return

            if active_count == 0:
                print("\nFAILURE: Network collapse (0 active nodes).")
                return

        print(f"\nFAILURE: Timeout after {MAX_TICKS} ticks. Reached: {sum(1 for n in self.nodes if n.height == 1)}")

if __name__ == "__main__":
    sim = NetworkSimulation()
    sim.run()
