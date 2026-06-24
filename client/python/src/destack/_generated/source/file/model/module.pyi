# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.package

"""Stable key for one module within a package."""
ModuleKey: typing.TypeAlias = int

def encode_module_key(writer: BinaryWriter, value: ModuleKey) -> None: ...
def decode_module_key(reader: BinaryReader) -> ModuleKey: ...
def to_json_module_key(value: ModuleKey) -> Json: ...
def from_json_module_key(value: Json) -> ModuleKey: ...

@dataclass(frozen=True, slots=True)
class ModuleId:
    """Unique identifier for Modules."""

    # the package this module belongs to
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this module within its package
    module_key: ModuleKey

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleId: ...

def encode_module_id(writer: BinaryWriter, value: ModuleId) -> None: ...
def decode_module_id(reader: BinaryReader) -> ModuleId: ...
def to_json_module_id(value: ModuleId) -> Json: ...
def from_json_module_id(value: Json) -> ModuleId: ...

__all__ = [
    "ModuleKey",
    "encode_module_key",
    "decode_module_key",
    "to_json_module_key",
    "from_json_module_key",
    "ModuleId",
    "encode_module_id",
    "decode_module_id",
    "to_json_module_id",
    "from_json_module_id",
]
