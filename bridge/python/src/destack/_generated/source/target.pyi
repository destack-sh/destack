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
class TargetId:
    """External target id crossing bridge boundaries."""

    """Owning package."""
    package: PackageId
    """Canonical lowercase hex target key within the package."""
    key: str

def encode_target_id(writer: Writer, value: TargetId) -> None: ...
def decode_target_id(reader: Reader) -> TargetId: ...

__all__ = [
    "TargetId",
    "encode_target_id",
    "decode_target_id",
]
