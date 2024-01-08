import typer

app = typer.Typer(short_help="users and auth")


@app.command()
def activate():
    """Activate a User."""
    raise NotImplementedError
