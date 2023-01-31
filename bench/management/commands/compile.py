import asyncio

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.language.parse import parse_file
from bench.language.reconstruct import render_file
from bench.language.type import Compilation
from bench.models.mapper import lookup_in_db_module
from bench.runtime.compile import compile, down


class Command(BaseCommand):
    help = "Compiles a task into optimized code"

    def add_arguments(self, parser: CommandParser):
        # project as organization/project[:compilation]
        parser.add_argument("path", type=str)
        # compile path as statement path
        parser.add_argument("compile_path", type=str)

    def handle(self, path: str, compile_path: str, **kwargs):
        lang_module, idx = parse_file(path, lookup_in_module=lookup_in_db_module)
        compilation = compile(idx.symbol(compile_path, Compilation))
        gen_symbols, mappings = asyncio.get_event_loop().run_until_complete(compilation)
        gen_file = down(gen_symbols)
        print(render_file(gen_file))
