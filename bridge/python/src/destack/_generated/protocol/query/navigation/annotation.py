# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.file.model.profile

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryModule,
        QueryTarget,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )


@dataclass(frozen=True, slots=True)
class AnnotationsRequest:
    """Request payload for annotation queries."""

    """The query scope."""
    scope: AnnotationScope
    """The annotation name filter."""
    name: str | None


def encode_annotations_request(writer: Writer, value: AnnotationsRequest) -> None:
    encode_annotation_scope(writer, value.scope)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)


def decode_annotations_request(reader: Reader) -> AnnotationsRequest:
    field_0 = decode_annotation_scope(reader)
    field_1 = reader.read_option(lambda: reader.read_string())

    return AnnotationsRequest(
        scope=field_0,
        name=field_1,
    )


@dataclass(frozen=True, slots=True)
class AnnotationScopeModule:
    """One module."""

    module: QueryModule
    kind: Literal["module"] = "module"


@dataclass(frozen=True, slots=True)
class AnnotationScopeWorkspace:
    """Workspace profiles."""

    """The profiles to search."""
    profile_ids: Sequence[ProfileId]
    kind: Literal["workspace"] = "workspace"


"""Scope for annotation queries."""
AnnotationScope: TypeAlias = AnnotationScopeModule | AnnotationScopeWorkspace


def encode_annotation_scope(writer: Writer, value: AnnotationScope) -> None:
    if value.kind == "module":
        writer.write_unsigned(0)
        destack._generated.protocol.query.core.target.encode_query_module(
            writer, value.module
        )
    elif value.kind == "workspace":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.profile_ids))
        for item_0 in value.profile_ids:
            destack._generated.protocol.source.file.model.profile.encode_profile_id(
                writer, item_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_scope(reader: Reader) -> AnnotationScope:
    variant = reader.read_number()

    if variant == 0:
        return AnnotationScopeModule(
            module=destack._generated.protocol.query.core.target.decode_query_module(
                reader
            )
        )
    elif variant == 1:
        field_0 = [
            destack._generated.protocol.source.file.model.profile.decode_profile_id(
                reader
            )
            for _ in range(reader.read_number())
        ]

        return AnnotationScopeWorkspace(
            profile_ids=field_0,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class AnnotationsResponse:
    """Response payload for annotation queries."""

    """Matching annotations."""
    annotations: Sequence[AnnotationItem]


def encode_annotations_response(writer: Writer, value: AnnotationsResponse) -> None:
    writer.write_unsigned(len(value.annotations))
    for item_0 in value.annotations:
        encode_annotation_item(writer, item_0)


def decode_annotations_response(reader: Reader) -> AnnotationsResponse:
    field_0 = [decode_annotation_item(reader) for _ in range(reader.read_number())]

    return AnnotationsResponse(
        annotations=field_0,
    )


@dataclass(frozen=True, slots=True)
class AnnotationItem:
    """One annotation query item."""

    """The annotation name when syntactically known."""
    name: str | None
    """The decorator expression target."""
    decorator: QueryTarget
    """The annotated target."""
    target: QueryTarget
    """The resolved annotation role."""
    role: AnnotationRole


def encode_annotation_item(writer: Writer, value: AnnotationItem) -> None:
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.decorator
    )
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )
    encode_annotation_role(writer, value.role)


def decode_annotation_item(reader: Reader) -> AnnotationItem:
    field_0 = reader.read_option(lambda: reader.read_string())
    field_1 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_2 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_3 = decode_annotation_role(reader)

    return AnnotationItem(
        name=field_0,
        decorator=field_1,
        target=field_2,
        role=field_3,
    )


"""Role of one annotation expression."""
AnnotationRole: TypeAlias = (
    Literal["annotation"] | Literal["decorator"] | Literal["unknown"]
)


def encode_annotation_role(writer: Writer, value: AnnotationRole) -> None:
    if value == "annotation":
        writer.write_unsigned(0)
    elif value == "decorator":
        writer.write_unsigned(1)
    elif value == "unknown":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_annotation_role(reader: Reader) -> AnnotationRole:
    variant = reader.read_number()

    if variant == 0:
        return "annotation"
    elif variant == 1:
        return "decorator"
    elif variant == 2:
        return "unknown"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


__all__ = [
    "AnnotationsRequest",
    "encode_annotations_request",
    "decode_annotations_request",
    "AnnotationScope",
    "encode_annotation_scope",
    "decode_annotation_scope",
    "AnnotationScopeModule",
    "AnnotationScopeWorkspace",
    "AnnotationsResponse",
    "encode_annotations_response",
    "decode_annotations_response",
    "AnnotationItem",
    "encode_annotation_item",
    "decode_annotation_item",
    "AnnotationRole",
    "encode_annotation_role",
    "decode_annotation_role",
]
