# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.cast
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class CoercionSegment:
    """Coercions added by one DIR phase."""

    # the module id of the coercion segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # coercions keyed by the value node being coerced
    coercions: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, Coercion]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CoercionSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CoercionSegment: ...

def encode_coercion_segment(writer: BinaryWriter, value: CoercionSegment) -> None: ...
def decode_coercion_segment(reader: BinaryReader) -> CoercionSegment: ...
def to_json_coercion_segment(value: CoercionSegment) -> Json: ...
def from_json_coercion_segment(value: Json) -> CoercionSegment: ...

@dataclass(frozen=True, slots=True)
class Coercion:
    """One type coercion attached to a value node."""

    # the source type before coercion
    source: destack._generated.dir.type.type.GlobalTypeId
    # the target type after coercion
    target: destack._generated.dir.type.type.GlobalTypeId
    # how the coercion entered DIR
    origin: destack._generated.dir.tree.cast.CastOrigin

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Coercion: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Coercion: ...

def encode_coercion(writer: BinaryWriter, value: Coercion) -> None: ...
def decode_coercion(reader: BinaryReader) -> Coercion: ...
def to_json_coercion(value: Coercion) -> Json: ...
def from_json_coercion(value: Json) -> Coercion: ...

__all__ = [
    "CoercionSegment",
    "encode_coercion_segment",
    "decode_coercion_segment",
    "to_json_coercion_segment",
    "from_json_coercion_segment",
    "Coercion",
    "encode_coercion",
    "decode_coercion",
    "to_json_coercion",
    "from_json_coercion",
]
