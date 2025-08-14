import os
from enum import Enum, StrEnum
from pathlib import Path
from typing import Optional, cast


class Environment(StrEnum):
    DEVELOPMENT = "development"
    TEST = "test"
    STAGING = "staging"
    PRODUCTION = "production"

    @property
    def slug(self) -> str:
        return self.value


def str_to_bool(value: str | None) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


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


def _load_dotenv(path: Path, *, override: bool = True) -> None:
    """Loads a .env file from the given path into `os.environ`."""

    for line in path.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        line_parts = line.split("=", 1)
        if len(line_parts) != 2:
            raise ValueError(f"{path}: {line} is invalid")
        key, value = line_parts
        if key in os.environ and not override:
            raise ValueError(f"{path}: {key} is already set")
        os.environ[key] = value


def setup_environment():
    """Loads .env files according to the local environment at the project root."""
    global _setup_env
    if _setup_env:
        return

    _setup_env = True

    if ENVIRONMENT == Environment.PRODUCTION:
        dot_env_files = [".env", ".env.prod", ".env.prod.local"]
    elif ENVIRONMENT == Environment.STAGING:
        dot_env_files = [".env", ".env.stage", ".env.stage.local"]
    elif ENVIRONMENT == Environment.TEST:
        dot_env_files = [".env", ".env.test", ".env.test.local"]
    elif ENVIRONMENT == Environment.DEVELOPMENT:
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
        _load_dotenv(dot_env_path, override=True)


ENVIRONMENT = get_from_env(
    "ENVIRONMENT", typ=Environment, description="The current system Environment"
)
setup_environment()
