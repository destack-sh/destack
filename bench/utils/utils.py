import os
from dataclasses import field
from typing import Any, Callable, Optional


def str_to_bool(value: str) -> bool:
    truthy_strs_lower = ("y", "yes", "t", "true", "on", "yup", "1")
    return value is not None and str(value).lower() in truthy_strs_lower


def get_from_env(
    key: str,
    default: Optional[Any] = None,
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
            raise ValueError(f'The environment variable "{key}" is missing and required for Bench.')
    if type_cast is not None:
        value = type_cast(value)
    return value


def required_field(**kwargs):
    """Hacky way to make a field required when subclassing a dataclass with defaults."""

    _field = None

    def _raise_must_set():
        raise ValueError(f"field {_field.name} must be set")

    _field = field(default_factory=_raise_must_set, **kwargs)
    return _field
