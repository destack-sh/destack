# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class LocalScope:
    """Local scope id and mark pair used for node and symbol insertion."""

    # the scope id
    id: LocalScopeId
    # the visible binding mark
    mark: LocalScopeMark

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_scope(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalScope:
        """Decode one LocalScope."""
        return decode_local_scope(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_scope(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalScope:
        """Return one LocalScope from one JSON value."""
        return from_json_local_scope(value)


def encode_local_scope(writer: BinaryWriter, value: LocalScope) -> None:
    """Encode one LocalScope."""
    encode_local_scope_id(writer, value.id)
    encode_local_scope_mark(writer, value.mark)


def decode_local_scope(reader: BinaryReader) -> LocalScope:
    """Decode one LocalScope."""
    id = decode_local_scope_id(reader)
    mark = decode_local_scope_mark(reader)

    return LocalScope(
        id=id,
        mark=mark,
    )


def to_json_local_scope(value: LocalScope) -> Json:
    """Return one JSON value for one LocalScope."""
    return {
        "id": to_json_local_scope_id(value.id),
        "mark": to_json_local_scope_mark(value.mark),
    }


def from_json_local_scope(value: Json) -> LocalScope:
    """Return one LocalScope from one JSON value."""
    object_ = json_object(value)

    return LocalScope(
        id=from_json_local_scope_id(json_field(object_, "id")),
        mark=from_json_local_scope_mark(json_field(object_, "mark")),
    )


"""Unique identifier for local scopes."""
LocalScopeId: typing.TypeAlias = int


def encode_local_scope_id(writer: BinaryWriter, value: LocalScopeId) -> None:
    """Encode one LocalScopeId."""
    writer.write_unsigned(value)


def decode_local_scope_id(reader: BinaryReader) -> LocalScopeId:
    """Decode one LocalScopeId."""
    return reader.read_number()


def to_json_local_scope_id(value: LocalScopeId) -> Json:
    """Return one JSON value for one LocalScopeId."""
    return value


def from_json_local_scope_id(value: Json) -> LocalScopeId:
    """Return one LocalScopeId from one JSON value."""
    return json_int(value)


"""Mark a position in a scope."""
LocalScopeMark: typing.TypeAlias = int


def encode_local_scope_mark(writer: BinaryWriter, value: LocalScopeMark) -> None:
    """Encode one LocalScopeMark."""
    writer.write_unsigned(value)


def decode_local_scope_mark(reader: BinaryReader) -> LocalScopeMark:
    """Decode one LocalScopeMark."""
    return reader.read_number()


def to_json_local_scope_mark(value: LocalScopeMark) -> Json:
    """Return one JSON value for one LocalScopeMark."""
    return value


def from_json_local_scope_mark(value: Json) -> LocalScopeMark:
    """Return one LocalScopeMark from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class Scope:
    """A lexical container for symbols."""

    # the kind of the scope
    kind: ScopeKind
    # the parent scope
    parent: LocalScope | None
    # the owner of the scope
    owner: destack._generated.dir.symbol.symbol.LocalSymbolId | None
    # the bindings in lexical order
    bindings: Sequence[ScopeBinding]
    # index for named bindings
    index: ScopeIndex
    # the children scopes
    children: Sequence[LocalScopeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Scope:
        """Decode one Scope."""
        return decode_scope(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope(self)

    @classmethod
    def from_json(cls, value: Json) -> Scope:
        """Return one Scope from one JSON value."""
        return from_json_scope(value)


def encode_scope(writer: BinaryWriter, value: Scope) -> None:
    """Encode one Scope."""
    encode_scope_kind(writer, value.kind)
    if value.parent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_local_scope(writer, value.parent)
    if value.owner is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(writer, value.owner)
    writer.write_unsigned(len(value.bindings))
    for item_value_bindings_0 in value.bindings:
        encode_scope_binding(writer, item_value_bindings_0)
    encode_scope_index(writer, value.index)
    writer.write_unsigned(len(value.children))
    for item_value_children_0 in value.children:
        encode_local_scope_id(writer, item_value_children_0)


def decode_scope(reader: BinaryReader) -> Scope:
    """Decode one Scope."""
    kind = decode_scope_kind(reader)
    parent = reader.read_option(lambda: decode_local_scope(reader))
    owner = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)
    )
    bindings = [decode_scope_binding(reader) for _ in range(reader.read_number())]
    index = decode_scope_index(reader)
    children = [decode_local_scope_id(reader) for _ in range(reader.read_number())]

    return Scope(
        kind=kind,
        parent=parent,
        owner=owner,
        bindings=bindings,
        index=index,
        children=children,
    )


def to_json_scope(value: Scope) -> Json:
    """Return one JSON value for one Scope."""
    return {
        "kind": to_json_scope_kind(value.kind),
        **(
            {}
            if value.parent is None
            else {"parent": to_json_local_scope(value.parent)}
        ),
        **(
            {}
            if value.owner is None
            else {
                "owner": destack._generated.dir.symbol.symbol.to_json_local_symbol_id(
                    value.owner
                )
            }
        ),
        "bindings": [to_json_scope_binding(item_0) for item_0 in value.bindings],
        "index": to_json_scope_index(value.index),
        "children": [to_json_local_scope_id(item_0) for item_0 in value.children],
    }


def from_json_scope(value: Json) -> Scope:
    """Return one Scope from one JSON value."""
    object_ = json_object(value)

    return Scope(
        kind=from_json_scope_kind(json_field(object_, "kind")),
        parent=json_optional(
            object_, "parent", lambda value: from_json_local_scope(value)
        ),
        owner=json_optional(
            object_,
            "owner",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_local_symbol_id(value)
            ),
        ),
        bindings=[
            from_json_scope_binding(item_0)
            for item_0 in json_array(json_field(object_, "bindings"))
        ],
        index=from_json_scope_index(json_field(object_, "index")),
        children=[
            from_json_local_scope_id(item_0)
            for item_0 in json_array(json_field(object_, "children"))
        ],
    )


"""The kind of a scope."""
ScopeKind: typing.TypeAlias = (
    typing.Literal["module"]
    | typing.Literal["global"]
    | typing.Literal["namespace"]
    | typing.Literal["function"]
    | typing.Literal["type"]
    | typing.Literal["typeConditional"]
    | typing.Literal["label"]
    | typing.Literal["block"]
)


def encode_scope_kind(writer: BinaryWriter, value: ScopeKind) -> None:
    """Encode one ScopeKind."""
    if value == "module":
        writer.write_unsigned(0)
    elif value == "global":
        writer.write_unsigned(1)
    elif value == "namespace":
        writer.write_unsigned(2)
    elif value == "function":
        writer.write_unsigned(3)
    elif value == "type":
        writer.write_unsigned(4)
    elif value == "typeConditional":
        writer.write_unsigned(5)
    elif value == "label":
        writer.write_unsigned(6)
    elif value == "block":
        writer.write_unsigned(7)
    else:
        raise SerdeError("unknown enum variant")


def decode_scope_kind(reader: BinaryReader) -> ScopeKind:
    """Decode one ScopeKind."""
    variant = reader.read_number()

    if variant == 0:
        return "module"
    elif variant == 1:
        return "global"
    elif variant == 2:
        return "namespace"
    elif variant == 3:
        return "function"
    elif variant == 4:
        return "type"
    elif variant == 5:
        return "typeConditional"
    elif variant == 6:
        return "label"
    elif variant == 7:
        return "block"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_scope_kind(value: ScopeKind) -> Json:
    """Return one JSON value for one ScopeKind."""
    return value


def from_json_scope_kind(value: Json) -> ScopeKind:
    """Return one ScopeKind from one JSON value."""
    variant = json_string(value)

    if variant == "module":
        return "module"
    elif variant == "global":
        return "global"
    elif variant == "namespace":
        return "namespace"
    elif variant == "function":
        return "function"
    elif variant == "type":
        return "type"
    elif variant == "typeConditional":
        return "typeConditional"
    elif variant == "label":
        return "label"
    elif variant == "block":
        return "block"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ScopeBinding:
    """One binding entry in lexical order."""

    # the binding key
    key: destack._generated.dir.symbol.key.StaticKey | None
    # the bound symbol
    symbol: destack._generated.dir.symbol.symbol.LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope_binding(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ScopeBinding:
        """Decode one ScopeBinding."""
        return decode_scope_binding(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope_binding(self)

    @classmethod
    def from_json(cls, value: Json) -> ScopeBinding:
        """Return one ScopeBinding from one JSON value."""
        return from_json_scope_binding(value)


def encode_scope_binding(writer: BinaryWriter, value: ScopeBinding) -> None:
    """Encode one ScopeBinding."""
    if value.key is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.symbol.symbol.encode_local_symbol_id(writer, value.symbol)


def decode_scope_binding(reader: BinaryReader) -> ScopeBinding:
    """Decode one ScopeBinding."""
    key = reader.read_option(
        lambda: destack._generated.dir.symbol.key.decode_static_key(reader)
    )
    symbol = destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)

    return ScopeBinding(
        key=key,
        symbol=symbol,
    )


def to_json_scope_binding(value: ScopeBinding) -> Json:
    """Return one JSON value for one ScopeBinding."""
    return {
        **(
            {}
            if value.key is None
            else {
                "key": destack._generated.dir.symbol.key.to_json_static_key(value.key)
            }
        ),
        "symbol": destack._generated.dir.symbol.symbol.to_json_local_symbol_id(
            value.symbol
        ),
    }


def from_json_scope_binding(value: Json) -> ScopeBinding:
    """Return one ScopeBinding from one JSON value."""
    object_ = json_object(value)

    return ScopeBinding(
        key=json_optional(
            object_,
            "key",
            lambda value: destack._generated.dir.symbol.key.from_json_static_key(value),
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
            json_field(object_, "symbol")
        ),
    )


@dataclass(frozen=True, slots=True)
class ScopeIndexEmpty:
    """No named bindings."""

    kind: typing.Literal["empty"] = "empty"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope_index(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope_index(self)


@dataclass(frozen=True, slots=True)
class ScopeIndexSmall:
    """A small inline table for common tiny scopes."""

    # binding indices grouped by key
    entries: Sequence[ScopeIndexEntry]
    kind: typing.Literal["small"] = "small"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope_index(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope_index(self)


@dataclass(frozen=True, slots=True)
class ScopeIndexLarge:
    """A keyed table for larger scopes."""

    # binding indices grouped by key
    table: Mapping[destack._generated.dir.symbol.key.StaticKey, Sequence[int]]
    kind: typing.Literal["large"] = "large"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope_index(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope_index(self)


"""A compact name index for one lexical scope."""
ScopeIndex: typing.TypeAlias = ScopeIndexEmpty | ScopeIndexSmall | ScopeIndexLarge


def encode_scope_index(writer: BinaryWriter, value: ScopeIndex) -> None:
    """Encode one ScopeIndex."""
    if value.kind == "empty":
        writer.write_unsigned(0)
    elif value.kind == "small":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.entries))
        for item_value_entries_0 in value.entries:
            encode_scope_index_entry(writer, item_value_entries_0)
    elif value.kind == "large":
        writer.write_unsigned(2)
        entries_value_table_0 = []
        for key_value_table_0, item_value_table_0 in value.table.items():

            def write_key_value_table_0(writer: BinaryWriter) -> None:
                destack._generated.dir.symbol.key.encode_static_key(
                    writer, key_value_table_0
                )

            key_bytes = nested_bytes(write_key_value_table_0)
            entries_value_table_0.append(
                (key_value_table_0, item_value_table_0, key_bytes)
            )
        entries_value_table_0.sort(key=lambda entry: entry[2])
        writer.write_unsigned(len(entries_value_table_0))
        for entry_value_table_0 in entries_value_table_0:
            destack._generated.dir.symbol.key.encode_static_key(
                writer, entry_value_table_0[0]
            )
            writer.write_unsigned(len(entry_value_table_0[1]))
            for item_entry_value_table_0_1_1 in entry_value_table_0[1]:
                writer.write_unsigned(item_entry_value_table_0_1_1)
    else:
        raise SerdeError("unknown enum variant")


def decode_scope_index(reader: BinaryReader) -> ScopeIndex:
    """Decode one ScopeIndex."""
    variant = reader.read_number()

    if variant == 0:
        return ScopeIndexEmpty()
    elif variant == 1:
        entries = [
            decode_scope_index_entry(reader) for _ in range(reader.read_number())
        ]

        return ScopeIndexSmall(
            entries=entries,
        )
    elif variant == 2:
        table = {
            destack._generated.dir.symbol.key.decode_static_key(reader): [
                reader.read_number() for _ in range(reader.read_number())
            ]
            for _ in range(reader.read_number())
        }

        return ScopeIndexLarge(
            table=table,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_scope_index(value: ScopeIndex) -> Json:
    """Return one JSON value for one ScopeIndex."""
    if value.kind == "empty":
        return {
            "kind": "empty",
        }
    elif value.kind == "small":
        return {
            "kind": "small",
            "entries": [to_json_scope_index_entry(item_0) for item_0 in value.entries],
        }
    elif value.kind == "large":
        return {
            "kind": "large",
            "table": [
                [
                    destack._generated.dir.symbol.key.to_json_static_key(key_0),
                    [item_1 for item_1 in item_0],
                ]
                for key_0, item_0 in value.table.items()
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_scope_index(value: Json) -> ScopeIndex:
    """Return one ScopeIndex from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "empty":
        return ScopeIndexEmpty()
    elif kind == "small":
        return ScopeIndexSmall(
            entries=[
                from_json_scope_index_entry(item_0)
                for item_0 in json_array(json_field(object_, "entries"))
            ],
        )
    elif kind == "large":
        return ScopeIndexLarge(
            table={
                destack._generated.dir.symbol.key.from_json_static_key(key_0): [
                    json_int(item_1) for item_1 in json_array(item_0)
                ]
                for key_0, item_0 in json_array(json_field(object_, "table"))
            },
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ScopeIndexEntry:
    """Binding indices for one key in a small scope lookup."""

    # the binding key
    key: destack._generated.dir.symbol.key.StaticKey
    # binding indices in lexical order
    indices: Sequence[int]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_scope_index_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ScopeIndexEntry:
        """Decode one ScopeIndexEntry."""
        return decode_scope_index_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_scope_index_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> ScopeIndexEntry:
        """Return one ScopeIndexEntry from one JSON value."""
        return from_json_scope_index_entry(value)


def encode_scope_index_entry(writer: BinaryWriter, value: ScopeIndexEntry) -> None:
    """Encode one ScopeIndexEntry."""
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    writer.write_unsigned(len(value.indices))
    for item_value_indices_0 in value.indices:
        writer.write_unsigned(item_value_indices_0)


def decode_scope_index_entry(reader: BinaryReader) -> ScopeIndexEntry:
    """Decode one ScopeIndexEntry."""
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    indices = [reader.read_number() for _ in range(reader.read_number())]

    return ScopeIndexEntry(
        key=key,
        indices=indices,
    )


def to_json_scope_index_entry(value: ScopeIndexEntry) -> Json:
    """Return one JSON value for one ScopeIndexEntry."""
    return {
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "indices": [item_0 for item_0 in value.indices],
    }


def from_json_scope_index_entry(value: Json) -> ScopeIndexEntry:
    """Return one ScopeIndexEntry from one JSON value."""
    object_ = json_object(value)

    return ScopeIndexEntry(
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        indices=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "indices"))
        ],
    )


@dataclass(frozen=True, slots=True)
class GlobalScopeId:
    """Global scope id across modules."""

    # the module id of the global scope
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global scope
    local_id: LocalScopeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_scope_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalScopeId:
        """Decode one GlobalScopeId."""
        return decode_global_scope_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_scope_id(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalScopeId:
        """Return one GlobalScopeId from one JSON value."""
        return from_json_global_scope_id(value)


def encode_global_scope_id(writer: BinaryWriter, value: GlobalScopeId) -> None:
    """Encode one GlobalScopeId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_scope_id(writer, value.local_id)


def decode_global_scope_id(reader: BinaryReader) -> GlobalScopeId:
    """Decode one GlobalScopeId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = decode_local_scope_id(reader)

    return GlobalScopeId(
        module_id=module_id,
        local_id=local_id,
    )


def to_json_global_scope_id(value: GlobalScopeId) -> Json:
    """Return one JSON value for one GlobalScopeId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": to_json_local_scope_id(value.local_id),
    }


def from_json_global_scope_id(value: Json) -> GlobalScopeId:
    """Return one GlobalScopeId from one JSON value."""
    object_ = json_object(value)

    return GlobalScopeId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=from_json_local_scope_id(json_field(object_, "localId")),
    )


__all__ = [
    "LocalScope",
    "encode_local_scope",
    "decode_local_scope",
    "to_json_local_scope",
    "from_json_local_scope",
    "LocalScopeId",
    "encode_local_scope_id",
    "decode_local_scope_id",
    "to_json_local_scope_id",
    "from_json_local_scope_id",
    "LocalScopeMark",
    "encode_local_scope_mark",
    "decode_local_scope_mark",
    "to_json_local_scope_mark",
    "from_json_local_scope_mark",
    "Scope",
    "encode_scope",
    "decode_scope",
    "to_json_scope",
    "from_json_scope",
    "ScopeKind",
    "encode_scope_kind",
    "decode_scope_kind",
    "to_json_scope_kind",
    "from_json_scope_kind",
    "ScopeBinding",
    "encode_scope_binding",
    "decode_scope_binding",
    "to_json_scope_binding",
    "from_json_scope_binding",
    "ScopeIndex",
    "encode_scope_index",
    "decode_scope_index",
    "to_json_scope_index",
    "from_json_scope_index",
    "ScopeIndexEmpty",
    "ScopeIndexSmall",
    "ScopeIndexLarge",
    "ScopeIndexEntry",
    "encode_scope_index_entry",
    "decode_scope_index_entry",
    "to_json_scope_index_entry",
    "from_json_scope_index_entry",
    "GlobalScopeId",
    "encode_global_scope_id",
    "decode_global_scope_id",
    "to_json_global_scope_id",
    "from_json_global_scope_id",
]
