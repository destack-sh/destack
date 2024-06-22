import os
import sys
from pathlib import Path

import typer
from rich.console import Console

# if not serving, default to ENVIRONMENT=dev
if (len(sys.argv) < 2 or sys.argv[1] != "serve") and os.getenv("ENVIRONMENT") is None:
    os.environ["ENVIRONMENT"] = "dev"

from bench.utils.env import setup_env

setup_env()

from bench.utils.logging import setup_logging  # noqa: E402
from bench.utils.tracing import setup_tracing  # noqa: E402

setup_logging()
setup_tracing()

# add all CLI 'apps' in our CLI folder
cli = typer.Typer(pretty_exceptions_enable=False)
for path in Path.glob(Path(__file__).parent / "bench" / "cli", "*.py"):
    if path.stem == "__init__":
        continue
    module = __import__(f"bench.cli.{path.stem}", fromlist=["app"])
    if hasattr(module, "app"):
        cli.add_typer(module.app, name=path.stem)

if __name__ == "__main__":
    console = Console()
    cli()
