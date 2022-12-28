import sys

from django.core.management import BaseCommand
from rich.console import Console
from rich.table import Table

from bench.language.lex import SourceFile, Token, TokenType, lex
from bench.language.parse import parse


class Command(BaseCommand):
    help = "Parses a string from a path or stdin for debugging"

    def add_arguments(self, parser):
        parser.add_argument("path", type=str)

    def handle(self, path, *args, **options):
        if path == "-":
            # read until EOF
            string = sys.stdin.read()
        else:
            with open(path, "r") as f:
                string = f.read()
        source_file = SourceFile(path=path if path != "-" else "<stdin>", content=string)

        console = Console()

        tokens = lex(source_file)
        console.print(pprint_tokens(tokens))

        files = parse(tokens)


def pprint_tokens(tokens: list[Token]) -> Table:
    table = Table(title=f"{len(tokens)} tokens")
    table.add_column("Type", style="yellow")
    table.add_column("Value", style="dim")
    table.add_column("Location", style="green")
    current_file = None
    for token in tokens:
        if token.type == TokenType.WHITESPACE:
            continue
        if token.source_file != current_file:
            current_file = token.source_file
            table.add_row("---", f"[bold]{current_file.path}[/bold]", "---")
        table.add_row(
            token.type.name,
            token.value_truncated,
            token.location_in_file,
        )
    return table
