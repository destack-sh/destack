from .parser import create_cli

cli = create_cli("generate", "Generate the libraries.")


@cli.command()
def generate():
    """Generate the libraries and runtimes."""
    # nocheckin(all): generate libraries
    pass
