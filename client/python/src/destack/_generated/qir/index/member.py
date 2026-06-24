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

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.qir.index.name
import destack._generated.source.file.model.file
import destack._generated.source.file.model.module
import destack._generated.source.file.model.span


@dataclass(frozen=True, slots=True)
class MemberIndex:
    """Searchable source member index."""

    # the member entries in stable source order
    entries: Sequence[MemberEntry]
    # member entry indexes ordered by name
    by_name: Sequence[int]
    # member entry indexes ordered by owner symbol
    by_owner: Sequence[int]
    # member entry indexes ordered by declaring symbol
    by_declaring: Sequence[int]
    # member entry indexes ordered by member symbol
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
    writer.write_unsigned(len(value.by_name))
    for item_value_by_name_0 in value.by_name:
        writer.write_unsigned(item_value_by_name_0)
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
    by_name = [reader.read_number() for _ in range(reader.read_number())]
    by_owner = [reader.read_number() for _ in range(reader.read_number())]
    by_declaring = [reader.read_number() for _ in range(reader.read_number())]
    by_symbol = [reader.read_number() for _ in range(reader.read_number())]

    return MemberIndex(
        entries=entries,
        by_name=by_name,
        by_owner=by_owner,
        by_declaring=by_declaring,
        by_symbol=by_symbol,
    )


def to_json_member_index(value: MemberIndex) -> Json:
    """Return one JSON value for one MemberIndex."""
    return {
        "entries": [to_json_member_entry(item_0) for item_0 in value.entries],
        "byName": [item_0 for item_0 in value.by_name],
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
        by_name=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "byName"))
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
    """Searchable source member entry."""

    # the member name
    name: destack._generated.qir.index.name.Name
    # the member kind
    kind: MemberKind
    # the owner symbol
    owner_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the symbol whose declaration contains this member
    declaring_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the member symbol
    member_symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the source node that declares the member
    source_node: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the module containing the member declaration
    module_id: destack._generated.source.file.model.module.ModuleId
    # the source file
    file_id: destack._generated.source.file.model.file.FileId
    # the source range
    range: destack._generated.source.file.model.span.Span
    # the containing symbol display name
    container_name: str | None
    # the checked type of the member when known
    declared_type: destack._generated.dir.type.type.GlobalTypeId | None
    # the member source family
    source: MemberSource
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
    destack._generated.qir.index.name.encode_name(writer, value.name)
    encode_member_kind(writer, value.kind)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.owner_symbol
    )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.declaring_symbol
    )
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(
        writer, value.member_symbol
    )
    destack._generated.dir.tree.node.encode_global_node_id_any(
        writer, value.source_node
    )
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    destack._generated.source.file.model.file.encode_file_id(writer, value.file_id)
    destack._generated.source.file.model.span.encode_span(writer, value.range)
    if value.container_name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.container_name)
    if value.declared_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(
            writer, value.declared_type
        )
    encode_member_source(writer, value.source)
    writer.write_bool(value.is_static)


def decode_member_entry(reader: BinaryReader) -> MemberEntry:
    """Decode one MemberEntry."""
    name = destack._generated.qir.index.name.decode_name(reader)
    kind = decode_member_kind(reader)
    owner_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    declaring_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(
        reader
    )
    member_symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    source_node = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    file_id = destack._generated.source.file.model.file.decode_file_id(reader)
    range_ = destack._generated.source.file.model.span.decode_span(reader)
    container_name = reader.read_option(lambda: reader.read_string())
    declared_type = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    source = decode_member_source(reader)
    is_static = reader.read_bool()

    return MemberEntry(
        name=name,
        kind=kind,
        owner_symbol=owner_symbol,
        declaring_symbol=declaring_symbol,
        member_symbol=member_symbol,
        source_node=source_node,
        module_id=module_id,
        file_id=file_id,
        range=range_,
        container_name=container_name,
        declared_type=declared_type,
        source=source,
        is_static=is_static,
    )


def to_json_member_entry(value: MemberEntry) -> Json:
    """Return one JSON value for one MemberEntry."""
    return {
        "name": destack._generated.qir.index.name.to_json_name(value.name),
        "kind": to_json_member_kind(value.kind),
        "ownerSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.owner_symbol
        ),
        "declaringSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.declaring_symbol
        ),
        "memberSymbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.member_symbol
        ),
        "sourceNode": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source_node
        ),
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "fileId": destack._generated.source.file.model.file.to_json_file_id(
            value.file_id
        ),
        "range": destack._generated.source.file.model.span.to_json_span(value.range),
        **(
            {}
            if value.container_name is None
            else {"containerName": value.container_name}
        ),
        **(
            {}
            if value.declared_type is None
            else {
                "declaredType": destack._generated.dir.type.type.to_json_global_type_id(
                    value.declared_type
                )
            }
        ),
        "source": to_json_member_source(value.source),
        "isStatic": value.is_static,
    }


def from_json_member_entry(value: Json) -> MemberEntry:
    """Return one MemberEntry from one JSON value."""
    object_ = json_object(value)

    return MemberEntry(
        name=destack._generated.qir.index.name.from_json_name(
            json_field(object_, "name")
        ),
        kind=from_json_member_kind(json_field(object_, "kind")),
        owner_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "ownerSymbol")
        ),
        declaring_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "declaringSymbol")
        ),
        member_symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "memberSymbol")
        ),
        source_node=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "sourceNode")
        ),
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        file_id=destack._generated.source.file.model.file.from_json_file_id(
            json_field(object_, "fileId")
        ),
        range=destack._generated.source.file.model.span.from_json_span(
            json_field(object_, "range")
        ),
        container_name=json_optional(
            object_, "containerName", lambda value: json_string(value)
        ),
        declared_type=json_optional(
            object_,
            "declaredType",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        source=from_json_member_source(json_field(object_, "source")),
        is_static=json_bool(json_field(object_, "isStatic")),
    )


"""Searchable source member kind."""
MemberKind: typing.TypeAlias = (
    typing.Literal["field"]
    | typing.Literal["method"]
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
    elif value == "associatedType":
        writer.write_unsigned(2)
    elif value == "associatedConst":
        writer.write_unsigned(3)
    elif value == "variant":
        writer.write_unsigned(4)
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
        return "associatedType"
    elif variant == 3:
        return "associatedConst"
    elif variant == 4:
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
    elif variant == "associatedType":
        return "associatedType"
    elif variant == "associatedConst":
        return "associatedConst"
    elif variant == "variant":
        return "variant"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Searchable source member source family."""
MemberSource: typing.TypeAlias = (
    typing.Literal["declaration"]
    | typing.Literal["typeMember"]
    | typing.Literal["enumVariant"]
    | typing.Literal["extension"]
)


def encode_member_source(writer: BinaryWriter, value: MemberSource) -> None:
    """Encode one MemberSource."""
    if value == "declaration":
        writer.write_unsigned(0)
    elif value == "typeMember":
        writer.write_unsigned(1)
    elif value == "enumVariant":
        writer.write_unsigned(2)
    elif value == "extension":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_member_source(reader: BinaryReader) -> MemberSource:
    """Decode one MemberSource."""
    variant = reader.read_number()

    if variant == 0:
        return "declaration"
    elif variant == 1:
        return "typeMember"
    elif variant == 2:
        return "enumVariant"
    elif variant == 3:
        return "extension"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_member_source(value: MemberSource) -> Json:
    """Return one JSON value for one MemberSource."""
    return value


def from_json_member_source(value: Json) -> MemberSource:
    """Return one MemberSource from one JSON value."""
    variant = json_string(value)

    if variant == "declaration":
        return "declaration"
    elif variant == "typeMember":
        return "typeMember"
    elif variant == "enumVariant":
        return "enumVariant"
    elif variant == "extension":
        return "extension"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
    "MemberSource",
    "encode_member_source",
    "decode_member_source",
    "to_json_member_source",
    "from_json_member_source",
]
