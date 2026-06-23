# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.workspace.file.image

if TYPE_CHECKING:
    from destack._generated.protocol.workspace.file.image import (
        FileUpdate,
    )


@dataclass(frozen=True, slots=True)
class Message:
    """Message payload emitted by one workspace operation."""

    """Message severity."""
    kind: MessageKind
    """Stable message code."""
    code: str
    """Human-readable message."""
    message: str


def encode_message(writer: Writer, value: Message) -> None:
    encode_message_kind(writer, value.kind)
    writer.write_string(value.code)
    writer.write_string(value.message)


def decode_message(reader: Reader) -> Message:
    field_0 = decode_message_kind(reader)
    field_1 = reader.read_string()
    field_2 = reader.read_string()

    return Message(
        kind=field_0,
        code=field_1,
        message=field_2,
    )


"""Message severity for one workspace operation."""
MessageKind: TypeAlias = Literal["info"] | Literal["warning"] | Literal["error"]


def encode_message_kind(writer: Writer, value: MessageKind) -> None:
    if value == "info":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_message_kind(reader: Reader) -> MessageKind:
    variant = reader.read_number()

    if variant == 0:
        return "info"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class UpdateBatch:
    """Result of applying local workspace updates."""

    """Update records produced by the operation."""
    updates: Sequence[FileUpdate]
    """Message records produced by the operation."""
    messages: Sequence[Message]


def encode_update_batch(writer: Writer, value: UpdateBatch) -> None:
    writer.write_unsigned(len(value.updates))
    for item_0 in value.updates:
        destack._generated.protocol.workspace.file.image.encode_file_update(
            writer, item_0
        )
    writer.write_unsigned(len(value.messages))
    for item_0 in value.messages:
        encode_message(writer, item_0)


def decode_update_batch(reader: Reader) -> UpdateBatch:
    field_0 = [
        destack._generated.protocol.workspace.file.image.decode_file_update(reader)
        for _ in range(reader.read_number())
    ]
    field_1 = [decode_message(reader) for _ in range(reader.read_number())]

    return UpdateBatch(
        updates=field_0,
        messages=field_1,
    )


__all__ = [
    "Message",
    "encode_message",
    "decode_message",
    "MessageKind",
    "encode_message_kind",
    "decode_message_kind",
    "UpdateBatch",
    "encode_update_batch",
    "decode_update_batch",
]
