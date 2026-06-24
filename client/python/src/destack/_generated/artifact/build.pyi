# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Build: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Build: ...

def encode_build(writer: BinaryWriter, value: Build) -> None: ...
def decode_build(reader: BinaryReader) -> Build: ...
def to_json_build(value: Build) -> Json: ...
def from_json_build(value: Json) -> Build: ...

__all__ = [
    "Build",
    "encode_build",
    "decode_build",
    "to_json_build",
    "from_json_build",
]
