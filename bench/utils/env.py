import sys
from enum import StrEnum
from pathlib import Path

import dotenv

from bench.utils.utils import get_from_env


class Env(StrEnum):
    DEV = "dev"
    TEST = "test"
    STAGE = "stage"
    PROD = "prod"


def setup_dotenv():
    """Loads .env files according to the local environment at the project root."""

    if EMV == Env.PROD:
        dot_env_files = [".env", ".env.prod", ".env.prod.local"]
    elif EMV == Env.STAGE:
        dot_env_files = [".env", ".env.stage", ".env.stage.local"]
    elif EMV == Env.TEST:
        dot_env_files = [".env", ".env.test", ".env.test.local"]
    elif EMV == Env.DEV:
        dot_env_files = [".env", ".env.dev", ".env.dev.local"]
    else:
        raise ValueError(f"unexpected environment: {EMV}")

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


IS_DEBUG: bool = hasattr(sys, "gettrace") and sys.gettrace() is not None
EMV = get_from_env("ENVIRONMENT", typ=Env)
IS_DEV = EMV == Env.DEV
IS_TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", default=False, typ=bool)
    or EMV == Env.TEST
)
IS_PROD = EMV == Env.PROD
IS_STAGE = EMV == Env.STAGE
REPOSITORY_PATH = Path(__file__).parent.parent.parent.resolve()
