# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.table.memory
import destack._generated.mir.tree.call
import destack._generated.mir.tree.memory
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class EffectTable:
    """Function and call effect tables for one MIR module."""

    # effects keyed by function id
    functions: Mapping[destack._generated.mir.tree.node.LocalNodeId, FunctionEffect]
    # effects keyed by callsite
    calls: Mapping[destack._generated.mir.tree.call.CallSite, CallEffect]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> EffectTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> EffectTable: ...

def encode_effect_table(writer: BinaryWriter, value: EffectTable) -> None: ...
def decode_effect_table(reader: BinaryReader) -> EffectTable: ...
def to_json_effect_table(value: EffectTable) -> Json: ...
def from_json_effect_table(value: Json) -> EffectTable: ...

@dataclass(frozen=True, slots=True)
class FunctionEffect:
    """Effects for one function body or declaration."""

    # memory touched by this function
    memory: MemoryEffect
    # behavioral effects of this function
    behavior: FunctionBehavior

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionEffect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionEffect: ...

def encode_function_effect(writer: BinaryWriter, value: FunctionEffect) -> None: ...
def decode_function_effect(reader: BinaryReader) -> FunctionEffect: ...
def to_json_function_effect(value: FunctionEffect) -> Json: ...
def from_json_function_effect(value: Json) -> FunctionEffect: ...

@dataclass(frozen=True, slots=True)
class MemoryEffect:
    """Memory access effect for a call or operation."""

    # storage regions this operation may read
    read: destack._generated.mir.tree.memory.StorageSet
    # storage regions this operation may write
    write: destack._generated.mir.tree.memory.StorageSet

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MemoryEffect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MemoryEffect: ...

def encode_memory_effect(writer: BinaryWriter, value: MemoryEffect) -> None: ...
def decode_memory_effect(reader: BinaryReader) -> MemoryEffect: ...
def to_json_memory_effect(value: MemoryEffect) -> Json: ...
def from_json_memory_effect(value: Json) -> MemoryEffect: ...

@dataclass(frozen=True, slots=True)
class FunctionBehavior:
    """Behavioral effects for calls and functions."""

    # determinism for this operation
    determinism: Determinism
    # whether this operation may suspend execution
    suspend: SuspendBehavior
    # panic behavior for this operation
    panic: PanicBehavior
    # return behavior for this operation
    return_behavior: ReturnBehavior
    # whether optimization must not duplicate this operation
    must_not_duplicate: bool
    # whether this operation may allocate storage
    allocates: bool
    # whether this operation may free storage
    frees: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionBehavior: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionBehavior: ...

def encode_function_behavior(writer: BinaryWriter, value: FunctionBehavior) -> None: ...
def decode_function_behavior(reader: BinaryReader) -> FunctionBehavior: ...
def to_json_function_behavior(value: FunctionBehavior) -> Json: ...
def from_json_function_behavior(value: Json) -> FunctionBehavior: ...

"""Determinism for a call or function."""
Determinism: typing.TypeAlias = (
    typing.Literal["deterministic"] | typing.Literal["nonDeterministic"]
)

def encode_determinism(writer: BinaryWriter, value: Determinism) -> None: ...
def decode_determinism(reader: BinaryReader) -> Determinism: ...
def to_json_determinism(value: Determinism) -> Json: ...
def from_json_determinism(value: Json) -> Determinism: ...

"""Suspend behavior for a call or function."""
SuspendBehavior: typing.TypeAlias = (
    typing.Literal["cannotSuspend"] | typing.Literal["maySuspend"]
)

def encode_suspend_behavior(writer: BinaryWriter, value: SuspendBehavior) -> None: ...
def decode_suspend_behavior(reader: BinaryReader) -> SuspendBehavior: ...
def to_json_suspend_behavior(value: SuspendBehavior) -> Json: ...
def from_json_suspend_behavior(value: Json) -> SuspendBehavior: ...

"""Panic behavior for a call or function."""
PanicBehavior: typing.TypeAlias = (
    typing.Literal["cannotPanic"] | typing.Literal["mayPanic"]
)

def encode_panic_behavior(writer: BinaryWriter, value: PanicBehavior) -> None: ...
def decode_panic_behavior(reader: BinaryReader) -> PanicBehavior: ...
def to_json_panic_behavior(value: PanicBehavior) -> Json: ...
def from_json_panic_behavior(value: Json) -> PanicBehavior: ...

"""Return behavior for a call or function."""
ReturnBehavior: typing.TypeAlias = (
    typing.Literal["mayReturn"]
    | typing.Literal["noReturn"]
    | typing.Literal["willReturn"]
)

def encode_return_behavior(writer: BinaryWriter, value: ReturnBehavior) -> None: ...
def decode_return_behavior(reader: BinaryReader) -> ReturnBehavior: ...
def to_json_return_behavior(value: ReturnBehavior) -> Json: ...
def from_json_return_behavior(value: Json) -> ReturnBehavior: ...

@dataclass(frozen=True, slots=True)
class CallEffect:
    """Effects for one callsite."""

    # memory touched by this call
    memory: MemoryEffect
    # behavioral effects of this call
    behavior: FunctionBehavior
    # resolved direct target when dispatch analysis proves one
    target: destack._generated.mir.tree.node.LocalNodeId | None
    # argument memory behavior when known
    arguments: Sequence[destack._generated.mir.table.memory.CallArgumentEffect]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallEffect: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallEffect: ...

def encode_call_effect(writer: BinaryWriter, value: CallEffect) -> None: ...
def decode_call_effect(reader: BinaryReader) -> CallEffect: ...
def to_json_call_effect(value: CallEffect) -> Json: ...
def from_json_call_effect(value: Json) -> CallEffect: ...

__all__ = [
    "EffectTable",
    "encode_effect_table",
    "decode_effect_table",
    "to_json_effect_table",
    "from_json_effect_table",
    "FunctionEffect",
    "encode_function_effect",
    "decode_function_effect",
    "to_json_function_effect",
    "from_json_function_effect",
    "MemoryEffect",
    "encode_memory_effect",
    "decode_memory_effect",
    "to_json_memory_effect",
    "from_json_memory_effect",
    "FunctionBehavior",
    "encode_function_behavior",
    "decode_function_behavior",
    "to_json_function_behavior",
    "from_json_function_behavior",
    "Determinism",
    "encode_determinism",
    "decode_determinism",
    "to_json_determinism",
    "from_json_determinism",
    "SuspendBehavior",
    "encode_suspend_behavior",
    "decode_suspend_behavior",
    "to_json_suspend_behavior",
    "from_json_suspend_behavior",
    "PanicBehavior",
    "encode_panic_behavior",
    "decode_panic_behavior",
    "to_json_panic_behavior",
    "from_json_panic_behavior",
    "ReturnBehavior",
    "encode_return_behavior",
    "decode_return_behavior",
    "to_json_return_behavior",
    "from_json_return_behavior",
    "CallEffect",
    "encode_call_effect",
    "decode_call_effect",
    "to_json_call_effect",
    "from_json_call_effect",
]
