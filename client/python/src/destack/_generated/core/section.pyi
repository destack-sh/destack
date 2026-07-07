# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionDirectory: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SectionDirectory: ...

def encode_section_directory(writer: BinaryWriter, value: SectionDirectory) -> None: ...
def decode_section_directory(reader: BinaryReader) -> SectionDirectory: ...
def to_json_section_directory(value: SectionDirectory) -> Json: ...
def from_json_section_directory(value: Json) -> SectionDirectory: ...

@dataclass(frozen=True, slots=True)
class SegmentTable:
    """Loadable segment table inside one image."""

    # segments indexed by dense segment id
    segments: Sequence[Segment]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SegmentTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SegmentTable: ...

def encode_segment_table(writer: BinaryWriter, value: SegmentTable) -> None: ...
def decode_segment_table(reader: BinaryReader) -> SegmentTable: ...
def to_json_segment_table(value: SegmentTable) -> Json: ...
def from_json_segment_table(value: Json) -> SegmentTable: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Segment: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Segment: ...

def encode_segment(writer: BinaryWriter, value: Segment) -> None: ...
def decode_segment(reader: BinaryReader) -> Segment: ...
def to_json_segment(value: Segment) -> Json: ...
def from_json_segment(value: Json) -> Segment: ...

@dataclass(frozen=True, slots=True)
class SegmentAccess:
    """Memory access policy for one segment."""

    # packed access flags
    bits: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SegmentAccess: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SegmentAccess: ...

def encode_segment_access(writer: BinaryWriter, value: SegmentAccess) -> None: ...
def decode_segment_access(reader: BinaryReader) -> SegmentAccess: ...
def to_json_segment_access(value: SegmentAccess) -> Json: ...
def from_json_segment_access(value: Json) -> SegmentAccess: ...

@dataclass(frozen=True, slots=True)
class SectionTable:
    """Logical section table inside one image."""

    # sections indexed by dense section id
    sections: Sequence[Section]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionTable: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SectionTable: ...

def encode_section_table(writer: BinaryWriter, value: SectionTable) -> None: ...
def decode_section_table(reader: BinaryReader) -> SectionTable: ...
def to_json_section_table(value: SectionTable) -> Json: ...
def from_json_section_table(value: Json) -> SectionTable: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Section: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Section: ...

def encode_section(writer: BinaryWriter, value: Section) -> None: ...
def decode_section(reader: BinaryReader) -> Section: ...
def to_json_section(value: Section) -> Json: ...
def from_json_section(value: Json) -> Section: ...

"""Dense image segment id."""
SegmentId: typing.TypeAlias = int

def encode_segment_id(writer: BinaryWriter, value: SegmentId) -> None: ...
def decode_segment_id(reader: BinaryReader) -> SegmentId: ...
def to_json_segment_id(value: SegmentId) -> Json: ...
def from_json_segment_id(value: Json) -> SegmentId: ...

@dataclass(frozen=True, slots=True)
class SectionSlice:
    """Typed entry slice stored in one image section."""

    # section containing the typed entries
    section: SectionId
    # byte offset from the containing section base
    byte_offset: int
    # number of typed entries in the slice
    len: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> SectionSlice: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> SectionSlice: ...

def encode_section_slice(writer: BinaryWriter, value: SectionSlice) -> None: ...
def decode_section_slice(reader: BinaryReader) -> SectionSlice: ...
def to_json_section_slice(value: SectionSlice) -> Json: ...
def from_json_section_slice(value: Json) -> SectionSlice: ...

"""Dense image section id."""
SectionId: typing.TypeAlias = int

def encode_section_id(writer: BinaryWriter, value: SectionId) -> None: ...
def decode_section_id(reader: BinaryReader) -> SectionId: ...
def to_json_section_id(value: SectionId) -> Json: ...
def from_json_section_id(value: Json) -> SectionId: ...

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
