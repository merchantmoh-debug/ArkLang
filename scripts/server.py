import http.server
import socketserver
import json
import os
import asyncio
import random
import shutil
import time
from collections import deque
from src.sandbox.local import LocalSandbox

try:
    import psutil
except ImportError:
    psutil = None

PORT = 8000
WEB_DIR = "web"

class NeuralTracker:
    def __init__(self):
        self.history = deque()
        self.lock = asyncio.Lock() # Not used in sync handler, but good practice if async

    def record_activity(self):
        """Record a neural event (execution/inference)"""
        now = time.time()
        self.history.append(now)

    def get_level(self):
        """Return activity level (0-100) based on events in last 60s"""
        now = time.time()
        # Clean up old events
        while self.history and self.history[0] < now - 60:
            self.history.popleft()

        count = len(self.history)
        # Scale: 0-60 events/min -> 0-100%
        # Base level is 10 (idle hum)
        level = min(10 + (count * 2), 100)
        return int(level)

# Global tracker instance
NEURAL_TRACKER = NeuralTracker()

def get_system_stats():
    # CPU
    if psutil:
        cpu = psutil.cpu_percent(interval=None)
        mem = psutil.virtual_memory().percent
    else:
        # Fallback using loadavg
        try:
            load = os.getloadavg()
            count = os.cpu_count() or 1
            # Normalize load to percentage (roughly)
            cpu = min((load[0] / count) * 100, 100.0)
        except:
            cpu = 0.0
        mem = 0.0 # Cannot get mem easily without psutil standard lib

    # Disk
    total, used, free = shutil.disk_usage("/")
    disk = (used / total) * 100

    # Neural Activity (Real Pulse)
    neural = NEURAL_TRACKER.get_level()

    return {
        "cpu": round(cpu, 1),
        "memory": round(mem, 1),
        "disk": round(disk, 1),
        "neural": neural,
        "sys_info": {
            "os": os.name,
            "platform": "Ark Sovereign Runtime",
            "version": "v112.0 (Prime)"
        }
    }

class ArkHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        # API Endpoints
        if self.path == "/api/stats":
            stats = get_system_stats()
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(stats).encode('utf-8'))
            return

        # Static File Serving (Hardened)
        clean_path = os.path.normpath(self.path)

        # Prevent Path Traversal
        if ".." in clean_path:
            self.send_error(403, "Forbidden: Path traversal detected")
            return

        if self.path == "/":
            self.path = "/web/index.html"
        elif not self.path.startswith("/web"):
            # Try to map root requests to web/ directory safely
            potential_path = os.path.normpath(os.path.join(WEB_DIR, self.path.lstrip("/")))

            # Security: Ensure resolved path is strictly inside WEB_DIR
            web_abs = os.path.abspath(WEB_DIR)
            pot_abs = os.path.abspath(potential_path)

            if pot_abs.startswith(web_abs) and os.path.exists(potential_path) and os.path.isfile(potential_path):
                self.path = "/web" + self.path
            else:
                # Security: Block access to files outside web/ unless explicitly mapped
                # If we didn't remap it to /web, and it doesn't start with /web, deny it.
                self.send_error(404, "File not found")
                return

        return super().do_GET()

    def do_POST(self):
        # Proprioception Check (Load Shedding)
        stats = get_system_stats()
        # If load is critical (>95% CPU or >90% RAM), shed load with backpressure
        if stats["cpu"] > 95.0 or stats["memory"] > 90.0:
             self.send_response(429) # Too Many Requests
             self.send_header('Content-type', 'application/json')
             self.send_header('Retry-After', '5')
             self.end_headers()
             msg = f"System Overload: CPU {stats['cpu']}%, MEM {stats['memory']}%"
             self.wfile.write(json.dumps({"error": msg, "stats": stats}).encode('utf-8'))
             return

        if self.path == "/api/run":
            # Record Neural Activity
            NEURAL_TRACKER.record_activity()

            content_len = int(self.headers.get('Content-Length'))
            post_body = self.rfile.read(content_len)
            try:
                data = json.loads(post_body)
                code = data.get("code", "")

                # Execute
                async def run():
                    sandbox = LocalSandbox()
                    # Use "ark" language which now uses Rust runtime
                    return await sandbox.execute(code, "ark")

                result = asyncio.run(run())

                response = {
                    "stdout": result.stdout,
                    "stderr": result.stderr,
                    "exit_code": result.exit_code,
                    "duration": result.duration_ms
                }

                self.send_response(200)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps(response).encode('utf-8'))

            except Exception as e:
                self.send_response(500)
                self.send_header('Content-type', 'application/json')
                self.end_headers()
                self.wfile.write(json.dumps({"error": str(e)}).encode('utf-8'))
        else:
            self.send_response(404)
            self.end_headers()

class ThreadingTCPServer(socketserver.ThreadingMixIn, socketserver.TCPServer):
    allow_reuse_address = True
    daemon_threads = True

if __name__ == "__main__":
    # Enforce Strict Mode by default if not set
    if "ARK_CAPABILITIES" not in os.environ:
        os.environ["ARK_CAPABILITIES"] = "exec,net,fs_read" # Default safe set for demo
        print("ARK_CAPABILITIES set to default strict mode: exec,net,fs_read")

    # Use ThreadingTCPServer for concurrent request handling
    with ThreadingTCPServer(("", PORT), ArkHandler) as httpd:
        print(f"Serving on Sovereign Runtime at http://localhost:{PORT}")
        httpd.serve_forever()
