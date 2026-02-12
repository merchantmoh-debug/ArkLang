import json
import os
from dataclasses import dataclass, field
from typing import List, Optional

@dataclass
class PackageConfig:
    name: str
    version: str
    authors: List[str] = field(default_factory=list)
    type: str = "app"

@dataclass
class BuildConfig:
    sources: List[str]
    output: str

@dataclass
class Manifest:
    package: PackageConfig
    build: Optional[BuildConfig] = None
    path: str = ""

    @classmethod
    def load(cls, path: str) -> 'Manifest':
        if not os.path.exists(path):
            raise FileNotFoundError(f"Manifest not found: {path}")

        with open(path, 'r') as f:
            try:
                data = json.load(f)
            except json.JSONDecodeError as e:
                raise ValueError(f"Invalid JSON in manifest {path}: {e}")

        pkg_data = data.get("package", {})
        if not pkg_data:
            raise ValueError("Manifest missing 'package' section")

        package = PackageConfig(
            name=pkg_data.get("name"),
            version=pkg_data.get("version"),
            authors=pkg_data.get("authors", []),
            type=pkg_data.get("type", "app")
        )

        build = None
        build_data = data.get("build")
        if build_data:
            build = BuildConfig(
                sources=build_data.get("sources", []),
                output=build_data.get("output")
            )

        return cls(package=package, build=build, path=path)

    def save(self):
        data = {
            "package": {
                "name": self.package.name,
                "version": self.package.version,
                "authors": self.package.authors,
                "type": self.package.type
            }
        }
        if self.build:
            data["build"] = {
                "sources": self.build.sources,
                "output": self.build.output
            }

        with open(self.path, 'w') as f:
            json.dump(data, f, indent=2)
