import sys

from django.apps import AppConfig

from bench.settings import DEBUG


def is_migrating():
    return "makemigrations" in sys.argv or "migrate" in sys.argv


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "The Bench"

    def ready(self) -> None:
        if DEBUG:
            # auto-update schema on startup during development
            from bench.api import schema
            from bench.management.commands.exportschema import write_schema

            write_schema("schema.gen.graphql", schema)
