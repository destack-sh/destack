# generated client target, do not edit

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
    json_string,
)

import destack._generated.mir.metadata.frame
import destack._generated.program.function


@dataclass(frozen=True, slots=True)
class EntryTable:
    """Native entry table keyed by program ids."""

    # native function entries keyed by program function id
    function: Sequence[Entry | None]
    # native resume entries keyed by frame state id
    resume: Sequence[Resume | None]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entry_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> EntryTable:
        """Decode one EntryTable."""
        return decode_entry_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entry_table(self)

    @classmethod
    def from_json(cls, value: Json) -> EntryTable:
        """Return one EntryTable from one JSON value."""
        return from_json_entry_table(value)


def encode_entry_table(writer: BinaryWriter, value: EntryTable) -> None:
    """Encode one EntryTable."""
    writer.write_unsigned(len(value.function))
    for item_value_function_0 in value.function:
        if item_value_function_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_entry(writer, item_value_function_0)
    writer.write_unsigned(len(value.resume))
    for item_value_resume_0 in value.resume:
        if item_value_resume_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_resume(writer, item_value_resume_0)


def decode_entry_table(reader: BinaryReader) -> EntryTable:
    """Decode one EntryTable."""
    function = [
        reader.read_option(lambda: decode_entry(reader))
        for _ in range(reader.read_number())
    ]
    resume = [
        reader.read_option(lambda: decode_resume(reader))
        for _ in range(reader.read_number())
    ]

    return EntryTable(
        function=function,
        resume=resume,
    )


def to_json_entry_table(value: EntryTable) -> Json:
    """Return one JSON value for one EntryTable."""
    return {
        "function": [
            None if item_0 is None else to_json_entry(item_0)
            for item_0 in value.function
        ],
        "resume": [
            None if item_0 is None else to_json_resume(item_0)
            for item_0 in value.resume
        ],
    }


def from_json_entry_table(value: Json) -> EntryTable:
    """Return one EntryTable from one JSON value."""
    object_ = json_object(value)

    return EntryTable(
        function=[
            None if item_0 is None else from_json_entry(item_0)
            for item_0 in json_array(json_field(object_, "function"))
        ],
        resume=[
            None if item_0 is None else from_json_resume(item_0)
            for item_0 in json_array(json_field(object_, "resume"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Entry:
    """Native function entry resolved by symbol name."""

    # the function implemented by this entry
    function: destack._generated.program.function.FunctionId
    # the native symbol exported by the linked image
    symbol: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Entry:
        """Decode one Entry."""
        return decode_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> Entry:
        """Return one Entry from one JSON value."""
        return from_json_entry(value)


def encode_entry(writer: BinaryWriter, value: Entry) -> None:
    """Encode one Entry."""
    destack._generated.program.function.encode_function_id(writer, value.function)
    writer.write_string(value.symbol)


def decode_entry(reader: BinaryReader) -> Entry:
    """Decode one Entry."""
    function = destack._generated.program.function.decode_function_id(reader)
    symbol = reader.read_string()

    return Entry(
        function=function,
        symbol=symbol,
    )


def to_json_entry(value: Entry) -> Json:
    """Return one JSON value for one Entry."""
    return {
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "symbol": value.symbol,
    }


def from_json_entry(value: Json) -> Entry:
    """Return one Entry from one JSON value."""
    object_ = json_object(value)

    return Entry(
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        symbol=json_string(json_field(object_, "symbol")),
    )


@dataclass(frozen=True, slots=True)
class Resume:
    """Native continuation resume entry resolved by symbol name."""

    # the frame state resumed by this entry
    frame_state: destack._generated.mir.metadata.frame.FrameStateId
    # the native symbol exported by the linked image
    symbol: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resume(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Resume:
        """Decode one Resume."""
        return decode_resume(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resume(self)

    @classmethod
    def from_json(cls, value: Json) -> Resume:
        """Return one Resume from one JSON value."""
        return from_json_resume(value)


def encode_resume(writer: BinaryWriter, value: Resume) -> None:
    """Encode one Resume."""
    destack._generated.mir.metadata.frame.encode_frame_state_id(
        writer, value.frame_state
    )
    writer.write_string(value.symbol)


def decode_resume(reader: BinaryReader) -> Resume:
    """Decode one Resume."""
    frame_state = destack._generated.mir.metadata.frame.decode_frame_state_id(reader)
    symbol = reader.read_string()

    return Resume(
        frame_state=frame_state,
        symbol=symbol,
    )


def to_json_resume(value: Resume) -> Json:
    """Return one JSON value for one Resume."""
    return {
        "frameState": destack._generated.mir.metadata.frame.to_json_frame_state_id(
            value.frame_state
        ),
        "symbol": value.symbol,
    }


def from_json_resume(value: Json) -> Resume:
    """Return one Resume from one JSON value."""
    object_ = json_object(value)

    return Resume(
        frame_state=destack._generated.mir.metadata.frame.from_json_frame_state_id(
            json_field(object_, "frameState")
        ),
        symbol=json_string(json_field(object_, "symbol")),
    )


__all__ = [
    "EntryTable",
    "encode_entry_table",
    "decode_entry_table",
    "to_json_entry_table",
    "from_json_entry_table",
    "Entry",
    "encode_entry",
    "decode_entry",
    "to_json_entry",
    "from_json_entry",
    "Resume",
    "encode_resume",
    "decode_resume",
    "to_json_resume",
    "from_json_resume",
]
