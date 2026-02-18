import os
import subprocess
import time
from playwright.sync_api import sync_playwright

# Start Server
server = subprocess.Popen(["python3", "-m", "http.server", "8000"], cwd="site", stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(2) # Wait for server

try:
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Capture logs
        page.on("console", lambda msg: print(f"Console: {msg.text}"))
        page.on("pageerror", lambda err: print(f"Page Error: {err}"))

        # Navigate
        page.goto("http://localhost:8000/index.html")

        # Wait for loading to finish (Status text changes)
        try:
            page.wait_for_selector("text=Running", timeout=5000)
        except:
            print("Timeout waiting for 'Running' status. Taking screenshot of error.")

        # Take screenshot
        os.makedirs("/home/jules/verification", exist_ok=True)
        page.screenshot(path="/home/jules/verification/snake_verification.png")

        browser.close()
finally:
    server.kill()
