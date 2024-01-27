import structlog
import typer

app = typer.Typer(short_help="users and auth")
logger = structlog.get_logger(__name__)


@app.command()
def track():
    """Track a User."""
    raise NotImplementedError
