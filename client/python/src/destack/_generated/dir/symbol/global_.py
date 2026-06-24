# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.export
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GlobalEntryLocal:
    """A local global declaration."""

    local: LocalGlobalEntry
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_entry(self)


@dataclass(frozen=True, slots=True)
class GlobalEntryIndirect:
    """A re-exported global declaration."""

    indirect: IndirectGlobalEntry
    kind: typing.Literal["indirect"] = "indirect"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_entry(self)


"""One global table entry."""
GlobalEntry: typing.TypeAlias = GlobalEntryLocal | GlobalEntryIndirect


def encode_global_entry(writer: BinaryWriter, value: GlobalEntry) -> None:
    """Encode one GlobalEntry."""
    if value.kind == "local":
        writer.write_unsigned(0)
        encode_local_global_entry(writer, value.local)
    elif value.kind == "indirect":
        writer.write_unsigned(1)
        encode_indirect_global_entry(writer, value.indirect)
    else:
        raise SerdeError("unknown enum variant")


def decode_global_entry(reader: BinaryReader) -> GlobalEntry:
    """Decode one GlobalEntry."""
    variant = reader.read_number()

    if variant == 0:
        local = decode_local_global_entry(reader)

        return GlobalEntryLocal(local=local)
    elif variant == 1:
        indirect = decode_indirect_global_entry(reader)

        return GlobalEntryIndirect(indirect=indirect)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_global_entry(value: GlobalEntry) -> Json:
    """Return one JSON value for one GlobalEntry."""
    if value.kind == "local":
        return {
            "kind": "local",
            "local": to_json_local_global_entry(value.local),
        }
    elif value.kind == "indirect":
        return {
            "kind": "indirect",
            "indirect": to_json_indirect_global_entry(value.indirect),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_global_entry(value: Json) -> GlobalEntry:
    """Return one GlobalEntry from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "local":
        return GlobalEntryLocal(
            local=from_json_local_global_entry(json_field(object_, "local"))
        )
    elif kind == "indirect":
        return GlobalEntryIndirect(
            indirect=from_json_indirect_global_entry(json_field(object_, "indirect"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class LocalGlobalEntry:
    """One local global declaration from a symbol declared in the current module."""

    # the global name
    key: destack._generated.dir.symbol.key.StaticKey
    # the local symbol exposed as a global
    source: destack._generated.dir.symbol.symbol.LocalSymbolId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_global_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalGlobalEntry:
        """Decode one LocalGlobalEntry."""
        return decode_local_global_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_global_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalGlobalEntry:
        """Return one LocalGlobalEntry from one JSON value."""
        return from_json_local_global_entry(value)


def encode_local_global_entry(writer: BinaryWriter, value: LocalGlobalEntry) -> None:
    """Encode one LocalGlobalEntry."""
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.symbol.symbol.encode_local_symbol_id(writer, value.source)


def decode_local_global_entry(reader: BinaryReader) -> LocalGlobalEntry:
    """Decode one LocalGlobalEntry."""
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    source = destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)

    return LocalGlobalEntry(
        key=key,
        source=source,
    )


def to_json_local_global_entry(value: LocalGlobalEntry) -> Json:
    """Return one JSON value for one LocalGlobalEntry."""
    return {
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "source": destack._generated.dir.symbol.symbol.to_json_local_symbol_id(
            value.source
        ),
    }


def from_json_local_global_entry(value: Json) -> LocalGlobalEntry:
    """Return one LocalGlobalEntry from one JSON value."""
    object_ = json_object(value)

    return LocalGlobalEntry(
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        source=destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
            json_field(object_, "source")
        ),
    )


@dataclass(frozen=True, slots=True)
class IndirectGlobalEntry:
    """One named global re-export from another module."""

    # the global name
    key: destack._generated.dir.symbol.key.StaticKey
    # the dependency item that declared the global export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None
    # the export selected from the target module
    imported: destack._generated.dir.symbol.export.ExportSelector

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_indirect_global_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectGlobalEntry:
        """Decode one IndirectGlobalEntry."""
        return decode_indirect_global_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_indirect_global_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> IndirectGlobalEntry:
        """Return one IndirectGlobalEntry from one JSON value."""
        return from_json_indirect_global_entry(value)


def encode_indirect_global_entry(
    writer: BinaryWriter, value: IndirectGlobalEntry
) -> None:
    """Encode one IndirectGlobalEntry."""
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.item)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target
        )
    destack._generated.dir.symbol.export.encode_export_selector(writer, value.imported)


def decode_indirect_global_entry(reader: BinaryReader) -> IndirectGlobalEntry:
    """Decode one IndirectGlobalEntry."""
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    item = destack._generated.dir.tree.node.decode_local_node_id(reader)
    target = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    imported = destack._generated.dir.symbol.export.decode_export_selector(reader)

    return IndirectGlobalEntry(
        key=key,
        item=item,
        target=target,
        imported=imported,
    )


def to_json_indirect_global_entry(value: IndirectGlobalEntry) -> Json:
    """Return one JSON value for one IndirectGlobalEntry."""
    return {
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "item": destack._generated.dir.tree.node.to_json_local_node_id(value.item),
        **(
            {}
            if value.target is None
            else {
                "target": destack._generated.source.file.model.module.to_json_module_id(
                    value.target
                )
            }
        ),
        "imported": destack._generated.dir.symbol.export.to_json_export_selector(
            value.imported
        ),
    }


def from_json_indirect_global_entry(value: Json) -> IndirectGlobalEntry:
    """Return one IndirectGlobalEntry from one JSON value."""
    object_ = json_object(value)

    return IndirectGlobalEntry(
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        item=destack._generated.dir.tree.node.from_json_local_node_id(
            json_field(object_, "item")
        ),
        target=json_optional(
            object_,
            "target",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
        imported=destack._generated.dir.symbol.export.from_json_export_selector(
            json_field(object_, "imported")
        ),
    )


__all__ = [
    "GlobalEntry",
    "encode_global_entry",
    "decode_global_entry",
    "to_json_global_entry",
    "from_json_global_entry",
    "GlobalEntryLocal",
    "GlobalEntryIndirect",
    "LocalGlobalEntry",
    "encode_local_global_entry",
    "decode_local_global_entry",
    "to_json_local_global_entry",
    "from_json_local_global_entry",
    "IndirectGlobalEntry",
    "encode_indirect_global_entry",
    "decode_indirect_global_entry",
    "to_json_indirect_global_entry",
    "from_json_indirect_global_entry",
]
