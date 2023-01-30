from __future__ import annotations

import asyncio
import json
from pathlib import Path
from uuid import uuid4

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench import language
from bench.language import lex
from bench.language.parse import parse
from bench.language.type import Code, SourceFile, StatementPath
from bench.models.mapper import lookup_in_db_module
from bench.runtime.execute import instantiate, run


class Command(BaseCommand):
    help = "Runs code in a project with the given arguments"

    def add_arguments(self, parser: CommandParser) -> None:
        # project as organization/project[:compilation]
        parser.add_argument("path", type=str)
        # add input string as only variable
        parser.add_argument("statement_path", type=str)
        # input str
        parser.add_argument("input", type=str)

    def handle(self, path: str, statement_path: str, input: str, **kwargs):
        statement_path = StatementPath(*statement_path.split(":"))
        module = language.Module(id=uuid4(), name=path.rsplit("/", 1)[-1])
        source_file = SourceFile(path=path, content=Path(path).read_text())
        lang_module, idx = parse(lex(source_file), module, lookup_in_db_module)
        code_instance = instantiate(idx.symbol(statement_path, Code), idx)
        ret = asyncio.get_event_loop().run_until_complete(
            run(code_instance, arguments=dict(input=input))
        )
        print(json.dumps(ret, indent=2, default=str))
