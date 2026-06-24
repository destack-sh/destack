# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class BuildManifestFile:
    """One public build manifest file record."""

    # the emitted output path, relative to the output root when possible
    path: str
    # the public file kind
    type: BuildManifestFileType
    # the emitted file loader
    loader: BuildManifestLoader
    # the logical chunk name when one exists
    name: str | None
    # the source input that produced this file when one exists
    input: str | None
    # whether this file is one entry output
    is_entry: bool | None
    # whether this file is one dynamic entry output
    is_dynamic_entry: bool | None
    # imported chunks or external specifiers
    imports: Sequence[str]
    # dynamically imported chunks or external specifiers
    dynamic_imports: Sequence[str]
    # associated emitted stylesheets
    stylesheets: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildManifestFile: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BuildManifestFile: ...

def encode_build_manifest_file(
    writer: BinaryWriter, value: BuildManifestFile
) -> None: ...
def decode_build_manifest_file(reader: BinaryReader) -> BuildManifestFile: ...
def to_json_build_manifest_file(value: BuildManifestFile) -> Json: ...
def from_json_build_manifest_file(value: Json) -> BuildManifestFile: ...

"""One public build manifest file kind."""
BuildManifestFileType: typing.TypeAlias = (
    typing.Literal["chunk"] | typing.Literal["asset"] | typing.Literal["binary"]
)

def encode_build_manifest_file_type(
    writer: BinaryWriter, value: BuildManifestFileType
) -> None: ...
def decode_build_manifest_file_type(reader: BinaryReader) -> BuildManifestFileType: ...
def to_json_build_manifest_file_type(value: BuildManifestFileType) -> Json: ...
def from_json_build_manifest_file_type(value: Json) -> BuildManifestFileType: ...

"""One public build manifest loader name."""
BuildManifestLoader: typing.TypeAlias = (
    typing.Literal["js"]
    | typing.Literal["css"]
    | typing.Literal["ts"]
    | typing.Literal["map"]
    | typing.Literal["json"]
    | typing.Literal["dts"]
    | typing.Literal["wasm"]
    | typing.Literal["object"]
    | typing.Literal["asset"]
)

def encode_build_manifest_loader(
    writer: BinaryWriter, value: BuildManifestLoader
) -> None: ...
def decode_build_manifest_loader(reader: BinaryReader) -> BuildManifestLoader: ...
def to_json_build_manifest_loader(value: BuildManifestLoader) -> Json: ...
def from_json_build_manifest_loader(value: Json) -> BuildManifestLoader: ...

@dataclass(frozen=True, slots=True)
class BuildManifest:
    """One public build manifest for one linked target."""

    # the primary entry path when one exists
    index: str | None
    # the emitted file records for this target
    files: Sequence[BuildManifestFile]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> BuildManifest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> BuildManifest: ...

def encode_build_manifest(writer: BinaryWriter, value: BuildManifest) -> None: ...
def decode_build_manifest(reader: BinaryReader) -> BuildManifest: ...
def to_json_build_manifest(value: BuildManifest) -> Json: ...
def from_json_build_manifest(value: Json) -> BuildManifest: ...

__all__ = [
    "BuildManifestFile",
    "encode_build_manifest_file",
    "decode_build_manifest_file",
    "to_json_build_manifest_file",
    "from_json_build_manifest_file",
    "BuildManifestFileType",
    "encode_build_manifest_file_type",
    "decode_build_manifest_file_type",
    "to_json_build_manifest_file_type",
    "from_json_build_manifest_file_type",
    "BuildManifestLoader",
    "encode_build_manifest_loader",
    "decode_build_manifest_loader",
    "to_json_build_manifest_loader",
    "from_json_build_manifest_loader",
    "BuildManifest",
    "encode_build_manifest",
    "decode_build_manifest",
    "to_json_build_manifest",
    "from_json_build_manifest",
]
