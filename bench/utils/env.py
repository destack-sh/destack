import sys
from pathlib import Path

import dotenv

from bench.utils.utils import get_from_env


def setup_dotenv():
    """Loads .env files according to the local environment at the project root."""

    if ENVIRONMENT == "prod":
        dot_env_files = [".env", ".env.prod", ".env.prod.local"]
    elif ENVIRONMENT == "stage":
        dot_env_files = [".env", ".env.stage", ".env.stage.local"]
    elif ENVIRONMENT == "test":
        dot_env_files = [".env", ".env.test", ".env.test.local"]
    elif ENVIRONMENT == "dev":
        dot_env_files = [".env", ".env.dev", ".env.dev.local"]
    else:
        raise ValueError(f"unexpected environment: {ENVIRONMENT}")

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


ENVIRONMENT = get_from_env("ENVIRONMENT")
IS_DEBUG: bool = get_from_env("IS_DEBUG", default=ENVIRONMENT == "dev", typ=bool)
IS_TEST: bool = (
    "test" in sys.argv or "pytest" in sys.argv[0] or get_from_env("TEST", default=False, typ=bool)
)
IS_DEV = ENVIRONMENT == "dev"
REPOSITORY_PATH = Path(__file__).parent.parent.parent.resolve()
