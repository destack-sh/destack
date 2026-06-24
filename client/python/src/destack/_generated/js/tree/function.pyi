# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.js.tree.node

@dataclass(frozen=True, slots=True)
class FunctionSignature:
    """The signature of a function."""

    # the asynchrony of the function
    asynchrony: destack._generated.js.tree.node.Asynchrony
    # the role of the function
    role: FunctionRole | None
    # the form of the function
    form: FunctionForm
    # the generic parameters of the function
    generic_parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the optional `this` parameter of the function
    this_parameter: destack._generated.js.tree.node.LocalNodeId | None
    # the runtime parameters of the function
    parameters: Sequence[destack._generated.js.tree.node.LocalNodeId]
    # the return type of the function
    return_type: destack._generated.js.tree.node.LocalNodeId | None
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

"""The role of a function."""
FunctionRole: typing.TypeAlias = (
    typing.Literal["getter"] | typing.Literal["setter"] | typing.Literal["constructor"]
)

def encode_function_role(writer: BinaryWriter, value: FunctionRole) -> None: ...
def decode_function_role(reader: BinaryReader) -> FunctionRole: ...
def to_json_function_role(value: FunctionRole) -> Json: ...
def from_json_function_role(value: Json) -> FunctionRole: ...

"""The style of a function."""
FunctionForm: typing.TypeAlias = typing.Literal["function"] | typing.Literal["lambda"]

def encode_function_form(writer: BinaryWriter, value: FunctionForm) -> None: ...
def decode_function_form(reader: BinaryReader) -> FunctionForm: ...
def to_json_function_form(value: FunctionForm) -> Json: ...
def from_json_function_form(value: Json) -> FunctionForm: ...

__all__ = [
    "FunctionSignature",
    "encode_function_signature",
    "decode_function_signature",
    "to_json_function_signature",
    "from_json_function_signature",
    "FunctionRole",
    "encode_function_role",
    "decode_function_role",
    "to_json_function_role",
    "from_json_function_role",
    "FunctionForm",
    "encode_function_form",
    "decode_function_form",
    "to_json_function_form",
    "from_json_function_form",
]
