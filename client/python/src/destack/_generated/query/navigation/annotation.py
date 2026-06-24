# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.query.core.target
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class AnnotationsRequest:
    """Request payload for annotation queries."""

    # the query scope
    scope: AnnotationScope
    # the annotation name filter
    name: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotations_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationsRequest:
        """Decode one AnnotationsRequest."""
        return decode_annotations_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotations_request(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationsRequest:
        """Return one AnnotationsRequest from one JSON value."""
        return from_json_annotations_request(value)


def encode_annotations_request(writer: BinaryWriter, value: AnnotationsRequest) -> None:
    """Encode one AnnotationsRequest."""
    encode_annotation_scope(writer, value.scope)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)


def decode_annotations_request(reader: BinaryReader) -> AnnotationsRequest:
    """Decode one AnnotationsRequest."""
    scope = decode_annotation_scope(reader)
    name = reader.read_option(lambda: reader.read_string())

    return AnnotationsRequest(
        scope=scope,
        name=name,
    )


def to_json_annotations_request(value: AnnotationsRequest) -> Json:
    """Return one JSON value for one AnnotationsRequest."""
    return {
        "scope": to_json_annotation_scope(value.scope),
        **({} if value.name is None else {"name": value.name}),
    }


def from_json_annotations_request(value: Json) -> AnnotationsRequest:
    """Return one AnnotationsRequest from one JSON value."""
    object_ = json_object(value)

    return AnnotationsRequest(
        scope=from_json_annotation_scope(json_field(object_, "scope")),
        name=json_optional(object_, "name", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class AnnotationScopeModule:
    """One module."""

    module: destack._generated.query.core.target.QueryModule
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_scope(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_scope(self)


@dataclass(frozen=True, slots=True)
class AnnotationScopeWorkspace:
    """Workspace profiles."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_scope(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_scope(self)


"""Scope for annotation queries."""
AnnotationScope: typing.TypeAlias = AnnotationScopeModule | AnnotationScopeWorkspace


def encode_annotation_scope(writer: BinaryWriter, value: AnnotationScope) -> None:
    """Encode one AnnotationScope."""
    if value.kind == "module":
        writer.write_unsigned(0)
        destack._generated.query.core.target.encode_query_module(writer, value.module)
    elif value.kind == "workspace":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.profile_ids))
        for item_value_profile_ids_0 in value.profile_ids:
            destack._generated.source.file.model.profile.encode_profile_id(
                writer, item_value_profile_ids_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_scope(reader: BinaryReader) -> AnnotationScope:
    """Decode one AnnotationScope."""
    variant = reader.read_number()

    if variant == 0:
        module = destack._generated.query.core.target.decode_query_module(reader)

        return AnnotationScopeModule(module=module)
    elif variant == 1:
        profile_ids = [
            destack._generated.source.file.model.profile.decode_profile_id(reader)
            for _ in range(reader.read_number())
        ]

        return AnnotationScopeWorkspace(
            profile_ids=profile_ids,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_annotation_scope(value: AnnotationScope) -> Json:
    """Return one JSON value for one AnnotationScope."""
    if value.kind == "module":
        return {
            "kind": "module",
            "module": destack._generated.query.core.target.to_json_query_module(
                value.module
            ),
        }
    elif value.kind == "workspace":
        return {
            "kind": "workspace",
            "profileIds": [
                destack._generated.source.file.model.profile.to_json_profile_id(item_0)
                for item_0 in value.profile_ids
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_annotation_scope(value: Json) -> AnnotationScope:
    """Return one AnnotationScope from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "module":
        return AnnotationScopeModule(
            module=destack._generated.query.core.target.from_json_query_module(
                json_field(object_, "module")
            )
        )
    elif kind == "workspace":
        return AnnotationScopeWorkspace(
            profile_ids=[
                destack._generated.source.file.model.profile.from_json_profile_id(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "profileIds"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class AnnotationsResponse:
    """Response payload for annotation queries."""

    # matching annotations
    annotations: Sequence[AnnotationItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotations_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationsResponse:
        """Decode one AnnotationsResponse."""
        return decode_annotations_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotations_response(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationsResponse:
        """Return one AnnotationsResponse from one JSON value."""
        return from_json_annotations_response(value)


def encode_annotations_response(
    writer: BinaryWriter, value: AnnotationsResponse
) -> None:
    """Encode one AnnotationsResponse."""
    writer.write_unsigned(len(value.annotations))
    for item_value_annotations_0 in value.annotations:
        encode_annotation_item(writer, item_value_annotations_0)


def decode_annotations_response(reader: BinaryReader) -> AnnotationsResponse:
    """Decode one AnnotationsResponse."""
    annotations = [decode_annotation_item(reader) for _ in range(reader.read_number())]

    return AnnotationsResponse(
        annotations=annotations,
    )


def to_json_annotations_response(value: AnnotationsResponse) -> Json:
    """Return one JSON value for one AnnotationsResponse."""
    return {
        "annotations": [
            to_json_annotation_item(item_0) for item_0 in value.annotations
        ],
    }


def from_json_annotations_response(value: Json) -> AnnotationsResponse:
    """Return one AnnotationsResponse from one JSON value."""
    object_ = json_object(value)

    return AnnotationsResponse(
        annotations=[
            from_json_annotation_item(item_0)
            for item_0 in json_array(json_field(object_, "annotations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AnnotationItem:
    """One annotation query item."""

    # the annotation name when syntactically known
    name: str | None
    # the decorator expression target
    decorator: destack._generated.query.core.target.QueryTarget
    # the annotated target
    target: destack._generated.query.core.target.QueryTarget
    # the resolved annotation role
    role: AnnotationRole

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_annotation_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationItem:
        """Decode one AnnotationItem."""
        return decode_annotation_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_annotation_item(self)

    @classmethod
    def from_json(cls, value: Json) -> AnnotationItem:
        """Return one AnnotationItem from one JSON value."""
        return from_json_annotation_item(value)


def encode_annotation_item(writer: BinaryWriter, value: AnnotationItem) -> None:
    """Encode one AnnotationItem."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    destack._generated.query.core.target.encode_query_target(writer, value.decorator)
    destack._generated.query.core.target.encode_query_target(writer, value.target)
    encode_annotation_role(writer, value.role)


def decode_annotation_item(reader: BinaryReader) -> AnnotationItem:
    """Decode one AnnotationItem."""
    name = reader.read_option(lambda: reader.read_string())
    decorator = destack._generated.query.core.target.decode_query_target(reader)
    target = destack._generated.query.core.target.decode_query_target(reader)
    role = decode_annotation_role(reader)

    return AnnotationItem(
        name=name,
        decorator=decorator,
        target=target,
        role=role,
    )


def to_json_annotation_item(value: AnnotationItem) -> Json:
    """Return one JSON value for one AnnotationItem."""
    return {
        **({} if value.name is None else {"name": value.name}),
        "decorator": destack._generated.query.core.target.to_json_query_target(
            value.decorator
        ),
        "target": destack._generated.query.core.target.to_json_query_target(
            value.target
        ),
        "role": to_json_annotation_role(value.role),
    }


def from_json_annotation_item(value: Json) -> AnnotationItem:
    """Return one AnnotationItem from one JSON value."""
    object_ = json_object(value)

    return AnnotationItem(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        decorator=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "decorator")
        ),
        target=destack._generated.query.core.target.from_json_query_target(
            json_field(object_, "target")
        ),
        role=from_json_annotation_role(json_field(object_, "role")),
    )


"""Role of one annotation expression."""
AnnotationRole: typing.TypeAlias = (
    typing.Literal["annotation"]
    | typing.Literal["decorator"]
    | typing.Literal["unknown"]
)


def encode_annotation_role(writer: BinaryWriter, value: AnnotationRole) -> None:
    """Encode one AnnotationRole."""
    if value == "annotation":
        writer.write_unsigned(0)
    elif value == "decorator":
        writer.write_unsigned(1)
    elif value == "unknown":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_role(reader: BinaryReader) -> AnnotationRole:
    """Decode one AnnotationRole."""
    variant = reader.read_number()

    if variant == 0:
        return "annotation"
    elif variant == 1:
        return "decorator"
    elif variant == 2:
        return "unknown"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_annotation_role(value: AnnotationRole) -> Json:
    """Return one JSON value for one AnnotationRole."""
    return value


def from_json_annotation_role(value: Json) -> AnnotationRole:
    """Return one AnnotationRole from one JSON value."""
    variant = json_string(value)

    if variant == "annotation":
        return "annotation"
    elif variant == "decorator":
        return "decorator"
    elif variant == "unknown":
        return "unknown"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "AnnotationsRequest",
    "encode_annotations_request",
    "decode_annotations_request",
    "to_json_annotations_request",
    "from_json_annotations_request",
    "AnnotationScope",
    "encode_annotation_scope",
    "decode_annotation_scope",
    "to_json_annotation_scope",
    "from_json_annotation_scope",
    "AnnotationScopeModule",
    "AnnotationScopeWorkspace",
    "AnnotationsResponse",
    "encode_annotations_response",
    "decode_annotations_response",
    "to_json_annotations_response",
    "from_json_annotations_response",
    "AnnotationItem",
    "encode_annotation_item",
    "decode_annotation_item",
    "to_json_annotation_item",
    "from_json_annotation_item",
    "AnnotationRole",
    "encode_annotation_role",
    "decode_annotation_role",
    "to_json_annotation_role",
    "from_json_annotation_role",
]
