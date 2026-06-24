# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.package

"""Stable key for one product within a package."""
ProductKey: typing.TypeAlias = int

def encode_product_key(writer: BinaryWriter, value: ProductKey) -> None: ...
def decode_product_key(reader: BinaryReader) -> ProductKey: ...
def to_json_product_key(value: ProductKey) -> Json: ...
def from_json_product_key(value: Json) -> ProductKey: ...

@dataclass(frozen=True, slots=True)
class ProductId:
    """Unique identifier for one product within a package."""

    # the owning package id
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this product within its package
    product_key: ProductKey

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProductId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProductId: ...

def encode_product_id(writer: BinaryWriter, value: ProductId) -> None: ...
def decode_product_id(reader: BinaryReader) -> ProductId: ...
def to_json_product_id(value: ProductId) -> Json: ...
def from_json_product_id(value: Json) -> ProductId: ...

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
