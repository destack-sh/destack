# copy version text from version file into package.json

import json
from pathlib import Path

version = Path("version").read_text()

with open("package.json", "r") as f:
    package = json.load(f)
    package["version"] = version
with open("package.json", "w") as f:
    json.dump(package, f, indent=2)

print("version", version)
