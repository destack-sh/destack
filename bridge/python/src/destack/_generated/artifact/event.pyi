# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEvent: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactEvent: ...

def encode_artifact_event(writer: BinaryWriter, value: ArtifactEvent) -> None: ...
def decode_artifact_event(reader: BinaryReader) -> ArtifactEvent: ...
def to_json_artifact_event(value: ArtifactEvent) -> Json: ...
def from_json_artifact_event(value: Json) -> ArtifactEvent: ...

"""One artifact event level."""
ArtifactEventLevel: typing.TypeAlias = (
    typing.Literal["info"] | typing.Literal["debug"] | typing.Literal["error"]
)

def encode_artifact_event_level(
    writer: BinaryWriter, value: ArtifactEventLevel
) -> None: ...
def decode_artifact_event_level(reader: BinaryReader) -> ArtifactEventLevel: ...
def to_json_artifact_event_level(value: ArtifactEventLevel) -> Json: ...
def from_json_artifact_event_level(value: Json) -> ArtifactEventLevel: ...

@dataclass(frozen=True, slots=True)
class ArtifactEventField:
    """One ordered event field."""

    # the field key
    key: str
    # the field value
    value: ArtifactEventValue

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEventField: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactEventField: ...

def encode_artifact_event_field(
    writer: BinaryWriter, value: ArtifactEventField
) -> None: ...
def decode_artifact_event_field(reader: BinaryReader) -> ArtifactEventField: ...
def to_json_artifact_event_field(value: ArtifactEventField) -> Json: ...
def from_json_artifact_event_field(value: Json) -> ArtifactEventField: ...

@dataclass(frozen=True, slots=True)
class ArtifactEventValueBool:
    """Boolean field value."""

    bool: bool
    kind: typing.Literal["bool"] = "bool"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactEventValueInteger:
    """Integer field value."""

    integer: int
    kind: typing.Literal["integer"] = "integer"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class ArtifactEventValueText:
    """Text field value."""

    text: str
    kind: typing.Literal["text"] = "text"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One artifact event field value."""
ArtifactEventValue: typing.TypeAlias = (
    ArtifactEventValueBool | ArtifactEventValueInteger | ArtifactEventValueText
)

def encode_artifact_event_value(
    writer: BinaryWriter, value: ArtifactEventValue
) -> None: ...
def decode_artifact_event_value(reader: BinaryReader) -> ArtifactEventValue: ...
def to_json_artifact_event_value(value: ArtifactEventValue) -> Json: ...
def from_json_artifact_event_value(value: Json) -> ArtifactEventValue: ...

@dataclass(frozen=True, slots=True)
class ArtifactEventLog:
    """Line-oriented artifact event log."""

    # the events in emission order
    events: Sequence[ArtifactEvent]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ArtifactEventLog: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ArtifactEventLog: ...

def encode_artifact_event_log(
    writer: BinaryWriter, value: ArtifactEventLog
) -> None: ...
def decode_artifact_event_log(reader: BinaryReader) -> ArtifactEventLog: ...
def to_json_artifact_event_log(value: ArtifactEventLog) -> Json: ...
def from_json_artifact_event_log(value: Json) -> ArtifactEventLog: ...

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
