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
    json_optional,
)

import destack._generated.protocol.workspace.file.image
import destack._generated.protocol.workspace.message
import destack._generated.repository.revision
import destack._generated.source.edit.update


@dataclass(frozen=True, slots=True)
class SourceUpdate:
    """One requested source mutation batch."""

    # optional expected base revision
    base: destack._generated.repository.revision.Revision | None
    # source file edits
    edits: Sequence[destack._generated.source.edit.update.Edit]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_update(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SourceUpdate:
        """Decode one SourceUpdate."""
        return decode_source_update(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_update(self)

    @classmethod
    def from_json(cls, value: Json) -> SourceUpdate:
        """Return one SourceUpdate from one JSON value."""
        return from_json_source_update(value)


def encode_source_update(writer: BinaryWriter, value: SourceUpdate) -> None:
    """Encode one SourceUpdate."""
    if value.base is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.revision.encode_revision(writer, value.base)
    writer.write_unsigned(len(value.edits))
    for item_value_edits_0 in value.edits:
        destack._generated.source.edit.update.encode_edit(writer, item_value_edits_0)


def decode_source_update(reader: BinaryReader) -> SourceUpdate:
    """Decode one SourceUpdate."""
    base = reader.read_option(
        lambda: destack._generated.repository.revision.decode_revision(reader)
    )
    edits = [
        destack._generated.source.edit.update.decode_edit(reader)
        for _ in range(reader.read_number())
    ]

    return SourceUpdate(
        base=base,
        edits=edits,
    )


def to_json_source_update(value: SourceUpdate) -> Json:
    """Return one JSON value for one SourceUpdate."""
    return {
        **(
            {}
            if value.base is None
            else {
                "base": destack._generated.repository.revision.to_json_revision(
                    value.base
                )
            }
        ),
        "edits": [
            destack._generated.source.edit.update.to_json_edit(item_0)
            for item_0 in value.edits
        ],
    }


def from_json_source_update(value: Json) -> SourceUpdate:
    """Return one SourceUpdate from one JSON value."""
    object_ = json_object(value)

    return SourceUpdate(
        base=json_optional(
            object_,
            "base",
            lambda value: destack._generated.repository.revision.from_json_revision(
                value
            ),
        ),
        edits=[
            destack._generated.source.edit.update.from_json_edit(item_0)
            for item_0 in json_array(json_field(object_, "edits"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Commit:
    """Workspace projection of one committed edit batch."""

    # previous repository revision
    before: destack._generated.repository.revision.Revision
    # updated repository revision
    after: destack._generated.repository.revision.Revision
    # workspace updates produced by the commit
    updates: Sequence[destack._generated.protocol.workspace.file.image.FileUpdate]
    # messages produced by the commit
    messages: Sequence[destack._generated.protocol.workspace.message.Message]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_commit(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Commit:
        """Decode one Commit."""
        return decode_commit(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_commit(self)

    @classmethod
    def from_json(cls, value: Json) -> Commit:
        """Return one Commit from one JSON value."""
        return from_json_commit(value)


def encode_commit(writer: BinaryWriter, value: Commit) -> None:
    """Encode one Commit."""
    destack._generated.repository.revision.encode_revision(writer, value.before)
    destack._generated.repository.revision.encode_revision(writer, value.after)
    writer.write_unsigned(len(value.updates))
    for item_value_updates_0 in value.updates:
        destack._generated.protocol.workspace.file.image.encode_file_update(
            writer, item_value_updates_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(
            writer, item_value_messages_0
        )


def decode_commit(reader: BinaryReader) -> Commit:
    """Decode one Commit."""
    before = destack._generated.repository.revision.decode_revision(reader)
    after = destack._generated.repository.revision.decode_revision(reader)
    updates = [
        destack._generated.protocol.workspace.file.image.decode_file_update(reader)
        for _ in range(reader.read_number())
    ]
    messages = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return Commit(
        before=before,
        after=after,
        updates=updates,
        messages=messages,
    )


def to_json_commit(value: Commit) -> Json:
    """Return one JSON value for one Commit."""
    return {
        "before": destack._generated.repository.revision.to_json_revision(value.before),
        "after": destack._generated.repository.revision.to_json_revision(value.after),
        "updates": [
            destack._generated.protocol.workspace.file.image.to_json_file_update(item_0)
            for item_0 in value.updates
        ],
        "messages": [
            destack._generated.protocol.workspace.message.to_json_message(item_0)
            for item_0 in value.messages
        ],
    }


def from_json_commit(value: Json) -> Commit:
    """Return one Commit from one JSON value."""
    object_ = json_object(value)

    return Commit(
        before=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "before")
        ),
        after=destack._generated.repository.revision.from_json_revision(
            json_field(object_, "after")
        ),
        updates=[
            destack._generated.protocol.workspace.file.image.from_json_file_update(
                item_0
            )
            for item_0 in json_array(json_field(object_, "updates"))
        ],
        messages=[
            destack._generated.protocol.workspace.message.from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
    )


__all__ = [
    "SourceUpdate",
    "encode_source_update",
    "decode_source_update",
    "to_json_source_update",
    "from_json_source_update",
    "Commit",
    "encode_commit",
    "decode_commit",
    "to_json_commit",
    "from_json_commit",
]
