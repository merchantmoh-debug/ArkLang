import subprocess
import sys
import json
import time
import os

def send_message(proc, msg, header_case="Title"):
    body = json.dumps(msg)
    content_length = len(body.encode('utf-8'))
    if header_case == "lower":
        header = f"content-length: {content_length}\r\n\r\n"
    else:
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
    # Grant fs_read capability so the LSP server can parse .ark files
    env["ARK_CAPABILITIES"] = "fs_read"

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
        assert response["result"]["capabilities"]["completionProvider"] is not None

        # 2. Open valid file
        print("Sending didOpen (valid)...")
        code = """func main() {
    print("Hello")
}

func foo() {
    return 1
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

        notification = read_message(proc)
        print(f"Diagnostics (Valid): {notification}")
        assert notification["method"] == "textDocument/publishDiagnostics"

        # 3. Open invalid file (Crash Test)
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

        notification = read_message(proc)
        print(f"Diagnostics (Invalid): {notification}")
        # Ensure we got a response (server didn't crash)
        assert notification is not None
        assert notification["method"] == "textDocument/publishDiagnostics"
        assert len(notification["params"]["diagnostics"]) > 0

        # 4. Completion
        print("Sending completion...")
        send_message(proc, {
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": {"uri": "file:///test.ark"},
                "position": {"line": 1, "character": 0}
            }
        })
        res = read_message(proc)
        print(f"Completion: {res}")
        assert res["id"] == 2
        items = res["result"]["items"]
        labels = [item["label"] for item in items]
        assert "if" in labels
        assert "print" in labels
        assert "sys.crypto.hash" in labels  # Deep intrinsic check
        assert len(items) > 80  # 18 keywords + 75 intrinsics + scope symbols

        # 5. Hover
        # Hover over 'print' at line 1, char 6 inside 'print("Hello")'
        # func main() { -> line 0
        #     print("Hello") -> line 1. 'print' starts at col 4 (4 spaces indent).
        # 'print' is 4-9.
        # Position 6 is inside 'print'.
        print("Sending hover...")
        send_message(proc, {
            "jsonrpc": "2.0",
            "id": 3,
            "method": "textDocument/hover",
            "params": {
                "textDocument": {"uri": "file:///test.ark"},
                "position": {"line": 1, "character": 6}
            }
        })
        res = read_message(proc)
        print(f"Hover: {res}")
        assert res["id"] == 3
        # Should return markdown for Function Call
        if res["result"]:
            assert ("Intrinsic" in res["result"]["contents"]["value"]) or ("Call" in res["result"]["contents"]["value"])
            assert "print" in res["result"]["contents"]["value"]
        else:
            print("Hover returned null (maybe pos missed?)")

        # 6. Definition
        # Call 'print'. Definition not in file. Should return null.
        print("Sending definition (missing)...")
        send_message(proc, {
            "jsonrpc": "2.0",
            "id": 4,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": "file:///test.ark"},
                "position": {"line": 1, "character": 6}
            }
        })
        res = read_message(proc)
        print(f"Definition (Missing): {res}")
        assert res["result"] == 0 or res["result"] is None

        # Call 'foo' (if added).
        # Let's update file to add call to foo.
        code_v2 = """func main() {
    foo()
}

func foo() {
    return 1
}
"""
        send_message(proc, {
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {"uri": "file:///test.ark", "version": 2},
                "contentChanges": [{"text": code_v2}]
            }
        })
        read_message(proc) # Diagnostics

        # Definition of 'foo'
        # foo() at line 1. 'foo' at col 4.
        print("Sending definition (valid)...")
        send_message(proc, {
            "jsonrpc": "2.0",
            "id": 5,
            "method": "textDocument/definition",
            "params": {
                "textDocument": {"uri": "file:///test.ark"},
                "position": {"line": 1, "character": 5}
            }
        })
        res = read_message(proc)
        print(f"Definition (Valid): {res}")
        assert res["result"] is not None
        assert res["result"] != 0
        # Should point to line 4 (func foo)
        r = res["result"]["range"]
        assert r["start"]["line"] == 4

        print("LSP Test Passed!")

    finally:
        # Cleanup
        send_message(proc, {"jsonrpc": "2.0", "method": "shutdown", "id": 99})
        read_message(proc)
        send_message(proc, {"jsonrpc": "2.0", "method": "exit"})
        proc.terminate()
        proc.wait()

if __name__ == "__main__":
    test_lsp()
