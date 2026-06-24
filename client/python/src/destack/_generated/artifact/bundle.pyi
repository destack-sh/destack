# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.emit
import destack._generated.source.file.model.file
import destack._generated.source.file.model.type
import destack._generated.source.file.path.uri

"""One section of a linked bundle."""
BundleSection: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["entry"]
    | typing.Literal["declaration"]
    | typing.Literal["asset"]
    | typing.Literal["manifest"]
    | typing.Literal["sourceMap"]
    | typing.Literal["native"]
)

def encode_bundle_section(writer: BinaryWriter, value: BundleSection) -> None: ...
def decode_bundle_section(reader: BinaryReader) -> BundleSection: ...
def to_json_bundle_section(value: BundleSection) -> Json: ...
def from_json_bundle_section(value: Json) -> BundleSection: ...

"""The assembly mode for one bundle."""
BundleMode: typing.TypeAlias = (
    typing.Literal["preserveModules"]
    | typing.Literal["singleFile"]
    | typing.Literal["chunked"]
)

def encode_bundle_mode(writer: BinaryWriter, value: BundleMode) -> None: ...
def decode_bundle_mode(reader: BinaryReader) -> BundleMode: ...
def to_json_bundle_mode(value: BundleMode) -> Json: ...
def from_json_bundle_mode(value: Json) -> BundleMode: ...

@dataclass(frozen=True, slots=True)
class BundleFile:
    """One derived output file."""

    # the bundle section this file belongs to
    section: BundleSection
    # the output URI
    uri: destack._generated.source.file.path.uri.Uri
    # the emitted file type
    file_type: destack._generated.source.file.model.type.FileType
    # the output content identity
    content: destack._generated.source.file.model.file.ContentId
    # the related source URI when one exists
    source: destack._generated.source.file.path.uri.Uri | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BundleFile: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BundleFile: ...

def encode_bundle_file(writer: BinaryWriter, value: BundleFile) -> None: ...
def decode_bundle_file(reader: BinaryReader) -> BundleFile: ...
def to_json_bundle_file(value: BundleFile) -> Json: ...
def from_json_bundle_file(value: Json) -> BundleFile: ...

@dataclass(frozen=True, slots=True)
class Bundle:
    """One linked file graph for one target."""

    # the emitted artifact family
    emit: destack._generated.artifact.emit.EmitFormat
    # the target-level assembly mode
    assembly: BundleMode
    # the files in this bundle
    files: Sequence[BundleFile]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Bundle: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Bundle: ...

def encode_bundle(writer: BinaryWriter, value: Bundle) -> None: ...
def decode_bundle(reader: BinaryReader) -> Bundle: ...
def to_json_bundle(value: Bundle) -> Json: ...
def from_json_bundle(value: Json) -> Bundle: ...

__all__ = [
    "BundleSection",
    "encode_bundle_section",
    "decode_bundle_section",
    "to_json_bundle_section",
    "from_json_bundle_section",
    "BundleMode",
    "encode_bundle_mode",
    "decode_bundle_mode",
    "to_json_bundle_mode",
    "from_json_bundle_mode",
    "BundleFile",
    "encode_bundle_file",
    "decode_bundle_file",
    "to_json_bundle_file",
    "from_json_bundle_file",
    "Bundle",
    "encode_bundle",
    "decode_bundle",
    "to_json_bundle",
    "from_json_bundle",
]
