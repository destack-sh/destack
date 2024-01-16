import os
from pathlib import Path

from django.apps import AppConfig
from django.conf import settings

project_root = Path(settings.BASE_DIR)
os.environ["VERSION"] = Path(project_root / "version").read_text().strip()


class BenchConfig(AppConfig):
    name = "bench"
    verbose_name = "Bench"

    def ready(self) -> None:
        pass
