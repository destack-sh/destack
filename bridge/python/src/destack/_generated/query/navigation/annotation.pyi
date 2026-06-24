# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.core.target
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class AnnotationsRequest:
    """Request payload for annotation queries."""

    # the query scope
    scope: AnnotationScope
    # the annotation name filter
    name: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationsRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationsRequest: ...

def encode_annotations_request(
    writer: BinaryWriter, value: AnnotationsRequest
) -> None: ...
def decode_annotations_request(reader: BinaryReader) -> AnnotationsRequest: ...
def to_json_annotations_request(value: AnnotationsRequest) -> Json: ...
def from_json_annotations_request(value: Json) -> AnnotationsRequest: ...

@dataclass(frozen=True, slots=True)
class AnnotationScopeModule:
    """One module."""

    module: destack._generated.query.core.target.QueryModule
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class AnnotationScopeWorkspace:
    """Workspace profiles."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Scope for annotation queries."""
AnnotationScope: typing.TypeAlias = AnnotationScopeModule | AnnotationScopeWorkspace

def encode_annotation_scope(writer: BinaryWriter, value: AnnotationScope) -> None: ...
def decode_annotation_scope(reader: BinaryReader) -> AnnotationScope: ...
def to_json_annotation_scope(value: AnnotationScope) -> Json: ...
def from_json_annotation_scope(value: Json) -> AnnotationScope: ...

@dataclass(frozen=True, slots=True)
class AnnotationsResponse:
    """Response payload for annotation queries."""

    # matching annotations
    annotations: Sequence[AnnotationItem]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationsResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationsResponse: ...

def encode_annotations_response(
    writer: BinaryWriter, value: AnnotationsResponse
) -> None: ...
def decode_annotations_response(reader: BinaryReader) -> AnnotationsResponse: ...
def to_json_annotations_response(value: AnnotationsResponse) -> Json: ...
def from_json_annotations_response(value: Json) -> AnnotationsResponse: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> AnnotationItem: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> AnnotationItem: ...

def encode_annotation_item(writer: BinaryWriter, value: AnnotationItem) -> None: ...
def decode_annotation_item(reader: BinaryReader) -> AnnotationItem: ...
def to_json_annotation_item(value: AnnotationItem) -> Json: ...
def from_json_annotation_item(value: Json) -> AnnotationItem: ...

"""Role of one annotation expression."""
AnnotationRole: typing.TypeAlias = (
    typing.Literal["annotation"]
    | typing.Literal["decorator"]
    | typing.Literal["unknown"]
)

def encode_annotation_role(writer: BinaryWriter, value: AnnotationRole) -> None: ...
def decode_annotation_role(reader: BinaryReader) -> AnnotationRole: ...
def to_json_annotation_role(value: AnnotationRole) -> Json: ...
def from_json_annotation_role(value: Json) -> AnnotationRole: ...

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
