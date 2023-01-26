from __future__ import annotations

from pathlib import Path
from uuid import uuid4

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench import language
from bench.language import lex
from bench.language.parse import parse, raise_error, resolve
from bench.language.type import SourceFile, StatementPath
from bench.models.mapper import lookup_in_db_module
from bench.runtime.execute import instantiate


class Command(BaseCommand):
    help = "Runs code in a project with the given arguments"

    def add_arguments(self, parser: CommandParser) -> None:
        # project as organization/project[:compilation]
        parser.add_argument("path", type=str)
        # add input string as only variable
        parser.add_argument("statement_path", type=str)

    def handle(self, path: str, statement_path: str, **kwargs):
        statement_path = StatementPath(*statement_path.split(":"))
        module = language.Module(id=uuid4(), name=path.rsplit("/", 1)[-1])
        source_file = SourceFile(path=path, content=Path(path).read_text())
        lang_module = parse(lex(source_file), module, lookup_in_db_module)
        idx = resolve(lang_module, lookup_in_db_module, on_error=raise_error)
        code = idx.statement(statement_path)
        code_instance = instantiate(code, idx)
