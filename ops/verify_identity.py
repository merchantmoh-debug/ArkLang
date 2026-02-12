
import os
import requests
import sys

# --- Configuration ---
try:
    from secrets import API_KEY
except ImportError:
    API_KEY = os.environ.get("JULES_API_KEY")
    
JULES_API_URL = "https://jules.googleapis.com/v1alpha"

def verify_identity():
    if not API_KEY:
        print("[FAIL] JULES_API_KEY not found in environment.")
        return

    print(f"[SYSTEM] Verifying Identity with Key: {API_KEY[:4]}...{API_KEY[-4:]}")
    
    headers = {
        "x-goog-api-key": API_KEY,
        "Content-Type": "application/json"
    }

    try:
        url = f"{JULES_API_URL}/sources"
        print(f"[DEBUG] GET {url}")
        resp = requests.get(url, headers=headers)
        
        if resp.status_code == 200:
            data = resp.json()
            sources = data.get('sources', [])
            print(f"[SUCCESS] Identity Verified. Access to {len(sources)} sources.")
            for s in sources:
                print(f"   - {s.get('displayName')} ({s.get('name')})")
        else:
            print(f"[FAIL] HTTP {resp.status_code}: {resp.text}")

    except Exception as e:
        print(f"[ERROR] Connection Failed: {e}")

if __name__ == "__main__":
    verify_identity()
