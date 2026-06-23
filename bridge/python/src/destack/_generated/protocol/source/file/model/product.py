# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.package

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.package import (
        PackageId,
    )


@dataclass(frozen=True, slots=True)
class ProductId:
    """Unique identifier for one product within a package."""

    """The owning package id."""
    package_id: PackageId
    """The stable key for this product within its package."""
    product_key: ProductKey


def encode_product_id(writer: Writer, value: ProductId) -> None:
    destack._generated.protocol.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_product_key(writer, value.product_key)


def decode_product_id(reader: Reader) -> ProductId:
    field_0 = destack._generated.protocol.source.file.model.package.decode_package_id(
        reader
    )
    field_1 = decode_product_key(reader)

    return ProductId(
        package_id=field_0,
        product_key=field_1,
    )


@dataclass(frozen=True, slots=True)
class ProductKey:
    """Stable key for one product within a package."""

    field_0: int


def encode_product_key(writer: Writer, value: ProductKey) -> None:
    writer.write_unsigned(value.field_0)


def decode_product_key(reader: Reader) -> ProductKey:
    field_0 = reader.read_unsigned()

    return ProductKey(
        field_0=field_0,
    )


__all__ = [
    "ProductId",
    "encode_product_id",
    "decode_product_id",
    "ProductKey",
    "encode_product_key",
    "decode_product_key",
]
