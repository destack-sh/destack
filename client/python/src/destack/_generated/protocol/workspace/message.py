# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_string,
)

import destack._generated.protocol.workspace.file.image


@dataclass(frozen=True, slots=True)
class Message:
    """Message payload emitted by one workspace operation."""

    # message severity
    kind: MessageKind
    # stable message code
    code: str
    # human-readable message
    message: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_message(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Message:
        """Decode one Message."""
        return decode_message(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_message(self)

    @classmethod
    def from_json(cls, value: Json) -> Message:
        """Return one Message from one JSON value."""
        return from_json_message(value)


def encode_message(writer: BinaryWriter, value: Message) -> None:
    """Encode one Message."""
    encode_message_kind(writer, value.kind)
    writer.write_string(value.code)
    writer.write_string(value.message)


def decode_message(reader: BinaryReader) -> Message:
    """Decode one Message."""
    kind = decode_message_kind(reader)
    code = reader.read_string()
    message = reader.read_string()

    return Message(
        kind=kind,
        code=code,
        message=message,
    )


def to_json_message(value: Message) -> Json:
    """Return one JSON value for one Message."""
    return {
        "kind": to_json_message_kind(value.kind),
        "code": value.code,
        "message": value.message,
    }


def from_json_message(value: Json) -> Message:
    """Return one Message from one JSON value."""
    object_ = json_object(value)

    return Message(
        kind=from_json_message_kind(json_field(object_, "kind")),
        code=json_string(json_field(object_, "code")),
        message=json_string(json_field(object_, "message")),
    )


"""Message severity for one workspace operation."""
MessageKind: typing.TypeAlias = (
    typing.Literal["info"] | typing.Literal["warning"] | typing.Literal["error"]
)


def encode_message_kind(writer: BinaryWriter, value: MessageKind) -> None:
    """Encode one MessageKind."""
    if value == "info":
        writer.write_unsigned(0)
    elif value == "warning":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_message_kind(reader: BinaryReader) -> MessageKind:
    """Decode one MessageKind."""
    variant = reader.read_number()

    if variant == 0:
        return "info"
    elif variant == 1:
        return "warning"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_message_kind(value: MessageKind) -> Json:
    """Return one JSON value for one MessageKind."""
    return value


def from_json_message_kind(value: Json) -> MessageKind:
    """Return one MessageKind from one JSON value."""
    variant = json_string(value)

    if variant == "info":
        return "info"
    elif variant == "warning":
        return "warning"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class UpdateBatch:
    """Result of applying local workspace updates."""

    # update records produced by the operation
    updates: Sequence[destack._generated.protocol.workspace.file.image.FileUpdate]
    # message records produced by the operation
    messages: Sequence[Message]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_update_batch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> UpdateBatch:
        """Decode one UpdateBatch."""
        return decode_update_batch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_update_batch(self)

    @classmethod
    def from_json(cls, value: Json) -> UpdateBatch:
        """Return one UpdateBatch from one JSON value."""
        return from_json_update_batch(value)


def encode_update_batch(writer: BinaryWriter, value: UpdateBatch) -> None:
    """Encode one UpdateBatch."""
    writer.write_unsigned(len(value.updates))
    for item_value_updates_0 in value.updates:
        destack._generated.protocol.workspace.file.image.encode_file_update(
            writer, item_value_updates_0
        )
    writer.write_unsigned(len(value.messages))
    for item_value_messages_0 in value.messages:
        encode_message(writer, item_value_messages_0)


def decode_update_batch(reader: BinaryReader) -> UpdateBatch:
    """Decode one UpdateBatch."""
    updates = [
        destack._generated.protocol.workspace.file.image.decode_file_update(reader)
        for _ in range(reader.read_number())
    ]
    messages = [decode_message(reader) for _ in range(reader.read_number())]

    return UpdateBatch(
        updates=updates,
        messages=messages,
    )


def to_json_update_batch(value: UpdateBatch) -> Json:
    """Return one JSON value for one UpdateBatch."""
    return {
        "updates": [
            destack._generated.protocol.workspace.file.image.to_json_file_update(item_0)
            for item_0 in value.updates
        ],
        "messages": [to_json_message(item_0) for item_0 in value.messages],
    }


def from_json_update_batch(value: Json) -> UpdateBatch:
    """Return one UpdateBatch from one JSON value."""
    object_ = json_object(value)

    return UpdateBatch(
        updates=[
            destack._generated.protocol.workspace.file.image.from_json_file_update(
                item_0
            )
            for item_0 in json_array(json_field(object_, "updates"))
        ],
        messages=[
            from_json_message(item_0)
            for item_0 in json_array(json_field(object_, "messages"))
        ],
    )


__all__ = [
    "Message",
    "encode_message",
    "decode_message",
    "to_json_message",
    "from_json_message",
    "MessageKind",
    "encode_message_kind",
    "decode_message_kind",
    "to_json_message_kind",
    "from_json_message_kind",
    "UpdateBatch",
    "encode_update_batch",
    "decode_update_batch",
    "to_json_update_batch",
    "from_json_update_batch",
]
