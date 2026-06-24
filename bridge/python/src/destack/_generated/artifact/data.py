# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class DataJson:
    """One parsed JSON-like module value."""

    json: typing.Any
    kind: typing.Literal["json"] = "json"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_data(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_data(self)


"""One parsed non-code module payload."""
Data: typing.TypeAlias = DataJson


def encode_data(writer: BinaryWriter, value: Data) -> None:
    """Encode one Data."""
    if value.kind == "json":
        writer.write_unsigned(0)
        writer.write_json(value.json)
    else:
        raise SerdeError("unknown enum variant")


def decode_data(reader: BinaryReader) -> Data:
    """Decode one Data."""
    variant = reader.read_number()

    if variant == 0:
        json = reader.read_json()

        return DataJson(json=json)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_data(value: Data) -> Json:
    """Return one JSON value for one Data."""
    if value.kind == "json":
        return {
            "kind": "json",
            "json": value.json,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_data(value: Json) -> Data:
    """Return one Data from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "json":
        return DataJson(json=json_field(object_, "json"))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Data",
    "encode_data",
    "decode_data",
    "to_json_data",
    "from_json_data",
    "DataJson",
]
