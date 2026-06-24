# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.source.file.model.file


@dataclass(frozen=True, slots=True)
class ArtifactSidecar:
    """One named artifact sidecar."""

    # the sidecar name
    name: str
    # the stable labels describing this sidecar
    labels: Mapping[str, str]
    # the sidecar content
    content: destack._generated.source.file.model.file.Content

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_sidecar(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactSidecar:
        """Decode one ArtifactSidecar."""
        return decode_artifact_sidecar(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_sidecar(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactSidecar:
        """Return one ArtifactSidecar from one JSON value."""
        return from_json_artifact_sidecar(value)


def encode_artifact_sidecar(writer: BinaryWriter, value: ArtifactSidecar) -> None:
    """Encode one ArtifactSidecar."""
    writer.write_string(value.name)
    entries_value_labels_0 = []
    for key_value_labels_0, item_value_labels_0 in value.labels.items():

        def write_key_value_labels_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_labels_0)

        key_bytes = nested_bytes(write_key_value_labels_0)
        entries_value_labels_0.append(
            (key_value_labels_0, item_value_labels_0, key_bytes)
        )
    entries_value_labels_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_labels_0))
    for entry_value_labels_0 in entries_value_labels_0:
        writer.write_string(entry_value_labels_0[0])
        writer.write_string(entry_value_labels_0[1])
    destack._generated.source.file.model.file.encode_content(writer, value.content)


def decode_artifact_sidecar(reader: BinaryReader) -> ArtifactSidecar:
    """Decode one ArtifactSidecar."""
    name = reader.read_string()
    labels = {
        reader.read_string(): reader.read_string() for _ in range(reader.read_number())
    }
    content = destack._generated.source.file.model.file.decode_content(reader)

    return ArtifactSidecar(
        name=name,
        labels=labels,
        content=content,
    )


def to_json_artifact_sidecar(value: ArtifactSidecar) -> Json:
    """Return one JSON value for one ArtifactSidecar."""
    return {
        "name": value.name,
        "labels": {key_0: item_0 for key_0, item_0 in value.labels.items()},
        "content": destack._generated.source.file.model.file.to_json_content(
            value.content
        ),
    }


def from_json_artifact_sidecar(value: Json) -> ArtifactSidecar:
    """Return one ArtifactSidecar from one JSON value."""
    object_ = json_object(value)

    return ArtifactSidecar(
        name=json_string(json_field(object_, "name")),
        labels={
            key_0: json_string(item_0)
            for key_0, item_0 in json_object(json_field(object_, "labels")).items()
        },
        content=destack._generated.source.file.model.file.from_json_content(
            json_field(object_, "content")
        ),
    )


__all__ = [
    "ArtifactSidecar",
    "encode_artifact_sidecar",
    "decode_artifact_sidecar",
    "to_json_artifact_sidecar",
    "from_json_artifact_sidecar",
]
