from playwright.sync_api import sync_playwright
import time

def run():
    with sync_playwright() as p:
        browser = p.chromium.launch(headless=True)
        page = browser.new_page()
        try:
            print("Navigating to Ark Web Interface...")
            page.goto("http://localhost:8000/web/index.html")

            # Wait for System Health Indicator
            print("Waiting for System Health...")
            page.wait_for_selector(".status-left span:has-text('SYSTEM STATUS')", timeout=10000)

            # Wait for Neural Canvas
            print("Waiting for Neural Canvas...")
            page.wait_for_selector("#neural-canvas")

            # Wait a bit for the graph to draw some data
            time.sleep(2)

            # Take screenshot
            path = "verification_screenshot.png"
            page.screenshot(path=path)
            print(f"Screenshot saved to {path}")

        except Exception as e:
            print(f"Verification failed: {e}")
        finally:
            browser.close()

if __name__ == "__main__":
    run()
