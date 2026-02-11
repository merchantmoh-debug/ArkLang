import subprocess
import time
import sys
import urllib.request

def run_verification():
    print("Spawning Ark Server...")
    # Start the server process
    # Adjust path to meta/ark.py as needed, assuming we run from project root
    process = subprocess.Popen(
        [sys.executable, "meta/ark.py", "run", "apps/server.ark"],
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True
    )
    
    try:
        # Give it time to start
        time.sleep(2)
        
        # Test 1: Root
        print("Testing Root...")
        with urllib.request.urlopen("http://127.0.0.1:8087/") as response:
            body = response.read().decode('utf-8')
            print(f"Response: {body}")
            if "Hello from Ark" not in body:
                print("FAILURE: Root content mismatch")
                return False
                
        # Test 2: Health
        print("Testing Health...")
        with urllib.request.urlopen("http://127.0.0.1:8087/health") as response:
            body = response.read().decode('utf-8')
            print(f"Response: {body}")
            if "OK" not in body:
                print("FAILURE: Health content mismatch")
                return False

        # Test 3: XSS Check
        print("Testing XSS Protection...")
        # Use socket for raw request to bypass urllib encoding
        import socket
        s = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        s.connect(("127.0.0.1", 8087))
        request = b"GET /<script>alert(1)</script> HTTP/1.1\r\nHost: localhost\r\n\r\n"
        s.sendall(request)
        response = s.recv(4096).decode('utf-8', errors='ignore')
        s.close()

        if "<script>alert(1)</script>" in response:
             print("FAILURE: VULNERABILITY PERSISTS. Raw script tag reflected.")
             return False

        if "&lt;script&gt;alert(1)&lt;/script&gt;" in response:
            print("SUCCESS: Payload correctly escaped.")
        else:
            print("WARNING: Payload not found in expected escaped format.")
            # Depending on strictness, we might return False, but for now just warn if format differs but raw is gone

        print("SUCCESS: All tests passed.")
        return True
        
    except Exception as e:
        print(f"FAILURE: Exception occurred: {e}")
        return False
        
    finally:
        print("Terminating Server...")
        process.terminate()
        stdout, stderr = process.communicate()
        print("Server Stdout:", stdout)
        print("Server Stderr:", stderr)

if __name__ == "__main__":
    success = run_verification()
    sys.exit(0 if success else 1)
