import os
import sys
from enum import Enum, StrEnum
from pathlib import Path
from typing import Optional, cast

import cachetools


class Env(StrEnum):
    DEV = "dev"
    TEST = "test"
    STAGE = "stage"
    PROD = "prod"

    @property
    def slug(self) -> str:
        return self.value


def str_to_bool(value: str | None) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


@cachetools.cached({}, key=lambda key, *args, **kwargs: key)
def get_from_env_maybe[T](
    key: str,
    *,
    description: str | None = None,
    default: Optional[T] = None,
    optional: bool = True,
    typ: type[T] = str,
) -> T | None:
    value = os.getenv(key)
    if value is None or value == "":
        if default is not None:
            return default
        elif optional:
            return None
        else:
            raise ValueError(
                f'environment variable {key} is required (type={typ}, description="{description}")'
            )
    try:
        if typ is bool:
            value = str_to_bool(value)
        elif issubclass(typ, StrEnum):
            try:
                value = typ(value)
            except ValueError:
                value = typ[value]
        elif issubclass(typ, Enum):
            try:
                value = int(value)  # type: ignore
                value = typ(value)
            except ValueError:
                if typ.__name__ in ("Region", "RegionZone", "RegionArea"):
                    # parse slug
                    value = cast(str, value).replace("_", "-").lower()
                    value = typ.get_by_slug(value)  # type: ignore
                else:
                    value = typ[cast(str, value).upper()]
        else:
            value = cast(T, typ(value))  # type: ignore
    except Exception as e:
        raise ValueError(
            f'environment variable is invalid (key={key}, value={value}, type_cast={typ}, description="{description}"'
        ) from e
    return cast(T, value)


def get_from_env[T](
    key: str,
    *,
    description: str | None = None,
    default: Optional[T] = None,
    typ: type[T] = str,
) -> T:
    value = get_from_env_maybe(
        key, description=description, default=default, optional=False, typ=typ
    )
    return cast(T, value)


_setup_env: bool = False


def setup_env():
    """Loads .env files according to the local environment at the project root."""
    global _setup_env
    if _setup_env:
        return

    try:
        import dotenv
    except ImportError:
        return

    _setup_env = True

    if ENV == Env.PROD:
        dot_env_files = [".env", ".env.prod", ".env.prod.local"]
    elif ENV == Env.STAGE:
        dot_env_files = [".env", ".env.stage", ".env.stage.local"]
    elif ENV == Env.TEST:
        dot_env_files = [".env", ".env.test", ".env.test.local"]
    elif ENV == Env.DEV:
        dot_env_files = [".env", ".env.dev", ".env.dev.local"]
    else:
        raise ValueError(f"unexpected environment: {ENV}")

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


ENV = get_from_env(
    "ENVIRONMENT", typ=Env, description="The current Environment [dev, test, stage, prod]"
)
setup_env()
IS_DEV = ENV == Env.DEV
IS_PROD = ENV == Env.PROD
IS_STAGE = ENV == Env.STAGE
IS_TEST: bool = (
    "test" in sys.argv
    or "pytest" in sys.argv[0]
    or get_from_env("TEST", default=False, typ=bool, description="Whether to run in test mode")
    or ENV == Env.TEST
)
REPOSITORY_PATH = Path(__file__).parent.parent.parent.resolve()
