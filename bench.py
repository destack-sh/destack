from pathlib import Path

import typer
from rich.console import Console

from bench.utils.env import setup_dotenv
from bench.utils.logging import configure_logging

setup_dotenv()
configure_logging()

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
