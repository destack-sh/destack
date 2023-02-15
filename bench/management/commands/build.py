import asyncio

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.language.parse import parse_file
from bench.language.reconstruct import render_file
from bench.language.type import Build
from bench.models.mapper import lookup_in_db_module
from bench.runtime.build import down, make_build


class Command(BaseCommand):
    help = "Builds optimized executable code for a task"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("path", type=str)
        parser.add_argument("compile_path", type=str)

    def handle(self, path: str, compile_path: str, **kwargs):
        lang_module, idx = parse_file(path, lookup_in_module=lookup_in_db_module)
        build = make_build(idx.symbol(compile_path, Build))
        gen_symbols, mappings = asyncio.get_event_loop().run_until_complete(build)
        gen_file = down(gen_symbols)
        print(render_file(gen_file))
