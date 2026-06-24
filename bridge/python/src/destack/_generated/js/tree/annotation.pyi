# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class AnnotationComment:
    """Comment annotation (like `//` or `/*`)."""

    position: AnnotationPosition
    string: destack._generated.core.string.StringId
    kind: typing.Literal["comment"] = "comment"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Annotation to a JS node."""
Annotation: typing.TypeAlias = AnnotationComment

def encode_annotation(writer: BinaryWriter, value: Annotation) -> None: ...
def decode_annotation(reader: BinaryReader) -> Annotation: ...
def to_json_annotation(value: Annotation) -> Json: ...
def from_json_annotation(value: Json) -> Annotation: ...

"""The position of a JS annotation."""
AnnotationPosition: typing.TypeAlias = (
    typing.Literal["prefix"] | typing.Literal["infix"] | typing.Literal["postfix"]
)

def encode_annotation_position(
    writer: BinaryWriter, value: AnnotationPosition
) -> None: ...
def decode_annotation_position(reader: BinaryReader) -> AnnotationPosition: ...
def to_json_annotation_position(value: AnnotationPosition) -> Json: ...
def from_json_annotation_position(value: Json) -> AnnotationPosition: ...

__all__ = [
    "Annotation",
    "encode_annotation",
    "decode_annotation",
    "to_json_annotation",
    "from_json_annotation",
    "AnnotationComment",
    "AnnotationPosition",
    "encode_annotation_position",
    "decode_annotation_position",
    "to_json_annotation_position",
    "from_json_annotation_position",
]
