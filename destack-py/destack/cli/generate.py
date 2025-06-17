import structlog
import typer

from destack.generate import GenerationScope

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def generate(only: GenerationScope | None = None):
    """Generate all the derived things."""
    from destack.generate import generate as generate_all

    generate_all((only,) if only else tuple(GenerationScope))
    logger.info("destack.generate")
