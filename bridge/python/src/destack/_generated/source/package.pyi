# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, Writer

@dataclass(frozen=True, slots=True)
class PackageId:
    """External package id crossing bridge boundaries."""

    """Canonical lowercase hex package id."""
    id: str

def encode_package_id(writer: Writer, value: PackageId) -> None: ...
def decode_package_id(reader: Reader) -> PackageId: ...

__all__ = [
    "PackageId",
    "encode_package_id",
    "decode_package_id",
]
