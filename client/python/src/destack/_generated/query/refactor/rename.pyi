# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.query.protocol.target
import destack._generated.source.edit.edit
import destack._generated.source.file.model.span

@dataclass(frozen=True, slots=True)
class RenameTargetRequest:
    """Request the rename target at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTargetRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameTargetRequest: ...

def encode_rename_target_request(
    writer: BinaryWriter, value: RenameTargetRequest
) -> None: ...
def decode_rename_target_request(reader: BinaryReader) -> RenameTargetRequest: ...
def to_json_rename_target_request(value: RenameTargetRequest) -> Json: ...
def from_json_rename_target_request(value: Json) -> RenameTargetRequest: ...

@dataclass(frozen=True, slots=True)
class RenameRequest:
    """Request rename edits at a cursor position."""

    # the queried position
    position: destack._generated.query.protocol.target.Position
    # the new name for the symbol
    new_name: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameRequest: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameRequest: ...

def encode_rename_request(writer: BinaryWriter, value: RenameRequest) -> None: ...
def decode_rename_request(reader: BinaryReader) -> RenameRequest: ...
def to_json_rename_request(value: RenameRequest) -> Json: ...
def from_json_rename_request(value: Json) -> RenameRequest: ...

@dataclass(frozen=True, slots=True)
class RenameTargetResponse:
    """Response payload for rename target queries."""

    # rename target, if available
    result: RenameTarget | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTargetResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameTargetResponse: ...

def encode_rename_target_response(
    writer: BinaryWriter, value: RenameTargetResponse
) -> None: ...
def decode_rename_target_response(reader: BinaryReader) -> RenameTargetResponse: ...
def to_json_rename_target_response(value: RenameTargetResponse) -> Json: ...
def from_json_rename_target_response(value: Json) -> RenameTargetResponse: ...

@dataclass(frozen=True, slots=True)
class RenameTarget:
    """Target of a rename query."""

    # the semantic rename target
    target: destack._generated.query.protocol.target.Target
    # the range of the symbol to rename
    range: destack._generated.source.file.model.span.Span
    # the current name (placeholder for rename dialog)
    placeholder: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameTarget: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameTarget: ...

def encode_rename_target(writer: BinaryWriter, value: RenameTarget) -> None: ...
def decode_rename_target(reader: BinaryReader) -> RenameTarget: ...
def to_json_rename_target(value: RenameTarget) -> Json: ...
def from_json_rename_target(value: Json) -> RenameTarget: ...

@dataclass(frozen=True, slots=True)
class RenameResponse:
    """Response payload for rename queries."""

    # rename edit, if available
    edit: destack._generated.source.edit.edit.PatchSet | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RenameResponse: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RenameResponse: ...

def encode_rename_response(writer: BinaryWriter, value: RenameResponse) -> None: ...
def decode_rename_response(reader: BinaryReader) -> RenameResponse: ...
def to_json_rename_response(value: RenameResponse) -> Json: ...
def from_json_rename_response(value: Json) -> RenameResponse: ...

__all__ = [
    "RenameTargetRequest",
    "encode_rename_target_request",
    "decode_rename_target_request",
    "to_json_rename_target_request",
    "from_json_rename_target_request",
    "RenameRequest",
    "encode_rename_request",
    "decode_rename_request",
    "to_json_rename_request",
    "from_json_rename_request",
    "RenameTargetResponse",
    "encode_rename_target_response",
    "decode_rename_target_response",
    "to_json_rename_target_response",
    "from_json_rename_target_response",
    "RenameTarget",
    "encode_rename_target",
    "decode_rename_target",
    "to_json_rename_target",
    "from_json_rename_target",
    "RenameResponse",
    "encode_rename_response",
    "decode_rename_response",
    "to_json_rename_response",
    "from_json_rename_response",
]
