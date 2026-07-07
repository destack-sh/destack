# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.tree.global_
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.node
import destack._generated.mir.tree.parameter
import destack._generated.mir.tree.symbol

@dataclass(frozen=True, slots=True)
class Function:
    """A function in MIR."""

    # the function's name (for linking and debugging)
    name: destack._generated.core.string.StringId
    # the function's persistent mangled symbol: its linkable identity
    symbol: destack._generated.mir.tree.symbol.Symbol
    # linkage (local, export, or import)
    linkage: destack._generated.mir.tree.global_.Linkage
    # function parameters as typed SSA slots
    parameters: Sequence[destack._generated.mir.tree.parameter.FunctionParameter]
    # lifetime parameters in function-local slot order
    lifetimes: Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter]
    # optional parameter names for diagnostics
    parameter_names: Sequence[destack._generated.core.string.StringId | None]
    # the return type
    return_type: destack._generated.mir.tree.node.LocalNodeId
    # the hidden environment type for this function when present
    environment: destack._generated.mir.tree.node.LocalNodeId | None
    # runtime binding name when this function has a binding identity
    binding: destack._generated.core.string.StringId | None
    # the executable function body when this function is defined
    body: FunctionBody | None
    # memory allocation restrictions for this function
    allocation: AllocationMode
    # the suspension kind when this function can suspend
    suspension: SuspensionKind | None

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

@dataclass(frozen=True, slots=True)
class FunctionBody:
    """The executable body of one MIR function."""

    # the entry block where execution starts
    entry: destack._generated.mir.tree.node.LocalNodeId
    # the function blocks in layout order
    blocks: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # the function locals in slot order
    locals: Sequence[destack._generated.mir.tree.node.LocalNodeId]
    # optional explicit SSA value names keyed by value id
    value_names: Sequence[destack._generated.core.string.StringId | None]
    # SSA value types keyed by value id
    value_types: Sequence[destack._generated.mir.tree.node.LocalNodeId | None]
    # counter for allocating unique SSA value ids
    next_value_id: int
    # instruction locations keyed by instruction id
    instruction_index: InstructionIndex

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBody: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionBody: ...

def encode_function_body(writer: BinaryWriter, value: FunctionBody) -> None: ...
def decode_function_body(reader: BinaryReader) -> FunctionBody: ...
def to_json_function_body(value: FunctionBody) -> Json: ...
def from_json_function_body(value: Json) -> FunctionBody: ...

@dataclass(frozen=True, slots=True)
class InstructionIndex:
    """Function-local instruction location index."""

    # instruction locations keyed by instruction id
    locations: Sequence[InstructionLocation | None]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InstructionIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InstructionIndex: ...

def encode_instruction_index(writer: BinaryWriter, value: InstructionIndex) -> None: ...
def decode_instruction_index(reader: BinaryReader) -> InstructionIndex: ...
def to_json_instruction_index(value: InstructionIndex) -> Json: ...
def from_json_instruction_index(value: Json) -> InstructionIndex: ...

@dataclass(frozen=True, slots=True)
class InstructionLocation:
    """Location of one instruction in a MIR function body."""

    # the block that owns the instruction
    block: destack._generated.mir.tree.node.LocalNodeId
    # the instruction index in the block
    index: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> InstructionLocation: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> InstructionLocation: ...

def encode_instruction_location(
    writer: BinaryWriter, value: InstructionLocation
) -> None: ...
def decode_instruction_location(reader: BinaryReader) -> InstructionLocation: ...
def to_json_instruction_location(value: InstructionLocation) -> Json: ...
def from_json_instruction_location(value: Json) -> InstructionLocation: ...

"""Memory allocation restrictions for a function."""
AllocationMode: typing.TypeAlias = (
    typing.Literal["any"] | typing.Literal["noManaged"] | typing.Literal["noHeap"]
)

def encode_allocation_mode(writer: BinaryWriter, value: AllocationMode) -> None: ...
def decode_allocation_mode(reader: BinaryReader) -> AllocationMode: ...
def to_json_allocation_mode(value: AllocationMode) -> Json: ...
def from_json_allocation_mode(value: Json) -> AllocationMode: ...

"""The suspension kind for one function."""
SuspensionKind: typing.TypeAlias = (
    typing.Literal["generator"]
    | typing.Literal["async"]
    | typing.Literal["asyncGenerator"]
)

def encode_suspension_kind(writer: BinaryWriter, value: SuspensionKind) -> None: ...
def decode_suspension_kind(reader: BinaryReader) -> SuspensionKind: ...
def to_json_suspension_kind(value: SuspensionKind) -> Json: ...
def from_json_suspension_kind(value: Json) -> SuspensionKind: ...

__all__ = [
    "Function",
    "encode_function",
    "decode_function",
    "to_json_function",
    "from_json_function",
    "FunctionBody",
    "encode_function_body",
    "decode_function_body",
    "to_json_function_body",
    "from_json_function_body",
    "InstructionIndex",
    "encode_instruction_index",
    "decode_instruction_index",
    "to_json_instruction_index",
    "from_json_instruction_index",
    "InstructionLocation",
    "encode_instruction_location",
    "decode_instruction_location",
    "to_json_instruction_location",
    "from_json_instruction_location",
    "AllocationMode",
    "encode_allocation_mode",
    "decode_allocation_mode",
    "to_json_allocation_mode",
    "from_json_allocation_mode",
    "SuspensionKind",
    "encode_suspension_kind",
    "decode_suspension_kind",
    "to_json_suspension_kind",
    "from_json_suspension_kind",
]
