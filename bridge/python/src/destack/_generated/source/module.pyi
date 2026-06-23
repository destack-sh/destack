# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

import destack._generated.source.package

if TYPE_CHECKING:
    from destack._generated.source.package import (
        PackageId,
    )

@dataclass(frozen=True, slots=True)
class ModuleId:
    """External module id crossing bridge boundaries."""

    """Owning package."""
    package: PackageId
    """Canonical lowercase hex module key within the package."""
    key: str

def encode_module_id(writer: Writer, value: ModuleId) -> None: ...
def decode_module_id(reader: Reader) -> ModuleId: ...

__all__ = [
    "ModuleId",
    "encode_module_id",
    "decode_module_id",
]
