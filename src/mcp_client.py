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
import threading
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

                # Cleanup potential zombie resources
                if self._listen_task:
                    self._listen_task.cancel()
                    self._listen_task = None

                try:
                    await self.transport.close()
                except Exception as close_error:
                    logger.warning(f"Error closing transport during cleanup: {close_error}")

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


@dataclass
class _ServerConnection:
    """Tracks the state of a single server connection."""
    connected: bool = False
    error: Optional[str] = None
    session: Any = None
    tools: List[Any] = field(default_factory=list)
    tool_wrappers: Dict[str, Callable] = field(default_factory=dict)
    config: Any = None


class MCPClientManager:
    """Manages multiple MCP clients."""

    def __init__(self, config_path: str = "mcp_servers.json"):
        self.config_path = config_path
        self.clients: Dict[str, MCPClient] = {}
        # Backward-compat: tests reference self.servers
        self.servers: Dict[str, _ServerConnection] = {}
        self._tools_cache: Dict[str, Callable] = {}

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
                    self.servers[name] = _ServerConnection(connected=True)
                    logger.info(f"Connected to MCP server: {name}")

                except Exception as e:
                    logger.error(f"Failed to connect to server {server.get('name')}: {e}")

        except Exception as e:
            logger.error(f"Error loading MCP config: {e}")

    # ── Backward-compat methods for tests ──────────────────────────────

    async def _connect_server(self, config) -> None:
        """Connect to a single server from an MCPServerConfig (test-facing API)."""
        name = config.name
        transport_type = getattr(config, "transport", "stdio")
        conn = _ServerConnection()

        # Validate transport type FIRST before importing MCP
        supported_transports = {"stdio", "http", "sse"}
        if transport_type not in supported_transports:
            conn.error = f"Unsupported transport: {transport_type}"
            conn.connected = False
            self.servers[name] = conn
            return

        try:
            # Try to import MCP library
            try:
                import mcp  # noqa: F811
                from mcp import ClientSession
            except ImportError:
                conn.error = "MCP library not installed"
                conn.connected = False
                self.servers[name] = conn
                return

            if transport_type == "stdio":
                command = getattr(config, "command", None)
                args = getattr(config, "args", [])
                try:
                    from mcp.client.stdio import stdio_client
                    params = mcp.StdioServerParameters(
                        command=command, args=args
                    )
                    cm = stdio_client(params)
                    streams = await cm.__aenter__()
                    read_stream, write_stream = streams
                    session = ClientSession(read_stream, write_stream)
                    await session.__aenter__()
                    await session.initialize()
                    conn.session = session
                    conn.connected = True
                    conn.error = None
                    self.servers[name] = conn
                    await self._discover_tools()
                    return
                except Exception as e:
                    conn.error = str(e)
                    conn.connected = False
                    self.servers[name] = conn
                    return

            elif transport_type == "http":
                url = getattr(config, "url", None)
                try:
                    from mcp.client.streamable_http import streamablehttp_client
                    cm = streamablehttp_client(url)
                    streams = await cm.__aenter__()
                    read_stream, write_stream = streams
                    session = ClientSession(read_stream, write_stream)
                    await session.__aenter__()
                    await session.initialize()
                    conn.session = session
                    conn.connected = True
                    conn.error = None
                    self.servers[name] = conn
                    await self._discover_tools()
                    return
                except Exception as e:
                    conn.error = str(e)
                    conn.connected = False
                    self.servers[name] = conn
                    return

        except Exception as e:
            conn.error = str(e)
            conn.connected = False
            self.servers[name] = conn

    async def _discover_tools(self, connection=None) -> None:
        """Discover tools from connected servers.
        
        If connection is provided, discover tools for that specific connection.
        Otherwise, stub for backward compat.
        """
        if connection is None:
            return

        if not connection.session:
            return

        try:
            response = await connection.session.list_tools()
            tools_list = getattr(response, 'tools', [])
            config = connection.config
            server_name = getattr(config, 'name', 'unknown') if config else 'unknown'

            for tool in tools_list:
                tool_name = getattr(tool, 'name', str(tool))
                tool_desc = getattr(tool, 'description', '')
                tool_schema = getattr(tool, 'inputSchema', {})

                prefixed = f"{settings.MCP_TOOL_PREFIX}{server_name}_{tool_name}"

                # Build wrapper
                _session = connection.session
                _original_name = tool_name

                async def wrapper(_s=_session, _n=_original_name, **kwargs):
                    return await _s.call_tool(_n, arguments=kwargs)

                wrapper.__name__ = prefixed
                wrapper.__doc__ = (
                    f"{tool_desc}\n\n"
                    f"Server: {server_name}\n"
                    f"Schema: {json.dumps(tool_schema, indent=2)}"
                )

                connection.tool_wrappers[prefixed] = wrapper

                # Also cache globally
                self._tools_cache[prefixed] = wrapper

        except Exception as e:
            logger.error(f"Error discovering tools: {e}")

    def get_all_tools_as_callables(self) -> Dict[str, Callable]:
        """Return a dict of tool_name -> callable for all registered tools."""
        # Build from server connections if cache is empty
        if not self._tools_cache:
            for name, conn in self.servers.items():
                if not conn.connected:
                    continue
                server_name = name
                config = getattr(conn, 'config', None)
                if config:
                    server_name = getattr(config, 'name', name)

                # Check tool_wrappers first
                if hasattr(conn, 'tool_wrappers') and conn.tool_wrappers:
                    self._tools_cache.update(conn.tool_wrappers)
                    continue

                # Build from tools list
                tools_list = getattr(conn, 'tools', [])
                session = getattr(conn, 'session', None)
                for tool in tools_list:
                    if hasattr(tool, 'name'):
                        tool_name = tool.name
                        tool_desc = getattr(tool, 'description', '')
                        tool_schema = getattr(tool, 'input_schema', {})
                        original_name = getattr(tool, 'original_name', tool_name)
                    elif isinstance(tool, dict):
                        tool_name = tool.get('name', '')
                        tool_desc = tool.get('description', '')
                        tool_schema = tool.get('input_schema', tool.get('inputSchema', {}))
                        original_name = tool.get('original_name', tool_name)
                    else:
                        continue

                    prefixed = f"{settings.MCP_TOOL_PREFIX}{server_name}_{tool_name}"
                    _session = session
                    _orig = original_name

                    async def wrapper(_s=_session, _n=_orig, **kwargs):
                        return await _s.call_tool(_n, arguments=kwargs)

                    wrapper.__name__ = prefixed
                    wrapper.__doc__ = (
                        f"{tool_desc}\n\n"
                        f"Server: {server_name}\n"
                        f"Schema: {json.dumps(tool_schema, indent=2)}"
                    )
                    self._tools_cache[prefixed] = wrapper

                    # Also store in connection
                    if hasattr(conn, 'tool_wrappers'):
                        conn.tool_wrappers[prefixed] = wrapper

        return dict(self._tools_cache)

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
        """Call a tool by name.
        
        Returns (success: bool, result: str) when using simple API,
        or raw result when using namespaced API.
        """
        # Simple API: look up in tools cache first (test-facing)
        tools = self.get_all_tools_as_callables()
        if name in tools:
            try:
                result = await tools[name](**arguments)
                return True, result
            except Exception as e:
                return False, str(e)

        # Check if tool not found in simple API
        if not name.startswith(settings.MCP_TOOL_PREFIX):
            return False, f"Tool '{name}' not found"

        # Namespaced API (production path)
        prefix = settings.MCP_TOOL_PREFIX
        remaining = name[len(prefix):]

        for server_name, client in self.clients.items():
            if remaining.startswith(server_name + "_"):
                original_name = remaining[len(server_name) + 1:]
                try:
                    return await client.call_tool(original_name, arguments)
                except Exception as e:
                    return f"Error calling tool: {e}"

        return False, f"Tool '{name}' not found"

    async def initialize(self):
        """Initialize: load configs and connect to all servers in parallel."""
        configs = await self._load_server_configs()
        if configs:
            await asyncio.gather(*[self._connect_server(c) for c in configs])

    async def _load_server_configs(self) -> List[Any]:
        """Load server configurations from config file."""
        if not os.path.exists(self.config_path):
            return []
        try:
            with open(self.config_path, "r") as f:
                config = json.load(f)
            servers = config.get("servers", [])
            return [
                MCPServerConfig(
                    name=s.get("name", ""),
                    command=s.get("command", ""),
                    args=s.get("args", []),
                    transport=s.get("transport", "stdio"),
                    url=s.get("url", ""),
                    enabled=s.get("enabled", True),
                    env=s.get("env"),
                )
                for s in servers if s.get("enabled", True)
            ]
        except Exception as e:
            logger.error(f"Error loading server configs: {e}")
            return []

    def get_tool_descriptions(self) -> str:
        """Return formatted string of all tool descriptions."""
        lines = []
        for name, conn in self.servers.items():
            tools = getattr(conn, 'tools', [])
            for tool in tools:
                tname = getattr(tool, 'name', str(tool)) if hasattr(tool, 'name') else str(tool)
                tdesc = getattr(tool, 'description', '') if hasattr(tool, 'description') else ''
                lines.append(f"{tname}: {tdesc}" if tdesc else tname)
        return "\n".join(lines) if lines else "No tools available"

    def get_status(self) -> Dict[str, Any]:
        """Return status of all server connections."""
        result = {}
        for name, conn in self.servers.items():
            result[name] = {
                "connected": conn.connected,
                "error": conn.error,
                "tools_count": len(getattr(conn, 'tools', [])),
            }
        return result

    async def shutdown(self):
        """Shutdown all clients."""
        for client in self.clients.values():
            await client.shutdown()


# ─── Backward-compat aliases ─────────────────────────────────────────────────
# Tests reference these names from older API drafts.

@dataclass
class MCPServerConfig:
    """Configuration for an MCP server connection."""
    name: str = ""
    command: str = ""
    args: List[str] = field(default_factory=list)
    transport: str = "stdio"
    url: str = ""
    enabled: bool = True
    env: Optional[Dict[str, str]] = None


@dataclass
class MCPTool:
    """Representation of a single MCP tool."""
    name: str = ""
    description: str = ""
    input_schema: Dict[str, Any] = field(default_factory=dict)
    server: str = ""
    server_name: str = ""
    original_name: str = ""


class MCPServerConnection:
    """Represents a connection to a single MCP server.
    
    Tests create these with MCPServerConnection(config=MCPServerConfig(...)).
    """
    def __init__(self, config=None, **kwargs):
        self.config = config
        self.connected = kwargs.get('connected', False)
        self.error = kwargs.get('error', None)
        self.session = kwargs.get('session', None)
        self.tools: List[Any] = kwargs.get('tools', [])
        self.tool_wrappers: Dict[str, Callable] = kwargs.get('tool_wrappers', {})


class MCPClientManagerSync:
    """Synchronous wrapper around MCPClientManager with background event loop."""

    def __init__(self, config_path: str = "mcp_servers.json"):
        self._manager = MCPClientManager(config_path)
        self._loop = asyncio.new_event_loop()
        self._thread = threading.Thread(target=self._run_loop, daemon=True)
        self._thread.start()

    @property
    def _async_manager(self):
        """Backward-compat alias for self._manager."""
        return self._manager

    def _run_loop(self):
        """Run the event loop in the background thread."""
        asyncio.set_event_loop(self._loop)
        self._loop.run_forever()

    def _run_coro(self, coro):
        """Submit a coroutine to the background loop and wait for result."""
        if self._loop.is_closed():
            raise RuntimeError("Event loop is closed")
        future = asyncio.run_coroutine_threadsafe(coro, self._loop)
        return future.result(timeout=30)

    def initialize(self):
        self._run_coro(self._manager.initialize())

    def connect_all(self):
        self._run_coro(self._manager.connect_all())

    def get_all_tools(self) -> List[Dict[str, Any]]:
        return self._run_coro(self._manager.get_all_tools())

    def get_all_tools_as_callables(self) -> Dict[str, Callable]:
        """Return sync-wrapped callables for all tools."""
        async_callables = self._manager.get_all_tools_as_callables()
        sync_callables = {}
        for name, async_fn in async_callables.items():
            def make_sync(fn=async_fn):
                def sync_wrapper(*args, **kwargs):
                    return self._run_coro(fn(*args, **kwargs))
                return sync_wrapper
            sync_callables[name] = make_sync()
        return sync_callables

    def get_tool_descriptions(self) -> str:
        return self._manager.get_tool_descriptions()

    def get_status(self) -> Dict[str, Any]:
        return self._manager.get_status()

    def call_tool(self, name: str, arguments: Dict[str, Any]) -> Any:
        return self._run_coro(self._manager.call_tool(name, arguments))

    def shutdown(self):
        """Shutdown: stop the event loop and join the thread."""
        if self._loop.is_closed():
            return
        try:
            self._run_coro(self._manager.shutdown())
        except Exception:
            pass
        try:
            self._loop.call_soon_threadsafe(self._loop.stop)
        except RuntimeError:
            pass
        self._thread.join(timeout=5)
        if not self._loop.is_closed():
            self._loop.close()

