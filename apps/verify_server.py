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
