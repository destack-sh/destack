# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.tree.node
import destack._generated.dir.tree.property

"""The source form used to spell a receiver."""
ThisForm: typing.TypeAlias = typing.Literal["implicit"] | typing.Literal["explicit"]

def encode_this_form(writer: BinaryWriter, value: ThisForm) -> None: ...
def decode_this_form(reader: BinaryReader) -> ThisForm: ...
def to_json_this_form(value: ThisForm) -> Json: ...
def from_json_this_form(value: Json) -> ThisForm: ...

@dataclass(frozen=True, slots=True)
class FunctionSignature:
    """The signature of a function."""

    # the asynchrony of the function
    asynchrony: destack._generated.dir.tree.node.Asynchrony
    # the special role of the function
    role: destack._generated.dir.tree.property.FunctionRole | None
    # the source form of the function
    form: FunctionForm
    # when the function may be called
    phase: FunctionPhase
    # the generic parameters of the function
    generic_parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the where clauses of the function
    where_clauses: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the source form used for the receiver
    this_form: ThisForm | None
    # the optional `this` parameter
    this_parameter: destack._generated.dir.tree.node.LocalNodeId | None
    # the dynamic parameters of the function
    parameters: Sequence[destack._generated.dir.tree.node.LocalNodeId]
    # the return type of the function
    return_type: destack._generated.dir.tree.node.LocalNodeId | None
    # whether the function is abstract
    is_abstract: bool
    # whether the function is an override
    is_override: bool
    # whether the function is a generator
    is_generator: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignature: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionSignature: ...

def encode_function_signature(
    writer: BinaryWriter, value: FunctionSignature
) -> None: ...
def decode_function_signature(reader: BinaryReader) -> FunctionSignature: ...
def to_json_function_signature(value: FunctionSignature) -> Json: ...
def from_json_function_signature(value: Json) -> FunctionSignature: ...

"""The source form of a function."""
FunctionForm: typing.TypeAlias = typing.Literal["function"] | typing.Literal["lambda"]

def encode_function_form(writer: BinaryWriter, value: FunctionForm) -> None: ...
def decode_function_form(reader: BinaryReader) -> FunctionForm: ...
def to_json_function_form(value: FunctionForm) -> Json: ...
def from_json_function_form(value: Json) -> FunctionForm: ...

"""When a function may be called."""
FunctionPhase: typing.TypeAlias = typing.Literal["normal"] | typing.Literal["comptime"]

def encode_function_phase(writer: BinaryWriter, value: FunctionPhase) -> None: ...
def decode_function_phase(reader: BinaryReader) -> FunctionPhase: ...
def to_json_function_phase(value: FunctionPhase) -> Json: ...
def from_json_function_phase(value: Json) -> FunctionPhase: ...

__all__ = [
    "ThisForm",
    "encode_this_form",
    "decode_this_form",
    "to_json_this_form",
    "from_json_this_form",
    "FunctionSignature",
    "encode_function_signature",
    "decode_function_signature",
    "to_json_function_signature",
    "from_json_function_signature",
    "FunctionForm",
    "encode_function_form",
    "decode_function_form",
    "to_json_function_form",
    "from_json_function_form",
    "FunctionPhase",
    "encode_function_phase",
    "decode_function_phase",
    "to_json_function_phase",
    "from_json_function_phase",
]
