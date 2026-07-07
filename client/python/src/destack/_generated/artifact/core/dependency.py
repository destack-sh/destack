# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_int,
    json_object,
    json_string,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_projection_dependency(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactProjectionDependency:
        """Decode one ArtifactProjectionDependency."""
        return decode_artifact_projection_dependency(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_projection_dependency(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactProjectionDependency:
        """Return one ArtifactProjectionDependency from one JSON value."""
        return from_json_artifact_projection_dependency(value)


def encode_artifact_projection_dependency(
    writer: BinaryWriter, value: ArtifactProjectionDependency
) -> None:
    """Encode one ArtifactProjectionDependency."""
    destack._generated.artifact.core.version.encode_artifact_version(
        writer, value.version
    )
    encode_artifact_projection(writer, value.projection)
    encode_artifact_projection_fingerprint(writer, value.fingerprint)


def decode_artifact_projection_dependency(
    reader: BinaryReader,
) -> ArtifactProjectionDependency:
    """Decode one ArtifactProjectionDependency."""
    version = destack._generated.artifact.core.version.decode_artifact_version(reader)
    projection = decode_artifact_projection(reader)
    fingerprint = decode_artifact_projection_fingerprint(reader)

    return ArtifactProjectionDependency(
        version=version,
        projection=projection,
        fingerprint=fingerprint,
    )


def to_json_artifact_projection_dependency(value: ArtifactProjectionDependency) -> Json:
    """Return one JSON value for one ArtifactProjectionDependency."""
    return {
        "version": destack._generated.artifact.core.version.to_json_artifact_version(
            value.version
        ),
        "projection": to_json_artifact_projection(value.projection),
        "fingerprint": to_json_artifact_projection_fingerprint(value.fingerprint),
    }


def from_json_artifact_projection_dependency(
    value: Json,
) -> ArtifactProjectionDependency:
    """Return one ArtifactProjectionDependency from one JSON value."""
    object_ = json_object(value)

    return ArtifactProjectionDependency(
        version=destack._generated.artifact.core.version.from_json_artifact_version(
            json_field(object_, "version")
        ),
        projection=from_json_artifact_projection(json_field(object_, "projection")),
        fingerprint=from_json_artifact_projection_fingerprint(
            json_field(object_, "fingerprint")
        ),
    )


@dataclass(frozen=True, slots=True)
class ArtifactProjection:
    """One artifact projection selected by owner artifact and projection key."""

    # the artifact that owns the projected payload
    artifact: destack._generated.artifact.core.key.ArtifactKey
    # the projected value inside the owning artifact
    key: ArtifactProjectionKey

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_projection(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactProjection:
        """Decode one ArtifactProjection."""
        return decode_artifact_projection(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_projection(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactProjection:
        """Return one ArtifactProjection from one JSON value."""
        return from_json_artifact_projection(value)


def encode_artifact_projection(writer: BinaryWriter, value: ArtifactProjection) -> None:
    """Encode one ArtifactProjection."""
    destack._generated.artifact.core.key.encode_artifact_key(writer, value.artifact)
    encode_artifact_projection_key(writer, value.key)


def decode_artifact_projection(reader: BinaryReader) -> ArtifactProjection:
    """Decode one ArtifactProjection."""
    artifact = destack._generated.artifact.core.key.decode_artifact_key(reader)
    key = decode_artifact_projection_key(reader)

    return ArtifactProjection(
        artifact=artifact,
        key=key,
    )


def to_json_artifact_projection(value: ArtifactProjection) -> Json:
    """Return one JSON value for one ArtifactProjection."""
    return {
        "artifact": destack._generated.artifact.core.key.to_json_artifact_key(
            value.artifact
        ),
        "key": to_json_artifact_projection_key(value.key),
    }


def from_json_artifact_projection(value: Json) -> ArtifactProjection:
    """Return one ArtifactProjection from one JSON value."""
    object_ = json_object(value)

    return ArtifactProjection(
        artifact=destack._generated.artifact.core.key.from_json_artifact_key(
            json_field(object_, "artifact")
        ),
        key=from_json_artifact_projection_key(json_field(object_, "key")),
    )


@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyComponentGraph:
    """A component graph projection."""

    component_graph: ComponentGraphProjection
    kind: typing.Literal["componentGraph"] = "componentGraph"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_projection_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_projection_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyDirChecked:
    """A checked DIR module inside a checked component."""

    dir_checked: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["dirChecked"] = "dirChecked"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_projection_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_projection_key(self)


@dataclass(frozen=True, slots=True)
class ArtifactProjectionKeyModuleIndex:
    """A module index projection."""

    module_index: destack._generated.artifact.index.ModuleIndexProjection
    kind: typing.Literal["moduleIndex"] = "moduleIndex"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_projection_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_projection_key(self)


"""One observable projection of an artifact payload."""
ArtifactProjectionKey: typing.TypeAlias = (
    ArtifactProjectionKeyComponentGraph
    | ArtifactProjectionKeyDirChecked
    | ArtifactProjectionKeyModuleIndex
)


def encode_artifact_projection_key(
    writer: BinaryWriter, value: ArtifactProjectionKey
) -> None:
    """Encode one ArtifactProjectionKey."""
    if value.kind == "componentGraph":
        writer.write_unsigned(0)
        encode_component_graph_projection(writer, value.component_graph)
    elif value.kind == "dirChecked":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.dir_checked
        )
    elif value.kind == "moduleIndex":
        writer.write_unsigned(2)
        destack._generated.artifact.index.encode_module_index_projection(
            writer, value.module_index
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_projection_key(reader: BinaryReader) -> ArtifactProjectionKey:
    """Decode one ArtifactProjectionKey."""
    variant = reader.read_number()

    if variant == 0:
        component_graph = decode_component_graph_projection(reader)

        return ArtifactProjectionKeyComponentGraph(component_graph=component_graph)
    elif variant == 1:
        dir_checked = destack._generated.source.file.model.module.decode_module_id(
            reader
        )

        return ArtifactProjectionKeyDirChecked(dir_checked=dir_checked)
    elif variant == 2:
        module_index = destack._generated.artifact.index.decode_module_index_projection(
            reader
        )

        return ArtifactProjectionKeyModuleIndex(module_index=module_index)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_projection_key(value: ArtifactProjectionKey) -> Json:
    """Return one JSON value for one ArtifactProjectionKey."""
    if value.kind == "componentGraph":
        return {
            "kind": "componentGraph",
            "component_graph": to_json_component_graph_projection(
                value.component_graph
            ),
        }
    elif value.kind == "dirChecked":
        return {
            "kind": "dirChecked",
            "dir_checked": destack._generated.source.file.model.module.to_json_module_id(
                value.dir_checked
            ),
        }
    elif value.kind == "moduleIndex":
        return {
            "kind": "moduleIndex",
            "module_index": destack._generated.artifact.index.to_json_module_index_projection(
                value.module_index
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_artifact_projection_key(value: Json) -> ArtifactProjectionKey:
    """Return one ArtifactProjectionKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "componentGraph":
        return ArtifactProjectionKeyComponentGraph(
            component_graph=from_json_component_graph_projection(
                json_field(object_, "component_graph")
            )
        )
    elif kind == "dirChecked":
        return ArtifactProjectionKeyDirChecked(
            dir_checked=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "dir_checked")
            )
        )
    elif kind == "moduleIndex":
        return ArtifactProjectionKeyModuleIndex(
            module_index=destack._generated.artifact.index.from_json_module_index_projection(
                json_field(object_, "module_index")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionComponentOf:
    """The component containing one module."""

    component_of: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["componentOf"] = "componentOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_component_graph_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_component_graph_projection(self)


@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionComponentEntryOf:
    """The component and entry containing one module."""

    component_entry_of: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["componentEntryOf"] = "componentEntryOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_component_graph_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_component_graph_projection(self)


@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionMembers:
    """The sorted modules belonging to one component."""

    members: destack._generated.source.file.model.component.ComponentId
    kind: typing.Literal["members"] = "members"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_component_graph_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_component_graph_projection(self)


@dataclass(frozen=True, slots=True)
class ComponentGraphProjectionDependencies:
    """The direct external components one component depends on."""

    dependencies: destack._generated.source.file.model.component.ComponentId
    kind: typing.Literal["dependencies"] = "dependencies"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_component_graph_projection(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_component_graph_projection(self)


"""One observable projection of a component graph artifact."""
ComponentGraphProjection: typing.TypeAlias = (
    ComponentGraphProjectionComponentOf
    | ComponentGraphProjectionComponentEntryOf
    | ComponentGraphProjectionMembers
    | ComponentGraphProjectionDependencies
)


def encode_component_graph_projection(
    writer: BinaryWriter, value: ComponentGraphProjection
) -> None:
    """Encode one ComponentGraphProjection."""
    if value.kind == "componentOf":
        writer.write_unsigned(0)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.component_of
        )
    elif value.kind == "componentEntryOf":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.component_entry_of
        )
    elif value.kind == "members":
        writer.write_unsigned(2)
        destack._generated.source.file.model.component.encode_component_id(
            writer, value.members
        )
    elif value.kind == "dependencies":
        writer.write_unsigned(3)
        destack._generated.source.file.model.component.encode_component_id(
            writer, value.dependencies
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_component_graph_projection(reader: BinaryReader) -> ComponentGraphProjection:
    """Decode one ComponentGraphProjection."""
    variant = reader.read_number()

    if variant == 0:
        component_of = destack._generated.source.file.model.module.decode_module_id(
            reader
        )

        return ComponentGraphProjectionComponentOf(component_of=component_of)
    elif variant == 1:
        component_entry_of = (
            destack._generated.source.file.model.module.decode_module_id(reader)
        )

        return ComponentGraphProjectionComponentEntryOf(
            component_entry_of=component_entry_of
        )
    elif variant == 2:
        members = destack._generated.source.file.model.component.decode_component_id(
            reader
        )

        return ComponentGraphProjectionMembers(members=members)
    elif variant == 3:
        dependencies = (
            destack._generated.source.file.model.component.decode_component_id(reader)
        )

        return ComponentGraphProjectionDependencies(dependencies=dependencies)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_component_graph_projection(value: ComponentGraphProjection) -> Json:
    """Return one JSON value for one ComponentGraphProjection."""
    if value.kind == "componentOf":
        return {
            "kind": "componentOf",
            "component_of": destack._generated.source.file.model.module.to_json_module_id(
                value.component_of
            ),
        }
    elif value.kind == "componentEntryOf":
        return {
            "kind": "componentEntryOf",
            "component_entry_of": destack._generated.source.file.model.module.to_json_module_id(
                value.component_entry_of
            ),
        }
    elif value.kind == "members":
        return {
            "kind": "members",
            "members": destack._generated.source.file.model.component.to_json_component_id(
                value.members
            ),
        }
    elif value.kind == "dependencies":
        return {
            "kind": "dependencies",
            "dependencies": destack._generated.source.file.model.component.to_json_component_id(
                value.dependencies
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_component_graph_projection(value: Json) -> ComponentGraphProjection:
    """Return one ComponentGraphProjection from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "componentOf":
        return ComponentGraphProjectionComponentOf(
            component_of=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "component_of")
            )
        )
    elif kind == "componentEntryOf":
        return ComponentGraphProjectionComponentEntryOf(
            component_entry_of=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "component_entry_of")
            )
        )
    elif kind == "members":
        return ComponentGraphProjectionMembers(
            members=destack._generated.source.file.model.component.from_json_component_id(
                json_field(object_, "members")
            )
        )
    elif kind == "dependencies":
        return ComponentGraphProjectionDependencies(
            dependencies=destack._generated.source.file.model.component.from_json_component_id(
                json_field(object_, "dependencies")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Stable fingerprint of one observed artifact projection."""
ArtifactProjectionFingerprint: typing.TypeAlias = int


def encode_artifact_projection_fingerprint(
    writer: BinaryWriter, value: ArtifactProjectionFingerprint
) -> None:
    """Encode one ArtifactProjectionFingerprint."""
    writer.write_unsigned(value)


def decode_artifact_projection_fingerprint(
    reader: BinaryReader,
) -> ArtifactProjectionFingerprint:
    """Decode one ArtifactProjectionFingerprint."""
    return reader.read_unsigned()


def to_json_artifact_projection_fingerprint(
    value: ArtifactProjectionFingerprint,
) -> Json:
    """Return one JSON value for one ArtifactProjectionFingerprint."""
    return value


def from_json_artifact_projection_fingerprint(
    value: Json,
) -> ArtifactProjectionFingerprint:
    """Return one ArtifactProjectionFingerprint from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class SourceDependency:
    """One exact source file content observed while building an artifact."""

    # the source file id
    file: destack._generated.source.file.model.file.FileId
    # the exact source content id
    content: destack._generated.source.file.model.file.ContentId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_dependency(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceDependency:
        """Decode one SourceDependency."""
        return decode_source_dependency(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_dependency(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceDependency:
        """Return one SourceDependency from one JSON value."""
        return from_json_source_dependency(value)


def encode_source_dependency(writer: BinaryWriter, value: SourceDependency) -> None:
    """Encode one SourceDependency."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.source.file.model.file.encode_content_id(writer, value.content)


def decode_source_dependency(reader: BinaryReader) -> SourceDependency:
    """Decode one SourceDependency."""
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    content = destack._generated.source.file.model.file.decode_content_id(reader)

    return SourceDependency(
        file=file,
        content=content,
    )


def to_json_source_dependency(value: SourceDependency) -> Json:
    """Return one JSON value for one SourceDependency."""
    return {
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "content": destack._generated.source.file.model.file.to_json_content_id(
            value.content
        ),
    }


def from_json_source_dependency(value: Json) -> SourceDependency:
    """Return one SourceDependency from one JSON value."""
    object_ = json_object(value)

    return SourceDependency(
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        content=destack._generated.source.file.model.file.from_json_content_id(
            json_field(object_, "content")
        ),
    )


@dataclass(frozen=True, slots=True)
class ArtifactDependencyArtifact:
    """Another exact artifact version."""

    artifact: destack._generated.artifact.core.version.ArtifactVersion
    kind: typing.Literal["artifact"] = "artifact"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_dependency(self)


@dataclass(frozen=True, slots=True)
class ArtifactDependencyProjection:
    """One exact projected artifact value."""

    projection: ArtifactProjectionDependency
    kind: typing.Literal["projection"] = "projection"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_dependency(self)


@dataclass(frozen=True, slots=True)
class ArtifactDependencySource:
    """One exact primitive source observation."""

    source: SourceDependency
    kind: typing.Literal["source"] = "source"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_dependency(self)


"""One exact dependency read while building an artifact."""
ArtifactDependency: typing.TypeAlias = (
    ArtifactDependencyArtifact | ArtifactDependencyProjection | ArtifactDependencySource
)


def encode_artifact_dependency(writer: BinaryWriter, value: ArtifactDependency) -> None:
    """Encode one ArtifactDependency."""
    if value.kind == "artifact":
        writer.write_unsigned(0)
        destack._generated.artifact.core.version.encode_artifact_version(
            writer, value.artifact
        )
    elif value.kind == "projection":
        writer.write_unsigned(1)
        encode_artifact_projection_dependency(writer, value.projection)
    elif value.kind == "source":
        writer.write_unsigned(2)
        encode_source_dependency(writer, value.source)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_dependency(reader: BinaryReader) -> ArtifactDependency:
    """Decode one ArtifactDependency."""
    variant = reader.read_number()

    if variant == 0:
        artifact = destack._generated.artifact.core.version.decode_artifact_version(
            reader
        )

        return ArtifactDependencyArtifact(artifact=artifact)
    elif variant == 1:
        projection = decode_artifact_projection_dependency(reader)

        return ArtifactDependencyProjection(projection=projection)
    elif variant == 2:
        source = decode_source_dependency(reader)

        return ArtifactDependencySource(source=source)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_dependency(value: ArtifactDependency) -> Json:
    """Return one JSON value for one ArtifactDependency."""
    if value.kind == "artifact":
        return {
            "kind": "artifact",
            "artifact": destack._generated.artifact.core.version.to_json_artifact_version(
                value.artifact
            ),
        }
    elif value.kind == "projection":
        return {
            "kind": "projection",
            "projection": to_json_artifact_projection_dependency(value.projection),
        }
    elif value.kind == "source":
        return {
            "kind": "source",
            "source": to_json_source_dependency(value.source),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_artifact_dependency(value: Json) -> ArtifactDependency:
    """Return one ArtifactDependency from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "artifact":
        return ArtifactDependencyArtifact(
            artifact=destack._generated.artifact.core.version.from_json_artifact_version(
                json_field(object_, "artifact")
            )
        )
    elif kind == "projection":
        return ArtifactDependencyProjection(
            projection=from_json_artifact_projection_dependency(
                json_field(object_, "projection")
            )
        )
    elif kind == "source":
        return ArtifactDependencySource(
            source=from_json_source_dependency(json_field(object_, "source"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
