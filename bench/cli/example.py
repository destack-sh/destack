import structlog
import typer
from rich.console import Console

app = typer.Typer(short_help="print examples")
logger = structlog.get_logger(__name__)
console = Console()


@app.command()
def print():
    """Print all registered examples with nice formatting."""
    from bench.language import Block  # noqa: F401
    from bench.runtime.model.example import EXAMPLES

    console.print("─" * 80)
    for example in EXAMPLES:
        console.print(f"\n[bold blue]{example.title}[/bold blue]")
        if example.text:
            console.print(f"[dim]{example.text}[/dim]\n")
        console.print(example.request.strip(), style="bold")
        console.print("[dim]→[/dim]")
        console.print(example.response.strip(), style="bold")
        console.print("\n" + "─" * 80)
