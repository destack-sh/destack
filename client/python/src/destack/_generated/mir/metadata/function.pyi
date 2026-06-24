# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.function import (
    FunctionMetadataTableImpl,
)

import destack._generated.mir.metadata.effect
import destack._generated.mir.metadata.memory
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class FunctionMetadataTable(FunctionMetadataTableImpl):
    """Function and call metadata derived from semantic MIR."""

    # metadata keyed by function id
    functions: Mapping[destack._generated.mir.tree.node.LocalNodeId, FunctionMetadata]
    # metadata keyed by callsite
    calls: Mapping[CallSite, CallMetadata]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionMetadataTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionMetadataTable: ...

def encode_function_metadata_table(
    writer: BinaryWriter, value: FunctionMetadataTable
) -> None: ...
def decode_function_metadata_table(reader: BinaryReader) -> FunctionMetadataTable: ...
def to_json_function_metadata_table(value: FunctionMetadataTable) -> Json: ...
def from_json_function_metadata_table(value: Json) -> FunctionMetadataTable: ...

@dataclass(frozen=True, slots=True)
class FunctionMetadata:
    """Metadata for one function body or declaration."""

    # memory touched by this function
    memory: destack._generated.mir.metadata.effect.MemoryEffect
    # behavioral effects of this function
    behavior: destack._generated.mir.metadata.effect.FunctionBehavior
    # allocation result size relation when known
    allocation_size: destack._generated.mir.metadata.memory.AllocationSize | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionMetadata: ...

def encode_function_metadata(writer: BinaryWriter, value: FunctionMetadata) -> None: ...
def decode_function_metadata(reader: BinaryReader) -> FunctionMetadata: ...
def to_json_function_metadata(value: FunctionMetadata) -> Json: ...
def from_json_function_metadata(value: Json) -> FunctionMetadata: ...

@dataclass(frozen=True, slots=True)
class CallSiteInstruction:
    """Callsite stored as an instruction."""

    instruction: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["instruction"] = "instruction"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CallSiteTerminator:
    """Callsite stored as a block terminator."""

    terminator: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["terminator"] = "terminator"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Stable identifier for one callsite inside a function body."""
CallSite: typing.TypeAlias = CallSiteInstruction | CallSiteTerminator

def encode_call_site(writer: BinaryWriter, value: CallSite) -> None: ...
def decode_call_site(reader: BinaryReader) -> CallSite: ...
def to_json_call_site(value: CallSite) -> Json: ...
def from_json_call_site(value: Json) -> CallSite: ...

@dataclass(frozen=True, slots=True)
class CallMetadata:
    """Metadata for one callsite."""

    # memory touched by this call
    memory: destack._generated.mir.metadata.effect.MemoryEffect
    # behavioral effects of this call
    behavior: destack._generated.mir.metadata.effect.FunctionBehavior
    # allocation result size relation when known
    allocation_size: destack._generated.mir.metadata.memory.AllocationSize | None
    # resolved direct target when dispatch analysis proves one
    target: destack._generated.mir.tree.node.LocalNodeId | None
    # argument memory behavior when known
    arguments: Sequence[destack._generated.mir.metadata.memory.CallArgumentEffect]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallMetadata: ...

def encode_call_metadata(writer: BinaryWriter, value: CallMetadata) -> None: ...
def decode_call_metadata(reader: BinaryReader) -> CallMetadata: ...
def to_json_call_metadata(value: CallMetadata) -> Json: ...
def from_json_call_metadata(value: Json) -> CallMetadata: ...

__all__ = [
    "FunctionMetadataTable",
    "encode_function_metadata_table",
    "decode_function_metadata_table",
    "to_json_function_metadata_table",
    "from_json_function_metadata_table",
    "FunctionMetadata",
    "encode_function_metadata",
    "decode_function_metadata",
    "to_json_function_metadata",
    "from_json_function_metadata",
    "CallSite",
    "encode_call_site",
    "decode_call_site",
    "to_json_call_site",
    "from_json_call_site",
    "CallSiteInstruction",
    "CallSiteTerminator",
    "CallMetadata",
    "encode_call_metadata",
    "decode_call_metadata",
    "to_json_call_metadata",
    "from_json_call_metadata",
]
