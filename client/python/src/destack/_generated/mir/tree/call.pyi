# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.tree.node
import destack._generated.mir.tree.value

@dataclass(frozen=True, slots=True)
class Call:
    """Shared payload for one call-like instruction or terminator."""

    # the call arguments
    arguments: destack._generated.mir.tree.value.ValueSlice
    # the signature type for the callee
    signature: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Call: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Call: ...

def encode_call(writer: BinaryWriter, value: Call) -> None: ...
def decode_call(reader: BinaryReader) -> Call: ...
def to_json_call(value: Call) -> Json: ...
def from_json_call(value: Json) -> Call: ...

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

__all__ = [
    "Call",
    "encode_call",
    "decode_call",
    "to_json_call",
    "from_json_call",
    "CallSite",
    "encode_call_site",
    "decode_call_site",
    "to_json_call_site",
    "from_json_call_site",
    "CallSiteInstruction",
    "CallSiteTerminator",
]
