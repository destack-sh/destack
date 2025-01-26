import structlog
import typer
from rich.console import Console

app = typer.Typer(short_help="print examples")
logger = structlog.get_logger(__name__)
console = Console()

_print = print


@app.command()
def print():
    """Print all registered examples with nice formatting."""
    from bench.language import Block  # noqa: F401
    from bench.runtime.model.example import EXAMPLES

    console.print("─" * 80)
    for i, example in enumerate(EXAMPLES):
        console.print(f"\n[bold blue]{example.title} ({i + 1}/{len(EXAMPLES)})[/bold blue]")
        if example.text:
            console.print(f"[dim]{example.text}[/dim]\n")
        _print(example.request)
        _print("[dim]→[/dim]")
        _print(example.response)
        console.print("\n" + "─" * 80)
