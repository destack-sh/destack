import structlog
import typer
from rich.console import Console

app = typer.Typer(short_help="print examples")
logger = structlog.get_logger(__name__)
console = Console()


@app.command()
def system():
    """Print the system prompt."""
    from bench.runtime.model.instruct import SYSTEM_PROMPT

    print(SYSTEM_PROMPT)  # noqa: T201


@app.command()
def schema():
    """Print the schema."""
    from bench.language import PackageNode
    from bench.runtime.model.instruct import render_builtin_hierarchy

    print(render_builtin_hierarchy(PackageNode))  # noqa: T201
