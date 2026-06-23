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
class ProductId:
    """External product id crossing bridge boundaries."""

    """Owning package."""
    package: PackageId
    """Canonical lowercase hex product key within the package."""
    key: str

def encode_product_id(writer: Writer, value: ProductId) -> None: ...
def decode_product_id(reader: Reader) -> ProductId: ...

__all__ = [
    "ProductId",
    "encode_product_id",
    "decode_product_id",
]
