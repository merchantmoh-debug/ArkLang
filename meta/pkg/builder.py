import os
from .manifest import Manifest

class Builder:
    def __init__(self, manifest: Manifest):
        self.manifest = manifest
        self.base_dir = os.path.dirname(os.path.abspath(manifest.path))

    def build(self):
        if not self.manifest.build:
            print("No build configuration found.")
            return

        print(f"Building {self.manifest.package.name} v{self.manifest.package.version}...")

        sources = self.manifest.build.sources
        output_file = self.manifest.build.output

        full_source = f"// Package: {self.manifest.package.name}\n"
        full_source += f"// Version: {self.manifest.package.version}\n\n"

        for src in sources:
            src_path = os.path.join(self.base_dir, src)
            if not os.path.exists(src_path):
                raise FileNotFoundError(f"Source file not found: {src_path}")

            print(f"  + Ingesting {src}...")
            with open(src_path, 'r', encoding='utf-8') as f:
                content = f.read()
                full_source += f"// --- COMPONENT: {src} ---\n"
                full_source += content + "\n\n"

        out_path = os.path.join(self.base_dir, output_file)
        with open(out_path, 'w', encoding='utf-8') as f:
            f.write(full_source)

        print(f"Build complete: {out_path}")
