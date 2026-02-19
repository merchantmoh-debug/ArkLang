from playwright.sync_api import sync_playwright
import os

def run(playwright):
    browser = playwright.chromium.launch()
    page = browser.new_page()
    page.goto("http://localhost:8000/web/index.html")

    # Wait for canvas to be visible
    page.wait_for_selector("#neural-canvas")

    # Take screenshot
    os.makedirs("/home/jules/verification", exist_ok=True)
    page.screenshot(path="/home/jules/verification/neural_ui.png")

    browser.close()

with sync_playwright() as playwright:
    run(playwright)
