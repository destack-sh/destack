import typer

app = typer.Typer()


@app.command()
def supervisor():
    """
    Run the supervisor.
    """
    raise NotImplementedError


@app.command()
def worker():
    """
    Run the worker.
    """
    raise NotImplementedError
