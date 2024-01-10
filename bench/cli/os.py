import structlog
import typer

logger = structlog.get_logger(__name__)
app = typer.Typer(short_help="pg management")
