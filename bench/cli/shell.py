import typer

app = typer.Typer(short_help="convenience shells")


@app.command()
def db(bench: str = None):
    """Open a psql shell to either the global or the local database."""
    raise NotImplementedError


@app.command()
def session(bench: str = None):
    """Open a Session shell."""
    raise NotImplementedError
