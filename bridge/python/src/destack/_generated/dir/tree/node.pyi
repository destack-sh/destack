# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class LocalNodeId:
    """Unique identifier for nodes in a local arena, parameterized by node type."""

    id: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalNodeId: ...

def encode_local_node_id(writer: BinaryWriter, value: LocalNodeId) -> None: ...
def decode_local_node_id(reader: BinaryReader) -> LocalNodeId: ...
def to_json_local_node_id(value: LocalNodeId) -> Json: ...
def from_json_local_node_id(value: Json) -> LocalNodeId: ...

"""A Mutability is a const, mutable, or exclusive access qualifier."""
Mutability: typing.TypeAlias = (
    typing.Literal["immutable"]
    | typing.Literal["mutable"]
    | typing.Literal["exclusive"]
)

def encode_mutability(writer: BinaryWriter, value: Mutability) -> None: ...
def decode_mutability(reader: BinaryReader) -> Mutability: ...
def to_json_mutability(value: Mutability) -> Json: ...
def from_json_mutability(value: Json) -> Mutability: ...

"""The asynchrony of a function."""
Asynchrony: typing.TypeAlias = typing.Literal["sync"] | typing.Literal["async"]

def encode_asynchrony(writer: BinaryWriter, value: Asynchrony) -> None: ...
def decode_asynchrony(reader: BinaryReader) -> Asynchrony: ...
def to_json_asynchrony(value: Asynchrony) -> Json: ...
def from_json_asynchrony(value: Json) -> Asynchrony: ...

"""A Visibility is the visibility of an item."""
Visibility: typing.TypeAlias = (
    typing.Literal["public"] | typing.Literal["protected"] | typing.Literal["private"]
)

def encode_visibility(writer: BinaryWriter, value: Visibility) -> None: ...
def decode_visibility(reader: BinaryReader) -> Visibility: ...
def to_json_visibility(value: Visibility) -> Json: ...
def from_json_visibility(value: Json) -> Visibility: ...

@dataclass(frozen=True, slots=True)
class GlobalNodeIdAny:
    """Global node id across modules."""

    # the module id of the global node
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global node
    local_id: LocalNodeIdAny

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalNodeIdAny: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalNodeIdAny: ...

def encode_global_node_id_any(writer: BinaryWriter, value: GlobalNodeIdAny) -> None: ...
def decode_global_node_id_any(reader: BinaryReader) -> GlobalNodeIdAny: ...
def to_json_global_node_id_any(value: GlobalNodeIdAny) -> Json: ...
def from_json_global_node_id_any(value: Json) -> GlobalNodeIdAny: ...

@dataclass(frozen=True, slots=True)
class LocalNodeIdAny:
    """Unique identifier for nodes with dynamic type in a local arena."""

    id: int
    ty: NodeType

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalNodeIdAny: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LocalNodeIdAny: ...

def encode_local_node_id_any(writer: BinaryWriter, value: LocalNodeIdAny) -> None: ...
def decode_local_node_id_any(reader: BinaryReader) -> LocalNodeIdAny: ...
def to_json_local_node_id_any(value: LocalNodeIdAny) -> Json: ...
def from_json_local_node_id_any(value: Json) -> LocalNodeIdAny: ...

"""The type of a node."""
NodeType: typing.TypeAlias = (
    typing.Literal["expression"]
    | typing.Literal["typeExpression"]
    | typing.Literal["block"]
    | typing.Literal["catch"]
    | typing.Literal["declaration"]
    | typing.Literal["declarator"]
    | typing.Literal["property"]
    | typing.Literal["typeMember"]
    | typing.Literal["typeMappedParameter"]
    | typing.Literal["member"]
    | typing.Literal["enumField"]
    | typing.Literal["whereClause"]
    | typing.Literal["dependencyItem"]
    | typing.Literal["genericParameter"]
    | typing.Literal["parameter"]
    | typing.Literal["genericArgument"]
    | typing.Literal["tupleElement"]
    | typing.Literal["argument"]
    | typing.Literal["matchCase"]
    | typing.Literal["pattern"]
    | typing.Literal["patternField"]
    | typing.Literal["assignPattern"]
    | typing.Literal["assignPatternField"]
    | typing.Literal["decorator"]
)

def encode_node_type(writer: BinaryWriter, value: NodeType) -> None: ...
def decode_node_type(reader: BinaryReader) -> NodeType: ...
def to_json_node_type(value: NodeType) -> Json: ...
def from_json_node_type(value: Json) -> NodeType: ...

@dataclass(frozen=True, slots=True)
class GlobalNodeId:
    """Global node id across modules."""

    # the module id of the global node
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global node
    local_id: LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalNodeId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> GlobalNodeId: ...

def encode_global_node_id(writer: BinaryWriter, value: GlobalNodeId) -> None: ...
def decode_global_node_id(reader: BinaryReader) -> GlobalNodeId: ...
def to_json_global_node_id(value: GlobalNodeId) -> Json: ...
def from_json_global_node_id(value: Json) -> GlobalNodeId: ...

__all__ = [
    "LocalNodeId",
    "encode_local_node_id",
    "decode_local_node_id",
    "to_json_local_node_id",
    "from_json_local_node_id",
    "Mutability",
    "encode_mutability",
    "decode_mutability",
    "to_json_mutability",
    "from_json_mutability",
    "Asynchrony",
    "encode_asynchrony",
    "decode_asynchrony",
    "to_json_asynchrony",
    "from_json_asynchrony",
    "Visibility",
    "encode_visibility",
    "decode_visibility",
    "to_json_visibility",
    "from_json_visibility",
    "GlobalNodeIdAny",
    "encode_global_node_id_any",
    "decode_global_node_id_any",
    "to_json_global_node_id_any",
    "from_json_global_node_id_any",
    "LocalNodeIdAny",
    "encode_local_node_id_any",
    "decode_local_node_id_any",
    "to_json_local_node_id_any",
    "from_json_local_node_id_any",
    "NodeType",
    "encode_node_type",
    "decode_node_type",
    "to_json_node_type",
    "from_json_node_type",
    "GlobalNodeId",
    "encode_global_node_id",
    "decode_global_node_id",
    "to_json_global_node_id",
    "from_json_global_node_id",
]
