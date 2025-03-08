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


@app.command()
def example():
    """Print all registered examples with nice formatting."""
    from bench.language import Block  # noqa: F401
    from bench.runtime.model.example import EXAMPLES

    console.print("─" * 80)
    for i, example in enumerate(EXAMPLES):
        console.print(f"\n[bold blue]{example.title} ({i + 1}/{len(EXAMPLES)})[/bold blue]")
        if example.text:
            console.print(f"[dim]{example.text}[/dim]\n")
        print(example.request)  # noqa: T201
        console.print("[dim]→[/dim]")
        print(example.response)  # noqa: T201
        console.print("\n" + "─" * 80)
