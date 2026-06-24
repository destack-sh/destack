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
    json_string,
)

import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class Patch:
    """A single patch: replace a span with new text."""

    # the span to replace
    span: destack._generated.source.file.model.span.Span
    # the replacement text (empty for deletion)
    new_text: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_patch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Patch:
        """Decode one Patch."""
        return decode_patch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_patch(self)

    @classmethod
    def from_json(cls, value: Json) -> Patch:
        """Return one Patch from one JSON value."""
        return from_json_patch(value)


def encode_patch(writer: BinaryWriter, value: Patch) -> None:
    """Encode one Patch."""
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    writer.write_string(value.new_text)


def decode_patch(reader: BinaryReader) -> Patch:
    """Decode one Patch."""
    span = destack._generated.source.file.model.span.decode_span(reader)
    new_text = reader.read_string()

    return Patch(
        span=span,
        new_text=new_text,
    )


def to_json_patch(value: Patch) -> Json:
    """Return one JSON value for one Patch."""
    return {
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        "newText": value.new_text,
    }


def from_json_patch(value: Json) -> Patch:
    """Return one Patch from one JSON value."""
    object_ = json_object(value)

    return Patch(
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        new_text=json_string(json_field(object_, "newText")),
    )


@dataclass(frozen=True, slots=True)
class FilePatch:
    """Patches for a single file."""

    # the file to patch
    file: destack._generated.source.file.model.file.FileId
    # the patches to apply (should be non-overlapping)
    patches: Sequence[Patch]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_patch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FilePatch:
        """Decode one FilePatch."""
        return decode_file_patch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_patch(self)

    @classmethod
    def from_json(cls, value: Json) -> FilePatch:
        """Return one FilePatch from one JSON value."""
        return from_json_file_patch(value)


def encode_file_patch(writer: BinaryWriter, value: FilePatch) -> None:
    """Encode one FilePatch."""
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    writer.write_unsigned(len(value.patches))
    for item_value_patches_0 in value.patches:
        encode_patch(writer, item_value_patches_0)


def decode_file_patch(reader: BinaryReader) -> FilePatch:
    """Decode one FilePatch."""
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    patches = [decode_patch(reader) for _ in range(reader.read_number())]

    return FilePatch(
        file=file,
        patches=patches,
    )


def to_json_file_patch(value: FilePatch) -> Json:
    """Return one JSON value for one FilePatch."""
    return {
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "patches": [to_json_patch(item_0) for item_0 in value.patches],
    }


def from_json_file_patch(value: Json) -> FilePatch:
    """Return one FilePatch from one JSON value."""
    object_ = json_object(value)

    return FilePatch(
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        patches=[
            from_json_patch(item_0)
            for item_0 in json_array(json_field(object_, "patches"))
        ],
    )


@dataclass(frozen=True, slots=True)
class PatchSet:
    """Patches across multiple files."""

    # per-file patches
    files: Sequence[FilePatch]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_patch_set(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PatchSet:
        """Decode one PatchSet."""
        return decode_patch_set(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_patch_set(self)

    @classmethod
    def from_json(cls, value: Json) -> PatchSet:
        """Return one PatchSet from one JSON value."""
        return from_json_patch_set(value)


def encode_patch_set(writer: BinaryWriter, value: PatchSet) -> None:
    """Encode one PatchSet."""
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        encode_file_patch(writer, item_value_files_0)


def decode_patch_set(reader: BinaryReader) -> PatchSet:
    """Decode one PatchSet."""
    files = [decode_file_patch(reader) for _ in range(reader.read_number())]

    return PatchSet(
        files=files,
    )


def to_json_patch_set(value: PatchSet) -> Json:
    """Return one JSON value for one PatchSet."""
    return {
        "files": [to_json_file_patch(item_0) for item_0 in value.files],
    }


def from_json_patch_set(value: Json) -> PatchSet:
    """Return one PatchSet from one JSON value."""
    object_ = json_object(value)

    return PatchSet(
        files=[
            from_json_file_patch(item_0)
            for item_0 in json_array(json_field(object_, "files"))
        ],
    )


__all__ = [
    "Patch",
    "encode_patch",
    "decode_patch",
    "to_json_patch",
    "from_json_patch",
    "FilePatch",
    "encode_file_patch",
    "decode_file_patch",
    "to_json_file_patch",
    "from_json_file_patch",
    "PatchSet",
    "encode_patch_set",
    "decode_patch_set",
    "to_json_patch_set",
    "from_json_patch_set",
]
