import time

import typer

from destack.utils.log import get_logger

logger = get_logger(__name__)
app = typer.Typer()


@app.callback(invoke_without_command=True)
@app.command()
def generate():
    """Generate all the derived things."""
    started_at = time.time()
    from destack.generate import generate as generate_all

    duration = time.time() - started_at
    logger.debug("destack.init", took=f"{(duration * 1000):.2f}ms")

    started_at = time.time()
    generate_all()
    duration = time.time() - started_at
    logger.debug("destack.generate", took=f"{(duration * 1000):.2f}ms")
