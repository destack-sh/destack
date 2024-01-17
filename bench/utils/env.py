from pathlib import Path

import dotenv

from bench.utils.utils import LOCAL_ENV


def setup_dotenv():
    """Loads .env files according to the local environment at the project root."""
    if LOCAL_ENV == "prod":
        dot_env_files = [".env", ".env.prod"]
    else:
        dot_env_files = [".env"]

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
