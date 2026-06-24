# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.effect import (
    MemoryEffectImpl,
)
from destack._impl.mir.metadata.effect import (
    FunctionBehaviorImpl,
)

import destack._generated.mir.tree.memory

@dataclass(frozen=True, slots=True)
class MemoryEffect(MemoryEffectImpl):
    """Memory effect summary for a call or operation."""

    # whether the operation may read memory
    reads: bool
    # whether the operation may write memory
    writes: bool
    # the memory spaces that may be accessed
    spaces: destack._generated.mir.tree.memory.SpaceSet

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
class FunctionBehavior(FunctionBehaviorImpl):
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

__all__ = [
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
]
