import os
import sys
from pathlib import Path

from django.apps import AppConfig
from django.conf import settings

from bench.settings import DEBUG

project_root = Path(settings.BASE_DIR)
os.environ["VERSION"] = Path(project_root / "version").read_text().strip()


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
            from bench.management.commands.genschema import write_schema

            write_schema("schema.gen.graphql", schema)
