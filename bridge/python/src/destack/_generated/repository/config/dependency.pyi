# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.repository.config.condition

@dataclass(frozen=True, slots=True)
class DependencyWorkspace:
    """Package resolved from the current workspace."""

    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DependencyRegistry:
    """Package resolved from the configured package registry."""

    # registry name
    registry: str | None
    # exact package version
    version: str
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DependencyPath:
    """Package resolved from one local package path."""

    # package path
    path: str
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class DependencyGit:
    """Package resolved from one Git repository."""

    # repository URL
    url: str
    # repository subdirectory containing the package
    path: str | None
    # exact commit revision
    rev: str | None
    # git tag selector
    tag: str | None
    # git branch selector
    branch: str | None
    kind: typing.Literal["git"] = "git"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Package dependency declaration."""
Dependency: typing.TypeAlias = (
    DependencyWorkspace | DependencyRegistry | DependencyPath | DependencyGit
)

def encode_dependency(writer: BinaryWriter, value: Dependency) -> None: ...
def decode_dependency(reader: BinaryReader) -> Dependency: ...
def to_json_dependency(value: Dependency) -> Json: ...
def from_json_dependency(value: Json) -> Dependency: ...

@dataclass(frozen=True, slots=True)
class ConditionalDependencies:
    """Dependency declarations guarded by one active condition predicate."""

    # condition predicate enabling these dependencies
    when: destack._generated.repository.config.condition.ConditionRef
    # dependency declarations enabled when the predicate matches
    dependencies: Mapping[str, Dependency]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionalDependencies: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ConditionalDependencies: ...

def encode_conditional_dependencies(
    writer: BinaryWriter, value: ConditionalDependencies
) -> None: ...
def decode_conditional_dependencies(
    reader: BinaryReader,
) -> ConditionalDependencies: ...
def to_json_conditional_dependencies(value: ConditionalDependencies) -> Json: ...
def from_json_conditional_dependencies(value: Json) -> ConditionalDependencies: ...

@dataclass(frozen=True, slots=True)
class PackagePatch:
    """Patch file applied to one resolved package."""

    # patch file path
    path: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackagePatch: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackagePatch: ...

def encode_package_patch(writer: BinaryWriter, value: PackagePatch) -> None: ...
def decode_package_patch(reader: BinaryReader) -> PackagePatch: ...
def to_json_package_patch(value: PackagePatch) -> Json: ...
def from_json_package_patch(value: Json) -> PackagePatch: ...

__all__ = [
    "Dependency",
    "encode_dependency",
    "decode_dependency",
    "to_json_dependency",
    "from_json_dependency",
    "DependencyWorkspace",
    "DependencyRegistry",
    "DependencyPath",
    "DependencyGit",
    "ConditionalDependencies",
    "encode_conditional_dependencies",
    "decode_conditional_dependencies",
    "to_json_conditional_dependencies",
    "from_json_conditional_dependencies",
    "PackagePatch",
    "encode_package_patch",
    "decode_package_patch",
    "to_json_package_patch",
    "from_json_package_patch",
]
