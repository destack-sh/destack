import time

import structlog
import typer
from opentelemetry import trace

app = typer.Typer()
logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


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
