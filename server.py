import http.server
import socketserver
import json
import os
import asyncio
import random
import shutil
from src.sandbox.local import LocalSandbox

try:
    import psutil
except ImportError:
    psutil = None

PORT = 8000
WEB_DIR = "web"

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

    # Neural Activity (Simulated Pulse)
    # In a real system, this would be requests/sec or tokens/sec
    neural = random.randint(20, 90)

    return {
        "cpu": round(cpu, 1),
        "memory": round(mem, 1),
        "disk": round(disk, 1),
        "neural": neural
    }

class ArkHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/api/stats":
            stats = get_system_stats()
            self.send_response(200)
            self.send_header('Content-type', 'application/json')
            self.end_headers()
            self.wfile.write(json.dumps(stats).encode('utf-8'))
            return

        if self.path == "/":
            self.path = "/web/index.html"
        elif not self.path.startswith("/web"):
            # Serve from web dir if not explicit
            if os.path.exists(os.path.join(WEB_DIR, self.path.lstrip("/"))):
                self.path = "/web" + self.path
        return super().do_GET()

    def do_POST(self):
        # Proprioception Check (Load Shedding)
        stats = get_system_stats()
        # If load is critical (>95% CPU), shed load
        # Note: simulated in fallback mode, but functional structure is here
        if stats["cpu"] > 95.0:
             self.send_response(503)
             self.send_header('Content-type', 'application/json')
             self.end_headers()
             self.wfile.write(json.dumps({"error": "System Overload: Proprioception limits exceeded."}).encode('utf-8'))
             return

        if self.path == "/api/run":
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

class ReusableTCPServer(socketserver.TCPServer):
    allow_reuse_address = True

if __name__ == "__main__":
    # Enforce Strict Mode by default if not set
    if "ARK_CAPABILITIES" not in os.environ:
        os.environ["ARK_CAPABILITIES"] = "exec,net,fs_read" # Default safe set for demo
        print("ARK_CAPABILITIES set to default strict mode: exec,net,fs_read")

    with ReusableTCPServer(("", PORT), ArkHandler) as httpd:
        print(f"Serving at http://localhost:{PORT}")
        httpd.serve_forever()
