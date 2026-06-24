# generated bridge target, do not edit

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
)

import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class CallIndex:
    """Call graph index."""

    # the call entries ordered by callee symbol
    by_callee: Sequence[CallEntry]
    # the call entries ordered by caller symbol
    by_caller: Sequence[CallEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallIndex:
        """Decode one CallIndex."""
        return decode_call_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_index(self)

    @classmethod
    def from_json(cls, value: Json) -> CallIndex:
        """Return one CallIndex from one JSON value."""
        return from_json_call_index(value)


def encode_call_index(writer: BinaryWriter, value: CallIndex) -> None:
    """Encode one CallIndex."""
    writer.write_unsigned(len(value.by_callee))
    for item_value_by_callee_0 in value.by_callee:
        encode_call_entry(writer, item_value_by_callee_0)
    writer.write_unsigned(len(value.by_caller))
    for item_value_by_caller_0 in value.by_caller:
        encode_call_entry(writer, item_value_by_caller_0)


def decode_call_index(reader: BinaryReader) -> CallIndex:
    """Decode one CallIndex."""
    by_callee = [decode_call_entry(reader) for _ in range(reader.read_number())]
    by_caller = [decode_call_entry(reader) for _ in range(reader.read_number())]

    return CallIndex(
        by_callee=by_callee,
        by_caller=by_caller,
    )


def to_json_call_index(value: CallIndex) -> Json:
    """Return one JSON value for one CallIndex."""
    return {
        "byCallee": [to_json_call_entry(item_0) for item_0 in value.by_callee],
        "byCaller": [to_json_call_entry(item_0) for item_0 in value.by_caller],
    }


def from_json_call_index(value: Json) -> CallIndex:
    """Return one CallIndex from one JSON value."""
    object_ = json_object(value)

    return CallIndex(
        by_callee=[
            from_json_call_entry(item_0)
            for item_0 in json_array(json_field(object_, "byCallee"))
        ],
        by_caller=[
            from_json_call_entry(item_0)
            for item_0 in json_array(json_field(object_, "byCaller"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CallEntry:
    """Call graph edge entry."""

    # the module containing the call
    module_id: destack._generated.source.file.model.module.ModuleId
    # the containing function symbol when known
    caller_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the called function symbol
    callee_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the call source range
    span: destack._generated.source.file.model.span.Span

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallEntry:
        """Decode one CallEntry."""
        return decode_call_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> CallEntry:
        """Return one CallEntry from one JSON value."""
        return from_json_call_entry(value)


def encode_call_entry(writer: BinaryWriter, value: CallEntry) -> None:
    """Encode one CallEntry."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    if value.caller_symbol is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.caller_symbol
        )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.callee_symbol
    )
    destack._generated.source.file.model.span.encode_span(writer, value.span)


def decode_call_entry(reader: BinaryReader) -> CallEntry:
    """Decode one CallEntry."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    caller_symbol = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    callee_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)

    return CallEntry(
        module_id=module_id,
        caller_symbol=caller_symbol,
        callee_symbol=callee_symbol,
        span=span,
    )


def to_json_call_entry(value: CallEntry) -> Json:
    """Return one JSON value for one CallEntry."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        **(
            {}
            if value.caller_symbol is None
            else {
                "callerSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.caller_symbol
                )
            }
        ),
        "calleeSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.callee_symbol
        ),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
    }


def from_json_call_entry(value: Json) -> CallEntry:
    """Return one CallEntry from one JSON value."""
    object_ = json_object(value)

    return CallEntry(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        caller_symbol=json_optional(
            object_,
            "callerSymbol",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        callee_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "calleeSymbol")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
    )


__all__ = [
    "CallIndex",
    "encode_call_index",
    "decode_call_index",
    "to_json_call_index",
    "from_json_call_index",
    "CallEntry",
    "encode_call_entry",
    "decode_call_entry",
    "to_json_call_entry",
    "from_json_call_entry",
]
