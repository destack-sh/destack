import time

from destack.cli.parser import create_cli
from destack.utils.log import get_logger

logger = get_logger(__name__)
cli = create_cli(help="Destack code / SDK generation.")


@cli.command()
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
