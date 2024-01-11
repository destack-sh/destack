from pathlib import Path

import dotenv
import typer
from rich.console import Console

from bench.utils.logging import configure_logging
from bench.utils.utils import get_from_env

# :Dotenv
LOCAL_ENV = get_from_env("LOCAL_ENV", "local")
if LOCAL_ENV == "prod":
    DOT_ENV_FILES = [".env", ".env.prod"]
else:
    DOT_ENV_FILES = [".env"]
for dot_env_file in DOT_ENV_FILES:
    dotenv.load_dotenv(dot_env_file, verbose=True, override=True)

configure_logging(apply_logging=True, apply_structlog=True)

# add all 'app' instances into CLI (from ./bench/management/*.py)
cli = typer.Typer(pretty_exceptions_enable=False)
for path in Path.glob(Path(__file__).parent / "bench" / "cli", "*.py"):
    if path.stem in ("__init__", "os", "local"):
        continue
    module = __import__(f"bench.cli.{path.stem}", fromlist=["app"])
    if hasattr(module, "app"):
        cli.add_typer(module.app, name=path.stem)

if __name__ == "__main__":
    console = Console()
    cli()
