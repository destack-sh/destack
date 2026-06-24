# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
)

import destack._generated.artifact.core.target
import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class Build:
    """One target-built toolchain payload."""

    # the build distribution profile
    profile: destack._generated.artifact.core.target.BuildProfile
    # the build linkage
    linkage: destack._generated.artifact.core.target.BuildLinkage
    # the encoded build content
    content: destack._generated.source.file.model.file.ContentId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_build(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Build:
        """Decode one Build."""
        return decode_build(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_build(self)

    @classmethod
    def from_json(cls, value: Json) -> Build:
        """Return one Build from one JSON value."""
        return from_json_build(value)


def encode_build(writer: BinaryWriter, value: Build) -> None:
    """Encode one Build."""
    destack._generated.artifact.core.target.encode_build_profile(writer, value.profile)
    destack._generated.artifact.core.target.encode_build_linkage(writer, value.linkage)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)


def decode_build(reader: BinaryReader) -> Build:
    """Decode one Build."""
    profile = destack._generated.artifact.core.target.decode_build_profile(reader)
    linkage = destack._generated.artifact.core.target.decode_build_linkage(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)

    return Build(
        profile=profile,
        linkage=linkage,
        content=content,
    )


def to_json_build(value: Build) -> Json:
    """Return one JSON value for one Build."""
    return {
        "profile": destack._generated.artifact.core.target.to_json_build_profile(
            value.profile
        ),
        "linkage": destack._generated.artifact.core.target.to_json_build_linkage(
            value.linkage
        ),
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
    }


def from_json_build(value: Json) -> Build:
    """Return one Build from one JSON value."""
    object_ = json_object(value)

    return Build(
        profile=destack._generated.artifact.core.target.from_json_build_profile(
            json_field(object_, "profile")
        ),
        linkage=destack._generated.artifact.core.target.from_json_build_linkage(
            json_field(object_, "linkage")
        ),
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
    )


__all__ = [
    "Build",
    "encode_build",
    "decode_build",
    "to_json_build",
    "from_json_build",
]
