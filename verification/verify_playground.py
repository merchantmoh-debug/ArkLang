import os
from playwright.sync_api import sync_playwright, expect

def test_playground():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()

        # 1. Load Page
        page.goto("http://localhost:8000/web/index.html")
        expect(page).to_have_title("Ark Web Playground")

        # 2. Wait for editor
        # CodeMirror content is in .cm-content
        page.wait_for_selector(".cm-content")

        # 3. Click Run
        page.click("#run-btn")

        # 4. Wait for output
        # Look for "Running on Sovereign Runtime..."
        page.wait_for_selector(".log-content:has-text('Running on Sovereign Runtime...')")

        # 5. Look for "Hello, Ark Sovereign World!" (Default code)
        # Note: Ark output might be wrapped in String("...") or raw.
        # Just check for "Hello"
        page.wait_for_selector(".log-content:has-text('Hello')")

        # 6. Screenshot
        os.makedirs("verification", exist_ok=True)
        page.screenshot(path="verification/playground.png")
        print("Screenshot saved to verification/playground.png")

        browser.close()

if __name__ == "__main__":
    try:
        test_playground()
        print("Verification Successful")
    except Exception as e:
        print(f"Verification Failed: {e}")
        exit(1)
