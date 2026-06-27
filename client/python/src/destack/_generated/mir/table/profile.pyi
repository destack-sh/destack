# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.call
import destack._generated.mir.tree.edge
import destack._generated.mir.tree.node
import destack._generated.mir.tree.value

"""Identifier for one emitted profile counter, positional within a function."""
CounterId: typing.TypeAlias = int

def encode_counter_id(writer: BinaryWriter, value: CounterId) -> None: ...
def decode_counter_id(reader: BinaryReader) -> CounterId: ...
def to_json_counter_id(value: CounterId) -> Json: ...
def from_json_counter_id(value: Json) -> CounterId: ...

@dataclass(frozen=True, slots=True)
class ProfileTable:
    """Static profile counter table for one MIR module."""

    # per-function profile counter tables
    functions: Mapping[
        destack._generated.mir.tree.node.LocalNodeId, FunctionProfileTable
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProfileTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProfileTable: ...

def encode_profile_table(writer: BinaryWriter, value: ProfileTable) -> None: ...
def decode_profile_table(reader: BinaryReader) -> ProfileTable: ...
def to_json_profile_table(value: ProfileTable) -> Json: ...
def from_json_profile_table(value: Json) -> ProfileTable: ...

@dataclass(frozen=True, slots=True)
class FunctionProfileTable:
    """Static profile counter table for one function."""

    # control-flow hash guarding against stale profile application
    hash: FunctionHash
    # profile points indexed by counter id
    points: Sequence[ProfilePoint]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionProfileTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FunctionProfileTable: ...

def encode_function_profile_table(
    writer: BinaryWriter, value: FunctionProfileTable
) -> None: ...
def decode_function_profile_table(reader: BinaryReader) -> FunctionProfileTable: ...
def to_json_function_profile_table(value: FunctionProfileTable) -> Json: ...
def from_json_function_profile_table(value: Json) -> FunctionProfileTable: ...

"""Structural hash of a function's profiled control flow, for stale detection."""
FunctionHash: typing.TypeAlias = int

def encode_function_hash(writer: BinaryWriter, value: FunctionHash) -> None: ...
def decode_function_hash(reader: BinaryReader) -> FunctionHash: ...
def to_json_function_hash(value: FunctionHash) -> Json: ...
def from_json_function_hash(value: Json) -> FunctionHash: ...

@dataclass(frozen=True, slots=True)
class ProfilePointEntry:
    """Function entry execution count."""

    kind: typing.Literal["entry"] = "entry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointEdge:
    """Control-flow edge count."""

    edge: destack._generated.mir.tree.edge.Edge
    kind: typing.Literal["edge"] = "edge"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointValue:
    """Value distribution for one SSA value."""

    value: destack._generated.mir.tree.value.Value
    kind: typing.Literal["value"] = "value"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointCallTarget:
    """Observed call target distribution for one callsite."""

    call_target: destack._generated.mir.tree.call.CallSite
    kind: typing.Literal["callTarget"] = "callTarget"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointReceiverType:
    """Observed receiver type distribution for one callsite."""

    receiver_type: destack._generated.mir.tree.call.CallSite
    kind: typing.Literal["receiverType"] = "receiverType"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointAllocation:
    """Observed allocation behavior for one allocation instruction."""

    allocation: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["allocation"] = "allocation"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ProfilePointSuspension:
    """Suspension behavior for one instruction."""

    suspension: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["suspension"] = "suspension"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Semantic meaning of one profile counter."""
ProfilePoint: typing.TypeAlias = (
    ProfilePointEntry
    | ProfilePointEdge
    | ProfilePointValue
    | ProfilePointCallTarget
    | ProfilePointReceiverType
    | ProfilePointAllocation
    | ProfilePointSuspension
)

def encode_profile_point(writer: BinaryWriter, value: ProfilePoint) -> None: ...
def decode_profile_point(reader: BinaryReader) -> ProfilePoint: ...
def to_json_profile_point(value: ProfilePoint) -> Json: ...
def from_json_profile_point(value: Json) -> ProfilePoint: ...

__all__ = [
    "CounterId",
    "encode_counter_id",
    "decode_counter_id",
    "to_json_counter_id",
    "from_json_counter_id",
    "ProfileTable",
    "encode_profile_table",
    "decode_profile_table",
    "to_json_profile_table",
    "from_json_profile_table",
    "FunctionProfileTable",
    "encode_function_profile_table",
    "decode_function_profile_table",
    "to_json_function_profile_table",
    "from_json_function_profile_table",
    "FunctionHash",
    "encode_function_hash",
    "decode_function_hash",
    "to_json_function_hash",
    "from_json_function_hash",
    "ProfilePoint",
    "encode_profile_point",
    "decode_profile_point",
    "to_json_profile_point",
    "from_json_profile_point",
    "ProfilePointEntry",
    "ProfilePointEdge",
    "ProfilePointValue",
    "ProfilePointCallTarget",
    "ProfilePointReceiverType",
    "ProfilePointAllocation",
    "ProfilePointSuspension",
]
