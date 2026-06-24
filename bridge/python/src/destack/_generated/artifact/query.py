# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
)

import destack._generated.artifact.core.version
import destack._generated.qir.index.index


@dataclass(frozen=True, slots=True)
class ModuleQueryIndex:
    """Query index for one module profile."""

    # the indexed query surface
    index: destack._generated.qir.index.index.QueryIndex

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_query_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleQueryIndex:
        """Decode one ModuleQueryIndex."""
        return decode_module_query_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_query_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleQueryIndex:
        """Return one ModuleQueryIndex from one JSON value."""
        return from_json_module_query_index(value)


def encode_module_query_index(writer: BinaryWriter, value: ModuleQueryIndex) -> None:
    """Encode one ModuleQueryIndex."""
    destack._generated.qir.index.index.encode_query_index(writer, value.index)


def decode_module_query_index(reader: BinaryReader) -> ModuleQueryIndex:
    """Decode one ModuleQueryIndex."""
    index = destack._generated.qir.index.index.decode_query_index(reader)

    return ModuleQueryIndex(
        index=index,
    )


def to_json_module_query_index(value: ModuleQueryIndex) -> Json:
    """Return one JSON value for one ModuleQueryIndex."""
    return {
        "index": destack._generated.qir.index.index.to_json_query_index(value.index),
    }


def from_json_module_query_index(value: Json) -> ModuleQueryIndex:
    """Return one ModuleQueryIndex from one JSON value."""
    object_ = json_object(value)

    return ModuleQueryIndex(
        index=destack._generated.qir.index.index.from_json_query_index(
            json_field(object_, "index")
        ),
    )


@dataclass(frozen=True, slots=True)
class WorkspaceQueryIndex:
    """Query index for one workspace profile."""

    # the module query indexes in this workspace profile
    modules: Sequence[destack._generated.artifact.core.version.ArtifactVersion]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_query_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceQueryIndex:
        """Decode one WorkspaceQueryIndex."""
        return decode_workspace_query_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_query_index(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceQueryIndex:
        """Return one WorkspaceQueryIndex from one JSON value."""
        return from_json_workspace_query_index(value)


def encode_workspace_query_index(
    writer: BinaryWriter, value: WorkspaceQueryIndex
) -> None:
    """Encode one WorkspaceQueryIndex."""
    writer.write_unsigned(len(value.modules))
    for item_value_modules_0 in value.modules:
        destack._generated.artifact.core.version.encode_artifact_version(
            writer, item_value_modules_0
        )


def decode_workspace_query_index(reader: BinaryReader) -> WorkspaceQueryIndex:
    """Decode one WorkspaceQueryIndex."""
    modules = [
        destack._generated.artifact.core.version.decode_artifact_version(reader)
        for _ in range(reader.read_number())
    ]

    return WorkspaceQueryIndex(
        modules=modules,
    )


def to_json_workspace_query_index(value: WorkspaceQueryIndex) -> Json:
    """Return one JSON value for one WorkspaceQueryIndex."""
    return {
        "modules": [
            destack._generated.artifact.core.version.to_json_artifact_version(item_0)
            for item_0 in value.modules
        ],
    }


def from_json_workspace_query_index(value: Json) -> WorkspaceQueryIndex:
    """Return one WorkspaceQueryIndex from one JSON value."""
    object_ = json_object(value)

    return WorkspaceQueryIndex(
        modules=[
            destack._generated.artifact.core.version.from_json_artifact_version(item_0)
            for item_0 in json_array(json_field(object_, "modules"))
        ],
    )


__all__ = [
    "ModuleQueryIndex",
    "encode_module_query_index",
    "decode_module_query_index",
    "to_json_module_query_index",
    "from_json_module_query_index",
    "WorkspaceQueryIndex",
    "encode_workspace_query_index",
    "decode_workspace_query_index",
    "to_json_workspace_query_index",
    "from_json_workspace_query_index",
]
