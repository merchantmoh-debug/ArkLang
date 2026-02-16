"""
MCP (Model Context Protocol) Client Implementation.

This module provides a pure Python implementation of the MCP client,
supporting JSON-RPC 2.0 over Stdio, HTTP, and SSE transports.
"""

import asyncio
import json
import logging
import os
import sys
import time
import uuid
import urllib.request
import urllib.error
import urllib.parse
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Union, Callable

from src.config import settings

# Configure logging
logging.basicConfig(level=logging.INFO)
logger = logging.getLogger("mcp_client")


class MCPError(Exception):
    """Base exception for MCP errors."""
    def __init__(self, message: str, code: Optional[int] = None, data: Any = None):
        super().__init__(message)
        self.code = code
        self.data = data


class MCPConnectionError(MCPError):
    """Error related to transport connection."""
    pass


class MCPTimeoutError(MCPError):
    """Request timed out."""
    pass


class MCPToolError(MCPError):
    """Error during tool execution."""
    pass


@dataclass
class JSONRPCRequest:
    method: str
    params: Optional[Dict[str, Any]] = None
    id: Optional[Union[str, int]] = None
    jsonrpc: str = "2.0"

    def to_dict(self) -> Dict[str, Any]:
        data = {"jsonrpc": self.jsonrpc, "method": self.method}
        if self.params is not None:
            data["params"] = self.params
        if self.id is not None:
            data["id"] = self.id
        return data


@dataclass
class JSONRPCResponse:
    id: Optional[Union[str, int]]
    result: Any = None
    error: Optional[Dict[str, Any]] = None
    jsonrpc: str = "2.0"

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "JSONRPCResponse":
        return cls(
            id=data.get("id"),
            result=data.get("result"),
            error=data.get("error"),
            jsonrpc=data.get("jsonrpc", "2.0"),
        )


class Transport(ABC):
    """Abstract base class for MCP transports."""

    @abstractmethod
    async def connect(self) -> None:
        """Establish connection."""
        pass

    @abstractmethod
    async def send(self, request: JSONRPCRequest) -> None:
        """Send a JSON-RPC request."""
        pass

    @abstractmethod
    async def receive(self) -> Optional[JSONRPCResponse]:
        """Receive a JSON-RPC response."""
        pass

    @abstractmethod
    async def close(self) -> None:
        """Close connection."""
        pass

    @abstractmethod
    def is_connected(self) -> bool:
        """Check if connected."""
        pass


class StdioTransport(Transport):
    """Transport over standard input/output streams."""

    def __init__(self, command: str, args: List[str], env: Optional[Dict[str, str]] = None):
        self.command = command
        self.args = args
        self.env = env or os.environ.copy()
        self.process: Optional[asyncio.subprocess.Process] = None
        self._read_queue: asyncio.Queue = asyncio.Queue()
        self._reader_task: Optional[asyncio.Task] = None

    async def connect(self) -> None:
        try:
            self.process = await asyncio.create_subprocess_exec(
                self.command,
                *self.args,
                stdin=asyncio.subprocess.PIPE,
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE,
                env=self.env,
            )
            self._reader_task = asyncio.create_task(self._read_loop())
            logger.info(f"StdioTransport connected to {self.command}")
        except Exception as e:
            raise MCPConnectionError(f"Failed to start process {self.command}: {e}")

    async def _read_loop(self):
        """Reads JSON-RPC messages from stdout."""
        if not self.process or not self.process.stdout:
            return

        while True:
            try:
                line = await self.process.stdout.readline()
                if not line:
                    break

                line_str = line.decode("utf-8").strip()
                if not line_str:
                    continue

                if line_str.startswith("Content-Length:"):
                    try:
                        length = int(line_str.split(":")[1].strip())
                        await self.process.stdout.readline()  # Empty line
                        body = await self.process.stdout.readexactly(length)
                        message = json.loads(body.decode("utf-8"))
                        await self._read_queue.put(JSONRPCResponse.from_dict(message))
                    except Exception as e:
                        logger.error(f"Error parsing content-length message: {e}")
                else:
                    try:
                        message = json.loads(line_str)
                        await self._read_queue.put(JSONRPCResponse.from_dict(message))
                    except json.JSONDecodeError:
                        pass
            except Exception as e:
                logger.error(f"Read loop error: {e}")
                break

        await self.close()

    async def send(self, request: JSONRPCRequest) -> None:
        if not self.process or not self.process.stdin:
            raise MCPConnectionError("Not connected")

        data = json.dumps(request.to_dict()).encode("utf-8")
        header = f"Content-Length: {len(data)}\r\n\r\n".encode("utf-8")

        try:
            self.process.stdin.write(header + data)
            await self.process.stdin.drain()
        except Exception as e:
            raise MCPConnectionError(f"Failed to send data: {e}")

    async def receive(self) -> Optional[JSONRPCResponse]:
        return await self._read_queue.get()

    async def close(self) -> None:
        if self._reader_task:
            self._reader_task.cancel()
        if self.process:
            try:
                self.process.terminate()
                await self.process.wait()
            except Exception:
                pass
            self.process = None

    def is_connected(self) -> bool:
        return self.process is not None and self.process.returncode is None


def _http_request(url: str, method: str = "GET", data: Optional[Dict] = None, timeout: int = 30) -> Dict:
    """Helper for synchronous HTTP requests using urllib."""
    req = urllib.request.Request(url, method=method)
    req.add_header("Content-Type", "application/json")

    body = None
    if data is not None:
        body = json.dumps(data).encode("utf-8")
        req.add_header("Content-Length", str(len(body)))

    try:
        with urllib.request.urlopen(req, data=body, timeout=timeout) as response:
            resp_body = response.read().decode("utf-8")
            if resp_body:
                return json.loads(resp_body)
            return {}
    except urllib.error.HTTPError as e:
        error_body = e.read().decode("utf-8")
        raise MCPConnectionError(f"HTTP {e.code}: {e.reason} - {error_body}")
    except Exception as e:
        raise MCPConnectionError(f"HTTP request failed: {e}")


class HttpTransport(Transport):
    """Transport over HTTP (Post-based)."""

    def __init__(self, url: str, timeout: int = 30):
        self.url = url
        self.timeout = timeout
        self._connected = False
        self._response_queue = asyncio.Queue()

    async def connect(self) -> None:
        try:
            # Simple GET to check connectivity
            await asyncio.to_thread(_http_request, self.url, method="GET", timeout=5)
            self._connected = True
        except Exception as e:
            # Some endpoints might not support GET, but we assume reachability
            # If GET fails with 405 Method Not Allowed, it's still reachable.
            if "HTTP 405" in str(e):
                self._connected = True
            else:
                raise MCPConnectionError(f"Failed to connect to {self.url}: {e}")

    async def send(self, request: JSONRPCRequest) -> None:
        if not self._connected:
            raise MCPConnectionError("Not connected")

        try:
            response_data = await asyncio.to_thread(
                _http_request,
                self.url,
                method="POST",
                data=request.to_dict(),
                timeout=self.timeout
            )
            await self._response_queue.put(JSONRPCResponse.from_dict(response_data))
        except Exception as e:
            raise MCPConnectionError(f"HTTP request failed: {e}")

    async def receive(self) -> Optional[JSONRPCResponse]:
        return await self._response_queue.get()

    async def close(self) -> None:
        self._connected = False

    def is_connected(self) -> bool:
        return self._connected


class SseTransport(Transport):
    """Transport over Server-Sent Events (SSE)."""

    def __init__(self, url: str):
        self.url = url
        self._connected = False
        self._queue = asyncio.Queue()
        self._task: Optional[asyncio.Task] = None
        self.post_url = url

    async def connect(self) -> None:
        self._connected = True
        self._task = asyncio.create_task(self._read_stream())

    async def _read_stream(self):
        try:
            req = urllib.request.Request(self.url)
            # SSE headers
            req.add_header("Accept", "text/event-stream")

            # Using urllib in a thread for blocking read
            def read_sse():
                with urllib.request.urlopen(req, timeout=None) as response:
                    for line in response:
                        if not self._connected:
                            break
                        yield line

            iterator = await asyncio.to_thread(read_sse)

            current_event = None

            # We need to iterate the generator in a way that allows yielding control
            # but read_sse is a generator running in a thread? No, to_thread runs the function.
            # Generator cannot be pickled/passed easily from to_thread if it yields?
            # Actually asyncio.to_thread runs the function and returns result.
            # If read_sse yields, to_thread returns a generator? No, it waits for function to return.
            # So I cannot use to_thread(read_sse).

            # I have to implement a loop that reads chunks/lines in a thread.

            while self._connected:
                # This is inefficient: opening a connection and keeping it open in a thread
                # blocking the thread pool. But okay for now.
                # Better: Run the whole reading loop in a thread and use call_soon_threadsafe to put to queue.

                await asyncio.to_thread(self._blocking_read_loop)
                break

        except Exception as e:
            logger.error(f"SSE stream error: {e}")
            self._connected = False

    def _blocking_read_loop(self):
        try:
            req = urllib.request.Request(self.url)
            req.add_header("Accept", "text/event-stream")

            with urllib.request.urlopen(req, timeout=None) as response:
                current_event = None
                for line in response:
                    if not self._connected:
                        break

                    decoded_line = line.decode('utf-8').strip()
                    if decoded_line.startswith('event: '):
                        current_event = decoded_line[7:]
                    elif decoded_line.startswith('data: '):
                        data = decoded_line[6:]
                        if current_event == 'endpoint':
                            self.post_url = data.strip()
                            if not self.post_url.startswith('http'):
                                self.post_url = urllib.parse.urljoin(self.url, self.post_url)
                            logger.info(f"SSE Transport: Post endpoint updated to {self.post_url}")
                        else:
                            try:
                                message = json.loads(data)
                                # Put to async queue from thread
                                asyncio.run_coroutine_threadsafe(
                                    self._queue.put(JSONRPCResponse.from_dict(message)),
                                    asyncio.get_event_loop()
                                )
                            except json.JSONDecodeError:
                                pass
                        current_event = None
        except Exception as e:
             logger.error(f"Blocking read loop error: {e}")

    async def send(self, request: JSONRPCRequest) -> None:
        target_url = getattr(self, 'post_url', self.url)
        try:
            await asyncio.to_thread(
                _http_request,
                target_url,
                method="POST",
                data=request.to_dict()
            )
        except Exception as e:
            raise MCPConnectionError(f"Failed to send to SSE endpoint {target_url}: {e}")

    async def receive(self) -> Optional[JSONRPCResponse]:
        return await self._queue.get()

    async def close(self) -> None:
        self._connected = False
        if self._task:
            self._task.cancel()

    def is_connected(self) -> bool:
        return self._connected


class MCPClient:
    """
    Client for Model Context Protocol.
    """

    def __init__(self, transport: Transport):
        self.transport = transport
        self._request_id = 0
        self._pending_requests: Dict[Union[str, int], asyncio.Future] = {}
        self._notification_handlers: Dict[str, Callable] = {}
        self._listen_task: Optional[asyncio.Task] = None
        self.capabilities: Dict[str, Any] = {}
        self.server_capabilities: Dict[str, Any] = {}

    async def connect(self):
        """Connect to the server and start listening, with retry logic."""
        retries = 3
        backoff = 1.0

        for attempt in range(retries):
            try:
                await self.transport.connect()
                self._listen_task = asyncio.create_task(self._listen_loop())
                await self.initialize()
                # Start health check
                asyncio.create_task(self._health_check_loop())
                return
            except Exception as e:
                logger.warning(f"Connection attempt {attempt+1}/{retries} failed: {e}")
                if attempt < retries - 1:
                    await asyncio.sleep(backoff)
                    backoff *= 2
                else:
                    logger.error("All connection attempts failed")
                    raise MCPConnectionError(f"Failed to connect after {retries} attempts: {e}")

    async def _health_check_loop(self):
        """Periodic health check."""
        while self.transport.is_connected():
            try:
                # Use a lightweight request like listing roots or tools to verify connectivity
                # Standard MCP might not have a dedicated ping, but list_roots is often fast.
                await self.list_roots()
            except Exception as e:
                logger.warning(f"Health check failed: {e}")
                # If health check fails, we might want to trigger reconnect, but for now just log.
                # Transport closure should be handled by _listen_loop or read failure.
            await asyncio.sleep(60) # Ping every 60s

    async def list_roots(self) -> List[Dict[str, Any]]:
        """List roots (filesystem roots)."""
        # This is part of MCP spec but not implemented in my previous draft
        try:
             result = await self.send_request("roots/list")
             return result.get("roots", [])
        except Exception:
             return []

    async def _listen_loop(self):
        """Listen for incoming messages."""
        while self.transport.is_connected():
            try:
                response = await self.transport.receive()
                if not response:
                    continue

                if response.id is not None:
                    # Response to a request
                    if response.id in self._pending_requests:
                        future = self._pending_requests.pop(response.id)
                        if not future.done():
                            if response.error:
                                future.set_exception(MCPError(
                                    response.error.get("message", "Unknown error"),
                                    response.error.get("code"),
                                    response.error.get("data")
                                ))
                            else:
                                future.set_result(response.result)
                else:
                    # Notification (no ID)
                    if response.method:
                         handler = self._notification_handlers.get(response.method)
                         if handler:
                             try:
                                 if asyncio.iscoroutinefunction(handler):
                                     await handler(response.params)
                                 else:
                                     handler(response.params)
                             except Exception as e:
                                 logger.error(f"Error handling notification {response.method}: {e}")
            except asyncio.CancelledError:
                break
            except Exception as e:
                logger.error(f"Error in listen loop: {e}")
                await asyncio.sleep(0.1)

    async def send_request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        """Send a request and wait for response (Public API)."""
        return await self._send_request(method, params)

    async def send_notification(self, method: str, params: Optional[Dict[str, Any]] = None) -> None:
        """Send a notification (no response expected)."""
        request = JSONRPCRequest(method=method, params=params, id=None)
        await self.transport.send(request)

    async def _send_request(self, method: str, params: Optional[Dict[str, Any]] = None) -> Any:
        """Send a request and wait for response."""
        request_id = self._request_id
        self._request_id += 1

        request = JSONRPCRequest(method=method, params=params, id=request_id)
        future = asyncio.Future()
        self._pending_requests[request_id] = future

        await self.transport.send(request)

        # Wait for response with timeout
        try:
            return await asyncio.wait_for(future, timeout=settings.MCP_CONNECTION_TIMEOUT)
        except asyncio.TimeoutError:
            self._pending_requests.pop(request_id, None)
            raise MCPTimeoutError(f"Request {method} timed out")

    async def initialize(self) -> Dict[str, Any]:
        """Initialize the session."""
        params = {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "roots": {"listChanged": True},
                "sampling": {}
            },
            "clientInfo": {
                "name": "ArkMCPClient",
                "version": "0.1.0"
            }
        }
        result = await self._send_request("initialize", params)
        self.server_capabilities = result.get("capabilities", {})

        # Send initialized notification
        await self.transport.send(JSONRPCRequest(method="notifications/initialized"))
        return result

    async def list_tools(self) -> List[Dict[str, Any]]:
        """List available tools."""
        result = await self._send_request("tools/list")
        return result.get("tools", [])

    async def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool."""
        params = {
            "name": name,
            "arguments": arguments
        }
        result = await self._send_request("tools/call", params)
        return result

    async def list_resources(self) -> List[Dict[str, Any]]:
        """List available resources."""
        result = await self._send_request("resources/list")
        return result.get("resources", [])

    async def read_resource(self, uri: str) -> str:
        """Read a resource."""
        params = {"uri": uri}
        result = await self._send_request("resources/read", params)
        # Result typically contains contents list
        contents = result.get("contents", [])
        if contents:
            return contents[0].get("text", "")
        return ""

    async def shutdown(self):
        """Shutdown the client."""
        if self._listen_task:
            self._listen_task.cancel()
        await self.transport.close()


class MCPClientManager:
    """Manages multiple MCP clients."""

    def __init__(self, config_path: str = "mcp_servers.json"):
        self.config_path = config_path
        self.clients: Dict[str, MCPClient] = {}

    async def connect_all(self):
        """Connect to all configured servers."""
        if not os.path.exists(self.config_path):
            return

        try:
            with open(self.config_path, "r") as f:
                config = json.load(f)

            servers = config.get("servers", [])
            for server in servers:
                if not server.get("enabled", True):
                    continue

                try:
                    name = server["name"]
                    transport_type = server.get("transport", "stdio")

                    if transport_type == "stdio":
                        transport = StdioTransport(
                            command=server["command"],
                            args=server.get("args", []),
                            env=server.get("env")
                        )
                    elif transport_type == "http":
                        transport = HttpTransport(url=server["url"])
                    elif transport_type == "sse":
                        transport = SseTransport(url=server["url"])
                    else:
                        logger.warning(f"Unknown transport: {transport_type}")
                        continue

                    client = MCPClient(transport)
                    await client.connect()
                    self.clients[name] = client
                    logger.info(f"Connected to MCP server: {name}")

                except Exception as e:
                    logger.error(f"Failed to connect to server {server.get('name')}: {e}")

        except Exception as e:
            logger.error(f"Error loading MCP config: {e}")

    async def get_all_tools(self) -> List[Dict[str, Any]]:
        """Get all tools from all connected servers."""
        all_tools = []
        for name, client in self.clients.items():
            try:
                tools = await client.list_tools()
                for tool in tools:
                    tool["server"] = name
                    # Namespace the tool name
                    tool["original_name"] = tool["name"]
                    tool["name"] = f"{settings.MCP_TOOL_PREFIX}{name}_{tool['name']}"
                    all_tools.append(tool)
            except Exception as e:
                logger.error(f"Error listing tools for {name}: {e}")
        return all_tools

    async def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        """Call a tool by its namespaced name."""
        prefix = settings.MCP_TOOL_PREFIX
        if not name.startswith(prefix):
             return f"Error: Tool name must start with {prefix}"

        remaining = name[len(prefix):]

        for server_name, client in self.clients.items():
            if remaining.startswith(server_name + "_"):
                original_name = remaining[len(server_name) + 1:]
                try:
                    return await client.call_tool(original_name, arguments)
                except Exception as e:
                    return f"Error calling tool: {e}"

        return f"Error: Tool {name} not found"

    async def shutdown(self):
        """Shutdown all clients."""
        for client in self.clients.values():
            await client.shutdown()
