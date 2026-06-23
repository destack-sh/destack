# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.edit.edit
import destack._generated.protocol.source.file.model.profile

if TYPE_CHECKING:
    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )


@dataclass(frozen=True, slots=True)
class RenameFilesRequest:
    """Request payload for file rename edits."""

    """Profiles that should participate in specifier rewrites."""
    profile_ids: Sequence[ProfileId]
    """The file rename entries to apply."""
    renames: Sequence[FileRenameEntry]


def encode_rename_files_request(writer: Writer, value: RenameFilesRequest) -> None:
    writer.write_unsigned(len(value.profile_ids))
    for item_0 in value.profile_ids:
        destack._generated.protocol.source.file.model.profile.encode_profile_id(
            writer, item_0
        )
    writer.write_unsigned(len(value.renames))
    for item_0 in value.renames:
        encode_file_rename_entry(writer, item_0)


def decode_rename_files_request(reader: Reader) -> RenameFilesRequest:
    field_0 = [
        destack._generated.protocol.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]
    field_1 = [decode_file_rename_entry(reader) for _ in range(reader.read_number())]

    return RenameFilesRequest(
        profile_ids=field_0,
        renames=field_1,
    )


@dataclass(frozen=True, slots=True)
class FileRenameEntry:
    """A file rename entry for refactor queries."""

    """The old path before the rename."""
    old_path: str
    """The new path after the rename."""
    new_path: str


def encode_file_rename_entry(writer: Writer, value: FileRenameEntry) -> None:
    writer.write_string(value.old_path)
    writer.write_string(value.new_path)


def decode_file_rename_entry(reader: Reader) -> FileRenameEntry:
    field_0 = reader.read_string()
    field_1 = reader.read_string()

    return FileRenameEntry(
        old_path=field_0,
        new_path=field_1,
    )


@dataclass(frozen=True, slots=True)
class RenameFilesResponse:
    """Response payload for file rename queries."""

    """File rename edit, if available."""
    edit: PatchSet | None


def encode_rename_files_response(writer: Writer, value: RenameFilesResponse) -> None:
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.edit.edit.encode_patch_set(
            writer, value.edit
        )


def decode_rename_files_response(reader: Reader) -> RenameFilesResponse:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    )

    return RenameFilesResponse(
        edit=field_0,
    )


__all__ = [
    "RenameFilesRequest",
    "encode_rename_files_request",
    "decode_rename_files_request",
    "FileRenameEntry",
    "encode_file_rename_entry",
    "decode_file_rename_entry",
    "RenameFilesResponse",
    "encode_rename_files_response",
    "decode_rename_files_response",
]
