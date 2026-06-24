# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class AnnotationComment:
    """Comment annotation (like `//` or `/*`)."""

    position: AnnotationPosition
    string: destack._generated.core.string.StringId
    kind: typing.Literal["comment"] = "comment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation(self)


"""Annotation to a JS node."""
Annotation: typing.TypeAlias = AnnotationComment


def encode_annotation(writer: BinaryWriter, value: Annotation) -> None:
    """Encode one Annotation."""
    if value.kind == "comment":
        writer.write_unsigned(0)
        encode_annotation_position(writer, value.position)
        destack._generated.core.string.encode_string_id(writer, value.string)
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation(reader: BinaryReader) -> Annotation:
    """Decode one Annotation."""
    variant = reader.read_number()

    if variant == 0:
        position = decode_annotation_position(reader)
        string = destack._generated.core.string.decode_string_id(reader)

        return AnnotationComment(
            position=position,
            string=string,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_annotation(value: Annotation) -> Json:
    """Return one JSON value for one Annotation."""
    if value.kind == "comment":
        return {
            "kind": "comment",
            "position": to_json_annotation_position(value.position),
            "string": destack._generated.core.string.to_json_string_id(value.string),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_annotation(value: Json) -> Annotation:
    """Return one Annotation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "comment":
        return AnnotationComment(
            position=from_json_annotation_position(json_field(object_, "position")),
            string=destack._generated.core.string.from_json_string_id(
                json_field(object_, "string")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""The position of a JS annotation."""
AnnotationPosition: typing.TypeAlias = (
    typing.Literal["prefix"] | typing.Literal["infix"] | typing.Literal["postfix"]
)


def encode_annotation_position(writer: BinaryWriter, value: AnnotationPosition) -> None:
    """Encode one AnnotationPosition."""
    if value == "prefix":
        writer.write_unsigned(0)
    elif value == "infix":
        writer.write_unsigned(1)
    elif value == "postfix":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_position(reader: BinaryReader) -> AnnotationPosition:
    """Decode one AnnotationPosition."""
    variant = reader.read_number()

    if variant == 0:
        return "prefix"
    elif variant == 1:
        return "infix"
    elif variant == 2:
        return "postfix"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_annotation_position(value: AnnotationPosition) -> Json:
    """Return one JSON value for one AnnotationPosition."""
    return value


def from_json_annotation_position(value: Json) -> AnnotationPosition:
    """Return one AnnotationPosition from one JSON value."""
    variant = json_string(value)

    if variant == "prefix":
        return "prefix"
    elif variant == "infix":
        return "infix"
    elif variant == "postfix":
        return "postfix"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
