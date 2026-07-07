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

import destack._generated.source.edit.edit
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class RenameFilesRequest:
    """Request payload for file rename edits."""

    # profiles that should participate in specifier rewrites
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    # the file rename entries to apply
    renames: Sequence[FileRenameEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_files_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameFilesRequest:
        """Decode one RenameFilesRequest."""
        return decode_rename_files_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_files_request(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameFilesRequest:
        """Return one RenameFilesRequest from one JSON value."""
        return from_json_rename_files_request(value)


def encode_rename_files_request(
    writer: BinaryWriter, value: RenameFilesRequest
) -> None:
    """Encode one RenameFilesRequest."""
    writer.write_unsigned(len(value.profile_ids))
    for item_value_profile_ids_0 in value.profile_ids:
        destack._generated.source.file.model.profile.encode_profile_id(
            writer, item_value_profile_ids_0
        )
    writer.write_unsigned(len(value.renames))
    for item_value_renames_0 in value.renames:
        encode_file_rename_entry(writer, item_value_renames_0)


def decode_rename_files_request(reader: BinaryReader) -> RenameFilesRequest:
    """Decode one RenameFilesRequest."""
    profile_ids = [
        destack._generated.source.file.model.profile.decode_profile_id(reader)
        for _ in range(reader.read_number())
    ]
    renames = [decode_file_rename_entry(reader) for _ in range(reader.read_number())]

    return RenameFilesRequest(
        profile_ids=profile_ids,
        renames=renames,
    )


def to_json_rename_files_request(value: RenameFilesRequest) -> Json:
    """Return one JSON value for one RenameFilesRequest."""
    return {
        "profileIds": [
            destack._generated.source.file.model.profile.to_json_profile_id(item_0)
            for item_0 in value.profile_ids
        ],
        "renames": [to_json_file_rename_entry(item_0) for item_0 in value.renames],
    }


def from_json_rename_files_request(value: Json) -> RenameFilesRequest:
    """Return one RenameFilesRequest from one JSON value."""
    object_ = json_object(value)

    return RenameFilesRequest(
        profile_ids=[
            destack._generated.source.file.model.profile.from_json_profile_id(item_0)
            for item_0 in json_array(json_field(object_, "profileIds"))
        ],
        renames=[
            from_json_file_rename_entry(item_0)
            for item_0 in json_array(json_field(object_, "renames"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FileRenameEntry:
    """A file rename entry for refactor queries."""

    # the old path before the rename
    old_path: str
    # the new path after the rename
    new_path: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_file_rename_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FileRenameEntry:
        """Decode one FileRenameEntry."""
        return decode_file_rename_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_file_rename_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> FileRenameEntry:
        """Return one FileRenameEntry from one JSON value."""
        return from_json_file_rename_entry(value)


def encode_file_rename_entry(writer: BinaryWriter, value: FileRenameEntry) -> None:
    """Encode one FileRenameEntry."""
    writer.write_string(value.old_path)
    writer.write_string(value.new_path)


def decode_file_rename_entry(reader: BinaryReader) -> FileRenameEntry:
    """Decode one FileRenameEntry."""
    old_path = reader.read_string()
    new_path = reader.read_string()

    return FileRenameEntry(
        old_path=old_path,
        new_path=new_path,
    )


def to_json_file_rename_entry(value: FileRenameEntry) -> Json:
    """Return one JSON value for one FileRenameEntry."""
    return {
        "oldPath": value.old_path,
        "newPath": value.new_path,
    }


def from_json_file_rename_entry(value: Json) -> FileRenameEntry:
    """Return one FileRenameEntry from one JSON value."""
    object_ = json_object(value)

    return FileRenameEntry(
        old_path=json_string(json_field(object_, "oldPath")),
        new_path=json_string(json_field(object_, "newPath")),
    )


@dataclass(frozen=True, slots=True)
class RenameFilesResponse:
    """Response payload for file rename queries."""

    # file rename edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_rename_files_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameFilesResponse:
        """Decode one RenameFilesResponse."""
        return decode_rename_files_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_rename_files_response(self)

    @classmethod
    def from_json(cls, value: Json) -> RenameFilesResponse:
        """Return one RenameFilesResponse from one JSON value."""
        return from_json_rename_files_response(value)


def encode_rename_files_response(
    writer: BinaryWriter, value: RenameFilesResponse
) -> None:
    """Encode one RenameFilesResponse."""
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.edit.edit.encode_patch_set(writer, value.edit)


def decode_rename_files_response(reader: BinaryReader) -> RenameFilesResponse:
    """Decode one RenameFilesResponse."""
    edit = reader.read_option(
        lambda: destack._generated.source.edit.edit.decode_patch_set(reader)
    )

    return RenameFilesResponse(
        edit=edit,
    )


def to_json_rename_files_response(value: RenameFilesResponse) -> Json:
    """Return one JSON value for one RenameFilesResponse."""
    return {
        **(
            {}
            if value.edit is None
            else {
                "edit": destack._generated.source.edit.edit.to_json_patch_set(
                    value.edit
                )
            }
        ),
    }


def from_json_rename_files_response(value: Json) -> RenameFilesResponse:
    """Return one RenameFilesResponse from one JSON value."""
    object_ = json_object(value)

    return RenameFilesResponse(
        edit=json_optional(
            object_,
            "edit",
            lambda value: destack._generated.source.edit.edit.from_json_patch_set(
                value
            ),
        ),
    )


__all__ = [
    "RenameFilesRequest",
    "encode_rename_files_request",
    "decode_rename_files_request",
    "to_json_rename_files_request",
    "from_json_rename_files_request",
    "FileRenameEntry",
    "encode_file_rename_entry",
    "decode_file_rename_entry",
    "to_json_file_rename_entry",
    "from_json_file_rename_entry",
    "RenameFilesResponse",
    "encode_rename_files_response",
    "decode_rename_files_response",
    "to_json_rename_files_response",
    "from_json_rename_files_response",
]
