import structlog
import typer

app = typer.Typer()
logger = structlog.get_logger(__name__)


@app.callback(invoke_without_command=True)
@app.command()
def generate():
    """Generate all the derived things."""
    from destack.generate import generate as generate_all

    generate_all()
    logger.info("destack.generate")
