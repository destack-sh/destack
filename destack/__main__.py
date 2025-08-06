import sys
from pathlib import Path

# add parent directory to path so destack imports work when running directly
sys.path.insert(0, str(Path(__file__).parent.parent))

from destack.core.local import console, create_cli

# create main CLI app
cli = create_cli(help="Destack CLI")

# add all CLIs in our CLI folder as sub-CLIs
for path in Path.glob(Path(__file__).parent / "core" / "local", "*.py"):
    try:
        module = __import__(f"destack.core.local.{path.stem}", fromlist=["cli"])
        if hasattr(module, "cli"):
            sub_cli = module.cli
            # add as a sub-CLI to preserve hierarchy
            cli.add_sub_cli(path.stem, sub_cli)
    except Exception as e:
        console.error(f"failed to load '{path.stem}': {e}")

if __name__ == "__main__":
    cli.run()
