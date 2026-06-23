# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class PatchSet:
    """Patches across multiple files."""

    """Per-file patches."""
    files: Sequence[FilePatch]


def encode_patch_set(writer: Writer, value: PatchSet) -> None:
    writer.write_unsigned(len(value.files))
    for item_0 in value.files:
        encode_file_patch(writer, item_0)


def decode_patch_set(reader: Reader) -> PatchSet:
    field_0 = [decode_file_patch(reader) for _ in range(reader.read_number())]

    return PatchSet(
        files=field_0,
    )


@dataclass(frozen=True, slots=True)
class FilePatch:
    """Patches for a single file."""

    """The file to patch."""
    file: FileId
    """The patches to apply (should be non-overlapping)."""
    patches: Sequence[Patch]


def encode_file_patch(writer: Writer, value: FilePatch) -> None:
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file
    )
    writer.write_unsigned(len(value.patches))
    for item_0 in value.patches:
        encode_patch(writer, item_0)


def decode_file_patch(reader: Reader) -> FilePatch:
    field_0 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_1 = [decode_patch(reader) for _ in range(reader.read_number())]

    return FilePatch(
        file=field_0,
        patches=field_1,
    )


@dataclass(frozen=True, slots=True)
class Patch:
    """A single patch: replace a span with new text."""

    """The span to replace."""
    span: Span
    """The replacement text (empty for deletion)."""
    new_text: str


def encode_patch(writer: Writer, value: Patch) -> None:
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.span)
    writer.write_string(value.new_text)


def decode_patch(reader: Reader) -> Patch:
    field_0 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_1 = reader.read_string()

    return Patch(
        span=field_0,
        new_text=field_1,
    )


__all__ = [
    "PatchSet",
    "encode_patch_set",
    "decode_patch_set",
    "FilePatch",
    "encode_file_patch",
    "decode_file_patch",
    "Patch",
    "encode_patch",
    "decode_patch",
]
