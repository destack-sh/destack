# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.dir.symbol.symbol
import destack._generated.protocol.dir.tree.node
import destack._generated.protocol.source.file.model.file
import destack._generated.protocol.source.file.model.module
import destack._generated.protocol.source.file.model.profile
import destack._generated.protocol.source.file.model.span

if TYPE_CHECKING:
    from destack._generated.protocol.dir.symbol.symbol import (
        GlobalSymbolId,
    )

    from destack._generated.protocol.dir.tree.node import (
        GlobalNodeIdAny,
    )

    from destack._generated.protocol.source.file.model.file import (
        FileId,
    )

    from destack._generated.protocol.source.file.model.module import (
        ModuleId,
    )

    from destack._generated.protocol.source.file.model.profile import (
        ProfileId,
    )

    from destack._generated.protocol.source.file.model.span import (
        Span,
    )


@dataclass(frozen=True, slots=True)
class QueryModule:
    """One module in one query profile."""

    """The queried module."""
    module_id: ModuleId
    """The queried profile."""
    profile_id: ProfileId


def encode_query_module(writer: Writer, value: QueryModule) -> None:
    destack._generated.protocol.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.protocol.source.file.model.profile.encode_profile_id(
        writer, value.profile_id
    )


def decode_query_module(reader: Reader) -> QueryModule:
    field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
        reader
    )
    field_1 = destack._generated.protocol.source.file.model.profile.decode_profile_id(
        reader
    )

    return QueryModule(
        module_id=field_0,
        profile_id=field_1,
    )


@dataclass(frozen=True, slots=True)
class QueryPosition:
    """One byte position in a module source file."""

    """The queried module profile."""
    module: QueryModule
    """The source file."""
    file_id: FileId
    """The byte offset in the source file."""
    offset: int


def encode_query_position(writer: Writer, value: QueryPosition) -> None:
    encode_query_module(writer, value.module)
    destack._generated.protocol.source.file.model.file.encode_file_id(
        writer, value.file_id
    )
    writer.write_unsigned(value.offset)


def decode_query_position(reader: Reader) -> QueryPosition:
    field_0 = decode_query_module(reader)
    field_1 = destack._generated.protocol.source.file.model.file.decode_file_id(reader)
    field_2 = reader.read_number()

    return QueryPosition(
        module=field_0,
        file_id=field_1,
        offset=field_2,
    )


@dataclass(frozen=True, slots=True)
class QueryRange:
    """One source range in a module."""

    """The queried module profile."""
    module: QueryModule
    """The source range."""
    span: Span


def encode_query_range(writer: Writer, value: QueryRange) -> None:
    encode_query_module(writer, value.module)
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.span)


def decode_query_range(reader: Reader) -> QueryRange:
    field_0 = decode_query_module(reader)
    field_1 = destack._generated.protocol.source.file.model.span.decode_span(reader)

    return QueryRange(
        module=field_0,
        span=field_1,
    )


@dataclass(frozen=True, slots=True)
class QueryTarget:
    """One source-backed query target."""

    """The target module profile."""
    module: QueryModule
    """The full source range."""
    span: Span
    """The primary selection range."""
    selection_span: Span | None
    """The target symbol when known."""
    symbol_id: GlobalSymbolId | None
    """The target node when known."""
    node_id: GlobalNodeIdAny | None


def encode_query_target(writer: Writer, value: QueryTarget) -> None:
    encode_query_module(writer, value.module)
    destack._generated.protocol.source.file.model.span.encode_span(writer, value.span)
    if value.selection_span is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.source.file.model.span.encode_span(
            writer, value.selection_span
        )
    if value.symbol_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol_id
        )
    if value.node_id is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.dir.tree.node.encode_global_node_id_any(
            writer, value.node_id
        )


def decode_query_target(reader: Reader) -> QueryTarget:
    field_0 = decode_query_module(reader)
    field_1 = destack._generated.protocol.source.file.model.span.decode_span(reader)
    field_2 = reader.read_option(
        lambda: destack._generated.protocol.source.file.model.span.decode_span(reader)
    )
    field_3 = reader.read_option(
        lambda: destack._generated.protocol.dir.symbol.symbol.decode_global_symbol_id(
            reader
        )
    )
    field_4 = reader.read_option(
        lambda: destack._generated.protocol.dir.tree.node.decode_global_node_id_any(
            reader
        )
    )

    return QueryTarget(
        module=field_0,
        span=field_1,
        selection_span=field_2,
        symbol_id=field_3,
        node_id=field_4,
    )


__all__ = [
    "QueryModule",
    "encode_query_module",
    "decode_query_module",
    "QueryPosition",
    "encode_query_position",
    "decode_query_position",
    "QueryRange",
    "encode_query_range",
    "decode_query_range",
    "QueryTarget",
    "encode_query_target",
    "decode_query_target",
]
