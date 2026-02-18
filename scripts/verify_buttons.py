import os
import subprocess
import time
from playwright.sync_api import sync_playwright

# Start Server (from root)
server = subprocess.Popen(["python3", "-m", "http.server", "8000"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(2)

try:
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        page.goto("http://localhost:8000/site/snake.html")

        # Verify Buttons exist
        if page.is_visible("#btn-pause") and page.is_visible("#btn-restart"):
            print("Buttons found.")
        else:
            print("Buttons NOT found.")
            exit(1)

        os.makedirs("verification", exist_ok=True)
        page.screenshot(path="verification/snake_buttons.png")
        print("Screenshot saved.")
finally:
    server.kill()
