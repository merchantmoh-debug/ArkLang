#!/usr/bin/env python3
"""
Ark Documentation Generator (meta/docgen.py)
Parses Ark intrinsics and standard library to generate API documentation.

Usage:
    python meta/docgen.py [--output-dir docs] [--format md|html]
"""
import sys
import os
import re
import argparse
import inspect
import json
from collections import defaultdict

# Ensure repo root is in sys.path
REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
if REPO_ROOT not in sys.path:
    sys.path.append(REPO_ROOT)

# Mock cryptography if missing, as we only need to inspect functions
try:
    import cryptography
except ImportError:
    from unittest.mock import MagicMock
    sys.modules["cryptography"] = MagicMock()
    sys.modules["cryptography.hazmat"] = MagicMock()
    sys.modules["cryptography.hazmat.primitives"] = MagicMock()
    sys.modules["cryptography.hazmat.primitives.asymmetric"] = MagicMock()
    sys.modules["cryptography.hazmat.primitives.ciphers"] = MagicMock()
    sys.modules["cryptography.hazmat.primitives.ciphers.aead"] = MagicMock()
    sys.modules["cryptography.hazmat.primitives.serialization"] = MagicMock()

try:
    from meta.ark_intrinsics import INTRINSICS
except ImportError as e:
    print(f"Error: Could not import meta.ark_intrinsics: {e}", file=sys.stderr)
    import traceback
    traceback.print_exc()
    sys.exit(1)

class DocGenerator:
    def __init__(self, output_dir="docs", format="md"):
        self.output_dir = output_dir
        self.format = format
        self.intrinsics_data = defaultdict(list)
        self.stdlib_data = defaultdict(list)

    def parse_intrinsics(self):
        """Parses intrinsics from meta.ark_intrinsics."""
        print(f"Parsing {len(INTRINSICS)} intrinsics...")
        for name, func in INTRINSICS.items():
            # Determine Category
            category = "core"

            if name.startswith("sys."):
                parts = name.split(".")
                if len(parts) > 2:
                    # e.g. sys.fs.read -> fs
                    category = parts[1]
                else:
                    # e.g. sys.exec, sys.exit -> sys
                    category = "sys"
            elif name.startswith("math."):
                # e.g. math.add -> math
                category = "math"
            elif name.startswith("intrinsic_"):
                # e.g. intrinsic_math_add -> math
                parts = name.split("_")
                if len(parts) > 1:
                    if parts[1] == "math":
                        category = "math"
                    else:
                        category = "core"
            elif "." in name:
                # Fallback for other dotted names
                category = name.split(".")[0]

            doc = inspect.getdoc(func) or "No description available."
            try:
                sig = inspect.signature(func)
                # Count params, excluding 'args' list wrapper if it's the only one
                params = str(sig)
                num_params = len(sig.parameters)
                # Most intrinsics take a single List[ArkValue] named args
                if num_params == 1 and "args" in sig.parameters:
                    # We can't easily know exact Ark args from Python signature alone for these generic handlers
                    # so we rely on docstring or just list it as (...)
                    params = "(...)"
            except ValueError:
                params = "(?)"

            self.intrinsics_data[category].append({
                "name": name,
                "params": params,
                "doc": doc
            })

    def parse_stdlib(self, stdlib_path="lib/std"):
        """Parses standard library .ark files."""
        if not os.path.exists(stdlib_path):
            print(f"Warning: Standard library path '{stdlib_path}' not found.")
            return

        print(f"Parsing standard library from {stdlib_path}...")
        for filename in os.listdir(stdlib_path):
            if not filename.endswith(".ark"):
                continue

            module_name = filename[:-4]
            filepath = os.path.join(stdlib_path, filename)

            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()

            # Regex to find functions: func name(args) {
            # Also captures preceding comments
            pattern = re.compile(r"((?://[^\n]*\n)*)\s*func\s+(\w+)\s*\(([^)]*)\)", re.MULTILINE)

            for match in pattern.finditer(content):
                comment_block, func_name, args = match.groups()

                # Clean up comments
                doc = ""
                if comment_block:
                    lines = [line.strip().lstrip("/").strip() for line in comment_block.strip().split("\n")]
                    doc = "\n".join(lines)

                if not doc:
                    doc = "No description available."

                self.stdlib_data[module_name].append({
                    "name": func_name,
                    "args": args.strip(),
                    "doc": doc
                })

    def _generate_markdown_content(self, title, data, is_stdlib=False):
        lines = [f"# {title}", ""]

        # Generate Table of Contents
        lines.append("## Table of Contents")
        sorted_categories = sorted(data.keys())
        for category in sorted_categories:
            lines.append(f"- [{category.capitalize()}](#{category.lower()})")
        lines.append("")

        for category, items in sorted(data.items()):
            lines.append(f"## {category.capitalize()}")
            for item in sorted(items, key=lambda x: x['name']):
                name = item['name']
                doc = item['doc']

                if is_stdlib:
                    args = item['args']
                    lines.append(f"### `{name}({args})`")
                else:
                    # Intrinsic
                    lines.append(f"### `{name}`")

                lines.append(f"{doc}\n")

                # Auto-generate example
                lines.append("```ark")
                if is_stdlib:
                    lines.append(f"// Example for {name}")
                    args_list = item['args'].split(",") if item['args'] else []
                    clean_args = [a.strip() for a in args_list]
                    dummy_args = ", ".join(["..." for _ in clean_args])
                    lines.append(f"{name}({dummy_args})")
                else:
                    lines.append(f"// Example for {name}")
                    lines.append(f"{name}(...)")
                lines.append("```\n")

        return "\n".join(lines)

    def generate_api_docs(self):
        """Generates docs/API_REFERENCE.md"""
        content = self._generate_markdown_content("Ark API Reference (Intrinsics)", self.intrinsics_data)
        self._write_file("API_REFERENCE", content)

    def generate_stdlib_docs(self):
        """Generates docs/STDLIB_REFERENCE.md"""
        content = self._generate_markdown_content("Ark Standard Library Reference", self.stdlib_data, is_stdlib=True)
        self._write_file("STDLIB_REFERENCE", content)

    def generate_index_html(self):
        """Generates docs/index.html if format is html"""
        if self.format != "html":
            return

        content = """# Ark Documentation

## Sections
- [Quick Start](QUICK_START.html)
- [API Reference (Intrinsics)](API_REFERENCE.html)
- [Standard Library Reference](STDLIB_REFERENCE.html)

## About
Generated automatically by `meta/docgen.py`.
"""
        self._write_file("index", content)

    def generate_quick_start(self):
        """Generates docs/QUICK_START.md"""
        content = """# Ark Quick Start

## Installation

Ark is a Python-based language. To run Ark, you need Python 3.10+.

1. Clone the repository:
   ```bash
   git clone https://github.com/your-repo/ark.git
   cd ark
   ```

2. Run the Ark CLI:
   ```bash
   python meta/ark.py --help
   ```

## Hello World

Create a file named `hello.ark`:

```ark
print("Hello, World!")
```

Run it:

```bash
python meta/ark.py run hello.ark
```

## Basic Syntax

### Variables
Ark uses `:=` for assignment (reassignment is allowed with `:=`).

```ark
x := 10
y := "Ark"
```

### Functions

```ark
func add(a, b) {
    return a + b
}
```

### Control Flow

```ark
if x > 5 {
    print("Big")
} else {
    print("Small")
}

i := 0
while i < 10 {
    print(i)
    i := i + 1
}
```
"""
        self._write_file("QUICK_START", content)

    def _write_file(self, name, content):
        if not os.path.exists(self.output_dir):
            os.makedirs(self.output_dir)

        filename = f"{name}.{self.format}"
        filepath = os.path.join(self.output_dir, filename)

        if self.format == "html":
            # HTML wrapper with highlight.js
            html_content = f"""<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<title>{name}</title>
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/styles/atom-one-dark.min.css">
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script>hljs.highlightAll();</script>
<style>
body {{ font-family: sans-serif; line-height: 1.6; padding: 20px; max_width: 800px; margin: 0 auto; background: #1e1e1e; color: #d4d4d4; }}
h1, h2, h3 {{ color: #569cd6; }}
code {{ background: #2d2d2d; padding: 2px 5px; border_radius: 3px; font-family: monospace; }}
pre {{ background: #2d2d2d; padding: 10px; border_radius: 5px; overflow-x: auto; }}
pre code {{ background: none; padding: 0; }}
a {{ color: #9cdcfe; }}
</style>
</head>
<body>
<nav>
<a href="index.html">Index</a> |
<a href="API_REFERENCE.html">API Reference</a> |
<a href="STDLIB_REFERENCE.html">StdLib Reference</a> |
<a href="QUICK_START.html">Quick Start</a>
</nav>
<hr>
{self._markdown_to_html(content)}
</body>
</html>"""
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(html_content)
        else:
            with open(filepath, "w", encoding="utf-8") as f:
                f.write(content)

        print(f"Generated {filepath}")

    def _markdown_to_html(self, md_content):
        # Very basic Markdown to HTML converter for dependency-free operation
        html = md_content
        html = html.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;")

        # Headers with IDs for TOC
        def header_replace(match):
            level = len(match.group(1))
            title = match.group(2).strip()
            slug = title.lower().replace(" ", "-").replace(".", "").replace("(", "").replace(")", "")
            return f'<h{level} id="{slug}">{title}</h{level}>'

        html = re.sub(r"^(#{1,3})\s+(.*)$", header_replace, html, flags=re.MULTILINE)

        # Links: [Text](url)
        html = re.sub(r"\[([^\]]+)\]\(([^)]+)\)", r'<a href="\2">\1</a>', html)

        # Code blocks
        html = re.sub(r"```.*?\n(.*?)```", r"<pre><code>\1</code></pre>", html, flags=re.DOTALL)

        # Inline code
        html = re.sub(r"`([^`]+)`", r"<code>\1</code>", html)

        # Paragraphs (simple)
        lines = html.split('\n')
        new_lines = []
        in_pre = False
        for line in lines:
            if "<pre>" in line: in_pre = True
            if "</pre>" in line:
                in_pre = False
                new_lines.append(line) # append closing pre line
                continue

            if in_pre:
                new_lines.append(line)
            elif not line.strip().startswith("<") and line.strip():
                new_lines.append(f"<p>{line}</p>")
            else:
                new_lines.append(line)

        return "\n".join(new_lines)

    def run(self):
        self.parse_intrinsics()
        self.parse_stdlib()
        self.generate_api_docs()
        self.generate_stdlib_docs()
        self.generate_quick_start()
        self.generate_index_html()

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Ark Documentation Generator")
    parser.add_argument("--output-dir", default="docs", help="Output directory")
    parser.add_argument("--format", choices=["md", "html"], default="md", help="Output format")
    args = parser.parse_args()

    gen = DocGenerator(output_dir=args.output_dir, format=args.format)
    gen.run()
