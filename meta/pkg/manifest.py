import tomllib
import os
from dataclasses import dataclass, field
from typing import Dict

@dataclass
class Manifest:
    path: str
    name: str
    version: str
    description: str
    dependencies: Dict[str, str] = field(default_factory=dict)

    @classmethod
    def load(cls, path: str) -> 'Manifest':
        if not os.path.exists(path):
            raise FileNotFoundError(f"Manifest not found: {path}")

        with open(path, "rb") as f:
            data = tomllib.load(f)

        pkg = data.get("package", {})
        deps = data.get("dependencies", {})

        return cls(
            path=path,
            name=pkg.get("name", "unknown"),
            version=pkg.get("version", "0.1.0"),
            description=pkg.get("description", ""),
            dependencies=deps
        )

    def save(self):
        def escape(s):
            return s.replace('\\', '\\\\').replace('"', '\\"')

        content = "[package]\n"
        content += f'name = "{escape(self.name)}"\n'
        content += f'version = "{escape(self.version)}"\n'
        content += f'description = "{escape(self.description)}"\n\n'

        content += "[dependencies]\n"
        for name, version in self.dependencies.items():
            content += f'{escape(name)} = "{escape(version)}"\n'

        with open(self.path, "w", encoding="utf-8") as f:
            f.write(content)
