# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.source.file.model.file

@dataclass(frozen=True, slots=True)
class Library:
    """Loadable native library image."""

    # the native library location
    source: LibrarySource
    # native unwind tables bytes
    unwind: destack._generated.source.file.model.file.ContentId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Library: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Library: ...

def encode_library(writer: BinaryWriter, value: Library) -> None: ...
def decode_library(reader: BinaryReader) -> Library: ...
def to_json_library(value: Library) -> Json: ...
def from_json_library(value: Json) -> Library: ...

@dataclass(frozen=True, slots=True)
class LibrarySourceName:
    """Library is loaded from a process or platform search path."""

    name: destack._generated.core.string.StringId
    kind: typing.Literal["name"] = "name"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LibrarySourceArtifact:
    """Library is packaged as a program artifact."""

    artifact: destack._generated.source.file.model.file.ContentId
    kind: typing.Literal["artifact"] = "artifact"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Native library source."""
LibrarySource: typing.TypeAlias = LibrarySourceName | LibrarySourceArtifact

def encode_library_source(writer: BinaryWriter, value: LibrarySource) -> None: ...
def decode_library_source(reader: BinaryReader) -> LibrarySource: ...
def to_json_library_source(value: LibrarySource) -> Json: ...
def from_json_library_source(value: Json) -> LibrarySource: ...

__all__ = [
    "Library",
    "encode_library",
    "decode_library",
    "to_json_library",
    "from_json_library",
    "LibrarySource",
    "encode_library_source",
    "decode_library_source",
    "to_json_library_source",
    "from_json_library_source",
    "LibrarySourceName",
    "LibrarySourceArtifact",
]
