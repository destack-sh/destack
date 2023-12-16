from django.core.management import BaseCommand
from django.core.management.base import CommandParser
from django.db import transaction

from bench.language.module import KNOWN_BENCH_TYPES
from bench.language.proto import generate_proto_schema


def write_schema(path: str, schema):
    with open(path, "w") as f:
        f.write(str(schema))


class Command(BaseCommand):
    help = "Manages the proto schema"

    def add_arguments(self, parser: CommandParser):
        # optional file name to write to (defaults to stdout)
        parser.add_argument("--file", type=str, required=False)

    @transaction.atomic
    def handle(self, *args, **options):
        proto = generate_proto_schema(KNOWN_BENCH_TYPES, message_postfix="Data")
        schema_str = proto.to_proto_source()

        if options["file"]:
            write_schema(options["file"], schema_str)
        else:
            print(schema_str)
