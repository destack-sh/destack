# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    nested_bytes,
)

import destack._generated.mir.metadata.frame
import destack._generated.mir.tree.value
import destack._generated.program.function


@dataclass(frozen=True, slots=True)
class ResumeTable:
    """VM resume states keyed by execution frame state."""

    # resume states by dense frame state id
    states: Sequence[ResumeState]
    # frame state id by lowered program point
    state_by_point: Mapping[
        ProgramPoint, destack._generated.mir.metadata.frame.FrameStateId
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resume_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeTable:
        """Decode one ResumeTable."""
        return decode_resume_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resume_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ResumeTable:
        """Return one ResumeTable from one JSON value."""
        return from_json_resume_table(value)


def encode_resume_table(writer: BinaryWriter, value: ResumeTable) -> None:
    """Encode one ResumeTable."""
    writer.write_unsigned(len(value.states))
    for item_value_states_0 in value.states:
        encode_resume_state(writer, item_value_states_0)
    entries_value_state_by_point_0 = []
    for (
        key_value_state_by_point_0,
        item_value_state_by_point_0,
    ) in value.state_by_point.items():

        def write_key_value_state_by_point_0(writer: BinaryWriter) -> None:
            encode_program_point(writer, key_value_state_by_point_0)

        key_bytes = nested_bytes(write_key_value_state_by_point_0)
        entries_value_state_by_point_0.append(
            (key_value_state_by_point_0, item_value_state_by_point_0, key_bytes)
        )
    entries_value_state_by_point_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_state_by_point_0))
    for entry_value_state_by_point_0 in entries_value_state_by_point_0:
        encode_program_point(writer, entry_value_state_by_point_0[0])
        destack._generated.mir.metadata.frame.encode_frame_state_id(
            writer, entry_value_state_by_point_0[1]
        )


def decode_resume_table(reader: BinaryReader) -> ResumeTable:
    """Decode one ResumeTable."""
    states = [decode_resume_state(reader) for _ in range(reader.read_number())]
    state_by_point = {
        decode_program_point(
            reader
        ): destack._generated.mir.metadata.frame.decode_frame_state_id(reader)
        for _ in range(reader.read_number())
    }

    return ResumeTable(
        states=states,
        state_by_point=state_by_point,
    )


def to_json_resume_table(value: ResumeTable) -> Json:
    """Return one JSON value for one ResumeTable."""
    return {
        "states": [to_json_resume_state(item_0) for item_0 in value.states],
        "stateByPoint": [
            [
                to_json_program_point(key_0),
                destack._generated.mir.metadata.frame.to_json_frame_state_id(item_0),
            ]
            for key_0, item_0 in value.state_by_point.items()
        ],
    }


def from_json_resume_table(value: Json) -> ResumeTable:
    """Return one ResumeTable from one JSON value."""
    object_ = json_object(value)

    return ResumeTable(
        states=[
            from_json_resume_state(item_0)
            for item_0 in json_array(json_field(object_, "states"))
        ],
        state_by_point={
            from_json_program_point(
                key_0
            ): destack._generated.mir.metadata.frame.from_json_frame_state_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "stateByPoint"))
        },
    )


@dataclass(frozen=True, slots=True)
class ResumeState:
    """VM state for one resumable frame."""

    # the lowered VM program point
    point: ProgramPoint
    # the source MIR point within the lowered block
    mir_point: int
    # entry bindings for block-entry states
    entry: FrameEntry | None
    # caller return destination for post-call states
    return_destination: destack._generated.mir.tree.value.Value | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_resume_state(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ResumeState:
        """Decode one ResumeState."""
        return decode_resume_state(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_resume_state(self)

    @classmethod
    def from_json(cls, value: Json) -> ResumeState:
        """Return one ResumeState from one JSON value."""
        return from_json_resume_state(value)


def encode_resume_state(writer: BinaryWriter, value: ResumeState) -> None:
    """Encode one ResumeState."""
    encode_program_point(writer, value.point)
    writer.write_unsigned(value.mir_point)
    if value.entry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_frame_entry(writer, value.entry)
    if value.return_destination is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.tree.value.encode_value(writer, value.return_destination)


def decode_resume_state(reader: BinaryReader) -> ResumeState:
    """Decode one ResumeState."""
    point = decode_program_point(reader)
    mir_point = reader.read_number()
    entry = reader.read_option(lambda: decode_frame_entry(reader))
    return_destination = reader.read_option(
        lambda: destack._generated.mir.tree.value.decode_value(reader)
    )

    return ResumeState(
        point=point,
        mir_point=mir_point,
        entry=entry,
        return_destination=return_destination,
    )


def to_json_resume_state(value: ResumeState) -> Json:
    """Return one JSON value for one ResumeState."""
    return {
        "point": to_json_program_point(value.point),
        "mirPoint": value.mir_point,
        **({} if value.entry is None else {"entry": to_json_frame_entry(value.entry)}),
        **(
            {}
            if value.return_destination is None
            else {
                "returnDestination": destack._generated.mir.tree.value.to_json_value(
                    value.return_destination
                )
            }
        ),
    }


def from_json_resume_state(value: Json) -> ResumeState:
    """Return one ResumeState from one JSON value."""
    object_ = json_object(value)

    return ResumeState(
        point=from_json_program_point(json_field(object_, "point")),
        mir_point=json_int(json_field(object_, "mirPoint")),
        entry=json_optional(
            object_, "entry", lambda value: from_json_frame_entry(value)
        ),
        return_destination=json_optional(
            object_,
            "returnDestination",
            lambda value: destack._generated.mir.tree.value.from_json_value(value),
        ),
    )


@dataclass(frozen=True, slots=True)
class ProgramPoint:
    """One lowered VM program point."""

    # the owning function
    function: destack._generated.program.function.FunctionId
    # the lowered block index
    block: int
    # the lowered program counter inside the block
    pc: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_point(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramPoint:
        """Decode one ProgramPoint."""
        return decode_program_point(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_point(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramPoint:
        """Return one ProgramPoint from one JSON value."""
        return from_json_program_point(value)


def encode_program_point(writer: BinaryWriter, value: ProgramPoint) -> None:
    """Encode one ProgramPoint."""
    destack._generated.program.function.encode_function_id(writer, value.function)
    writer.write_unsigned(value.block)
    writer.write_unsigned(value.pc)


def decode_program_point(reader: BinaryReader) -> ProgramPoint:
    """Decode one ProgramPoint."""
    function = destack._generated.program.function.decode_function_id(reader)
    block = reader.read_number()
    pc = reader.read_number()

    return ProgramPoint(
        function=function,
        block=block,
        pc=pc,
    )


def to_json_program_point(value: ProgramPoint) -> Json:
    """Return one JSON value for one ProgramPoint."""
    return {
        "function": destack._generated.program.function.to_json_function_id(
            value.function
        ),
        "block": value.block,
        "pc": value.pc,
    }


def from_json_program_point(value: Json) -> ProgramPoint:
    """Return one ProgramPoint from one JSON value."""
    object_ = json_object(value)

    return ProgramPoint(
        function=destack._generated.program.function.from_json_function_id(
            json_field(object_, "function")
        ),
        block=json_int(json_field(object_, "block")),
        pc=json_int(json_field(object_, "pc")),
    )


@dataclass(frozen=True, slots=True)
class FrameEntry:
    """VM entry bindings for one resume state."""

    # the slot bindings applied on entry
    bindings: Sequence[FrameBinding]
    # the implicit received value slot
    received_value: destack._generated.mir.metadata.frame.FrameSlotId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameEntry:
        """Decode one FrameEntry."""
        return decode_frame_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameEntry:
        """Return one FrameEntry from one JSON value."""
        return from_json_frame_entry(value)


def encode_frame_entry(writer: BinaryWriter, value: FrameEntry) -> None:
    """Encode one FrameEntry."""
    writer.write_unsigned(len(value.bindings))
    for item_value_bindings_0 in value.bindings:
        encode_frame_binding(writer, item_value_bindings_0)
    if value.received_value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.mir.metadata.frame.encode_frame_slot_id(
            writer, value.received_value
        )


def decode_frame_entry(reader: BinaryReader) -> FrameEntry:
    """Decode one FrameEntry."""
    bindings = [decode_frame_binding(reader) for _ in range(reader.read_number())]
    received_value = reader.read_option(
        lambda: destack._generated.mir.metadata.frame.decode_frame_slot_id(reader)
    )

    return FrameEntry(
        bindings=bindings,
        received_value=received_value,
    )


def to_json_frame_entry(value: FrameEntry) -> Json:
    """Return one JSON value for one FrameEntry."""
    return {
        "bindings": [to_json_frame_binding(item_0) for item_0 in value.bindings],
        **(
            {}
            if value.received_value is None
            else {
                "receivedValue": destack._generated.mir.metadata.frame.to_json_frame_slot_id(
                    value.received_value
                )
            }
        ),
    }


def from_json_frame_entry(value: Json) -> FrameEntry:
    """Return one FrameEntry from one JSON value."""
    object_ = json_object(value)

    return FrameEntry(
        bindings=[
            from_json_frame_binding(item_0)
            for item_0 in json_array(json_field(object_, "bindings"))
        ],
        received_value=json_optional(
            object_,
            "receivedValue",
            lambda value: destack._generated.mir.metadata.frame.from_json_frame_slot_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class FrameBinding:
    """One frame slot binding."""

    # the source frame slot
    source: destack._generated.mir.metadata.frame.FrameSlotId
    # the destination frame slot
    destination: destack._generated.mir.metadata.frame.FrameSlotId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_frame_binding(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FrameBinding:
        """Decode one FrameBinding."""
        return decode_frame_binding(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_frame_binding(self)

    @classmethod
    def from_json(cls, value: Json) -> FrameBinding:
        """Return one FrameBinding from one JSON value."""
        return from_json_frame_binding(value)


def encode_frame_binding(writer: BinaryWriter, value: FrameBinding) -> None:
    """Encode one FrameBinding."""
    destack._generated.mir.metadata.frame.encode_frame_slot_id(writer, value.source)
    destack._generated.mir.metadata.frame.encode_frame_slot_id(
        writer, value.destination
    )


def decode_frame_binding(reader: BinaryReader) -> FrameBinding:
    """Decode one FrameBinding."""
    source = destack._generated.mir.metadata.frame.decode_frame_slot_id(reader)
    destination = destack._generated.mir.metadata.frame.decode_frame_slot_id(reader)

    return FrameBinding(
        source=source,
        destination=destination,
    )


def to_json_frame_binding(value: FrameBinding) -> Json:
    """Return one JSON value for one FrameBinding."""
    return {
        "source": destack._generated.mir.metadata.frame.to_json_frame_slot_id(
            value.source
        ),
        "destination": destack._generated.mir.metadata.frame.to_json_frame_slot_id(
            value.destination
        ),
    }


def from_json_frame_binding(value: Json) -> FrameBinding:
    """Return one FrameBinding from one JSON value."""
    object_ = json_object(value)

    return FrameBinding(
        source=destack._generated.mir.metadata.frame.from_json_frame_slot_id(
            json_field(object_, "source")
        ),
        destination=destack._generated.mir.metadata.frame.from_json_frame_slot_id(
            json_field(object_, "destination")
        ),
    )


__all__ = [
    "ResumeTable",
    "encode_resume_table",
    "decode_resume_table",
    "to_json_resume_table",
    "from_json_resume_table",
    "ResumeState",
    "encode_resume_state",
    "decode_resume_state",
    "to_json_resume_state",
    "from_json_resume_state",
    "ProgramPoint",
    "encode_program_point",
    "decode_program_point",
    "to_json_program_point",
    "from_json_program_point",
    "FrameEntry",
    "encode_frame_entry",
    "decode_frame_entry",
    "to_json_frame_entry",
    "from_json_frame_entry",
    "FrameBinding",
    "encode_frame_binding",
    "decode_frame_binding",
    "to_json_frame_binding",
    "from_json_frame_binding",
]
