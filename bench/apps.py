import sys

from django.apps import AppConfig


def is_migrating():
    return "makemigrations" in sys.argv or "migrate" in sys.argv


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "The Bench"

    def ready(self) -> None:
        pass
