import sys
from pathlib import Path
from typing import TYPE_CHECKING

import dotenv

from bench.utils.utils import get_from_env, str_to_bool


def setup_dotenv():
    """Loads .env files according to the local environment at the project root."""

    if ENVIRONMENT == "prod":
        dot_env_files = [".env", ".env.prod"]
    elif ENVIRONMENT == "test":
        dot_env_files = [".env", ".env.test"]
    elif ENVIRONMENT == "dev":
        dot_env_files = [".env"]
    else:
        raise ValueError(f"unknown environment: {ENVIRONMENT}")

    # find .env files (walk up from current directory)
    dot_env_paths = []
    dir = Path.cwd()
    while dir != dir.parent:
        for dot_env_file in dot_env_files:
            dot_env_path = dir / dot_env_file
            if dot_env_path.exists():
                dot_env_paths.append(dot_env_path)
        dir = dir.parent

    for dot_env_path in dot_env_paths:
        dotenv.load_dotenv(dot_env_path, verbose=True, override=True)


ENVIRONMENT = get_from_env("ENVIRONMENT", default="dev")
IS_DEBUG: bool = get_from_env("DEBUG", False, type_cast=str_to_bool)
IS_TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", False, type_cast=str_to_bool)
)
IS_DEV = ENVIRONMENT == "dev"
SOME_TYPE_CHECKING = TYPE_CHECKING or "mypy" in sys.argv[0] or IS_TEST
REPOSITORY_PATH = Path(__file__).parent.parent.parent.resolve()
