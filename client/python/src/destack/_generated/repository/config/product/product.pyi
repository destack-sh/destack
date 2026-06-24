# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.policy
import destack._generated.repository.config.product.app
import destack._generated.repository.config.stage

@dataclass(frozen=True, slots=True)
class Product:
    """Product assembled from one or more build targets."""

    # release stage for this product
    stage: destack._generated.repository.config.stage.Stage | None
    # active source graph modes for this product
    modes: Sequence[str]
    # active source graph roles for this product
    roles: Sequence[str]
    # active source graph features for this product
    features: Sequence[str]
    # active source graph tags for this product
    tags: Sequence[str]
    # target names keyed by product role
    targets: Mapping[str, str]
    # app declaration used for host integration
    app: destack._generated.repository.config.product.app.App
    # product policy declarations and rules
    policy: destack._generated.repository.config.policy.Policy

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
    "Product",
    "encode_product",
    "decode_product",
    "to_json_product",
    "from_json_product",
]
