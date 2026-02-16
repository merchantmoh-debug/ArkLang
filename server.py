import http.server
import socketserver
import json
import os
import asyncio
from src.sandbox.local import LocalSandbox

PORT = 8000
WEB_DIR = "web"

class ArkHandler(http.server.SimpleHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/":
            self.path = "/web/index.html"
        elif not self.path.startswith("/web"):
            # Serve from web dir if not explicit
            if os.path.exists(os.path.join(WEB_DIR, self.path.lstrip("/"))):
                self.path = "/web" + self.path
        return super().do_GET()

    def do_POST(self):
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

with socketserver.TCPServer(("", PORT), ArkHandler) as httpd:
    print(f"Serving at http://localhost:{PORT}")
    httpd.serve_forever()
