# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.artifact.core.key
import destack._generated.artifact.core.version
import destack._generated.artifact.index
import destack._generated.source.file.model.component
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module

@dataclass(frozen=True, slots=True)
class ArtifactProjectionDependency:
    """One exact projected artifact value observed while building an artifact."""

    # the exact artifact version that supplied the projected value
    version: destack._generated.artifact.core.version.ArtifactVersion
    # the projected artifact value
    projection: ArtifactProjection
    # the exact projection fingerprint read from the owner artifact
    fingerprint: ArtifactProjectionFingerprint

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactProjectionDependency: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactProjectionDependency: ...

def encode_artifact_projection_dependency(
    writer: BinaryWriter, value: ArtifactProjectionDependency
) -> None: ...
def decode_artifact_projection_dependency(
    reader: BinaryReader,
) -> ArtifactProjectionDependency: ...
def to_json_artifact_projection_dependency(
    value: ArtifactProjectionDependency,
) -> Json: ...
def from_json_artifact_projection_dependency(
    value: Json,
) -> ArtifactProjectionDependency: ...

@dataclass(frozen=True, slots=True)
class ArtifactProjection:
    """One artifact projection selected by owner artifact and projection key."""

    # the artifact that owns the projected payload
    artifact: destack._generated.artifact.core.key.ArtifactKey
    # the projected value inside the owning artifact
    key: ArtifactProjectionKey

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactProjection: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactProjection: ...

def encode_artifact_projection(
    writer: BinaryWriter, value: ArtifactProjection
) -> None: ...
def decode_artifact_projection(reader: BinaryReader) -> ArtifactProjection: ...
def to_json_artifact_projection(value: ArtifactProjection) -> Json: ...
def from_json_artifact_projection(value: Json) -> ArtifactProjection: ...

@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyComponentGraph:
    """A component graph projection."""

    component_graph: ComponentGraphProjection
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyDirChecked:
    """A checked DIR module inside a checked component."""

    dir_checked: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyModuleIndex:
    """A module index projection."""

    module_index: destack._generated.artifact.index.ModuleIndexProjection
    kind: typing.Literal["moduleIndex"] = "moduleIndex"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One observable projection of an artifact payload."""
ArtifactProjectionKey: typing.TypeAlias = (
    ArtifactProjectionKeyComponentGraph
    | ArtifactProjectionKeyDirChecked
    | ArtifactProjectionKeyModuleIndex
)

def encode_artifact_projection_key(
    writer: BinaryWriter, value: ArtifactProjectionKey
) -> None: ...
def decode_artifact_projection_key(reader: BinaryReader) -> ArtifactProjectionKey: ...
def to_json_artifact_projection_key(value: ArtifactProjectionKey) -> Json: ...
def from_json_artifact_projection_key(value: Json) -> ArtifactProjectionKey: ...

@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionComponentOf:
    """The component containing one module."""

    component_of: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["componentOf"] = "componentOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionComponentEntryOf:
    """The component and entry containing one module."""

    component_entry_of: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["componentEntryOf"] = "componentEntryOf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionMembers:
    """The sorted modules belonging to one component."""

    members: destack._generated.source.file.model.component.ComponentId
    kind: typing.Literal["members"] = "members"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionDependencies:
    """The direct external components one component depends on."""

    dependencies: destack._generated.source.file.model.component.ComponentId
    kind: typing.Literal["dependencies"] = "dependencies"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One observable projection of a component graph artifact."""
ComponentGraphProjection: typing.TypeAlias = (
    ComponentGraphProjectionComponentOf
    | ComponentGraphProjectionComponentEntryOf
    | ComponentGraphProjectionMembers
    | ComponentGraphProjectionDependencies
)

def encode_component_graph_projection(
    writer: BinaryWriter, value: ComponentGraphProjection
) -> None: ...
def decode_component_graph_projection(
    reader: BinaryReader,
) -> ComponentGraphProjection: ...
def to_json_component_graph_projection(value: ComponentGraphProjection) -> Json: ...
def from_json_component_graph_projection(value: Json) -> ComponentGraphProjection: ...

"""Stable fingerprint of one observed artifact projection."""
ArtifactProjectionFingerprint: typing.TypeAlias = int

def encode_artifact_projection_fingerprint(
    writer: BinaryWriter, value: ArtifactProjectionFingerprint
) -> None: ...
def decode_artifact_projection_fingerprint(
    reader: BinaryReader,
) -> ArtifactProjectionFingerprint: ...
def to_json_artifact_projection_fingerprint(
    value: ArtifactProjectionFingerprint,
) -> Json: ...
def from_json_artifact_projection_fingerprint(
    value: Json,
) -> ArtifactProjectionFingerprint: ...

@dataclass(frozen=True, slots=True)
class SourceDependency:
    """One exact source file content observed while building an artifact."""

    # the source file id
    file: destack._generated.source.file.model.file.FileId
    # the exact source content id
    content: destack._generated.source.file.model.file.ContentId

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceDependency: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SourceDependency: ...

def encode_source_dependency(writer: BinaryWriter, value: SourceDependency) -> None: ...
def decode_source_dependency(reader: BinaryReader) -> SourceDependency: ...
def to_json_source_dependency(value: SourceDependency) -> Json: ...
def from_json_source_dependency(value: Json) -> SourceDependency: ...

@dataclass(frozen=True, slots=True)
class ArtifactDependencyArtifact:
    """Another exact artifact version."""

    artifact: destack._generated.artifact.core.version.ArtifactVersion
    kind: typing.Literal["artifact"] = "artifact"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactDependencyProjection:
    """One exact projected artifact value."""

    projection: ArtifactProjectionDependency
    kind: typing.Literal["projection"] = "projection"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactDependencySource:
    """One exact primitive source observation."""

    source: SourceDependency
    kind: typing.Literal["source"] = "source"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One exact dependency read while building an artifact."""
ArtifactDependency: typing.TypeAlias = (
    ArtifactDependencyArtifact | ArtifactDependencyProjection | ArtifactDependencySource
)

def encode_artifact_dependency(
    writer: BinaryWriter, value: ArtifactDependency
) -> None: ...
def decode_artifact_dependency(reader: BinaryReader) -> ArtifactDependency: ...
def to_json_artifact_dependency(value: ArtifactDependency) -> Json: ...
def from_json_artifact_dependency(value: Json) -> ArtifactDependency: ...

__all__ = [
    "ArtifactProjectionDependency",
    "encode_artifact_projection_dependency",
    "decode_artifact_projection_dependency",
    "to_json_artifact_projection_dependency",
    "from_json_artifact_projection_dependency",
    "ArtifactProjection",
    "encode_artifact_projection",
    "decode_artifact_projection",
    "to_json_artifact_projection",
    "from_json_artifact_projection",
    "ArtifactProjectionKey",
    "encode_artifact_projection_key",
    "decode_artifact_projection_key",
    "to_json_artifact_projection_key",
    "from_json_artifact_projection_key",
    "ArtifactProjectionKeyComponentGraph",
    "ArtifactProjectionKeyDirChecked",
    "ArtifactProjectionKeyModuleIndex",
    "ComponentGraphProjection",
    "encode_component_graph_projection",
    "decode_component_graph_projection",
    "to_json_component_graph_projection",
    "from_json_component_graph_projection",
    "ComponentGraphProjectionComponentOf",
    "ComponentGraphProjectionComponentEntryOf",
    "ComponentGraphProjectionMembers",
    "ComponentGraphProjectionDependencies",
    "ArtifactProjectionFingerprint",
    "encode_artifact_projection_fingerprint",
    "decode_artifact_projection_fingerprint",
    "to_json_artifact_projection_fingerprint",
    "from_json_artifact_projection_fingerprint",
    "SourceDependency",
    "encode_source_dependency",
    "decode_source_dependency",
    "to_json_source_dependency",
    "from_json_source_dependency",
    "ArtifactDependency",
    "encode_artifact_dependency",
    "decode_artifact_dependency",
    "to_json_artifact_dependency",
    "from_json_artifact_dependency",
    "ArtifactDependencyArtifact",
    "ArtifactDependencyProjection",
    "ArtifactDependencySource",
]
