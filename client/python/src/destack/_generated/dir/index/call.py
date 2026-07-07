# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class CallIndex:
    """Call graph index."""

    # the calls ordered by callee symbol
    by_callee: Sequence[CallEntry]
    # the calls ordered by caller symbol
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
    """One call graph edge."""

    # the call-like expression node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the expression or type node naming the callee
    target: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the kind of call-like operation
    kind: CallKind
    # the containing function symbol when known
    caller: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the called function symbol
    callee: destack._generated.dir.symbol.symbol.GlobalSymbolId
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
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.target)
    encode_call_kind(writer, value.kind)
    if value.caller is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.caller
        )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.callee)
    destack._generated.source.file.model.span.encode_span(writer, value.span)


def decode_call_entry(reader: BinaryReader) -> CallEntry:
    """Decode one CallEntry."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    target = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    kind = decode_call_kind(reader)
    caller = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    callee = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)

    return CallEntry(
        source=source,
        target=target,
        kind=kind,
        caller=caller,
        callee=callee,
        span=span,
    )


def to_json_call_entry(value: CallEntry) -> Json:
    """Return one JSON value for one CallEntry."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "target": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.target
        ),
        "kind": to_json_call_kind(value.kind),
        **(
            {}
            if value.caller is None
            else {
                "caller": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.caller
                )
            }
        ),
        "callee": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.callee
        ),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
    }


def from_json_call_entry(value: Json) -> CallEntry:
    """Return one CallEntry from one JSON value."""
    object_ = json_object(value)

    return CallEntry(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        target=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "target")
        ),
        kind=from_json_call_kind(json_field(object_, "kind")),
        caller=json_optional(
            object_,
            "caller",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        callee=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "callee")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
    )


"""Kind of call-like operation."""
CallKind: typing.TypeAlias = typing.Literal["call"] | typing.Literal["construct"]


def encode_call_kind(writer: BinaryWriter, value: CallKind) -> None:
    """Encode one CallKind."""
    if value == "call":
        writer.write_unsigned(0)
    elif value == "construct":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_call_kind(reader: BinaryReader) -> CallKind:
    """Decode one CallKind."""
    variant = reader.read_number()

    if variant == 0:
        return "call"
    elif variant == 1:
        return "construct"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_call_kind(value: CallKind) -> Json:
    """Return one JSON value for one CallKind."""
    return value


def from_json_call_kind(value: Json) -> CallKind:
    """Return one CallKind from one JSON value."""
    variant = json_string(value)

    if variant == "call":
        return "call"
    elif variant == "construct":
        return "construct"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class CallPostings:
    """Call postings by caller and callee symbols."""

    # caller symbol postings
    callers: destack._generated.dir.index.postings.Postings
    # callee symbol postings
    callees: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallPostings:
        """Decode one CallPostings."""
        return decode_call_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> CallPostings:
        """Return one CallPostings from one JSON value."""
        return from_json_call_postings(value)


def encode_call_postings(writer: BinaryWriter, value: CallPostings) -> None:
    """Encode one CallPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.callers)
    destack._generated.dir.index.postings.encode_postings(writer, value.callees)


def decode_call_postings(reader: BinaryReader) -> CallPostings:
    """Decode one CallPostings."""
    callers = destack._generated.dir.index.postings.decode_postings(reader)
    callees = destack._generated.dir.index.postings.decode_postings(reader)

    return CallPostings(
        callers=callers,
        callees=callees,
    )


def to_json_call_postings(value: CallPostings) -> Json:
    """Return one JSON value for one CallPostings."""
    return {
        "callers": destack._generated.dir.index.postings.to_json_postings(
            value.callers
        ),
        "callees": destack._generated.dir.index.postings.to_json_postings(
            value.callees
        ),
    }


def from_json_call_postings(value: Json) -> CallPostings:
    """Return one CallPostings from one JSON value."""
    object_ = json_object(value)

    return CallPostings(
        callers=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "callers")
        ),
        callees=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "callees")
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
    "CallKind",
    "encode_call_kind",
    "decode_call_kind",
    "to_json_call_kind",
    "from_json_call_kind",
    "CallPostings",
    "encode_call_postings",
    "decode_call_postings",
    "to_json_call_postings",
    "from_json_call_postings",
]
