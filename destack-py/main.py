import os
import sys
from pathlib import Path

# if not serving, default to ENVIRONMENT=dev
if (len(sys.argv) < 2 or sys.argv[1] != "serve") and os.getenv("ENVIRONMENT") is None:
    os.environ["ENVIRONMENT"] = "dev"

from destack.utils.env import setup_env

setup_env()

from destack.cli import console, create_cli  # noqa: E402

# create main CLI app
cli = create_cli(help="Destack CLI")

# add all CLI 'apps' in our CLI folder as sub-CLIs
for path in Path.glob(Path(__file__).parent / "destack" / "cli", "*.py"):
    try:
        module = __import__(f"destack.cli.{path.stem}", fromlist=["cli"])
        if hasattr(module, "cli"):
            sub_cli = module.cli
            # add as a sub-CLI to preserve hierarchy
            cli.add_sub_cli(path.stem, sub_cli)
    except Exception as e:
        console.error(f"Failed to load {path.stem}: {e}")

if __name__ == "__main__":
    cli.run()
