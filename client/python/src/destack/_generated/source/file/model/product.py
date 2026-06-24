# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_int,
    json_object,
)

import destack._generated.source.file.model.package

"""Stable key for one product within a package."""
ProductKey: typing.TypeAlias = int


def encode_product_key(writer: BinaryWriter, value: ProductKey) -> None:
    """Encode one ProductKey."""
    writer.write_unsigned(value)


def decode_product_key(reader: BinaryReader) -> ProductKey:
    """Decode one ProductKey."""
    return reader.read_unsigned()


def to_json_product_key(value: ProductKey) -> Json:
    """Return one JSON value for one ProductKey."""
    return value


def from_json_product_key(value: Json) -> ProductKey:
    """Return one ProductKey from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class ProductId:
    """Unique identifier for one product within a package."""

    # the owning package id
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this product within its package
    product_key: ProductKey

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_product_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProductId:
        """Decode one ProductId."""
        return decode_product_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_product_id(self)

    @classmethod
    def from_json(cls, value: Json) -> ProductId:
        """Return one ProductId from one JSON value."""
        return from_json_product_id(value)


def encode_product_id(writer: BinaryWriter, value: ProductId) -> None:
    """Encode one ProductId."""
    destack._generated.source.file.model.package.encode_package_id(
        writer, value.package_id
    )
    encode_product_key(writer, value.product_key)


def decode_product_id(reader: BinaryReader) -> ProductId:
    """Decode one ProductId."""
    package_id = destack._generated.source.file.model.package.decode_package_id(reader)
    product_key = decode_product_key(reader)

    return ProductId(
        package_id=package_id,
        product_key=product_key,
    )


def to_json_product_id(value: ProductId) -> Json:
    """Return one JSON value for one ProductId."""
    return {
        "packageId": destack._generated.source.file.model.package.to_json_package_id(
            value.package_id
        ),
        "productKey": to_json_product_key(value.product_key),
    }


def from_json_product_id(value: Json) -> ProductId:
    """Return one ProductId from one JSON value."""
    object_ = json_object(value)

    return ProductId(
        package_id=destack._generated.source.file.model.package.from_json_package_id(
            json_field(object_, "packageId")
        ),
        product_key=from_json_product_key(json_field(object_, "productKey")),
    )


__all__ = [
    "ProductKey",
    "encode_product_key",
    "decode_product_key",
    "to_json_product_key",
    "from_json_product_key",
    "ProductId",
    "encode_product_id",
    "decode_product_id",
    "to_json_product_id",
    "from_json_product_id",
]
