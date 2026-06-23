# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.source.file.model.module

if TYPE_CHECKING:
    from destack._generated.protocol.source.file.model.module import (
        ModuleId,
    )


@dataclass(frozen=True, slots=True)
class GlobalSymbolId:
    """Global symbol id across modules."""

    """The module id of the global symbol."""
    module_id: ModuleId
    """The local id of the global symbol."""
    local_id: LocalSymbolId


def encode_global_symbol_id(writer: Writer, value: GlobalSymbolId) -> None:
    destack._generated.protocol.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_symbol_id(writer, value.local_id)


def decode_global_symbol_id(reader: Reader) -> GlobalSymbolId:
    field_0 = destack._generated.protocol.source.file.model.module.decode_module_id(
        reader
    )
    field_1 = decode_local_symbol_id(reader)

    return GlobalSymbolId(
        module_id=field_0,
        local_id=field_1,
    )


@dataclass(frozen=True, slots=True)
class LocalSymbolId:
    """Unique identifier for Symbols."""

    """The numeric id."""
    id: int


def encode_local_symbol_id(writer: Writer, value: LocalSymbolId) -> None:
    writer.write_unsigned(value.id)


def decode_local_symbol_id(reader: Reader) -> LocalSymbolId:
    field_0 = reader.read_number()

    return LocalSymbolId(
        id=field_0,
    )


__all__ = [
    "GlobalSymbolId",
    "encode_global_symbol_id",
    "decode_global_symbol_id",
    "LocalSymbolId",
    "encode_local_symbol_id",
    "decode_local_symbol_id",
]
