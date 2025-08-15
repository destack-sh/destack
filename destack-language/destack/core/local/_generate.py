from ._parser import create_cli

cli = create_cli(
    "generate",
    aliases=["g", "gen"],
    help="Generate the libraries.",
)


@cli.command()
def generate():
    """Generate the libraries / runtimes."""
    from ..generation._rust import regenerate

    regenerate()
