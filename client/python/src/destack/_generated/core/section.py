# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)


@dataclass(frozen=True, slots=True)
class SectionDirectory:
    """Section ids and byte ranges for one image."""

    # loadable image segments
    segments: SegmentTable
    # logical image sections
    sections: SectionTable
    # segment holding immutable entry tables
    table_segment: SegmentId
    # number of initialized bytes in the table segment
    table_byte_len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_section_directory(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionDirectory:
        """Decode one SectionDirectory."""
        return decode_section_directory(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_section_directory(self)

    @classmethod
    def from_json(cls, value: Json) -> SectionDirectory:
        """Return one SectionDirectory from one JSON value."""
        return from_json_section_directory(value)


def encode_section_directory(writer: BinaryWriter, value: SectionDirectory) -> None:
    """Encode one SectionDirectory."""
    encode_segment_table(writer, value.segments)
    encode_section_table(writer, value.sections)
    encode_segment_id(writer, value.table_segment)
    writer.write_unsigned(value.table_byte_len)


def decode_section_directory(reader: BinaryReader) -> SectionDirectory:
    """Decode one SectionDirectory."""
    segments = decode_segment_table(reader)
    sections = decode_section_table(reader)
    table_segment = decode_segment_id(reader)
    table_byte_len = reader.read_number()

    return SectionDirectory(
        segments=segments,
        sections=sections,
        table_segment=table_segment,
        table_byte_len=table_byte_len,
    )


def to_json_section_directory(value: SectionDirectory) -> Json:
    """Return one JSON value for one SectionDirectory."""
    return {
        "segments": to_json_segment_table(value.segments),
        "sections": to_json_section_table(value.sections),
        "tableSegment": to_json_segment_id(value.table_segment),
        "tableByteLen": value.table_byte_len,
    }


def from_json_section_directory(value: Json) -> SectionDirectory:
    """Return one SectionDirectory from one JSON value."""
    object_ = json_object(value)

    return SectionDirectory(
        segments=from_json_segment_table(json_field(object_, "segments")),
        sections=from_json_section_table(json_field(object_, "sections")),
        table_segment=from_json_segment_id(json_field(object_, "tableSegment")),
        table_byte_len=json_int(json_field(object_, "tableByteLen")),
    )


@dataclass(frozen=True, slots=True)
class SegmentTable:
    """Loadable segment table inside one image."""

    # segments indexed by dense segment id
    segments: Sequence[Segment]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_segment_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SegmentTable:
        """Decode one SegmentTable."""
        return decode_segment_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_segment_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SegmentTable:
        """Return one SegmentTable from one JSON value."""
        return from_json_segment_table(value)


def encode_segment_table(writer: BinaryWriter, value: SegmentTable) -> None:
    """Encode one SegmentTable."""
    writer.write_unsigned(len(value.segments))
    for item_value_segments_0 in value.segments:
        encode_segment(writer, item_value_segments_0)


def decode_segment_table(reader: BinaryReader) -> SegmentTable:
    """Decode one SegmentTable."""
    segments = [decode_segment(reader) for _ in range(reader.read_number())]

    return SegmentTable(
        segments=segments,
    )


def to_json_segment_table(value: SegmentTable) -> Json:
    """Return one JSON value for one SegmentTable."""
    return {
        "segments": [to_json_segment(item_0) for item_0 in value.segments],
    }


def from_json_segment_table(value: Json) -> SegmentTable:
    """Return one SegmentTable from one JSON value."""
    object_ = json_object(value)

    return SegmentTable(
        segments=[
            from_json_segment(item_0)
            for item_0 in json_array(json_field(object_, "segments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Segment:
    """Loadable byte segment inside one image."""

    # byte offset from the image base
    offset: int
    # byte length stored in the image
    byte_len: int
    # byte length reserved after loading
    memory_len: int
    # required byte alignment
    alignment: int
    # segment memory access policy
    access: SegmentAccess

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Segment:
        """Decode one Segment."""
        return decode_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> Segment:
        """Return one Segment from one JSON value."""
        return from_json_segment(value)


def encode_segment(writer: BinaryWriter, value: Segment) -> None:
    """Encode one Segment."""
    writer.write_unsigned(value.offset)
    writer.write_unsigned(value.byte_len)
    writer.write_unsigned(value.memory_len)
    writer.write_unsigned(value.alignment)
    encode_segment_access(writer, value.access)


def decode_segment(reader: BinaryReader) -> Segment:
    """Decode one Segment."""
    offset = reader.read_number()
    byte_len = reader.read_number()
    memory_len = reader.read_number()
    alignment = reader.read_number()
    access = decode_segment_access(reader)

    return Segment(
        offset=offset,
        byte_len=byte_len,
        memory_len=memory_len,
        alignment=alignment,
        access=access,
    )


def to_json_segment(value: Segment) -> Json:
    """Return one JSON value for one Segment."""
    return {
        "offset": value.offset,
        "byteLen": value.byte_len,
        "memoryLen": value.memory_len,
        "alignment": value.alignment,
        "access": to_json_segment_access(value.access),
    }


def from_json_segment(value: Json) -> Segment:
    """Return one Segment from one JSON value."""
    object_ = json_object(value)

    return Segment(
        offset=json_int(json_field(object_, "offset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        memory_len=json_int(json_field(object_, "memoryLen")),
        alignment=json_int(json_field(object_, "alignment")),
        access=from_json_segment_access(json_field(object_, "access")),
    )


@dataclass(frozen=True, slots=True)
class SegmentAccess:
    """Memory access policy for one segment."""

    # packed access flags
    bits: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_segment_access(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SegmentAccess:
        """Decode one SegmentAccess."""
        return decode_segment_access(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_segment_access(self)

    @classmethod
    def from_json(cls, value: Json) -> SegmentAccess:
        """Return one SegmentAccess from one JSON value."""
        return from_json_segment_access(value)


def encode_segment_access(writer: BinaryWriter, value: SegmentAccess) -> None:
    """Encode one SegmentAccess."""
    writer.write_unsigned(value.bits)


def decode_segment_access(reader: BinaryReader) -> SegmentAccess:
    """Decode one SegmentAccess."""
    bits = reader.read_number()

    return SegmentAccess(
        bits=bits,
    )


def to_json_segment_access(value: SegmentAccess) -> Json:
    """Return one JSON value for one SegmentAccess."""
    return {
        "bits": value.bits,
    }


def from_json_segment_access(value: Json) -> SegmentAccess:
    """Return one SegmentAccess from one JSON value."""
    object_ = json_object(value)

    return SegmentAccess(
        bits=json_int(json_field(object_, "bits")),
    )


@dataclass(frozen=True, slots=True)
class SectionTable:
    """Logical section table inside one image."""

    # sections indexed by dense section id
    sections: Sequence[Section]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_section_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionTable:
        """Decode one SectionTable."""
        return decode_section_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_section_table(self)

    @classmethod
    def from_json(cls, value: Json) -> SectionTable:
        """Return one SectionTable from one JSON value."""
        return from_json_section_table(value)


def encode_section_table(writer: BinaryWriter, value: SectionTable) -> None:
    """Encode one SectionTable."""
    writer.write_unsigned(len(value.sections))
    for item_value_sections_0 in value.sections:
        encode_section(writer, item_value_sections_0)


def decode_section_table(reader: BinaryReader) -> SectionTable:
    """Decode one SectionTable."""
    sections = [decode_section(reader) for _ in range(reader.read_number())]

    return SectionTable(
        sections=sections,
    )


def to_json_section_table(value: SectionTable) -> Json:
    """Return one JSON value for one SectionTable."""
    return {
        "sections": [to_json_section(item_0) for item_0 in value.sections],
    }


def from_json_section_table(value: Json) -> SectionTable:
    """Return one SectionTable from one JSON value."""
    object_ = json_object(value)

    return SectionTable(
        sections=[
            from_json_section(item_0)
            for item_0 in json_array(json_field(object_, "sections"))
        ],
    )


@dataclass(frozen=True, slots=True)
class Section:
    """Logical byte section inside one image."""

    # segment containing this section
    segment: SegmentId
    # byte offset from the containing segment base
    byte_offset: int
    # section byte length inside the containing segment
    byte_len: int
    # required byte alignment
    alignment: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_section(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Section:
        """Decode one Section."""
        return decode_section(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_section(self)

    @classmethod
    def from_json(cls, value: Json) -> Section:
        """Return one Section from one JSON value."""
        return from_json_section(value)


def encode_section(writer: BinaryWriter, value: Section) -> None:
    """Encode one Section."""
    encode_segment_id(writer, value.segment)
    writer.write_unsigned(value.byte_offset)
    writer.write_unsigned(value.byte_len)
    writer.write_unsigned(value.alignment)


def decode_section(reader: BinaryReader) -> Section:
    """Decode one Section."""
    segment = decode_segment_id(reader)
    byte_offset = reader.read_number()
    byte_len = reader.read_number()
    alignment = reader.read_number()

    return Section(
        segment=segment,
        byte_offset=byte_offset,
        byte_len=byte_len,
        alignment=alignment,
    )


def to_json_section(value: Section) -> Json:
    """Return one JSON value for one Section."""
    return {
        "segment": to_json_segment_id(value.segment),
        "byteOffset": value.byte_offset,
        "byteLen": value.byte_len,
        "alignment": value.alignment,
    }


def from_json_section(value: Json) -> Section:
    """Return one Section from one JSON value."""
    object_ = json_object(value)

    return Section(
        segment=from_json_segment_id(json_field(object_, "segment")),
        byte_offset=json_int(json_field(object_, "byteOffset")),
        byte_len=json_int(json_field(object_, "byteLen")),
        alignment=json_int(json_field(object_, "alignment")),
    )


"""Dense image segment id."""
SegmentId: typing.TypeAlias = int


def encode_segment_id(writer: BinaryWriter, value: SegmentId) -> None:
    """Encode one SegmentId."""
    writer.write_unsigned(value)


def decode_segment_id(reader: BinaryReader) -> SegmentId:
    """Decode one SegmentId."""
    return reader.read_number()


def to_json_segment_id(value: SegmentId) -> Json:
    """Return one JSON value for one SegmentId."""
    return value


def from_json_segment_id(value: Json) -> SegmentId:
    """Return one SegmentId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class SectionSlice:
    """Typed entry slice stored in one image section."""

    # section containing the typed entries
    section: SectionId
    # byte offset from the containing section base
    byte_offset: int
    # number of typed entries in the slice
    len: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_section_slice(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionSlice:
        """Decode one SectionSlice."""
        return decode_section_slice(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_section_slice(self)

    @classmethod
    def from_json(cls, value: Json) -> SectionSlice:
        """Return one SectionSlice from one JSON value."""
        return from_json_section_slice(value)


def encode_section_slice(writer: BinaryWriter, value: SectionSlice) -> None:
    """Encode one SectionSlice."""
    encode_section_id(writer, value.section)
    writer.write_unsigned(value.byte_offset)
    writer.write_unsigned(value.len)


def decode_section_slice(reader: BinaryReader) -> SectionSlice:
    """Decode one SectionSlice."""
    section = decode_section_id(reader)
    byte_offset = reader.read_number()
    len = reader.read_number()

    return SectionSlice(
        section=section,
        byte_offset=byte_offset,
        len=len,
    )


def to_json_section_slice(value: SectionSlice) -> Json:
    """Return one JSON value for one SectionSlice."""
    return {
        "section": to_json_section_id(value.section),
        "byteOffset": value.byte_offset,
        "len": value.len,
    }


def from_json_section_slice(value: Json) -> SectionSlice:
    """Return one SectionSlice from one JSON value."""
    object_ = json_object(value)

    return SectionSlice(
        section=from_json_section_id(json_field(object_, "section")),
        byte_offset=json_int(json_field(object_, "byteOffset")),
        len=json_int(json_field(object_, "len")),
    )


"""Dense image section id."""
SectionId: typing.TypeAlias = int


def encode_section_id(writer: BinaryWriter, value: SectionId) -> None:
    """Encode one SectionId."""
    writer.write_unsigned(value)


def decode_section_id(reader: BinaryReader) -> SectionId:
    """Decode one SectionId."""
    return reader.read_number()


def to_json_section_id(value: SectionId) -> Json:
    """Return one JSON value for one SectionId."""
    return value


def from_json_section_id(value: Json) -> SectionId:
    """Return one SectionId from one JSON value."""
    return json_int(value)


__all__ = [
    "SectionDirectory",
    "encode_section_directory",
    "decode_section_directory",
    "to_json_section_directory",
    "from_json_section_directory",
    "SegmentTable",
    "encode_segment_table",
    "decode_segment_table",
    "to_json_segment_table",
    "from_json_segment_table",
    "Segment",
    "encode_segment",
    "decode_segment",
    "to_json_segment",
    "from_json_segment",
    "SegmentAccess",
    "encode_segment_access",
    "decode_segment_access",
    "to_json_segment_access",
    "from_json_segment_access",
    "SectionTable",
    "encode_section_table",
    "decode_section_table",
    "to_json_section_table",
    "from_json_section_table",
    "Section",
    "encode_section",
    "decode_section",
    "to_json_section",
    "from_json_section",
    "SegmentId",
    "encode_segment_id",
    "decode_segment_id",
    "to_json_segment_id",
    "from_json_segment_id",
    "SectionSlice",
    "encode_section_slice",
    "decode_section_slice",
    "to_json_section_slice",
    "from_json_section_slice",
    "SectionId",
    "encode_section_id",
    "decode_section_id",
    "to_json_section_id",
    "from_json_section_id",
]
