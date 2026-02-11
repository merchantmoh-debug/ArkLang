import subprocess
import sys
import json
import time
import os

def send_message(proc, msg):
    body = json.dumps(msg)
    content_length = len(body.encode('utf-8'))
    header = f"Content-Length: {content_length}\r\n\r\n"
    proc.stdin.write(header.encode('ascii'))
    proc.stdin.write(body.encode('utf-8'))
    proc.stdin.flush()

def read_message(proc):
    headers = {}
    while True:
        line = proc.stdout.readline()
        if not line:
            return None
        line = line.decode('ascii').strip()
        if not line:
            break
        parts = line.split(":", 1)
        if len(parts) == 2:
            headers[parts[0].strip()] = parts[1].strip()

    if "Content-Length" in headers:
        length = int(headers["Content-Length"])
        body = proc.stdout.read(length).decode('utf-8')
        return json.loads(body)
    return None

def test_lsp():
    print("Starting LSP server...")
    # Ensure PYTHONPATH includes current dir so imports work if needed
    env = os.environ.copy()
    env["PYTHONPATH"] = os.getcwd()

    proc = subprocess.Popen(
        [sys.executable, "meta/ark_lsp.py"],
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        env=env
    )

    try:
        # 1. Initialize
        print("Sending initialize...")
        send_message(proc, {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {}
        })

        response = read_message(proc)
        print(f"Initialize Response: {response}")
        assert response["id"] == 1
        assert response["result"]["capabilities"]["textDocumentSync"] == 1

        # 2. Open valid file
        print("Sending didOpen (valid)...")
        code = """
        func main() {
            print("Hello")
        }
        """
        send_message(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///test.ark",
                    "languageId": "ark",
                    "version": 1,
                    "text": code
                }
            }
        })

        # Expect publishDiagnostics with empty array
        notification = read_message(proc)
        print(f"Diagnostics (Valid): {notification}")
        assert notification["method"] == "textDocument/publishDiagnostics"
        assert len(notification["params"]["diagnostics"]) == 0

        # 3. Open invalid file
        print("Sending didOpen (invalid)...")
        bad_code = """
        func main() {
            print("Hello"
        }
        """
        send_message(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": "file:///bad.ark",
                    "languageId": "ark",
                    "version": 1,
                    "text": bad_code
                }
            }
        })

        # Expect publishDiagnostics with errors
        notification = read_message(proc)
        print(f"Diagnostics (Invalid): {notification}")
        assert notification["method"] == "textDocument/publishDiagnostics"
        assert len(notification["params"]["diagnostics"]) > 0
        diag = notification["params"]["diagnostics"][0]
        print(f"Diagnostic message: {diag['message']}")

        print("LSP Test Passed!")

    finally:
        proc.terminate()

if __name__ == "__main__":
    test_lsp()
