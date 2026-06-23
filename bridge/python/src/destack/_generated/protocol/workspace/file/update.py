# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.repository.revision
import destack._generated.protocol.source.edit.update
import destack._generated.protocol.workspace.file.image
import destack._generated.protocol.workspace.message

if TYPE_CHECKING:
    from destack._generated.protocol.repository.revision import (
        Revision,
    )

    from destack._generated.protocol.source.edit.update import (
        Edit,
    )

    from destack._generated.protocol.workspace.file.image import (
        FileUpdate,
    )

    from destack._generated.protocol.workspace.message import (
        Message,
    )


@dataclass(frozen=True, slots=True)
class SourceUpdate:
    """One requested source mutation batch."""

    """Optional expected base revision."""
    base: Revision | None
    """Source file edits."""
    edits: Sequence[Edit]


def encode_source_update(writer: Writer, value: SourceUpdate) -> None:
    if value.base is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.repository.revision.encode_revision(
            writer, value.base
        )
    writer.write_unsigned(len(value.edits))
    for item_0 in value.edits:
        destack._generated.protocol.source.edit.update.encode_edit(writer, item_0)


def decode_source_update(reader: Reader) -> SourceUpdate:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.repository.revision.decode_revision(reader)
    )
    field_1 = [
        destack._generated.protocol.source.edit.update.decode_edit(reader)
        for _ in range(reader.read_number())
    ]

    return SourceUpdate(
        base=field_0,
        edits=field_1,
    )


@dataclass(frozen=True, slots=True)
class Commit:
    """Workspace projection of one committed edit batch."""

    """Previous repository revision."""
    before: Revision
    """Updated repository revision."""
    after: Revision
    """Workspace updates produced by the commit."""
    updates: Sequence[FileUpdate]
    """Messages produced by the commit."""
    messages: Sequence[Message]


def encode_commit(writer: Writer, value: Commit) -> None:
    destack._generated.protocol.repository.revision.encode_revision(
        writer, value.before
    )
    destack._generated.protocol.repository.revision.encode_revision(writer, value.after)
    writer.write_unsigned(len(value.updates))
    for item_0 in value.updates:
        destack._generated.protocol.workspace.file.image.encode_file_update(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        destack._generated.protocol.workspace.message.encode_message(writer, item_0)


def decode_commit(reader: Reader) -> Commit:
    field_0 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_1 = destack._generated.protocol.repository.revision.decode_revision(reader)
    field_2 = [
        destack._generated.protocol.workspace.file.image.decode_file_update(reader)
        for _ in range(reader.read_number())
    ]
    field_3 = [
        destack._generated.protocol.workspace.message.decode_message(reader)
        for _ in range(reader.read_number())
    ]

    return Commit(
        before=field_0,
        after=field_1,
        updates=field_2,
        messages=field_3,
    )


__all__ = [
    "SourceUpdate",
    "encode_source_update",
    "decode_source_update",
    "Commit",
    "encode_commit",
    "decode_commit",
]
