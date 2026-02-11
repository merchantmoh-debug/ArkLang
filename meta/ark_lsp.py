#!/usr/bin/env python3
import sys
import json
import os
import logging

# Configure logging to stderr so it doesn't interfere with stdout
logging.basicConfig(stream=sys.stderr, level=logging.INFO, format='%(asctime)s - %(levelname)s - %(message)s')

try:
    from lark import Lark, exceptions
except ImportError:
    logging.error("Lark not found. Please install it.")
    sys.exit(1)

class ArkLSP:
    """
    A basic Language Server Protocol (LSP) stub implementation for Ark.
    Communicates via JSON-RPC over stdio.
    """
    def __init__(self):
        # Determine the path to the grammar file relative to this script
        self.grammar_path = os.path.join(os.path.dirname(__file__), "ark.lark")
        try:
            with open(self.grammar_path, "r") as f:
                grammar = f.read()
            # Use LALR parser as in meta/ark.py for consistency
            self.parser = Lark(grammar, start="start", parser="lalr")
            logging.info(f"Loaded grammar from {self.grammar_path}")
        except Exception as e:
            logging.error(f"Failed to load grammar: {e}")
            self.parser = None

    def run(self):
        """
        Main loop: Read headers, read body, handle message.
        """
        logging.info("Ark LSP started")
        while True:
            headers = {}
            try:
                # Read headers (Content-Length: ...)
                while True:
                    line = sys.stdin.buffer.readline()
                    if not line: # EOF
                        return
                    line = line.decode('ascii').strip()
                    if not line: # End of headers (blank line)
                        break
                    parts = line.split(":", 1)
                    if len(parts) == 2:
                        # Normalize header keys to lowercase for case-insensitive lookup
                        headers[parts[0].strip().lower()] = parts[1].strip()

                # Read body based on Content-Length
                if "content-length" in headers:
                    length = int(headers["content-length"])
                    body = sys.stdin.buffer.read(length).decode('utf-8')
                    request = json.loads(body)
                    self.handle_message(request)
            except Exception as e:
                logging.error(f"Error in run loop: {e}")
                # Don't break on errors, try to recover for next message
                pass

    def handle_message(self, msg):
        """
        Dispatch message based on 'method'.
        """
        method = msg.get("method")
        msg_id = msg.get("id")
        params = msg.get("params", {})

        logging.info(f"Received method: {method}")

        if method == "initialize":
            result = {
                "capabilities": {
                    "textDocumentSync": 1 # Full sync
                }
            }
            self.send_response(msg_id, result)

        elif method == "shutdown":
            self.send_response(msg_id, None)

        elif method == "exit":
            sys.exit(0)

        elif method == "textDocument/didOpen":
            self.validate(params["textDocument"]["uri"], params["textDocument"]["text"])

        elif method == "textDocument/didChange":
            # Sync 1 means we get full text in contentChanges[0]['text']
            if params.get("contentChanges"):
                text = params["contentChanges"][0]["text"]
                self.validate(params["textDocument"]["uri"], text)

    def validate(self, uri, text):
        """
        Validate source code using Lark parser and publish diagnostics.
        """
        if not self.parser:
            return

        diagnostics = []
        try:
            self.parser.parse(text)
        except exceptions.UnexpectedToken as e:
            # Line/Col are 1-based in Lark, 0-based in LSP
            line = e.line - 1
            col = e.column - 1
            # Try to determine length of the problematic token
            length = 1
            if hasattr(e.token, "value"):
                length = len(e.token.value)

            diagnostics.append({
                "range": {
                    "start": {"line": line, "character": col},
                    "end": {"line": line, "character": col + length}
                },
                "severity": 1, # Error
                "message": f"Unexpected token: {e.token.value} (Expected: {', '.join(e.expected)})",
                "source": "ark-lsp"
            })
        except exceptions.UnexpectedCharacters as e:
            line = e.line - 1
            col = e.column - 1
            diagnostics.append({
                "range": {
                    "start": {"line": line, "character": col},
                    "end": {"line": line, "character": col + 1}
                },
                "severity": 1,
                "message": f"Unexpected characters: {e.char}",
                "source": "ark-lsp"
            })
        except Exception as e:
            logging.error(f"Validation error: {e}")
            pass

        self.send_notification("textDocument/publishDiagnostics", {
            "uri": uri,
            "diagnostics": diagnostics
        })

    def send_response(self, msg_id, result):
        response = {
            "jsonrpc": "2.0",
            "id": msg_id,
            "result": result
        }
        self.write_message(response)

    def send_notification(self, method, params):
        notification = {
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }
        self.write_message(notification)

    def write_message(self, msg):
        body = json.dumps(msg)
        content_length = len(body.encode('utf-8'))
        response = f"Content-Length: {content_length}\r\n\r\n{body}"
        sys.stdout.buffer.write(response.encode('utf-8'))
        sys.stdout.buffer.flush()

if __name__ == "__main__":
    server = ArkLSP()
    server.run()
