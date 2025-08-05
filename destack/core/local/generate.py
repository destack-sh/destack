from .parser import create_cli

cli = create_cli("generate")


@cli.command()
def generate():
    """Generate the Destack language SDK."""
    pass
