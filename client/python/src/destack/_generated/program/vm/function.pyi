# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value
import destack._generated.program.function
import destack._generated.program.vm.instruction
import destack._generated.program.vm.range

@dataclass(frozen=True, slots=True)
class FunctionTable:
    """Lowered function registry owned by one program."""

    # lowered functions by dense index
    functions: Sequence[Function]
    # call target by function id
    target_by_id: Mapping[destack._generated.program.function.FunctionId, CallTarget]

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
    """Lowered function with executable code and frame metadata."""

    # runtime function id
    function: destack._generated.program.function.FunctionId
    # the logical frame layout for this function
    frame_layout: destack._generated.mir.metadata.frame.FrameLayoutId
    # function parameters
    parameters: destack._generated.program.vm.range.ArgumentRange
    # entry block index
    entry: int
    # contiguous instruction code
    code: Sequence[destack._generated.program.vm.instruction.Instruction]
    # lowered block ranges
    blocks: Sequence[Block]
    # pool of argument values referenced by ranges
    argument_pool: Sequence[destack._generated.mir.tree.value.Value]
    # pool of value move pairs referenced by ranges
    move_pool: Sequence[destack._generated.program.vm.range.MovePair]

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
class Block:
    """Lowered basic block position inside one function."""

    # original MIR block id
    mir_block: destack._generated.mir.tree.node.LocalNodeId
    # first instruction in the function code
    start: int
    # number of instructions in this block
    len: int
    # source MIR point for each lowered PC in this block
    mir_point_by_pc: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Block: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Block: ...

def encode_block(writer: BinaryWriter, value: Block) -> None: ...
def decode_block(reader: BinaryReader) -> Block: ...
def to_json_block(value: Block) -> Json: ...
def from_json_block(value: Json) -> Block: ...

@dataclass(frozen=True, slots=True)
class CallTargetImport:
    """The function id names one imported function."""

    kind: typing.Literal["import"] = "import"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class CallTargetLocal:
    """The function id names one lowered function."""

    local: int
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Program call target for one function id."""
CallTarget: typing.TypeAlias = CallTargetImport | CallTargetLocal

def encode_call_target(writer: BinaryWriter, value: CallTarget) -> None: ...
def decode_call_target(reader: BinaryReader) -> CallTarget: ...
def to_json_call_target(value: CallTarget) -> Json: ...
def from_json_call_target(value: Json) -> CallTarget: ...

@dataclass(frozen=True, slots=True)
class SwitchCase:
    """One lowered switch case."""

    # match value
    value: int
    # target block
    target: int
    # block parameter moves
    moves: destack._generated.program.vm.range.MoveRange

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SwitchCase: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SwitchCase: ...

def encode_switch_case(writer: BinaryWriter, value: SwitchCase) -> None: ...
def decode_switch_case(reader: BinaryReader) -> SwitchCase: ...
def to_json_switch_case(value: SwitchCase) -> Json: ...
def from_json_switch_case(value: Json) -> SwitchCase: ...

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
    "Block",
    "encode_block",
    "decode_block",
    "to_json_block",
    "from_json_block",
    "CallTarget",
    "encode_call_target",
    "decode_call_target",
    "to_json_call_target",
    "from_json_call_target",
    "CallTargetImport",
    "CallTargetLocal",
    "SwitchCase",
    "encode_switch_case",
    "decode_switch_case",
    "to_json_switch_case",
    "from_json_switch_case",
]
