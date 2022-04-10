import os
from typing import Any, Callable, Optional

from django.core.exceptions import ImproperlyConfigured


def str_to_bool(value: str) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


def get_from_env(
    key: str,
    default: Any = None,
    *,
    optional: bool = False,
    type_cast: Optional[Callable] = None,
) -> Any:
    value = os.getenv(key)
    if value is None or value == "":
        if optional:
            return None
        elif default is not None:
            value = default
        else:
            raise ImproperlyConfigured(
                f'The environment variable "{key}" is missing and required for Bench.'
            )
    if type_cast is not None:
        value = type_cast(value)
    return value
