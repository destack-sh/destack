# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.symbol.language
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class AnnotationSegment:
    """Annotation invocations added by one DIR phase."""

    # the module id of the annotation segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first annotation invocation id owned by this table segment
    first_invocation_id: int
    # annotation invocations owned by this segment
    invocations: Sequence[AnnotationInvocation]
    # annotation invocations attached to each owner
    invocations_by_owner: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny, Sequence[LocalAnnotationId]
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationSegment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationSegment: ...

def encode_annotation_segment(
    writer: BinaryWriter, value: AnnotationSegment
) -> None: ...
def decode_annotation_segment(reader: BinaryReader) -> AnnotationSegment: ...
def to_json_annotation_segment(value: AnnotationSegment) -> Json: ...
def from_json_annotation_segment(value: Json) -> AnnotationSegment: ...

@dataclass(frozen=True, slots=True)
class AnnotationInvocation:
    """Checked annotation invocation attached to one owner node."""

    # the decorator node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the node annotated by this invocation
    owner: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the decorator target expression
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the resolved annotation target
    resolution: AnnotationTarget
    # the invocation arguments
    arguments: Sequence[AnnotationArgument]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationInvocation: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationInvocation: ...

def encode_annotation_invocation(
    writer: BinaryWriter, value: AnnotationInvocation
) -> None: ...
def decode_annotation_invocation(reader: BinaryReader) -> AnnotationInvocation: ...
def to_json_annotation_invocation(value: AnnotationInvocation) -> Json: ...
def from_json_annotation_invocation(value: Json) -> AnnotationInvocation: ...

@dataclass(frozen=True, slots=True)
class AnnotationTargetLanguageItem:
    """Compiler language item annotation."""

    language_item: destack._generated.dir.symbol.language.LanguageItem
    kind: typing.Literal["languageItem"] = "languageItem"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AnnotationTargetSymbol:
    """User-defined annotation symbol."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AnnotationTargetUnknown:
    """Unresolved or non-symbol annotation target."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Resolved annotation target."""
AnnotationTarget: typing.TypeAlias = (
    AnnotationTargetLanguageItem | AnnotationTargetSymbol | AnnotationTargetUnknown
)

def encode_annotation_target(writer: BinaryWriter, value: AnnotationTarget) -> None: ...
def decode_annotation_target(reader: BinaryReader) -> AnnotationTarget: ...
def to_json_annotation_target(value: AnnotationTarget) -> Json: ...
def from_json_annotation_target(value: Json) -> AnnotationTarget: ...

@dataclass(frozen=True, slots=True)
class AnnotationArgument:
    """Checked annotation argument."""

    # the argument node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the committed static value when one exists
    value: destack._generated.dir.tree.static.GlobalStaticId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationArgument: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationArgument: ...

def encode_annotation_argument(
    writer: BinaryWriter, value: AnnotationArgument
) -> None: ...
def decode_annotation_argument(reader: BinaryReader) -> AnnotationArgument: ...
def to_json_annotation_argument(value: AnnotationArgument) -> Json: ...
def from_json_annotation_argument(value: Json) -> AnnotationArgument: ...

"""Unique identifier for an annotation invocation."""
LocalAnnotationId: typing.TypeAlias = int

def encode_local_annotation_id(
    writer: BinaryWriter, value: LocalAnnotationId
) -> None: ...
def decode_local_annotation_id(reader: BinaryReader) -> LocalAnnotationId: ...
def to_json_local_annotation_id(value: LocalAnnotationId) -> Json: ...
def from_json_local_annotation_id(value: Json) -> LocalAnnotationId: ...

__all__ = [
    "AnnotationSegment",
    "encode_annotation_segment",
    "decode_annotation_segment",
    "to_json_annotation_segment",
    "from_json_annotation_segment",
    "AnnotationInvocation",
    "encode_annotation_invocation",
    "decode_annotation_invocation",
    "to_json_annotation_invocation",
    "from_json_annotation_invocation",
    "AnnotationTarget",
    "encode_annotation_target",
    "decode_annotation_target",
    "to_json_annotation_target",
    "from_json_annotation_target",
    "AnnotationTargetLanguageItem",
    "AnnotationTargetSymbol",
    "AnnotationTargetUnknown",
    "AnnotationArgument",
    "encode_annotation_argument",
    "decode_annotation_argument",
    "to_json_annotation_argument",
    "from_json_annotation_argument",
    "LocalAnnotationId",
    "encode_local_annotation_id",
    "decode_local_annotation_id",
    "to_json_local_annotation_id",
    "from_json_local_annotation_id",
]
