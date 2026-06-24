# generated bridge target, do not edit

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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ArtifactEvent:
    """One artifact event with ordered fields."""

    # the stable event name
    name: str
    # the logical event timestamp inside its log
    timestamp: int
    # the event level
    level: ArtifactEventLevel
    # the event fields in display order
    fields: Sequence[ArtifactEventField]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEvent:
        """Decode one ArtifactEvent."""
        return decode_artifact_event(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactEvent:
        """Return one ArtifactEvent from one JSON value."""
        return from_json_artifact_event(value)


def encode_artifact_event(writer: BinaryWriter, value: ArtifactEvent) -> None:
    """Encode one ArtifactEvent."""
    writer.write_string(value.name)
    writer.write_unsigned(value.timestamp)
    encode_artifact_event_level(writer, value.level)
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_artifact_event_field(writer, item_value_fields_0)


def decode_artifact_event(reader: BinaryReader) -> ArtifactEvent:
    """Decode one ArtifactEvent."""
    name = reader.read_string()
    timestamp = reader.read_number()
    level = decode_artifact_event_level(reader)
    fields = [decode_artifact_event_field(reader) for _ in range(reader.read_number())]

    return ArtifactEvent(
        name=name,
        timestamp=timestamp,
        level=level,
        fields=fields,
    )


def to_json_artifact_event(value: ArtifactEvent) -> Json:
    """Return one JSON value for one ArtifactEvent."""
    return {
        "name": value.name,
        "timestamp": value.timestamp,
        "level": to_json_artifact_event_level(value.level),
        "fields": [to_json_artifact_event_field(item_0) for item_0 in value.fields],
    }


def from_json_artifact_event(value: Json) -> ArtifactEvent:
    """Return one ArtifactEvent from one JSON value."""
    object_ = json_object(value)

    return ArtifactEvent(
        name=json_string(json_field(object_, "name")),
        timestamp=json_int(json_field(object_, "timestamp")),
        level=from_json_artifact_event_level(json_field(object_, "level")),
        fields=[
            from_json_artifact_event_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
    )


"""One artifact event level."""
ArtifactEventLevel: typing.TypeAlias = (
    typing.Literal["info"] | typing.Literal["debug"] | typing.Literal["error"]
)


def encode_artifact_event_level(
    writer: BinaryWriter, value: ArtifactEventLevel
) -> None:
    """Encode one ArtifactEventLevel."""
    if value == "info":
        writer.write_unsigned(0)
    elif value == "debug":
        writer.write_unsigned(1)
    elif value == "error":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_event_level(reader: BinaryReader) -> ArtifactEventLevel:
    """Decode one ArtifactEventLevel."""
    variant = reader.read_number()

    if variant == 0:
        return "info"
    elif variant == 1:
        return "debug"
    elif variant == 2:
        return "error"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_event_level(value: ArtifactEventLevel) -> Json:
    """Return one JSON value for one ArtifactEventLevel."""
    return value


def from_json_artifact_event_level(value: Json) -> ArtifactEventLevel:
    """Return one ArtifactEventLevel from one JSON value."""
    variant = json_string(value)

    if variant == "info":
        return "info"
    elif variant == "debug":
        return "debug"
    elif variant == "error":
        return "error"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ArtifactEventField:
    """One ordered event field."""

    # the field key
    key: str
    # the field value
    value: ArtifactEventValue

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEventField:
        """Decode one ArtifactEventField."""
        return decode_artifact_event_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event_field(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactEventField:
        """Return one ArtifactEventField from one JSON value."""
        return from_json_artifact_event_field(value)


def encode_artifact_event_field(
    writer: BinaryWriter, value: ArtifactEventField
) -> None:
    """Encode one ArtifactEventField."""
    writer.write_string(value.key)
    encode_artifact_event_value(writer, value.value)


def decode_artifact_event_field(reader: BinaryReader) -> ArtifactEventField:
    """Decode one ArtifactEventField."""
    key = reader.read_string()
    value_ = decode_artifact_event_value(reader)

    return ArtifactEventField(
        key=key,
        value=value_,
    )


def to_json_artifact_event_field(value: ArtifactEventField) -> Json:
    """Return one JSON value for one ArtifactEventField."""
    return {
        "key": value.key,
        "value": to_json_artifact_event_value(value.value),
    }


def from_json_artifact_event_field(value: Json) -> ArtifactEventField:
    """Return one ArtifactEventField from one JSON value."""
    object_ = json_object(value)

    return ArtifactEventField(
        key=json_string(json_field(object_, "key")),
        value=from_json_artifact_event_value(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class ArtifactEventValueBool:
    """Boolean field value."""

    bool: bool
    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event_value(self)


@dataclass(frozen=True, slots=True)
class ArtifactEventValueInteger:
    """Integer field value."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event_value(self)


@dataclass(frozen=True, slots=True)
class ArtifactEventValueText:
    """Text field value."""

    text: str
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event_value(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event_value(self)


"""One artifact event field value."""
ArtifactEventValue: typing.TypeAlias = (
    ArtifactEventValueBool | ArtifactEventValueInteger | ArtifactEventValueText
)


def encode_artifact_event_value(
    writer: BinaryWriter, value: ArtifactEventValue
) -> None:
    """Encode one ArtifactEventValue."""
    if value.kind == "bool":
        writer.write_unsigned(0)
        writer.write_bool(value.bool)
    elif value.kind == "integer":
        writer.write_unsigned(1)
        writer.write_signed(value.integer)
    elif value.kind == "text":
        writer.write_unsigned(2)
        writer.write_string(value.text)
    else:
        raise SerdeError("unknown enum variant")


def decode_artifact_event_value(reader: BinaryReader) -> ArtifactEventValue:
    """Decode one ArtifactEventValue."""
    variant = reader.read_number()

    if variant == 0:
        bool = reader.read_bool()

        return ArtifactEventValueBool(bool=bool)
    elif variant == 1:
        integer = reader.read_signed_number()

        return ArtifactEventValueInteger(integer=integer)
    elif variant == 2:
        text = reader.read_string()

        return ArtifactEventValueText(text=text)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_artifact_event_value(value: ArtifactEventValue) -> Json:
    """Return one JSON value for one ArtifactEventValue."""
    if value.kind == "bool":
        return {
            "kind": "bool",
            "bool": value.bool,
        }
    elif value.kind == "integer":
        return {
            "kind": "integer",
            "integer": value.integer,
        }
    elif value.kind == "text":
        return {
            "kind": "text",
            "text": value.text,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_artifact_event_value(value: Json) -> ArtifactEventValue:
    """Return one ArtifactEventValue from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "bool":
        return ArtifactEventValueBool(bool=json_bool(json_field(object_, "bool")))
    elif kind == "integer":
        return ArtifactEventValueInteger(
            integer=json_int(json_field(object_, "integer"))
        )
    elif kind == "text":
        return ArtifactEventValueText(text=json_string(json_field(object_, "text")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ArtifactEventLog:
    """Line-oriented artifact event log."""

    # the events in emission order
    events: Sequence[ArtifactEvent]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_artifact_event_log(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEventLog:
        """Decode one ArtifactEventLog."""
        return decode_artifact_event_log(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_artifact_event_log(self)

    @classmethod
    def from_json(cls, value: Json) -> ArtifactEventLog:
        """Return one ArtifactEventLog from one JSON value."""
        return from_json_artifact_event_log(value)


def encode_artifact_event_log(writer: BinaryWriter, value: ArtifactEventLog) -> None:
    """Encode one ArtifactEventLog."""
    writer.write_unsigned(len(value.events))
    for item_value_events_0 in value.events:
        encode_artifact_event(writer, item_value_events_0)


def decode_artifact_event_log(reader: BinaryReader) -> ArtifactEventLog:
    """Decode one ArtifactEventLog."""
    events = [decode_artifact_event(reader) for _ in range(reader.read_number())]

    return ArtifactEventLog(
        events=events,
    )


def to_json_artifact_event_log(value: ArtifactEventLog) -> Json:
    """Return one JSON value for one ArtifactEventLog."""
    return {
        "events": [to_json_artifact_event(item_0) for item_0 in value.events],
    }


def from_json_artifact_event_log(value: Json) -> ArtifactEventLog:
    """Return one ArtifactEventLog from one JSON value."""
    object_ = json_object(value)

    return ArtifactEventLog(
        events=[
            from_json_artifact_event(item_0)
            for item_0 in json_array(json_field(object_, "events"))
        ],
    )


__all__ = [
    "ArtifactEvent",
    "encode_artifact_event",
    "decode_artifact_event",
    "to_json_artifact_event",
    "from_json_artifact_event",
    "ArtifactEventLevel",
    "encode_artifact_event_level",
    "decode_artifact_event_level",
    "to_json_artifact_event_level",
    "from_json_artifact_event_level",
    "ArtifactEventField",
    "encode_artifact_event_field",
    "decode_artifact_event_field",
    "to_json_artifact_event_field",
    "from_json_artifact_event_field",
    "ArtifactEventValue",
    "encode_artifact_event_value",
    "decode_artifact_event_value",
    "to_json_artifact_event_value",
    "from_json_artifact_event_value",
    "ArtifactEventValueBool",
    "ArtifactEventValueInteger",
    "ArtifactEventValueText",
    "ArtifactEventLog",
    "encode_artifact_event_log",
    "decode_artifact_event_log",
    "to_json_artifact_event_log",
    "from_json_artifact_event_log",
]
