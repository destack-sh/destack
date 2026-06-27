# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class Edge:
    """Control-flow edge selected by a terminator successor."""

    # the source block
    source: destack._generated.mir.tree.node.LocalNodeId
    # the successor field selected from the source terminator
    successor: Successor
    # the target block
    target: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Edge: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Edge: ...

def encode_edge(writer: BinaryWriter, value: Edge) -> None: ...
def decode_edge(reader: BinaryReader) -> Edge: ...
def to_json_edge(value: Edge) -> Json: ...
def from_json_edge(value: Json) -> Edge: ...

@dataclass(frozen=True, slots=True)
class SuccessorJump:
    """The target of an unconditional jump."""

    kind: typing.Literal["jump"] = "jump"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorCallReturn:
    """The return continuation of a call terminator."""

    kind: typing.Literal["callReturn"] = "callReturn"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorCallUnwind:
    """The unwind continuation of a call terminator."""

    kind: typing.Literal["callUnwind"] = "callUnwind"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorBranchThen:
    """The then target of a branch terminator."""

    kind: typing.Literal["branchThen"] = "branchThen"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorBranchElse:
    """The else target of a branch terminator."""

    kind: typing.Literal["branchElse"] = "branchElse"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorCheckSuccess:
    """The success target of a check terminator."""

    kind: typing.Literal["checkSuccess"] = "checkSuccess"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorCheckFailure:
    """The failure target of a check terminator."""

    kind: typing.Literal["checkFailure"] = "checkFailure"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorTrySuccess:
    """The success target of a fallible terminator."""

    kind: typing.Literal["trySuccess"] = "trySuccess"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorTryFailure:
    """The failure target of a fallible terminator."""

    kind: typing.Literal["tryFailure"] = "tryFailure"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorSwitchCase:
    """One switch case target."""

    # the matched case value
    value: int
    kind: typing.Literal["switchCase"] = "switchCase"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorSwitchDefault:
    """The default target of a switch terminator."""

    kind: typing.Literal["switchDefault"] = "switchDefault"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorYieldResume:
    """The resume target of a yield terminator."""

    kind: typing.Literal["yieldResume"] = "yieldResume"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SuccessorYieldUnwind:
    """The unwind target of a yield terminator."""

    kind: typing.Literal["yieldUnwind"] = "yieldUnwind"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Successor selected by one MIR terminator."""
Successor: typing.TypeAlias = (
    SuccessorJump
    | SuccessorCallReturn
    | SuccessorCallUnwind
    | SuccessorBranchThen
    | SuccessorBranchElse
    | SuccessorCheckSuccess
    | SuccessorCheckFailure
    | SuccessorTrySuccess
    | SuccessorTryFailure
    | SuccessorSwitchCase
    | SuccessorSwitchDefault
    | SuccessorYieldResume
    | SuccessorYieldUnwind
)

def encode_successor(writer: BinaryWriter, value: Successor) -> None: ...
def decode_successor(reader: BinaryReader) -> Successor: ...
def to_json_successor(value: Successor) -> Json: ...
def from_json_successor(value: Json) -> Successor: ...

__all__ = [
    "Edge",
    "encode_edge",
    "decode_edge",
    "to_json_edge",
    "from_json_edge",
    "Successor",
    "encode_successor",
    "decode_successor",
    "to_json_successor",
    "from_json_successor",
    "SuccessorJump",
    "SuccessorCallReturn",
    "SuccessorCallUnwind",
    "SuccessorBranchThen",
    "SuccessorBranchElse",
    "SuccessorCheckSuccess",
    "SuccessorCheckFailure",
    "SuccessorTrySuccess",
    "SuccessorTryFailure",
    "SuccessorSwitchCase",
    "SuccessorSwitchDefault",
    "SuccessorYieldResume",
    "SuccessorYieldUnwind",
]
