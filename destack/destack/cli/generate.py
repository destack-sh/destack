import time

from destack.cli.parser import create_cli

cli = create_cli(help="Destack code / SDK generation.")


@cli.command()
def generate():
    """Generate all the derived things."""
    started_at = time.time()
    from destack.generate import generate as generate_all

    duration = time.time() - started_at
    print("destack.init", f"{(duration * 1000):.2f}ms")  # noqa: T201

    started_at = time.time()
    generate_all()
    duration = time.time() - started_at
    print("destack.generate", f"{(duration * 1000):.2f}ms")  # noqa: T201
