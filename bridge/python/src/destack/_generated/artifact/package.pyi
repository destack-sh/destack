# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.package
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class PackageImportIndex:
    """Active import index for one package."""

    # the package this index belongs to
    package: destack._generated.source.file.model.package.PackageId
    # package root path when filesystem backed
    root: str | None
    # active direct dependencies keyed by package specifier
    dependencies: Mapping[
        str, destack._generated.source.file.model.package.PackageId | None
    ]
    # active public exports
    exports: ExportIndex

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageImportIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackageImportIndex: ...

def encode_package_import_index(
    writer: BinaryWriter, value: PackageImportIndex
) -> None: ...
def decode_package_import_index(reader: BinaryReader) -> PackageImportIndex: ...
def to_json_package_import_index(value: PackageImportIndex) -> Json: ...
def from_json_package_import_index(value: Json) -> PackageImportIndex: ...

@dataclass(frozen=True, slots=True)
class ExportIndex:
    """Active package exports indexed for module import resolution."""

    # the package this export index belongs to
    package: destack._generated.source.file.model.package.PackageId
    # exact exports keyed by export specifier
    exact: Mapping[str, ExportTarget]
    # pattern exports sorted from most specific to least specific
    patterns: Sequence[ExportPattern]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportIndex: ...

def encode_export_index(writer: BinaryWriter, value: ExportIndex) -> None: ...
def decode_export_index(reader: BinaryReader) -> ExportIndex: ...
def to_json_export_index(value: ExportIndex) -> Json: ...
def from_json_export_index(value: Json) -> ExportIndex: ...

@dataclass(frozen=True, slots=True)
class ExportTarget:
    """Active package export target."""

    # package relative export path
    path: str
    # whether the export can be imported as a source module
    is_module: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportTarget: ...

def encode_export_target(writer: BinaryWriter, value: ExportTarget) -> None: ...
def decode_export_target(reader: BinaryReader) -> ExportTarget: ...
def to_json_export_target(value: ExportTarget) -> Json: ...
def from_json_export_target(value: Json) -> ExportTarget: ...

@dataclass(frozen=True, slots=True)
class ExportPattern:
    """Active pattern export target."""

    # export key prefix before `*`
    prefix: str
    # export key suffix after `*`
    suffix: str
    # pattern export target
    target: ExportTarget

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportPattern: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ExportPattern: ...

def encode_export_pattern(writer: BinaryWriter, value: ExportPattern) -> None: ...
def decode_export_pattern(reader: BinaryReader) -> ExportPattern: ...
def to_json_export_pattern(value: ExportPattern) -> Json: ...
def from_json_export_pattern(value: Json) -> ExportPattern: ...

@dataclass(frozen=True, slots=True)
class PackageIndex:
    """Active package dependency and export index for one profile."""

    # the profile this index belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # indexed packages keyed by package id
    packages: Mapping[
        destack._generated.source.file.model.package.PackageId, PackageImportIndex
    ]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackageIndex: ...

def encode_package_index(writer: BinaryWriter, value: PackageIndex) -> None: ...
def decode_package_index(reader: BinaryReader) -> PackageIndex: ...
def to_json_package_index(value: PackageIndex) -> Json: ...
def from_json_package_index(value: Json) -> PackageIndex: ...

__all__ = [
    "PackageImportIndex",
    "encode_package_import_index",
    "decode_package_import_index",
    "to_json_package_import_index",
    "from_json_package_import_index",
    "ExportIndex",
    "encode_export_index",
    "decode_export_index",
    "to_json_export_index",
    "from_json_export_index",
    "ExportTarget",
    "encode_export_target",
    "decode_export_target",
    "to_json_export_target",
    "from_json_export_target",
    "ExportPattern",
    "encode_export_pattern",
    "decode_export_pattern",
    "to_json_export_pattern",
    "from_json_export_pattern",
    "PackageIndex",
    "encode_package_index",
    "decode_package_index",
    "to_json_package_index",
    "from_json_package_index",
]
