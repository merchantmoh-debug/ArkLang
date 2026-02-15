import asyncio
import json
import logging
import secrets
import struct
import time
from collections import deque
from typing import Dict, List, Tuple, Optional, Set, Any

# Configure logging
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s [%(levelname)s] %(name)s: %(message)s'
)
logger = logging.getLogger("P2PNode")

# Constants
K_BUCKET_SIZE = 20
ALPHA = 3
ID_BITS = 160

class NodeID:
    """Represents a 160-bit Node ID for Kademlia distance metrics."""
    def __init__(self, value: int = None):
        if value is None:
            self.value = secrets.randbits(ID_BITS)
        else:
            self.value = value

    def __xor__(self, other: 'NodeID') -> int:
        return self.value ^ other.value

    def __eq__(self, other):
        return isinstance(other, NodeID) and self.value == other.value

    def __hash__(self):
        return hash(self.value)

    def __repr__(self):
        return f"{self.value:040x}"

    def __str__(self):
        return self.__repr__()

    @classmethod
    def from_hex(cls, hex_str: str):
        return cls(int(hex_str, 16))

class PeerInfo:
    """Stores contact information for a peer."""
    def __init__(self, node_id: NodeID, host: str, port: int):
        self.node_id = node_id
        self.host = host
        self.port = port
        self.last_seen = time.time()

    def to_dict(self):
        return {
            "id": str(self.node_id),
            "host": self.host,
            "port": self.port
        }

    @classmethod
    def from_dict(cls, data):
        return cls(NodeID.from_hex(data['id']), data['host'], data['port'])

    def __repr__(self):
        return f"<Peer {str(self.node_id)[:8]}... @ {self.host}:{self.port}>"

class RoutingTable:
    """Kademlia Routing Table using K-Buckets."""
    def __init__(self, local_node_id: NodeID):
        self.local_node_id = local_node_id
        self.buckets: List[List[PeerInfo]] = [[] for _ in range(ID_BITS)]

    def add_peer(self, peer: PeerInfo):
        if peer.node_id == self.local_node_id:
            return

        bucket_index = self._get_bucket_index(peer.node_id)
        bucket = self.buckets[bucket_index]

        # Check if peer already exists
        for i, p in enumerate(bucket):
            if p.node_id == peer.node_id:
                bucket.pop(i)
                bucket.append(peer)  # Move to tail (most recently seen)
                return

        if len(bucket) < K_BUCKET_SIZE:
            bucket.append(peer)
        else:
            # In a full implementation, we would ping the head (oldest)
            # and only replace if it doesn't respond. For now, we drop the new one
            # or strictly adhering to Kademlia: replace if old is dead.
            # Simplified: Drop new.
            pass

    def remove_peer(self, node_id: NodeID):
        bucket_index = self._get_bucket_index(node_id)
        bucket = self.buckets[bucket_index]
        for i, p in enumerate(bucket):
            if p.node_id == node_id:
                bucket.pop(i)
                return

    def get_closest_nodes(self, target_id: NodeID, count=K_BUCKET_SIZE) -> List[PeerInfo]:
        all_peers = []
        for bucket in self.buckets:
            all_peers.extend(bucket)

        # Sort by XOR distance
        all_peers.sort(key=lambda p: p.node_id ^ target_id)
        return all_peers[:count]

    def get_all_peers(self) -> List[PeerInfo]:
        all_peers = []
        for bucket in self.buckets:
            all_peers.extend(bucket)
        return all_peers

    def _get_bucket_index(self, node_id: NodeID) -> int:
        distance = self.local_node_id ^ node_id
        if distance == 0:
            return 0
        return (distance).bit_length() - 1

class P2PProtocol(asyncio.DatagramProtocol):
    def __init__(self, node: 'P2PNode'):
        self.node = node

    def connection_made(self, transport):
        pass # Transport is handled in node

    def datagram_received(self, data, addr):
        asyncio.create_task(self.node.handle_udp_packet(data, addr))

class P2PNode:
    """
    AsyncIO P2P Node implementing Kademlia Discovery (UDP)
    and Gossip Protocol (TCP).
    """
    def __init__(self, host: str, port: int, bootstrap_nodes: List[Tuple[str, int]] = []):
        self.node_id = NodeID()
        self.host = host
        self.port = port
        self.routing_table = RoutingTable(self.node_id)
        self.udp_transport = None
        self.tcp_server = None
        self.bootstrap_nodes = bootstrap_nodes
        self.seen_messages: deque = deque(maxlen=10000)
        self.active_connections: Dict[NodeID, Tuple[asyncio.StreamReader, asyncio.StreamWriter]] = {}
        self.running = False
        self.message_queue = asyncio.Queue() # For testing/consumption

    async def start(self):
        self.running = True
        loop = asyncio.get_running_loop()

        # Start UDP Listener
        self.udp_transport, _ = await loop.create_datagram_endpoint(
            lambda: P2PProtocol(self),
            local_addr=(self.host, self.port)
        )
        # logger.info(f"Node {self.node_id} UDP listening on {self.host}:{self.port}")

        # Start TCP Server
        self.tcp_server = await asyncio.start_server(
            self.handle_tcp_connection, self.host, self.port
        )
        # logger.info(f"Node {self.node_id} TCP listening on {self.host}:{self.port}")

        # Bootstrap
        if self.bootstrap_nodes:
            await self.bootstrap()

    async def stop(self):
        self.running = False
        if self.udp_transport:
            self.udp_transport.close()

        if self.tcp_server:
            self.tcp_server.close()
            await self.tcp_server.wait_closed()

        # Close all active TCP connections
        for _, writer in list(self.active_connections.values()):
            writer.close()
            try:
                await writer.wait_closed()
            except:
                pass
        self.active_connections.clear()

    async def bootstrap(self):
        for b_host, b_port in self.bootstrap_nodes:
            # Send FIND_NODE to bootstrap node
            # We treat ourselves as the target to find closest nodes to us
            self.send_udp_message({
                "type": "FIND_NODE",
                "target": str(self.node_id)
            }, (b_host, b_port))

    def send_udp_message(self, message: dict, addr: Tuple[str, int]):
        message['sender_id'] = str(self.node_id)
        message['sender_host'] = self.host
        message['sender_port'] = self.port
        try:
            data = json.dumps(message).encode('utf-8')
            if self.udp_transport:
                self.udp_transport.sendto(data, addr)
        except Exception as e:
            logger.error(f"UDP Send Error: {e}")

    async def handle_udp_packet(self, data: bytes, addr: Tuple[str, int]):
        if not self.running: return
        try:
            msg = json.loads(data.decode('utf-8'))
        except:
            return

        sender_id_str = msg.get('sender_id')
        if not sender_id_str: return

        sender_id = NodeID.from_hex(sender_id_str)
        sender_host = msg.get('sender_host', addr[0])
        sender_port = msg.get('sender_port', addr[1])

        # Update Routing Table
        peer = PeerInfo(sender_id, sender_host, sender_port)
        self.routing_table.add_peer(peer)

        msg_type = msg.get('type')

        if msg_type == 'PING':
            self.send_udp_message({"type": "PONG"}, addr)

        elif msg_type == 'FIND_NODE':
            target_id = NodeID.from_hex(msg.get('target'))
            closest = self.routing_table.get_closest_nodes(target_id)
            response = {
                "type": "FOUND_NODES",
                "nodes": [p.to_dict() for p in closest]
            }
            self.send_udp_message(response, addr)

        elif msg_type == 'FOUND_NODES':
            nodes = msg.get('nodes', [])
            for n in nodes:
                try:
                    new_peer = PeerInfo.from_dict(n)
                    if new_peer.node_id != self.node_id:
                        self.routing_table.add_peer(new_peer)
                except:
                    pass

    async def handle_tcp_connection(self, reader, writer):
        addr = writer.get_extra_info('peername')
        connected_peer_id = None

        # Check if this writer is in active_connections (initiated by us)
        for pid, (r, w) in list(self.active_connections.items()):
            if w == writer:
                connected_peer_id = pid
                break

        try:
            while self.running:
                # Read 4-byte length prefix EXACTLY
                try:
                    length_data = await reader.readexactly(4)
                except asyncio.IncompleteReadError:
                    break

                length = struct.unpack('!I', length_data)[0]

                # Read payload EXACTLY
                try:
                    data = await reader.readexactly(length)
                except asyncio.IncompleteReadError:
                    break

                try:
                    msg = json.loads(data.decode('utf-8'))

                    if msg.get('type') == 'HANDSHAKE':
                         sid = NodeID.from_hex(msg.get('sender_id'))
                         connected_peer_id = sid

                    await self.handle_tcp_message(msg, reader, writer)
                except json.JSONDecodeError:
                    logger.error("TCP Decode Error")
                    break
        except asyncio.CancelledError:
            pass
        except Exception as e:
            pass
        finally:
            if connected_peer_id and connected_peer_id in self.active_connections:
                if self.active_connections[connected_peer_id][1] == writer:
                     del self.active_connections[connected_peer_id]

            writer.close()
            try:
                await writer.wait_closed()
            except:
                pass

    async def handle_tcp_message(self, msg: dict, reader, writer):
        msg_type = msg.get('type')
        msg_id = msg.get('id')

        if msg_id and msg_id in self.seen_messages:
            return
        if msg_id:
            self.seen_messages.append(msg_id)

        if msg_type == 'HANDSHAKE':
            sender_id = NodeID.from_hex(msg.get('sender_id'))
            sender_host = msg.get('sender_host')
            sender_port = msg.get('sender_port')

            peer = PeerInfo(sender_id, sender_host, sender_port)
            self.routing_table.add_peer(peer)
            self.active_connections[sender_id] = (reader, writer)

        elif msg_type == 'GOSSIP':
            await self.message_queue.put(msg) # Expose to application
            await self.gossip(msg, exclude_writer=writer)

    async def connect_tcp(self, peer: PeerInfo):
        if peer.node_id in self.active_connections:
            return self.active_connections[peer.node_id]

        try:
            reader, writer = await asyncio.open_connection(peer.host, peer.port)

            self.active_connections[peer.node_id] = (reader, writer)

            asyncio.create_task(self.handle_tcp_connection(reader, writer))

            # Send Handshake
            handshake = {
                "type": "HANDSHAKE",
                "sender_id": str(self.node_id),
                "sender_host": self.host,
                "sender_port": self.port,
                "id": secrets.token_hex(16)
            }
            await self.send_tcp_message(writer, handshake)

            return reader, writer
        except Exception as e:
            self.routing_table.remove_peer(peer.node_id)
            if peer.node_id in self.active_connections:
                del self.active_connections[peer.node_id]
            return None

    async def send_tcp_message(self, writer, msg: dict):
        data = json.dumps(msg).encode('utf-8')
        length = struct.pack('!I', len(data))
        writer.write(length + data)
        await writer.drain()

    async def gossip(self, message: dict, exclude_writer=None):
        if 'id' not in message:
            message['id'] = secrets.token_hex(16)

        if message['id'] not in self.seen_messages:
             self.seen_messages.append(message['id'])

        peers = self.routing_table.get_closest_nodes(self.node_id, count=8)

        tasks = []
        for peer in peers:
            tasks.append(self._gossip_to_peer(peer, message, exclude_writer))

        if tasks:
            await asyncio.gather(*tasks)

    async def _gossip_to_peer(self, peer: PeerInfo, message: dict, exclude_writer):
        writer = None
        if peer.node_id in self.active_connections:
            _, writer = self.active_connections[peer.node_id]
        else:
            res = await self.connect_tcp(peer)
            if res:
                _, writer = res

        if writer and writer != exclude_writer:
            try:
                await self.send_tcp_message(writer, message)
            except:
                pass
