import typer

app = typer.Typer()


@app.command()
def db(bench: str = None):
    """Open a psql shell to either the global or the local database."""
    raise NotImplementedError


@app.command()
def session(bench: str = None):
    """Open a Session shell."""
    raise NotImplementedError
