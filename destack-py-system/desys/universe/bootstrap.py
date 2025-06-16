import structlog
import typer

from destack.language import Region, Session

app = typer.Typer(short_help="some language-level utilities")

logger = structlog.get_logger(__name__)


async def create_system_destackes(
    session: Session,
    region: Region,
    *,
    upsert: bool = False,
):
    """Bootstrap the Destack system."""
    raise NotImplementedError
