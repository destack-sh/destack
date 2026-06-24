# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.dependency

@dataclass(frozen=True, slots=True)
class PackageLock:
    """Locked package source and dependency record."""

    # package name read from the locked manifest
    name: str
    # package version read from the locked manifest
    version: str | None
    # resolved source locator
    source: SourceLock
    # hash of the package source tree
    source_hash: str
    # hash of the package manifest
    manifest_hash: str
    # locked dependency declarations
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]
    # whether this package is an editable source
    editable: bool

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageLock: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackageLock: ...

def encode_package_lock(writer: BinaryWriter, value: PackageLock) -> None: ...
def decode_package_lock(reader: BinaryReader) -> PackageLock: ...
def to_json_package_lock(value: PackageLock) -> Json: ...
def from_json_package_lock(value: Json) -> PackageLock: ...

@dataclass(frozen=True, slots=True)
class SourceLockWorkspace:
    """Package resolved from a workspace member."""

    # workspace relative package path
    path: str
    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SourceLockRegistry:
    """Package resolved from a registry."""

    # registry name
    registry: str | None
    # exact package version
    version: str
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SourceLockPath:
    """Package resolved from a local path."""

    # package path
    path: str
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class SourceLockGit:
    """Package resolved from one Git repository."""

    # repository URL
    url: str
    # exact commit revision
    rev: str
    # repository subdirectory containing the package
    path: str | None
    kind: typing.Literal["git"] = "git"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Locked source locator."""
SourceLock: typing.TypeAlias = (
    SourceLockWorkspace | SourceLockRegistry | SourceLockPath | SourceLockGit
)

def encode_source_lock(writer: BinaryWriter, value: SourceLock) -> None: ...
def decode_source_lock(reader: BinaryReader) -> SourceLock: ...
def to_json_source_lock(value: SourceLock) -> Json: ...
def from_json_source_lock(value: Json) -> SourceLock: ...

@dataclass(frozen=True, slots=True)
class DestackLock:
    """Destack lock document."""

    # lock document format version
    version: int
    # root package name
    root: str | None
    # locked packages keyed by package import name
    packages: Mapping[str, PackageLock]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLock: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DestackLock: ...

def encode_destack_lock(writer: BinaryWriter, value: DestackLock) -> None: ...
def decode_destack_lock(reader: BinaryReader) -> DestackLock: ...
def to_json_destack_lock(value: DestackLock) -> Json: ...
def from_json_destack_lock(value: Json) -> DestackLock: ...

__all__ = [
    "PackageLock",
    "encode_package_lock",
    "decode_package_lock",
    "to_json_package_lock",
    "from_json_package_lock",
    "SourceLock",
    "encode_source_lock",
    "decode_source_lock",
    "to_json_source_lock",
    "from_json_source_lock",
    "SourceLockWorkspace",
    "SourceLockRegistry",
    "SourceLockPath",
    "SourceLockGit",
    "DestackLock",
    "encode_destack_lock",
    "decode_destack_lock",
    "to_json_destack_lock",
    "from_json_destack_lock",
]
