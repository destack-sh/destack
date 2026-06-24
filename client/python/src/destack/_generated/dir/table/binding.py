# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    nested_bytes,
)

import destack._generated.dir.symbol.scope
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class BindingSegment:
    """Lexical scopes and symbols added by one DIR phase."""

    # the module id of the binding segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first symbol id owned by this table segment
    first_symbol_id: int
    # the first scope id owned by this table segment
    first_scope_id: int
    # the symbols in the table
    symbols: Sequence[destack._generated.dir.symbol.symbol.Symbol]
    # the scopes in the table
    scopes: Sequence[destack._generated.dir.symbol.scope.Scope]
    # symbols keyed by their declaration node
    symbol_by_declaration: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.symbol.LocalSymbolId,
    ]
    # implicit receiver symbols keyed by their owner node
    implicit_receiver_by_node: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.symbol.LocalSymbolId,
    ]
    # scopes keyed by their owner or member node
    scope_by_node: Mapping[
        destack._generated.dir.tree.node.GlobalNodeIdAny,
        destack._generated.dir.symbol.scope.LocalScope,
    ]
    # scopes keyed by their owner symbol
    scope_by_owner: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId,
        destack._generated.dir.symbol.scope.LocalScopeId,
    ]
    # replacements for visible symbols copied into this segment
    replaced_symbol_by_id: Mapping[
        destack._generated.dir.symbol.symbol.LocalSymbolId,
        destack._generated.dir.symbol.symbol.Symbol,
    ]
    # replacements for visible scopes copied into this segment
    replaced_scope_by_id: Mapping[
        destack._generated.dir.symbol.scope.LocalScopeId,
        destack._generated.dir.symbol.scope.Scope,
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_binding_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> BindingSegment:
        """Decode one BindingSegment."""
        return decode_binding_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_binding_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> BindingSegment:
        """Return one BindingSegment from one JSON value."""
        return from_json_binding_segment(value)


def encode_binding_segment(writer: BinaryWriter, value: BindingSegment) -> None:
    """Encode one BindingSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_symbol_id)
    writer.write_unsigned(value.first_scope_id)
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        destack._generated.dir.symbol.symbol.encode_symbol(writer, item_value_symbols_0)
    writer.write_unsigned(len(value.scopes))
    for item_value_scopes_0 in value.scopes:
        destack._generated.dir.symbol.scope.encode_scope(writer, item_value_scopes_0)
    entries_value_symbol_by_declaration_0 = []
    for (
        key_value_symbol_by_declaration_0,
        item_value_symbol_by_declaration_0,
    ) in value.symbol_by_declaration.items():

        def write_key_value_symbol_by_declaration_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_symbol_by_declaration_0
            )

        key_bytes = nested_bytes(write_key_value_symbol_by_declaration_0)
        entries_value_symbol_by_declaration_0.append(
            (
                key_value_symbol_by_declaration_0,
                item_value_symbol_by_declaration_0,
                key_bytes,
            )
        )
    entries_value_symbol_by_declaration_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_symbol_by_declaration_0))
    for entry_value_symbol_by_declaration_0 in entries_value_symbol_by_declaration_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_symbol_by_declaration_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(
            writer, entry_value_symbol_by_declaration_0[1]
        )
    entries_value_implicit_receiver_by_node_0 = []
    for (
        key_value_implicit_receiver_by_node_0,
        item_value_implicit_receiver_by_node_0,
    ) in value.implicit_receiver_by_node.items():

        def write_key_value_implicit_receiver_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_implicit_receiver_by_node_0
            )

        key_bytes = nested_bytes(write_key_value_implicit_receiver_by_node_0)
        entries_value_implicit_receiver_by_node_0.append(
            (
                key_value_implicit_receiver_by_node_0,
                item_value_implicit_receiver_by_node_0,
                key_bytes,
            )
        )
    entries_value_implicit_receiver_by_node_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_implicit_receiver_by_node_0))
    for (
        entry_value_implicit_receiver_by_node_0
    ) in entries_value_implicit_receiver_by_node_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_implicit_receiver_by_node_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(
            writer, entry_value_implicit_receiver_by_node_0[1]
        )
    entries_value_scope_by_node_0 = []
    for (
        key_value_scope_by_node_0,
        item_value_scope_by_node_0,
    ) in value.scope_by_node.items():

        def write_key_value_scope_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_scope_by_node_0
            )

        key_bytes = nested_bytes(write_key_value_scope_by_node_0)
        entries_value_scope_by_node_0.append(
            (key_value_scope_by_node_0, item_value_scope_by_node_0, key_bytes)
        )
    entries_value_scope_by_node_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_scope_by_node_0))
    for entry_value_scope_by_node_0 in entries_value_scope_by_node_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_scope_by_node_0[0]
        )
        destack._generated.dir.symbol.scope.encode_local_scope(
            writer, entry_value_scope_by_node_0[1]
        )
    entries_value_scope_by_owner_0 = []
    for (
        key_value_scope_by_owner_0,
        item_value_scope_by_owner_0,
    ) in value.scope_by_owner.items():

        def write_key_value_scope_by_owner_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_local_symbol_id(
                writer, key_value_scope_by_owner_0
            )

        key_bytes = nested_bytes(write_key_value_scope_by_owner_0)
        entries_value_scope_by_owner_0.append(
            (key_value_scope_by_owner_0, item_value_scope_by_owner_0, key_bytes)
        )
    entries_value_scope_by_owner_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_scope_by_owner_0))
    for entry_value_scope_by_owner_0 in entries_value_scope_by_owner_0:
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(
            writer, entry_value_scope_by_owner_0[0]
        )
        destack._generated.dir.symbol.scope.encode_local_scope_id(
            writer, entry_value_scope_by_owner_0[1]
        )
    entries_value_replaced_symbol_by_id_0 = []
    for (
        key_value_replaced_symbol_by_id_0,
        item_value_replaced_symbol_by_id_0,
    ) in value.replaced_symbol_by_id.items():

        def write_key_value_replaced_symbol_by_id_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.symbol.encode_local_symbol_id(
                writer, key_value_replaced_symbol_by_id_0
            )

        key_bytes = nested_bytes(write_key_value_replaced_symbol_by_id_0)
        entries_value_replaced_symbol_by_id_0.append(
            (
                key_value_replaced_symbol_by_id_0,
                item_value_replaced_symbol_by_id_0,
                key_bytes,
            )
        )
    entries_value_replaced_symbol_by_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_replaced_symbol_by_id_0))
    for entry_value_replaced_symbol_by_id_0 in entries_value_replaced_symbol_by_id_0:
        destack._generated.dir.symbol.symbol.encode_local_symbol_id(
            writer, entry_value_replaced_symbol_by_id_0[0]
        )
        destack._generated.dir.symbol.symbol.encode_symbol(
            writer, entry_value_replaced_symbol_by_id_0[1]
        )
    entries_value_replaced_scope_by_id_0 = []
    for (
        key_value_replaced_scope_by_id_0,
        item_value_replaced_scope_by_id_0,
    ) in value.replaced_scope_by_id.items():

        def write_key_value_replaced_scope_by_id_0(writer: BinaryWriter) -> None:
            destack._generated.dir.symbol.scope.encode_local_scope_id(
                writer, key_value_replaced_scope_by_id_0
            )

        key_bytes = nested_bytes(write_key_value_replaced_scope_by_id_0)
        entries_value_replaced_scope_by_id_0.append(
            (
                key_value_replaced_scope_by_id_0,
                item_value_replaced_scope_by_id_0,
                key_bytes,
            )
        )
    entries_value_replaced_scope_by_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_replaced_scope_by_id_0))
    for entry_value_replaced_scope_by_id_0 in entries_value_replaced_scope_by_id_0:
        destack._generated.dir.symbol.scope.encode_local_scope_id(
            writer, entry_value_replaced_scope_by_id_0[0]
        )
        destack._generated.dir.symbol.scope.encode_scope(
            writer, entry_value_replaced_scope_by_id_0[1]
        )


def decode_binding_segment(reader: BinaryReader) -> BindingSegment:
    """Decode one BindingSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_symbol_id = reader.read_number()
    first_scope_id = reader.read_number()
    symbols = [
        destack._generated.dir.symbol.symbol.decode_symbol(reader)
        for _ in range(reader.read_number())
    ]
    scopes = [
        destack._generated.dir.symbol.scope.decode_scope(reader)
        for _ in range(reader.read_number())
    ]
    symbol_by_declaration = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)
        for _ in range(reader.read_number())
    }
    implicit_receiver_by_node = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)
        for _ in range(reader.read_number())
    }
    scope_by_node = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): destack._generated.dir.symbol.scope.decode_local_scope(reader)
        for _ in range(reader.read_number())
    }
    scope_by_owner = {
        destack._generated.dir.symbol.symbol.decode_local_symbol_id(
            reader
        ): destack._generated.dir.symbol.scope.decode_local_scope_id(reader)
        for _ in range(reader.read_number())
    }
    replaced_symbol_by_id = {
        destack._generated.dir.symbol.symbol.decode_local_symbol_id(
            reader
        ): destack._generated.dir.symbol.symbol.decode_symbol(reader)
        for _ in range(reader.read_number())
    }
    replaced_scope_by_id = {
        destack._generated.dir.symbol.scope.decode_local_scope_id(
            reader
        ): destack._generated.dir.symbol.scope.decode_scope(reader)
        for _ in range(reader.read_number())
    }

    return BindingSegment(
        module_id=module_id,
        first_symbol_id=first_symbol_id,
        first_scope_id=first_scope_id,
        symbols=symbols,
        scopes=scopes,
        symbol_by_declaration=symbol_by_declaration,
        implicit_receiver_by_node=implicit_receiver_by_node,
        scope_by_node=scope_by_node,
        scope_by_owner=scope_by_owner,
        replaced_symbol_by_id=replaced_symbol_by_id,
        replaced_scope_by_id=replaced_scope_by_id,
    )


def to_json_binding_segment(value: BindingSegment) -> Json:
    """Return one JSON value for one BindingSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstSymbolId": value.first_symbol_id,
        "firstScopeId": value.first_scope_id,
        "symbols": [
            destack._generated.dir.symbol.symbol.to_json_symbol(item_0)
            for item_0 in value.symbols
        ],
        "scopes": [
            destack._generated.dir.symbol.scope.to_json_scope(item_0)
            for item_0 in value.scopes
        ],
        "symbolByDeclaration": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.symbol.symbol.to_json_local_symbol_id(item_0),
            ]
            for key_0, item_0 in value.symbol_by_declaration.items()
        ],
        "implicitReceiverByNode": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.symbol.symbol.to_json_local_symbol_id(item_0),
            ]
            for key_0, item_0 in value.implicit_receiver_by_node.items()
        ],
        "scopeByNode": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                destack._generated.dir.symbol.scope.to_json_local_scope(item_0),
            ]
            for key_0, item_0 in value.scope_by_node.items()
        ],
        "scopeByOwner": [
            [
                destack._generated.dir.symbol.symbol.to_json_local_symbol_id(key_0),
                destack._generated.dir.symbol.scope.to_json_local_scope_id(item_0),
            ]
            for key_0, item_0 in value.scope_by_owner.items()
        ],
        "replacedSymbolById": [
            [
                destack._generated.dir.symbol.symbol.to_json_local_symbol_id(key_0),
                destack._generated.dir.symbol.symbol.to_json_symbol(item_0),
            ]
            for key_0, item_0 in value.replaced_symbol_by_id.items()
        ],
        "replacedScopeById": [
            [
                destack._generated.dir.symbol.scope.to_json_local_scope_id(key_0),
                destack._generated.dir.symbol.scope.to_json_scope(item_0),
            ]
            for key_0, item_0 in value.replaced_scope_by_id.items()
        ],
    }


def from_json_binding_segment(value: Json) -> BindingSegment:
    """Return one BindingSegment from one JSON value."""
    object_ = json_object(value)

    return BindingSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_symbol_id=json_int(json_field(object_, "firstSymbolId")),
        first_scope_id=json_int(json_field(object_, "firstScopeId")),
        symbols=[
            destack._generated.dir.symbol.symbol.from_json_symbol(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
        scopes=[
            destack._generated.dir.symbol.scope.from_json_scope(item_0)
            for item_0 in json_array(json_field(object_, "scopes"))
        ],
        symbol_by_declaration={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_local_symbol_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "symbolByDeclaration"))
        },
        implicit_receiver_by_node={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_local_symbol_id(item_0)
            for key_0, item_0 in json_array(
                json_field(object_, "implicitReceiverByNode")
            )
        },
        scope_by_node={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): destack._generated.dir.symbol.scope.from_json_local_scope(item_0)
            for key_0, item_0 in json_array(json_field(object_, "scopeByNode"))
        },
        scope_by_owner={
            destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
                key_0
            ): destack._generated.dir.symbol.scope.from_json_local_scope_id(item_0)
            for key_0, item_0 in json_array(json_field(object_, "scopeByOwner"))
        },
        replaced_symbol_by_id={
            destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
                key_0
            ): destack._generated.dir.symbol.symbol.from_json_symbol(item_0)
            for key_0, item_0 in json_array(json_field(object_, "replacedSymbolById"))
        },
        replaced_scope_by_id={
            destack._generated.dir.symbol.scope.from_json_local_scope_id(
                key_0
            ): destack._generated.dir.symbol.scope.from_json_scope(item_0)
            for key_0, item_0 in json_array(json_field(object_, "replacedScopeById"))
        },
    )


__all__ = [
    "BindingSegment",
    "encode_binding_segment",
    "decode_binding_segment",
    "to_json_binding_segment",
    "from_json_binding_segment",
]
