from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.api import schema


class Command(BaseCommand):
    help = "Exports GraphQL schema (which requires Django to be loaded)"

    def add_arguments(self, parser: CommandParser):
        # optional file name to write to (defaults to stdout)
        parser.add_argument("--file", type=str, required=False)

    @transaction.atomic
    def handle(self, *args, **options):
        schema_str = str(schema)

        if options["file"]:
            with open(options["file"], "w") as f:
                f.write(schema_str)
        else:
            print(schema_str)
