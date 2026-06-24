# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.metadata.drop import (
    DropMetadataImpl,
)
from destack._impl.mir.metadata.drop import (
    DropGlueImpl,
)
from destack._impl.mir.metadata.drop import (
    DropHookImpl,
)

import destack._generated.mir.metadata.dispatch
import destack._generated.mir.tree.node

@dataclass(frozen=True, slots=True)
class DropMetadata(DropMetadataImpl):
    """Drop metadata for one MIR module."""

    # full drop glue keyed by type id
    glue_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropGlue]
    # user-authored drop hooks keyed by type id
    hooks_by_type: Mapping[destack._generated.mir.tree.node.LocalNodeId, DropHook]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DropMetadata: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DropMetadata: ...

def encode_drop_metadata(writer: BinaryWriter, value: DropMetadata) -> None: ...
def decode_drop_metadata(reader: BinaryReader) -> DropMetadata: ...
def to_json_drop_metadata(value: DropMetadata) -> Json: ...
def from_json_drop_metadata(value: Json) -> DropMetadata: ...

@dataclass(frozen=True, slots=True)
class DropGlueNone(DropGlueImpl):
    """No drop glue is required."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DropGlueGenerated(DropGlueImpl):
    """Call compiler-generated drop glue."""

    # the drop function
    function: destack._generated.mir.tree.node.LocalNodeId
    kind: typing.Literal["generated"] = "generated"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DropGlueDynamic(DropGlueImpl):
    """Dispatch through a runtime drop slot."""

    # the drop dispatch slot
    slot: destack._generated.mir.metadata.dispatch.DispatchSlot
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
class DropHook(DropHookImpl):
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
    "DropMetadata",
    "encode_drop_metadata",
    "decode_drop_metadata",
    "to_json_drop_metadata",
    "from_json_drop_metadata",
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
