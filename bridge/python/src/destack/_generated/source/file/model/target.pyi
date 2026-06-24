# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.file.model.package

"""Stable key for one target within a package."""
TargetKey: typing.TypeAlias = int

def encode_target_key(writer: BinaryWriter, value: TargetKey) -> None: ...
def decode_target_key(reader: BinaryReader) -> TargetKey: ...
def to_json_target_key(value: TargetKey) -> Json: ...
def from_json_target_key(value: Json) -> TargetKey: ...

@dataclass(frozen=True, slots=True)
class TargetId:
    """Unique identifier for a build target within a package."""

    # the owning package id
    package_id: destack._generated.source.file.model.package.PackageId
    # the stable key for this target within its package
    target_key: TargetKey

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetId: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetId: ...

def encode_target_id(writer: BinaryWriter, value: TargetId) -> None: ...
def decode_target_id(reader: BinaryReader) -> TargetId: ...
def to_json_target_id(value: TargetId) -> Json: ...
def from_json_target_id(value: Json) -> TargetId: ...

__all__ = [
    "TargetKey",
    "encode_target_key",
    "decode_target_key",
    "to_json_target_key",
    "from_json_target_key",
    "TargetId",
    "encode_target_id",
    "decode_target_id",
    "to_json_target_id",
    "from_json_target_id",
]
