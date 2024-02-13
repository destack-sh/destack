import structlog
import typer

app = typer.Typer(short_help="users and auth")
logger = structlog.get_logger(__name__)
