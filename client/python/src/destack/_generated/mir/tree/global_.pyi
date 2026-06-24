# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.constant
import destack._generated.mir.tree.node
import destack._generated.mir.tree.symbol
import destack._generated.mir.tree.type

"""Symbol linkage (visibility and definition location)."""
Linkage: typing.TypeAlias = (
    typing.Literal["local"] | typing.Literal["export"] | typing.Literal["import"]
)

def encode_linkage(writer: BinaryWriter, value: Linkage) -> None: ...
def decode_linkage(reader: BinaryReader) -> Linkage: ...
def to_json_linkage(value: Linkage) -> Json: ...
def from_json_linkage(value: Json) -> Linkage: ...

@dataclass(frozen=True, slots=True)
class Global:
    """Global data definition (module-level variable or constant)."""

    # name for linking and debugging
    name: destack._generated.core.string.StringId
    # the global's persistent mangled symbol: its linkable identity
    symbol: destack._generated.mir.tree.symbol.Symbol
    # the type of the global
    ty: destack._generated.mir.tree.node.LocalNodeId
    # whether this global is mutable
    mutability: destack._generated.mir.tree.type.Mutability
    # the space that owns this global storage
    space: destack._generated.mir.tree.type.Space
    # linkage (local, export, or import)
    linkage: Linkage
    # initial value. None for imported globals
    initializer: GlobalInitializer | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Global: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Global: ...

def encode_global(writer: BinaryWriter, value: Global) -> None: ...
def decode_global(reader: BinaryReader) -> Global: ...
def to_json_global(value: Global) -> Json: ...
def from_json_global(value: Json) -> Global: ...

@dataclass(frozen=True, slots=True)
class GlobalInitializerZero:
    """Zero-initialized (all bytes zero)."""

    kind: typing.Literal["zero"] = "zero"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GlobalInitializerScalar:
    """Scalar constant (bool, int, float)."""

    scalar: destack._generated.mir.tree.constant.Constant
    kind: typing.Literal["scalar"] = "scalar"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GlobalInitializerFunctionAddress:
    """Address of one function inside the program."""

    function_address: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["functionAddress"] = "functionAddress"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GlobalInitializerBytes:
    """Raw bytes (blobs)."""

    bytes: builtins.bytes | bytearray | Sequence[int]
    kind: typing.Literal["bytes"] = "bytes"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class GlobalInitializerAggregate:
    """Aggregate (array/struct fields)."""

    aggregate: Sequence[GlobalInitializer]
    kind: typing.Literal["aggregate"] = "aggregate"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Initializer for global data."""
GlobalInitializer: typing.TypeAlias = (
    GlobalInitializerZero
    | GlobalInitializerScalar
    | GlobalInitializerFunctionAddress
    | GlobalInitializerBytes
    | GlobalInitializerAggregate
)

def encode_global_initializer(
    writer: BinaryWriter, value: GlobalInitializer
) -> None: ...
def decode_global_initializer(reader: BinaryReader) -> GlobalInitializer: ...
def to_json_global_initializer(value: GlobalInitializer) -> Json: ...
def from_json_global_initializer(value: Json) -> GlobalInitializer: ...

__all__ = [
    "Linkage",
    "encode_linkage",
    "decode_linkage",
    "to_json_linkage",
    "from_json_linkage",
    "Global",
    "encode_global",
    "decode_global",
    "to_json_global",
    "from_json_global",
    "GlobalInitializer",
    "encode_global_initializer",
    "decode_global_initializer",
    "to_json_global_initializer",
    "from_json_global_initializer",
    "GlobalInitializerZero",
    "GlobalInitializerScalar",
    "GlobalInitializerFunctionAddress",
    "GlobalInitializerBytes",
    "GlobalInitializerAggregate",
]
