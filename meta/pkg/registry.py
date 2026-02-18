"""
Ark Package Registry — Fetches packages with version resolution.

Resolves versions against a static index (index.json) and downloads
specific versioned releases. Integrates with lockfile for reproducibility.
"""

import os
import re
import json
import urllib.request
import urllib.error
from typing import Optional, List, Dict, Any
from .lockfile import Lockfile


class Registry:
    INDEX_URL = "https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/index.json"
    PACKAGE_URL = "https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/{name}/{version}/lib.ark"
    # Fallback for unversioned packages (legacy)
    LEGACY_URL = "https://raw.githubusercontent.com/merchantmoh-debug/ark-packages/main/{name}/lib.ark"

    def __init__(self):
        self._index_cache: Optional[Dict[str, Any]] = None

    def fetch_index(self) -> Dict[str, Any]:
        """Fetch and cache the package index."""
        if self._index_cache is not None:
            return self._index_cache

        # Try remote first
        try:
            with urllib.request.urlopen(self.INDEX_URL, timeout=10) as response:
                self._index_cache = json.loads(response.read().decode())
                return self._index_cache
        except Exception:
            pass

        # Fallback: load local index.json bundled with the CLI
        local_index = os.path.join(os.path.dirname(__file__), "index.json")
        if os.path.exists(local_index):
            with open(local_index, "r", encoding="utf-8") as f:
                self._index_cache = json.load(f)
                return self._index_cache

        # No index available — return empty
        return {"packages": {}}

    def resolve_version(self, name: str, constraint: str = "*") -> Optional[str]:
        """
        Resolve a version constraint against the index.

        Constraints:
          "*"       → latest
          "0.2.0"   → exact
          ">=0.2.0" → minimum version
          "^0.2.0"  → compatible (same major, >= specified)
        """
        index = self.fetch_index()
        packages = index.get("packages", {})

        if name not in packages:
            return None

        versions = packages[name].get("versions", [])
        if not versions:
            return None

        if constraint == "*" or constraint == "latest":
            return packages[name].get("latest", versions[-1])

        # Exact version
        if constraint in versions:
            return constraint

        # >= constraint
        if constraint.startswith(">="):
            min_ver = constraint[2:]
            candidates = [v for v in versions if _compare_versions(v, min_ver) >= 0]
            return candidates[-1] if candidates else None

        # ^ (caret) constraint — same major version
        if constraint.startswith("^"):
            base = constraint[1:]
            major = base.split(".")[0]
            candidates = [v for v in versions
                         if v.split(".")[0] == major
                         and _compare_versions(v, base) >= 0]
            return candidates[-1] if candidates else None

        return None

    def install(self, name: str, lib_dir: str, version: str = "*") -> tuple:
        """
        Install a package with version resolution.

        Returns (version, content_bytes) on success.
        """
        if not re.match(r"^[a-zA-Z0-9_-]+$", name):
            raise ValueError(f"Invalid package name: '{name}'. Only alphanumeric, underscores, and hyphens allowed.")

        # Resolve version
        resolved = self.resolve_version(name, version)
        if resolved is None:
            # Try legacy (unversioned) fallback
            resolved = "0.0.0"

        target_path = os.path.join(lib_dir, name, "lib.ark")

        if not os.path.exists(os.path.dirname(target_path)):
            os.makedirs(os.path.dirname(target_path))

        # Try versioned URL first
        url = self.PACKAGE_URL.format(name=name, version=resolved)
        content = None

        print(f"Resolving {name}@{version} → {resolved}")
        try:
            with urllib.request.urlopen(url) as response:
                content = response.read()
        except urllib.error.HTTPError:
            # Fallback to legacy URL
            url = self.LEGACY_URL.format(name=name)
            try:
                with urllib.request.urlopen(url) as response:
                    content = response.read()
            except urllib.error.HTTPError as e:
                self._cleanup_dir(os.path.dirname(target_path))
                if e.code == 404:
                    raise ValueError(f"Package '{name}' not found at registry.")
                else:
                    raise RuntimeError(f"Error downloading package: {e}")
            except Exception:
                self._cleanup_dir(os.path.dirname(target_path))
                raise

        with open(target_path, "wb") as f:
            f.write(content)

        print(f"Installed {name}@{resolved} to {target_path}")
        return resolved, content

    def search(self, query: str) -> List[Dict[str, Any]]:
        """Search packages from the index."""
        index = self.fetch_index()
        packages = index.get("packages", {})
        results = []

        for name, info in packages.items():
            if query.lower() in name.lower() or query.lower() in info.get("description", "").lower():
                results.append({
                    "name": name,
                    "latest": info.get("latest", "?"),
                    "versions": info.get("versions", []),
                    "description": info.get("description", ""),
                    "author": info.get("author", "unknown"),
                })

        if not results:
            # Fallback: check if exact package exists via HEAD request
            if re.match(r"^[a-zA-Z0-9_-]+$", query):
                url = self.LEGACY_URL.format(name=query)
                try:
                    request = urllib.request.Request(url, method="HEAD")
                    with urllib.request.urlopen(request) as response:
                        if response.status == 200:
                            results.append({
                                "name": query,
                                "latest": "unknown",
                                "versions": [],
                                "description": "(unlisted package)",
                                "author": "unknown",
                            })
                except Exception:
                    pass

        return results

    def list_all(self) -> Dict[str, Dict[str, Any]]:
        """List all available packages from the index."""
        index = self.fetch_index()
        return index.get("packages", {})

    @staticmethod
    def _cleanup_dir(dirname: str):
        """Remove empty directory after failed install."""
        if os.path.exists(dirname) and not os.listdir(dirname):
            os.rmdir(dirname)


def _compare_versions(a: str, b: str) -> int:
    """
    Compare semver strings. Returns:
      -1 if a < b
       0 if a == b
       1 if a > b
    """
    def to_tuple(v: str):
        parts = v.split(".")
        return tuple(int(p) for p in parts if p.isdigit())

    ta, tb = to_tuple(a), to_tuple(b)
    if ta < tb:
        return -1
    elif ta > tb:
        return 1
    return 0
