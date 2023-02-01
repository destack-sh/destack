from __future__ import annotations

import asyncio
import json

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.language.parse import parse_file
from bench.language.type import Code
from bench.models.mapper import lookup_in_db_module
from bench.runtime.execute import instantiate, run


class Command(BaseCommand):
    help = "Runs code in a module with the given arguments"

    def add_arguments(self, parser: CommandParser) -> None:
        parser.add_argument("path", type=str)
        parser.add_argument("code_path", type=str)
        parser.add_argument("input", type=str)

    def handle(self, path: str, code_path: str, input: str, **kwargs):
        lang_module, idx = parse_file(path, lookup_in_module=lookup_in_db_module)
        code_instance = instantiate(idx.symbol(code_path, Code))
        ret = asyncio.get_event_loop().run_until_complete(
            run(code_instance, arguments={"input": input})
        )
        print(json.dumps(ret, indent=2, default=str))
