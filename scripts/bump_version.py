# bump the version in 'version'
# options are major, minor, patch

import pathlib
import subprocess
import sys

version = pathlib.Path("version").read_text().strip()
version_parts = version.split(".")
major = int(version_parts[0])
minor = int(version_parts[1])
patch = int(version_parts[2])

if len(sys.argv) == 1:
    sys.exit(f"usage: {sys.argv[0]} <major|minor|patch>")
elif sys.argv[1] == "major":
    major += 1
    minor = 0
    patch = 0
elif sys.argv[1] == "minor":
    minor += 1
    patch = 0
elif sys.argv[1] == "patch":
    patch += 1
else:
    sys.exit(f"usage: {sys.argv[0]} <major|minor|patch>")

version = f"{major}.{minor}.{patch}"
pathlib.Path("version").write_text(version)
print(f"bump {sys.argv[1]} version to {version}")

# also run copy_version.py
subprocess.run(["python", "scripts/copy_version.py"])
