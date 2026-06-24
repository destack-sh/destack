# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.type

@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Runtime function metadata carried by one durable program."""

    # dense function records keyed by program function id
    functions: Sequence[Function | None]
    # function ids keyed by exported source name
    function_by_name: Mapping[str, FunctionId]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionTable: ...

def encode_function_table(writer: BinaryWriter, value: FunctionTable) -> None: ...
def decode_function_table(reader: BinaryReader) -> FunctionTable: ...
def to_json_function_table(value: FunctionTable) -> Json: ...
def from_json_function_table(value: Json) -> FunctionTable: ...

@dataclass(frozen=True, slots=True)
class Function:
    """Runtime function metadata."""

    # the source-facing function name
    name: str
    # function parameter types
    parameters: Sequence[destack._generated.program.type.TypeId]
    # function return type
    return_type: destack._generated.program.type.TypeId
    # captured closure environment type when one exists
    environment: destack._generated.program.type.TypeId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Function: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Function: ...

def encode_function(writer: BinaryWriter, value: Function) -> None: ...
def decode_function(reader: BinaryReader) -> Function: ...
def to_json_function(value: Function) -> Json: ...
def from_json_function(value: Json) -> Function: ...

"""Durable runtime function id inside one program."""
FunctionId: typing.TypeAlias = int

def encode_function_id(writer: BinaryWriter, value: FunctionId) -> None: ...
def decode_function_id(reader: BinaryReader) -> FunctionId: ...
def to_json_function_id(value: FunctionId) -> Json: ...
def from_json_function_id(value: Json) -> FunctionId: ...

__all__ = [
    "FunctionTable",
    "encode_function_table",
    "decode_function_table",
    "to_json_function_table",
    "from_json_function_table",
    "Function",
    "encode_function",
    "decode_function",
    "to_json_function",
    "from_json_function",
    "FunctionId",
    "encode_function_id",
    "decode_function_id",
    "to_json_function_id",
    "from_json_function_id",
]
