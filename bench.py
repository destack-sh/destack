from pathlib import Path

import dotenv
import typer
from rich.console import Console

from bench.utils.utils import get_from_env

# :Dotenv
LOCAL_ENV = get_from_env("LOCAL_ENV", "local")
if LOCAL_ENV == "prod":
    DOT_ENV_FILES = [".env", ".env.prod"]
else:
    DOT_ENV_FILES = [".env"]
for dot_env_file in DOT_ENV_FILES:
    dotenv.load_dotenv(dot_env_file, verbose=True, override=True)


cli = typer.Typer()

# add all 'app' instances from ./bench/management/*.py
for path in Path.glob(Path(__file__).parent / "bench" / "management", "*.py"):
    if path.stem in ("__init__", "os", "local"):
        continue
    module = __import__(f"bench.management.{path.stem}", fromlist=["app"])
    if hasattr(module, "app"):
        cli.add_typer(module.app, name=path.stem)

if __name__ == "__main__":
    from bench.language.const import VERSION

    console = Console()
    console.rule(f"Bench CLI - {VERSION}", align="center", characters="=")
    cli()
