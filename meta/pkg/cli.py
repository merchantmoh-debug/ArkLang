"""
Ark Package Manager CLI.

Commands:
    ark-pkg init [path]     Initialize a new package (creates ark.toml)
    ark-pkg install <name>  Install a package with version resolution
    ark-pkg list            List project dependencies
    ark-pkg search <query>  Search the package index
    ark-pkg publish         Create a distributable tarball
    ark-pkg update          Re-resolve all dependencies to latest compatible
    ark-pkg outdated        Show packages with newer versions available
"""

import argparse
import os
import sys
import tarfile
from .manifest import Manifest
from .registry import Registry
from .lockfile import Lockfile


def cmd_init(args):
    target_dir = os.path.abspath(args.path)
    if not os.path.exists(target_dir):
        os.makedirs(target_dir)

    path = os.path.join(target_dir, "ark.toml")
    if os.path.exists(path):
        print(f"Error: {path} already exists.")
        return

    name = os.path.basename(target_dir) or "unknown"
    version = "0.1.0"
    description = f"Package {name}"

    manifest = Manifest(
        path=path,
        name=name,
        version=version,
        description=description,
        dependencies={}
    )
    manifest.save()
    print(f"Initialized package in {path}")


def cmd_install(args):
    registry = Registry()

    manifest_path = "ark.toml"
    if not os.path.exists(manifest_path):
        print("Error: ark.toml not found. Run 'ark-pkg init' first.")
        return

    try:
        manifest = Manifest.load(manifest_path)
    except Exception as e:
        print(f"Error loading manifest: {e}")
        return

    lib_dir = "lib"

    # Load or create lockfile
    lockfile = Lockfile.load("ark.lock") or Lockfile()

    try:
        # Resolve version constraint from manifest or CLI
        constraint = manifest.dependencies.get(args.name, "*")
        resolved_version, content = registry.install(args.name, lib_dir, version=constraint)

        # Update manifest with resolved constraint
        manifest.dependencies[args.name] = f"^{resolved_version}"
        manifest.save()

        # Update lockfile with integrity hash
        source_url = registry.PACKAGE_URL.format(name=args.name, version=resolved_version)
        integrity = Lockfile.compute_integrity(content)
        lockfile.update_package(args.name, resolved_version, source_url, integrity)
        lockfile.save()

        print(f"Added {args.name}@{resolved_version} to dependencies.")
        print(f"Lockfile updated: ark.lock")
    except Exception as e:
        print(f"Install failed: {e}")
        sys.exit(1)


def cmd_list(args):
    manifest_path = "ark.toml"
    if not os.path.exists(manifest_path):
        print("Error: ark.toml not found.")
        return

    try:
        manifest = Manifest.load(manifest_path)
    except Exception as e:
        print(f"Error: {e}")
        return

    lockfile = Lockfile.load("ark.lock")

    print(f"[{manifest.name} v{manifest.version}]")
    print()

    if not manifest.dependencies:
        print("  (no dependencies)")
        return

    print("  Dependencies:")
    for name, constraint in manifest.dependencies.items():
        locked = ""
        if lockfile:
            locked_ver = lockfile.get_locked_version(name)
            if locked_ver:
                locked = f" (locked: {locked_ver})"
        print(f"    {name} = \"{constraint}\"{locked}")


def cmd_publish(args):
    manifest_path = "ark.toml"
    if not os.path.exists(manifest_path):
        print("Error: ark.toml not found.")
        return

    try:
        manifest = Manifest.load(manifest_path)
        name = manifest.name
        version = manifest.version
        filename = f"{name}-{version}.tar.gz"

        with tarfile.open(filename, "w:gz") as tar:
            for root, dirs, files in os.walk("."):
                # Exclude common build/vcs directories
                for exclude in (".git", "__pycache__", "lib", ".ark_cache"):
                    if exclude in dirs:
                        dirs.remove(exclude)

                for file in files:
                    if file == filename:
                        continue
                    if file.endswith(".pyc"):
                        continue

                    full_path = os.path.join(root, file)
                    tar.add(full_path, arcname=full_path)

        print(f"Published to {filename}")
        print(f"  Package: {name}")
        print(f"  Version: {version}")

    except Exception as e:
        print(f"Publish failed: {e}")
        sys.exit(1)


def cmd_search(args):
    registry = Registry()
    results = registry.search(args.query)

    if not results:
        print(f"No packages found matching '{args.query}'.")
        return

    print(f"Found {len(results)} package(s):")
    print()
    for pkg in results:
        versions_str = ", ".join(pkg["versions"][-3:]) if pkg["versions"] else "?"
        print(f"  {pkg['name']} ({pkg['latest']})")
        print(f"    {pkg['description']}")
        print(f"    versions: [{versions_str}]")
        print()


def cmd_update(args):
    """Re-resolve all dependencies to latest compatible versions."""
    manifest_path = "ark.toml"
    if not os.path.exists(manifest_path):
        print("Error: ark.toml not found.")
        return

    try:
        manifest = Manifest.load(manifest_path)
    except Exception as e:
        print(f"Error: {e}")
        return

    if not manifest.dependencies:
        print("No dependencies to update.")
        return

    registry = Registry()
    lockfile = Lockfile.load("ark.lock") or Lockfile()
    lib_dir = "lib"
    updated = 0

    for name, constraint in manifest.dependencies.items():
        old_version = lockfile.get_locked_version(name)
        new_version = registry.resolve_version(name, constraint)

        if new_version is None:
            print(f"  {name}: could not resolve '{constraint}'")
            continue

        if old_version == new_version:
            print(f"  {name}: up-to-date ({new_version})")
            continue

        try:
            resolved, content = registry.install(name, lib_dir, version=constraint)
            source_url = registry.PACKAGE_URL.format(name=name, version=resolved)
            integrity = Lockfile.compute_integrity(content)
            lockfile.update_package(name, resolved, source_url, integrity)
            print(f"  {name}: {old_version or '(new)'} → {resolved}")
            updated += 1
        except Exception as e:
            print(f"  {name}: update failed — {e}")

    lockfile.save()
    print(f"\nUpdated {updated} package(s). Lockfile saved.")


def cmd_outdated(args):
    """Show packages with newer versions available."""
    manifest_path = "ark.toml"
    if not os.path.exists(manifest_path):
        print("Error: ark.toml not found.")
        return

    try:
        manifest = Manifest.load(manifest_path)
    except Exception as e:
        print(f"Error: {e}")
        return

    if not manifest.dependencies:
        print("No dependencies.")
        return

    registry = Registry()
    lockfile = Lockfile.load("ark.lock")

    print(f"{'Package':<20} {'Locked':<12} {'Latest':<12} {'Constraint':<12}")
    print("-" * 56)

    outdated_count = 0
    index_packages = registry.list_all()

    for name, constraint in manifest.dependencies.items():
        locked_ver = "—"
        if lockfile:
            locked_ver = lockfile.get_locked_version(name) or "—"

        latest = "?"
        if name in index_packages:
            latest = index_packages[name].get("latest", "?")

        is_outdated = locked_ver != "—" and latest != "?" and locked_ver != latest
        marker = " ← update available" if is_outdated else ""
        if is_outdated:
            outdated_count += 1

        print(f"  {name:<18} {locked_ver:<12} {latest:<12} {constraint:<12}{marker}")

    if outdated_count:
        print(f"\n{outdated_count} package(s) can be updated. Run 'ark-pkg update'.")
    else:
        print("\nAll packages up-to-date.")


def main():
    parser = argparse.ArgumentParser(
        description="Ark Package Manager",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="Use 'ark-pkg <command> --help' for more info on a command."
    )
    subparsers = parser.add_subparsers(dest="command", help="Command to execute")

    # Init
    parser_init = subparsers.add_parser("init", help="Initialize a new package")
    parser_init.add_argument("path", nargs="?", default=".", help="Directory to initialize")

    # Install
    parser_install = subparsers.add_parser("install", help="Install a package")
    parser_install.add_argument("name", help="Package name")

    # List
    subparsers.add_parser("list", help="List dependencies")

    # Publish
    subparsers.add_parser("publish", help="Publish package to tarball")

    # Search
    parser_search = subparsers.add_parser("search", help="Search for a package")
    parser_search.add_argument("query", help="Search query")

    # Update
    subparsers.add_parser("update", help="Re-resolve all dependencies to latest")

    # Outdated
    subparsers.add_parser("outdated", help="Show packages with newer versions")

    args = parser.parse_args()

    commands = {
        "init": cmd_init,
        "install": cmd_install,
        "list": cmd_list,
        "publish": cmd_publish,
        "search": cmd_search,
        "update": cmd_update,
        "outdated": cmd_outdated,
    }

    handler = commands.get(args.command)
    if handler:
        handler(args)
    else:
        parser.print_help()


if __name__ == "__main__":
    main()
