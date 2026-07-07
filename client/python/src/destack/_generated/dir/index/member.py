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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.index.postings
import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.file
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class MemberIndex:
    """Indexed checked members."""

    # the members in stable source order
    entries: Sequence[MemberEntry]
    # member indexes ordered by owner symbol
    by_owner: Sequence[int]
    # member indexes ordered by declaring symbol
    by_declaring: Sequence[int]
    # member indexes ordered by member symbol
    by_symbol: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberIndex:
        """Decode one MemberIndex."""
        return decode_member_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_index(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberIndex:
        """Return one MemberIndex from one JSON value."""
        return from_json_member_index(value)


def encode_member_index(writer: BinaryWriter, value: MemberIndex) -> None:
    """Encode one MemberIndex."""
    writer.write_unsigned(len(value.entries))
    for item_value_entries_0 in value.entries:
        encode_member_entry(writer, item_value_entries_0)
    writer.write_unsigned(len(value.by_owner))
    for item_value_by_owner_0 in value.by_owner:
        writer.write_unsigned(item_value_by_owner_0)
    writer.write_unsigned(len(value.by_declaring))
    for item_value_by_declaring_0 in value.by_declaring:
        writer.write_unsigned(item_value_by_declaring_0)
    writer.write_unsigned(len(value.by_symbol))
    for item_value_by_symbol_0 in value.by_symbol:
        writer.write_unsigned(item_value_by_symbol_0)


def decode_member_index(reader: BinaryReader) -> MemberIndex:
    """Decode one MemberIndex."""
    entries = [decode_member_entry(reader) for _ in range(reader.read_number())]
    by_owner = [reader.read_number() for _ in range(reader.read_number())]
    by_declaring = [reader.read_number() for _ in range(reader.read_number())]
    by_symbol = [reader.read_number() for _ in range(reader.read_number())]

    return MemberIndex(
        entries=entries,
        by_owner=by_owner,
        by_declaring=by_declaring,
        by_symbol=by_symbol,
    )


def to_json_member_index(value: MemberIndex) -> Json:
    """Return one JSON value for one MemberIndex."""
    return {
        "entries": [to_json_member_entry(item_0) for item_0 in value.entries],
        "byOwner": [item_0 for item_0 in value.by_owner],
        "byDeclaring": [item_0 for item_0 in value.by_declaring],
        "bySymbol": [item_0 for item_0 in value.by_symbol],
    }


def from_json_member_index(value: Json) -> MemberIndex:
    """Return one MemberIndex from one JSON value."""
    object_ = json_object(value)

    return MemberIndex(
        entries=[
            from_json_member_entry(item_0)
            for item_0 in json_array(json_field(object_, "entries"))
        ],
        by_owner=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "byOwner"))
        ],
        by_declaring=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "byDeclaring"))
        ],
        by_symbol=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "bySymbol"))
        ],
    )


@dataclass(frozen=True, slots=True)
class MemberEntry:
    """One indexed member."""

    # the member name
    name: str
    # the member kind
    kind: MemberKind
    # the symbol whose member surface receives this member
    owner: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the symbol whose definition declares this member
    declaring: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member symbol when this member declares one
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the source node that defines the member
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the source file
    file: destack._generated.source.file.model.file.FileId
    # the source range
    span: destack._generated.source.file.model.span.Span
    # the containing symbol display name
    container: str | None
    # the checked type of the member when known
    ty: destack._generated.dir.type.type.GlobalTypeId | None
    # the member origin
    origin: MemberOrigin
    # the checked member space
    space: destack._generated.dir.table.definition.MemberSpace
    # whether implementers must supply this member
    is_abstract: bool
    # whether this member overrides an inherited member
    is_override: bool
    # whether this member supplies a default implementation
    is_default: bool
    # whether this member belongs to the static surface
    is_static: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberEntry:
        """Decode one MemberEntry."""
        return decode_member_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberEntry:
        """Return one MemberEntry from one JSON value."""
        return from_json_member_entry(value)


def encode_member_entry(writer: BinaryWriter, value: MemberEntry) -> None:
    """Encode one MemberEntry."""
    writer.write_string(value.name)
    encode_member_kind(writer, value.kind)
    if value.owner is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.owner
        )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.declaring
    )
    if value.symbol is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.source.file.model.file.encode_file_id(writer, value.file)
    destack._generated.source.file.model.span.encode_span(writer, value.span)
    if value.container is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container)
    if value.ty is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    encode_member_origin(writer, value.origin)
    destack._generated.dir.table.definition.encode_member_space(writer, value.space)
    writer.write_bool(value.is_abstract)
    writer.write_bool(value.is_override)
    writer.write_bool(value.is_default)
    writer.write_bool(value.is_static)


def decode_member_entry(reader: BinaryReader) -> MemberEntry:
    """Decode one MemberEntry."""
    name = reader.read_string()
    kind = decode_member_kind(reader)
    owner = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    declaring = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    symbol = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    file = destack._generated.source.file.model.file.decode_file_id(reader)
    span = destack._generated.source.file.model.span.decode_span(reader)
    container = reader.read_option(lambda: reader.read_string())
    ty = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    origin = decode_member_origin(reader)
    space = destack._generated.dir.table.definition.decode_member_space(reader)
    is_abstract = reader.read_bool()
    is_override = reader.read_bool()
    is_default = reader.read_bool()
    is_static = reader.read_bool()

    return MemberEntry(
        name=name,
        kind=kind,
        owner=owner,
        declaring=declaring,
        symbol=symbol,
        source=source,
        file=file,
        span=span,
        container=container,
        ty=ty,
        origin=origin,
        space=space,
        is_abstract=is_abstract,
        is_override=is_override,
        is_default=is_default,
        is_static=is_static,
    )


def to_json_member_entry(value: MemberEntry) -> Json:
    """Return one JSON value for one MemberEntry."""
    return {
        "name": value.name,
        "kind": to_json_member_kind(value.kind),
        **(
            {}
            if value.owner is None
            else {
                "owner": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.owner
                )
            }
        ),
        "declaring": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.declaring
        ),
        **(
            {}
            if value.symbol is None
            else {
                "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.symbol
                )
            }
        ),
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "file": destack._generated.source.file.model.file.to_json_file_id(value.file),
        "span": destack._generated.source.file.model.span.to_json_span(value.span),
        **({} if value.container is None else {"container": value.container}),
        **(
            {}
            if value.ty is None
            else {
                "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty)
            }
        ),
        "origin": to_json_member_origin(value.origin),
        "space": destack._generated.dir.table.definition.to_json_member_space(
            value.space
        ),
        "isAbstract": value.is_abstract,
        "isOverride": value.is_override,
        "isDefault": value.is_default,
        "isStatic": value.is_static,
    }


def from_json_member_entry(value: Json) -> MemberEntry:
    """Return one MemberEntry from one JSON value."""
    object_ = json_object(value)

    return MemberEntry(
        name=json_string(json_field(object_, "name")),
        kind=from_json_member_kind(json_field(object_, "kind")),
        owner=json_optional(
            object_,
            "owner",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        declaring=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "declaring")
        ),
        symbol=json_optional(
            object_,
            "symbol",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        file=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "file")
        ),
        span=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "span")
        ),
        container=json_optional(object_, "container", lambda value: json_string(value)),
        ty=json_optional(
            object_,
            "ty",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        origin=from_json_member_origin(json_field(object_, "origin")),
        space=destack._generated.dir.table.definition.from_json_member_space(
            json_field(object_, "space")
        ),
        is_abstract=json_bool(json_field(object_, "isAbstract")),
        is_override=json_bool(json_field(object_, "isOverride")),
        is_default=json_bool(json_field(object_, "isDefault")),
        is_static=json_bool(json_field(object_, "isStatic")),
    )


"""Indexed member kind."""
MemberKind: typing.TypeAlias = (
    typing.Literal["field"]
    | typing.Literal["method"]
    | typing.Literal["constructor"]
    | typing.Literal["callSignature"]
    | typing.Literal["constructSignature"]
    | typing.Literal["indexSignature"]
    | typing.Literal["associatedType"]
    | typing.Literal["associatedConst"]
    | typing.Literal["variant"]
)


def encode_member_kind(writer: BinaryWriter, value: MemberKind) -> None:
    """Encode one MemberKind."""
    if value == "field":
        writer.write_unsigned(0)
    elif value == "method":
        writer.write_unsigned(1)
    elif value == "constructor":
        writer.write_unsigned(2)
    elif value == "callSignature":
        writer.write_unsigned(3)
    elif value == "constructSignature":
        writer.write_unsigned(4)
    elif value == "indexSignature":
        writer.write_unsigned(5)
    elif value == "associatedType":
        writer.write_unsigned(6)
    elif value == "associatedConst":
        writer.write_unsigned(7)
    elif value == "variant":
        writer.write_unsigned(8)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_kind(reader: BinaryReader) -> MemberKind:
    """Decode one MemberKind."""
    variant = reader.read_number()

    if variant == 0:
        return "field"
    elif variant == 1:
        return "method"
    elif variant == 2:
        return "constructor"
    elif variant == 3:
        return "callSignature"
    elif variant == 4:
        return "constructSignature"
    elif variant == 5:
        return "indexSignature"
    elif variant == 6:
        return "associatedType"
    elif variant == 7:
        return "associatedConst"
    elif variant == 8:
        return "variant"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_kind(value: MemberKind) -> Json:
    """Return one JSON value for one MemberKind."""
    return value


def from_json_member_kind(value: Json) -> MemberKind:
    """Return one MemberKind from one JSON value."""
    variant = json_string(value)

    if variant == "field":
        return "field"
    elif variant == "method":
        return "method"
    elif variant == "constructor":
        return "constructor"
    elif variant == "callSignature":
        return "callSignature"
    elif variant == "constructSignature":
        return "constructSignature"
    elif variant == "indexSignature":
        return "indexSignature"
    elif variant == "associatedType":
        return "associatedType"
    elif variant == "associatedConst":
        return "associatedConst"
    elif variant == "variant":
        return "variant"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Indexed member origin."""
MemberOrigin: typing.TypeAlias = (
    typing.Literal["definition"] | typing.Literal["extension"]
)


def encode_member_origin(writer: BinaryWriter, value: MemberOrigin) -> None:
    """Encode one MemberOrigin."""
    if value == "definition":
        writer.write_unsigned(0)
    elif value == "extension":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_origin(reader: BinaryReader) -> MemberOrigin:
    """Decode one MemberOrigin."""
    variant = reader.read_number()

    if variant == 0:
        return "definition"
    elif variant == 1:
        return "extension"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_origin(value: MemberOrigin) -> Json:
    """Return one JSON value for one MemberOrigin."""
    return value


def from_json_member_origin(value: Json) -> MemberOrigin:
    """Return one MemberOrigin from one JSON value."""
    variant = json_string(value)

    if variant == "definition":
        return "definition"
    elif variant == "extension":
        return "extension"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class MemberPostings:
    """Member postings by lookup key."""

    # member name postings
    names: destack._generated.dir.index.postings.Postings
    # member owner postings
    owners: destack._generated.dir.index.postings.Postings
    # member declaring symbol postings
    declaring: destack._generated.dir.index.postings.Postings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_postings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberPostings:
        """Decode one MemberPostings."""
        return decode_member_postings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_postings(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberPostings:
        """Return one MemberPostings from one JSON value."""
        return from_json_member_postings(value)


def encode_member_postings(writer: BinaryWriter, value: MemberPostings) -> None:
    """Encode one MemberPostings."""
    destack._generated.dir.index.postings.encode_postings(writer, value.names)
    destack._generated.dir.index.postings.encode_postings(writer, value.owners)
    destack._generated.dir.index.postings.encode_postings(writer, value.declaring)


def decode_member_postings(reader: BinaryReader) -> MemberPostings:
    """Decode one MemberPostings."""
    names = destack._generated.dir.index.postings.decode_postings(reader)
    owners = destack._generated.dir.index.postings.decode_postings(reader)
    declaring = destack._generated.dir.index.postings.decode_postings(reader)

    return MemberPostings(
        names=names,
        owners=owners,
        declaring=declaring,
    )


def to_json_member_postings(value: MemberPostings) -> Json:
    """Return one JSON value for one MemberPostings."""
    return {
        "names": destack._generated.dir.index.postings.to_json_postings(value.names),
        "owners": destack._generated.dir.index.postings.to_json_postings(value.owners),
        "declaring": destack._generated.dir.index.postings.to_json_postings(
            value.declaring
        ),
    }


def from_json_member_postings(value: Json) -> MemberPostings:
    """Return one MemberPostings from one JSON value."""
    object_ = json_object(value)

    return MemberPostings(
        names=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "names")
        ),
        owners=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "owners")
        ),
        declaring=destack._generated.dir.index.postings.from_json_postings(
            json_field(object_, "declaring")
        ),
    )


__all__ = [
    "MemberIndex",
    "encode_member_index",
    "decode_member_index",
    "to_json_member_index",
    "from_json_member_index",
    "MemberEntry",
    "encode_member_entry",
    "decode_member_entry",
    "to_json_member_entry",
    "from_json_member_entry",
    "MemberKind",
    "encode_member_kind",
    "decode_member_kind",
    "to_json_member_kind",
    "from_json_member_kind",
    "MemberOrigin",
    "encode_member_origin",
    "decode_member_origin",
    "to_json_member_origin",
    "from_json_member_origin",
    "MemberPostings",
    "encode_member_postings",
    "decode_member_postings",
    "to_json_member_postings",
    "from_json_member_postings",
]
