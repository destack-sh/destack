# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.table.dispatch
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class DropTable:
    """Drop table for one MIR module."""

    # full drop glue keyed by type id
    glue_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropGlue]
    # user-authored drop hooks keyed by type id
    hooks_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropHook]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DropTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DropTable: ...

def encode_drop_table(writer: BinaryWriter, value: DropTable) -> None: ...
def decode_drop_table(reader: BinaryReader) -> DropTable: ...
def to_json_drop_table(value: DropTable) -> Json: ...
def from_json_drop_table(value: Json) -> DropTable: ...

@dataclass(frozen=True, slots=True)
class DropGlueNone:
    """No drop glue is required."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DropGlueGenerated:
    """Call compiler-generated drop glue."""

    # the drop function
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["generated"] = "generated"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DropGlueDynamic:
    """Dispatch through a runtime drop slot."""

    # the drop dispatch slot
    slot: destack._generated.mir.table.dispatch.DispatchSlot
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Full drop glue selected for one MIR type."""
DropGlue: typing.TypeAlias = DropGlueNone | DropGlueGenerated | DropGlueDynamic

def encode_drop_glue(writer: BinaryWriter, value: DropGlue) -> None: ...
def decode_drop_glue(reader: BinaryReader) -> DropGlue: ...
def to_json_drop_glue(value: DropGlue) -> Json: ...
def from_json_drop_glue(value: Json) -> DropGlue: ...

@dataclass(frozen=True, slots=True)
class DropHook:
    """User-authored drop hook for one MIR type."""

    # the hook function
    function: destack._generated.mir.tree.node.LocalNodeId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DropHook: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DropHook: ...

def encode_drop_hook(writer: BinaryWriter, value: DropHook) -> None: ...
def decode_drop_hook(reader: BinaryReader) -> DropHook: ...
def to_json_drop_hook(value: DropHook) -> Json: ...
def from_json_drop_hook(value: Json) -> DropHook: ...

__all__ = [
    "DropTable",
    "encode_drop_table",
    "decode_drop_table",
    "to_json_drop_table",
    "from_json_drop_table",
    "DropGlue",
    "encode_drop_glue",
    "decode_drop_glue",
    "to_json_drop_glue",
    "from_json_drop_glue",
    "DropGlueNone",
    "DropGlueGenerated",
    "DropGlueDynamic",
    "DropHook",
    "encode_drop_hook",
    "decode_drop_hook",
    "to_json_drop_hook",
    "from_json_drop_hook",
]
