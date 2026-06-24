# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.target
import destack._generated.source.file.model.target

@dataclass(frozen=True, slots=True)
class ProductTarget:
    """One linked target assembled into a product."""

    # the configured product target name
    name: str
    # the repository target assembled into this product
    target: destack._generated.source.file.model.target.TargetId
    # the runtime contract this target expects
    runtime: destack._generated.artifact.core.target.Runtime
    # the host environment this target expects
    host: destack._generated.artifact.core.target.Host
    # the platform this target expects
    platform: destack._generated.artifact.core.target.Platform
    # whether this product target includes its toolchain build payload
    includes_build: bool
    # whether this product target includes its linked bundle
    includes_bundle: bool
    # whether this product target includes its executable program
    includes_program: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProductTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProductTarget: ...

def encode_product_target(writer: BinaryWriter, value: ProductTarget) -> None: ...
def decode_product_target(reader: BinaryReader) -> ProductTarget: ...
def to_json_product_target(value: ProductTarget) -> Json: ...
def from_json_product_target(value: Json) -> ProductTarget: ...

@dataclass(frozen=True, slots=True)
class Product:
    """One linked product assembled from one or more target artifacts."""

    # the configured product name
    name: str
    # the linked targets keyed by configured product target name
    targets: Mapping[str, ProductTarget]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Product: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Product: ...

def encode_product(writer: BinaryWriter, value: Product) -> None: ...
def decode_product(reader: BinaryReader) -> Product: ...
def to_json_product(value: Product) -> Json: ...
def from_json_product(value: Json) -> Product: ...

__all__ = [
    "ProductTarget",
    "encode_product_target",
    "decode_product_target",
    "to_json_product_target",
    "from_json_product_target",
    "Product",
    "encode_product",
    "decode_product",
    "to_json_product",
    "from_json_product",
]
