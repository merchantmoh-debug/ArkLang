import os

def build_site():
    # Determine base directories
    script_dir = os.path.dirname(os.path.abspath(__file__))
    # Assuming script is in site/
    site_dir = script_dir
    # Assuming web/ is sibling to site/
    web_dir = os.path.join(os.path.dirname(script_dir), 'web')

    print(f"Building site from {web_dir} to {site_dir}...")

    # 1. Ensure site/js exists
    js_dir = os.path.join(site_dir, 'js')
    os.makedirs(js_dir, exist_ok=True)
    print(f"Ensured {js_dir} exists.")

    # 2. Process index.html
    web_index = os.path.join(web_dir, 'index.html')
    site_index = os.path.join(site_dir, 'index.html')

    if not os.path.exists(web_index):
        print(f"Error: {web_index} does not exist.")
        return

    with open(web_index, 'r', encoding='utf-8') as f:
        content = f.read()

    # Replace script tag
    old_script = '<script src="main.js"></script>'
    new_script = '<script src="js/main.js"></script>'

    if old_script in content:
        content = content.replace(old_script, new_script)
        print(f"Updated script tag in index.html")
    else:
        print(f"WARNING: Could not find '{old_script}' in index.html")

    with open(site_index, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Wrote {site_index}")

    # 3. Process main.js
    web_js = os.path.join(web_dir, 'main.js')
    site_js = os.path.join(js_dir, 'main.js')

    if not os.path.exists(web_js):
        print(f"Error: {web_js} does not exist.")
        return

    with open(web_js, 'r', encoding='utf-8') as f:
        js_content = f.read()

    # Replace WASM path
    old_wasm = "const WASM_PATH = '../target/wasm32-unknown-unknown/release/ark_0_zheng.wasm';"
    new_wasm = "const WASM_PATH = 'wasm/ark.wasm';"

    if old_wasm in js_content:
        js_content = js_content.replace(old_wasm, new_wasm)
        print(f"Updated WASM_PATH in main.js")
    else:
        print(f"WARNING: Could not find WASM_PATH definition in main.js")

    with open(site_js, 'w', encoding='utf-8') as f:
        f.write(js_content)
    print(f"Wrote {site_js}")

    print("Build complete.")

if __name__ == "__main__":
    build_site()
