from .parser import create_cli

cli = create_cli("manual")


@cli.command()
def manual():
    """Interactive manual for the Destack language SDK."""
    pass
