import argparse
import os
import sys
import tarfile
from .manifest import Manifest
from .registry import Registry

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
    # lib_dir relative to where command is run? Or relative to ark.toml?
    # Assuming current working directory which should contain ark.toml

    try:
        registry.install(args.name, lib_dir)
        # Assuming install succeeds, update manifest
        manifest.dependencies[args.name] = "*"
        manifest.save()
        print(f"Added {args.name} to dependencies.")
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
        print("[dependencies]")
        for name, version in manifest.dependencies.items():
            print(f"{name} = \"{version}\"")
    except Exception as e:
        print(f"Error: {e}")

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
                # Exclude .git, __pycache__
                if ".git" in dirs:
                    dirs.remove(".git")
                if "__pycache__" in dirs:
                    dirs.remove("__pycache__")

                for file in files:
                    if file == filename:
                        continue
                    if file.endswith(".pyc"):
                        continue

                    full_path = os.path.join(root, file)
                    tar.add(full_path, arcname=full_path)

        print(f"Published to {filename}")

    except Exception as e:
        print(f"Publish failed: {e}")
        sys.exit(1)

def cmd_search(args):
    registry = Registry()
    registry.search(args.query)

def main():
    parser = argparse.ArgumentParser(description="Ark Package Manager")
    subparsers = parser.add_subparsers(dest="command", help="Command to execute")

    # Init
    parser_init = subparsers.add_parser("init", help="Initialize a new package")
    parser_init.add_argument("path", nargs="?", default=".", help="Directory to initialize")

    # Install
    parser_install = subparsers.add_parser("install", help="Install a package")
    parser_install.add_argument("name", help="Package name")

    # List
    parser_list = subparsers.add_parser("list", help="List dependencies")

    # Publish
    parser_publish = subparsers.add_parser("publish", help="Publish package to tarball")

    # Search
    parser_search = subparsers.add_parser("search", help="Search for a package")
    parser_search.add_argument("query", help="Search query")

    args = parser.parse_args()

    if args.command == "init":
        cmd_init(args)
    elif args.command == "install":
        cmd_install(args)
    elif args.command == "list":
        cmd_list(args)
    elif args.command == "publish":
        cmd_publish(args)
    elif args.command == "search":
        cmd_search(args)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
