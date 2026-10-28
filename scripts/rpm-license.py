#!/usr/bin/env python3
"""Prints the SPDX License tag of the Linux binary: the licenses of Soliton,
of the Rust crates it links on x86_64 Linux (build and dev dependencies
excluded) and of the embedded Inter font and Lucide icons."""
import json
import subprocess
import sys

# Embedded assets, see assets/NOTICE.md.
ASSETS = ["OFL-1.1", "ISC"]

metadata = json.loads(subprocess.check_output([
    "cargo", "metadata", "--locked", "--format-version", "1",
    "--filter-platform", "x86_64-unknown-linux-gnu",
]))
packages = {package["id"]: package for package in metadata["packages"]}
nodes = {node["id"]: node for node in metadata["resolve"]["nodes"]}
pending = [i for i, package in packages.items() if package["name"] == "soliton-app"]
linked = set()
while pending:
    current = pending.pop()
    if current in linked:
        continue
    linked.add(current)
    for dependency in nodes[current]["deps"]:
        if any(kind["kind"] is None for kind in dependency["dep_kinds"]):
            pending.append(dependency["pkg"])

expressions = set(ASSETS)
for current in linked:
    package = packages[current]
    if not package["license"]:
        sys.exit(f"{package['name']} {package['version']} declares no SPDX license.")
    expression = " ".join(package["license"].replace("/", " OR ").split())
    if "(" not in expression and " OR " not in expression:
        expressions.update(expression.split(" AND "))
    elif "(" not in expression and " AND " not in expression:
        # A plain choice: the same alternatives in any order are one expression.
        expressions.add("(" + " OR ".join(sorted(expression.split(" OR "))) + ")")
    else:
        expressions.add(f"({expression})")
# Soliton's own license first, then a stable order.
expressions.discard("GPL-3.0-only")
print(" AND ".join(["GPL-3.0-only", *sorted(expressions)]))
