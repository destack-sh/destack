from __future__ import annotations

from typing import Any


class ModelImpl:
    """Base class for generated data model implementation mixins."""

    def __getattr__(self, name: str) -> Any:
        """Return generated data fields supplied by subclasses."""

        raise AttributeError(name)


__all__ = [
    "ModelImpl",
]
