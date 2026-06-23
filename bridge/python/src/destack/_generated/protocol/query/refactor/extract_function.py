# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.query.core.target
import destack._generated.protocol.source.edit.edit

if TYPE_CHECKING:
    from destack._generated.protocol.query.core.target import (
        QueryRange,
    )

    from destack._generated.protocol.source.edit.edit import (
        PatchSet,
    )


@dataclass(frozen=True, slots=True)
class ExtractFunctionRequest:
    """Request payload for extract function queries."""

    """The selected source range."""
    range: QueryRange
    """The name for the extracted function."""
    new_name: str


def encode_extract_function_request(
    writer: Writer, value: ExtractFunctionRequest
) -> None:
    destack._generated.protocol.query.core.target.encode_query_range(
        writer, value.range
    )
    writer.write_string(value.new_name)


def decode_extract_function_request(reader: Reader) -> ExtractFunctionRequest:
    field_0 = destack._generated.protocol.query.core.target.decode_query_range(reader)
    field_1 = reader.read_string()

    return ExtractFunctionRequest(
        range=field_0,
        new_name=field_1,
    )


@dataclass(frozen=True, slots=True)
class ExtractFunctionResponse:
    """Response payload for extract function queries."""

    """Extract function edit, if available."""
    edit: PatchSet | None


def encode_extract_function_response(
    writer: Writer, value: ExtractFunctionResponse
) -> None:
    if value.edit is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.edit.edit.encode_patch_set(
            writer, value.edit
        )


def decode_extract_function_response(reader: Reader) -> ExtractFunctionResponse:
    field_0 = reader.read_option(
        lambda: destack._generated.protocol.source.edit.edit.decode_patch_set(reader)
    )

    return ExtractFunctionResponse(
        edit=field_0,
    )


__all__ = [
    "ExtractFunctionRequest",
    "encode_extract_function_request",
    "decode_extract_function_request",
    "ExtractFunctionResponse",
    "encode_extract_function_response",
    "decode_extract_function_response",
]
