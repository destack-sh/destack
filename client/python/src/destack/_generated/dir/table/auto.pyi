# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class AutoSegment:
    """Auto-derived implementations added by one DIR phase."""

    # the module id of the auto segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # auto-derived implementations in emission order
    implementations: Sequence[AutoDerivedImplementation]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AutoSegment: ...

def encode_auto_segment(writer: BinaryWriter, value: AutoSegment) -> None: ...
def decode_auto_segment(reader: BinaryReader) -> AutoSegment: ...
def to_json_auto_segment(value: AutoSegment) -> Json: ...
def from_json_auto_segment(value: Json) -> AutoSegment: ...

@dataclass(frozen=True, slots=True)
class AutoDerivedImplementation:
    """Generated implementation for one auto interface."""

    # the source node that requested this implementation
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generated interface
    interface: AutoInterface
    # the implemented type
    target: destack._generated.dir.type.type.GlobalTypeId
    # the generated members
    members: Sequence[AutoImplementationMember]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoDerivedImplementation: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AutoDerivedImplementation: ...

def encode_auto_derived_implementation(
    writer: BinaryWriter, value: AutoDerivedImplementation
) -> None: ...
def decode_auto_derived_implementation(
    reader: BinaryReader,
) -> AutoDerivedImplementation: ...
def to_json_auto_derived_implementation(value: AutoDerivedImplementation) -> Json: ...
def from_json_auto_derived_implementation(value: Json) -> AutoDerivedImplementation: ...

"""Interface whose implementation can be provided by compiler rules."""
AutoInterface: typing.TypeAlias = (
    typing.Literal["compare"]
    | typing.Literal["concrete"]
    | typing.Literal["copy"]
    | typing.Literal["clone"]
    | typing.Literal["debug"]
    | typing.Literal["default"]
    | typing.Literal["deserialize"]
    | typing.Literal["dynamicSafe"]
    | typing.Literal["equal"]
    | typing.Literal["float"]
    | typing.Literal["hash"]
    | typing.Literal["integer"]
    | typing.Literal["overwriteStable"]
    | typing.Literal["partialCompare"]
    | typing.Literal["partialEqual"]
    | typing.Literal["serialize"]
    | typing.Literal["send"]
    | typing.Literal["sync"]
    | typing.Literal["unpin"]
    | typing.Literal["zeroable"]
)

def encode_auto_interface(writer: BinaryWriter, value: AutoInterface) -> None: ...
def decode_auto_interface(reader: BinaryReader) -> AutoInterface: ...
def to_json_auto_interface(value: AutoInterface) -> Json: ...
def from_json_auto_interface(value: Json) -> AutoInterface: ...

@dataclass(frozen=True, slots=True)
class AutoImplementationMember:
    """Member generated for one auto-derived implementation."""

    # the source node that owns the generated member
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generated member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generated member type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoImplementationMember: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AutoImplementationMember: ...

def encode_auto_implementation_member(
    writer: BinaryWriter, value: AutoImplementationMember
) -> None: ...
def decode_auto_implementation_member(
    reader: BinaryReader,
) -> AutoImplementationMember: ...
def to_json_auto_implementation_member(value: AutoImplementationMember) -> Json: ...
def from_json_auto_implementation_member(value: Json) -> AutoImplementationMember: ...

__all__ = [
    "AutoSegment",
    "encode_auto_segment",
    "decode_auto_segment",
    "to_json_auto_segment",
    "from_json_auto_segment",
    "AutoDerivedImplementation",
    "encode_auto_derived_implementation",
    "decode_auto_derived_implementation",
    "to_json_auto_derived_implementation",
    "from_json_auto_derived_implementation",
    "AutoInterface",
    "encode_auto_interface",
    "decode_auto_interface",
    "to_json_auto_interface",
    "from_json_auto_interface",
    "AutoImplementationMember",
    "encode_auto_implementation_member",
    "decode_auto_implementation_member",
    "to_json_auto_implementation_member",
    "from_json_auto_implementation_member",
]
