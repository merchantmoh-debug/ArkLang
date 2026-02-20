from playwright.sync_api import sync_playwright
import os

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # Load the local HTML file
        # We need absolute path
        cwd = os.getcwd()
        path = f"file://{cwd}/web/index.html"
        print(f"Loading {path}")
        page.goto(path)

        # Check if button exists
        btn = page.locator("#sovereign-btn")
        if btn.is_visible():
            print("[+] Sovereign Button found.")
        else:
            print("[-] Sovereign Button NOT found.")
            browser.close()
            return

        # Click it
        btn.click()

        # Check for class
        # Note: JS modules might not load via file:// due to CORS policies in some browsers,
        # but Playwright usually handles local files okay if not fetching external modules.
        # However, main.js is a module: <script type="module" src="main.js"></script>
        # This will fail on file:// without a server.

        # Let's take a screenshot anyway to see the button.
        page.screenshot(path="verification_screenshot.png")
        print("Screenshot saved to verification_screenshot.png")

        browser.close()

if __name__ == "__main__":
    run()
