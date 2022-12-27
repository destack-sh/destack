import sys

from django.core.management import BaseCommand

from bench.language.lex import SourceFile, lex


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

        print(repr(source_file))

        tokens = lex(source_file)
        print("\n".join(map(str, tokens)))
