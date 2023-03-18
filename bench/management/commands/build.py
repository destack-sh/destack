import asyncio

from django.core.management import BaseCommand
from django.core.management.base import CommandParser

from bench.language.parse import parse_file
from bench.language.reconstruct import render_file
from bench.language.type import Build
from bench.models.mapper import lookup_in_db_module
from bench.runtime.build import BuildResult, build, generate


class Command(BaseCommand):
    help = "Builds optimized executable code for a task"

    def add_arguments(self, parser: CommandParser):
        parser.add_argument("path", type=str)
        parser.add_argument("compile_path", type=str)

    def handle(self, path: str, compile_path: str, **kwargs):
        lang_module, idx = parse_file(path, lookup_in_module=lookup_in_db_module)
        build = build(idx.symbol(compile_path, Build))
        build_result: BuildResult = asyncio.get_event_loop().run_until_complete(build)
        gen_file = generate(build_result.target_symbols, build_result.weak_references)
        print(render_file(gen_file))
