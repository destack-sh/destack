import sys

from django.core.management import BaseCommand
from rich import markup
from rich.console import Console
from rich.panel import Panel
from rich.table import Table

from bench import language
from bench.backend.mapper import lookup_requirement
from bench.language import File, Requirement
from bench.language.lex import SourceFile, Token, lex
from bench.language.parse import index_module, parse
from bench.language.reconstruct import render, render_file
from bench.language.types import Statement, StatementPath


class Command(BaseCommand):
    help = "Parses a string from a path or stdin for debugging"

    def add_arguments(self, parser):
        parser.add_argument("path", type=str)
        # whether to show final render only as 'render_only'
        parser.add_argument("-r", "--reconstruct", action="store_true")
        # whether to use libraries from the database
        parser.add_argument("-d", "--database", action="store_true")

    def handle(self, path: str, reconstruct: bool, database: bool, *args, **options):
        console = Console()
        if path == "-":
            # read until EOF
            string = sys.stdin.read()
        else:
            with open(path, "r") as f:
                string = f.read()
        source_file = SourceFile(path=path if path != "-" else "<stdin>", content=string)
        tokens = lex(source_file)
        if not reconstruct:
            console.print(pprint_tokens(tokens))

        if database:
            module = parse(tokens, lookup_module=lookup_module_in_db)
        else:
            module = parse(tokens)

        if not reconstruct:
            for panel in pprint_files(module.files):
                console.print(panel)
        else:
            reconstruction = render(module.files)
            console.print(reconstruction, markup=False, highlight=False)


def lookup_module_in_db(requirement: Requirement, path: StatementPath) -> Statement:
    from bench.backend import mapper

    version = lookup_requirement(requirement)
    if version is None:
        raise ValueError(f"could not find module {requirement}")

    module: language.Module = mapper.read(version, path)
    idx = index_module(module)
    return idx.statements_by_path[path]


def pprint_tokens(tokens: list[Token]) -> Table:
    table = Table(title=f"{len(tokens)} tokens")
    table.add_column("Type", style="yellow")
    table.add_column("Value", style="dim")
    table.add_column("Extras", style="cyan")
    table.add_column("Location", style="green")
    current_file = None
    for token in tokens:
        if token.source_file != current_file:
            current_file = token.source_file
            table.add_row("---", f"[bold]{current_file.path}[/bold]", "---")
        extras_str = ", ".join(f"{key}={value}" for key, value in token.value_extras.items())
        table.add_row(
            token.type.name,
            markup.escape(token.value_truncated),
            extras_str,
            token.location_in_file,
        )
    return table


def pprint_files(files: list[File]) -> list[Panel]:
    # arrange files as panels
    panels: list[Panel] = []
    for file in files:
        file_str = render_file(file)
        panels.append(Panel(markup.escape(file_str), title=file.path))
    return panels
