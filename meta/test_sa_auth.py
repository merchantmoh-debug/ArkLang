
import os
import json
import requests
from google.oauth2 import service_account
from google.auth.transport.requests import Request

# Path to the service account key in the brain directory
KEY_PATH = r"c:\Users\Stran\.gemini\antigravity\brain\87a06051-a8dc-48c3-9d36-cf0f67b80b77\gcloud_credentials.json"
JULES_API_URL = "https://jules.googleapis.com/v1alpha/sources"

def test_sa_auth():
    print("[TEST] Attempting Service Account Authentication for Jules API...")
    
    if not os.path.exists(KEY_PATH):
        print(f"[ERROR] Key file not found: {KEY_PATH}")
        return

    try:
        # Load credentials with cloud-platform scope (broadest)
        creds = service_account.Credentials.from_service_account_file(
            KEY_PATH, 
            scopes=["https://www.googleapis.com/auth/cloud-platform"]
        )
        
        # Refresh to get token
        creds.refresh(Request())
        token = creds.token
        print(f"[INFO] OAuth2 Token Retrieved: {token[:10]}...")
        
        # Make request with Bearer token
        headers = {
            "Authorization": f"Bearer {token}",
            "Content-Type": "application/json"
        }
        
        resp = requests.get(JULES_API_URL, headers=headers)
        
        if resp.status_code == 200:
            print("[SUCCESS] Service Account Authenticated!")
            print(json.dumps(resp.json(), indent=2))
        else:
            print(f"[FAILURE] Status: {resp.status_code}")
            print(f"Response: {resp.text}")
            
    except Exception as e:
        print(f"[ERROR] Auth failed: {e}")

if __name__ == "__main__":
    test_sa_auth()
