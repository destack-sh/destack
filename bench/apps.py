import asyncio
import sys

from django.apps import AppConfig

from bench.settings import DEBUG


def is_migrating():
    return "makemigrations" in sys.argv or "migrate" in sys.argv


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "The Bench"

    def ready(self) -> None:
        # auto-update schema on startup during development
        # (and if we're not running a command that doesn't run the server)
        if DEBUG and "runserver" in sys.argv:
            from bench.api.root import schema
            from bench.management.commands.exportschema import write_schema

            write_schema("schema.gen.graphql", schema)

        if DEBUG:
            # run internal server
            from bench.runtime.dbserver import InternalServer

            # asyncio.get_running_loop().create_task(InternalServer().serve()) # nocheckin start server & worker
