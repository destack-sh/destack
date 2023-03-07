#!/usr/bin/env python
"""Django's command-line utility for administrative tasks."""
import os
import sys
from pathlib import Path

import dotenv

from bench.utils.utils import get_from_env

LOCAL_ENV = get_from_env("LOCAL_ENV", "local")
if LOCAL_ENV == "prod":
    DOT_ENV_FILES = [".env", ".env.prod"]
else:
    DOT_ENV_FILES = [".env"]


def main():
    for dot_env_file in DOT_ENV_FILES:
        dotenv.load_dotenv(dot_env_file, verbose=True, override=True)
    os.environ["VERSION"] = Path("version").read_text().strip()

    """Run administrative tasks."""
    os.environ.setdefault("DJANGO_SETTINGS_MODULE", "bench.settings")
    try:
        from django.core.management import execute_from_command_line
    except ImportError as exc:
        raise ImportError(
            "Couldn't import Django. Are you sure it's installed and "
            "available on your PYTHONPATH environment variable? Did you "
            "forget to activate a virtual environment?"
        ) from exc
    execute_from_command_line(sys.argv)


if __name__ == "__main__":
    main()
