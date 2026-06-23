# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.edit.edit
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryPosition,
        QueryTarget,
    )

    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class RenameTargetRequest:
    """Request the rename target at a cursor position."""

    """The queried position."""
    position: QueryPosition


def encode_rename_target_request(writer: Writer, value: RenameTargetRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )


def decode_rename_target_request(reader: Reader) -> RenameTargetRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )

    return RenameTargetRequest(
        position=field_0,
    )


@dataclass(frozen=True, slots=True)
class RenameRequest:
    """Request rename edits at a cursor position."""

    """The queried position."""
    position: QueryPosition
    """The new name for the symbol."""
    new_name: str


def encode_rename_request(writer: Writer, value: RenameRequest) -> None:
    destack._generated.protocol.query.core.target.encode_query_position(
        writer, value.position
    )
    writer.write_string(value.new_name)


def decode_rename_request(reader: Reader) -> RenameRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_position(
        reader
    )
    field_1 = reader.read_string()

    return RenameRequest(
        position=field_0,
        new_name=field_1,
    )


@dataclass(frozen=True, slots=True)
class RenameTargetResponse:
    """Response payload for rename target queries."""

    """Rename target, if available."""
    result: RenameTarget | None


def encode_rename_target_response(writer: Writer, value: RenameTargetResponse) -> None:
    if value.result is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_rename_target(writer, value.result)


def decode_rename_target_response(reader: Reader) -> RenameTargetResponse:
    field_0 = reader.read_option(lambda: decode_rename_target(reader))

    return RenameTargetResponse(
        result=field_0,
    )


@dataclass(frozen=True, slots=True)
class RenameTarget:
    """Target of a rename query."""

    """The semantic rename target."""
    target: QueryTarget
    """The range of the symbol to rename."""
    range: Span
    """The current name (placeholder for rename dialog)."""
    placeholder: str


def encode_rename_target(writer: Writer, value: RenameTarget) -> None:
    destack._generated.protocol.query.core.target.encode_query_target(
        writer, value.target
    )
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.range)
    writer.write_string(value.placeholder)


def decode_rename_target(reader: Reader) -> RenameTarget:
    field_0 = destack._generated.protocol.query.core.target.decode_query_target(reader)
    field_1 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_2 = reader.read_string()

    return RenameTarget(
        target=field_0,
        range=field_1,
        placeholder=field_2,
    )


@dataclass(frozen=True, slots=True)
class RenameResponse:
    """Response payload for rename queries."""

    """Rename edit, if available."""
    edit: PatchSet | None


def encode_rename_response(writer: Writer, value: RenameResponse) -> None:
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.edit.edit.encode_patch_set(
            writer, value.edit
        )


def decode_rename_response(reader: Reader) -> RenameResponse:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    )

    return RenameResponse(
        edit=field_0,
    )


__all__ = [
    "RenameTargetRequest",
    "encode_rename_target_request",
    "decode_rename_target_request",
    "RenameRequest",
    "encode_rename_request",
    "decode_rename_request",
    "RenameTargetResponse",
    "encode_rename_target_response",
    "decode_rename_target_response",
    "RenameTarget",
    "encode_rename_target",
    "decode_rename_target",
    "RenameResponse",
    "encode_rename_response",
    "decode_rename_response",
]
