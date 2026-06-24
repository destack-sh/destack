# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.source.edit.edit
import destack._generated.source.file.model.profile

@dataclass(frozen=True, slots=True)
class RenameFilesRequest:
    """Request payload for file rename edits."""

    # profiles that should participate in specifier rewrites
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    # the file rename entries to apply
    renames: Sequence[FileRenameEntry]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameFilesRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameFilesRequest: ...

def encode_rename_files_request(
    writer: BinaryWriter, value: RenameFilesRequest
) -> None: ...
def decode_rename_files_request(reader: BinaryReader) -> RenameFilesRequest: ...
def to_json_rename_files_request(value: RenameFilesRequest) -> Json: ...
def from_json_rename_files_request(value: Json) -> RenameFilesRequest: ...

@dataclass(frozen=True, slots=True)
class FileRenameEntry:
    """A file rename entry for refactor queries."""

    # the old path before the rename
    old_path: str
    # the new path after the rename
    new_path: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> FileRenameEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> FileRenameEntry: ...

def encode_file_rename_entry(writer: BinaryWriter, value: FileRenameEntry) -> None: ...
def decode_file_rename_entry(reader: BinaryReader) -> FileRenameEntry: ...
def to_json_file_rename_entry(value: FileRenameEntry) -> Json: ...
def from_json_file_rename_entry(value: Json) -> FileRenameEntry: ...

@dataclass(frozen=True, slots=True)
class RenameFilesResponse:
    """Response payload for file rename queries."""

    # file rename edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameFilesResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameFilesResponse: ...

def encode_rename_files_response(
    writer: BinaryWriter, value: RenameFilesResponse
) -> None: ...
def decode_rename_files_response(reader: BinaryReader) -> RenameFilesResponse: ...
def to_json_rename_files_response(value: RenameFilesResponse) -> Json: ...
def from_json_rename_files_response(value: Json) -> RenameFilesResponse: ...

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
