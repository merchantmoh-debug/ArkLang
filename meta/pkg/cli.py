import argparse
import os
import sys
from .manifest import Manifest, PackageConfig, BuildConfig
from .builder import Builder

def cmd_init(args):
    target_dir = os.path.abspath(args.path)
    if not os.path.exists(target_dir):
        os.makedirs(target_dir)

    path = os.path.join(target_dir, "ark.json")
    if os.path.exists(path):
        print(f"Error: {path} already exists.")
        return

    name = os.path.basename(target_dir)
    manifest = Manifest(
        package=PackageConfig(name=name, version="0.1.0"),
        build=BuildConfig(sources=["main.ark"], output=f"{name}.ark"),
        path=path
    )
    manifest.save()
    print(f"Initialized package in {path}")

def cmd_build(args):
    target_dir = os.path.abspath(args.path)
    manifest_path = os.path.join(target_dir, "ark.json")
    if not os.path.exists(manifest_path):
        print(f"Error: {manifest_path} not found.")
        return

    try:
        manifest = Manifest.load(manifest_path)
        builder = Builder(manifest)
        builder.build()
    except Exception as e:
        print(f"Build failed: {e}")
        sys.exit(1)

def main():
    parser = argparse.ArgumentParser(description="Ark Package Manager")
    subparsers = parser.add_subparsers(dest="command", help="Command to execute")

    # Init
    parser_init = subparsers.add_parser("init", help="Initialize a new package")
    parser_init.add_argument("path", nargs="?", default=".", help="Directory to initialize")

    # Build
    parser_build = subparsers.add_parser("build", help="Build the package")
    parser_build.add_argument("path", nargs="?", default=".", help="Directory containing ark.json")

    args = parser.parse_args()

    if args.command == "init":
        cmd_init(args)
    elif args.command == "build":
        cmd_build(args)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
