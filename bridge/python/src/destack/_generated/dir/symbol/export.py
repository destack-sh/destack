# generated bridge target, do not edit

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

import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ExportKeyDefault:
    """The ECMAScript default export name."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_key(self)


@dataclass(frozen=True, slots=True)
class ExportKeyNamed:
    """A named export key."""

    named: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_key(self)


"""The exported name in one module record."""
ExportKey: typing.TypeAlias = ExportKeyDefault | ExportKeyNamed


def encode_export_key(writer: BinaryWriter, value: ExportKey) -> None:
    """Encode one ExportKey."""
    if value.kind == "default":
        writer.write_unsigned(0)
    elif value.kind == "named":
        writer.write_unsigned(1)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.named)
    else:
        raise SerdeError("unknown enum variant")


def decode_export_key(reader: BinaryReader) -> ExportKey:
    """Decode one ExportKey."""
    variant = reader.read_number()

    if variant == 0:
        return ExportKeyDefault()
    elif variant == 1:
        named = destack._generated.dir.symbol.key.decode_static_key(reader)

        return ExportKeyNamed(named=named)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_export_key(value: ExportKey) -> Json:
    """Return one JSON value for one ExportKey."""
    if value.kind == "default":
        return {
            "kind": "default",
        }
    elif value.kind == "named":
        return {
            "kind": "named",
            "named": destack._generated.dir.symbol.key.to_json_static_key(value.named),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_export_key(value: Json) -> ExportKey:
    """Return one ExportKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "default":
        return ExportKeyDefault()
    elif kind == "named":
        return ExportKeyNamed(
            named=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "named")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ExportEntryLocal:
    """A local export."""

    local: LocalExportEntry
    kind: typing.Literal["local"] = "local"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_entry(self)


@dataclass(frozen=True, slots=True)
class ExportEntryIndirect:
    """A re-export from another module."""

    indirect: IndirectExportEntry
    kind: typing.Literal["indirect"] = "indirect"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_entry(self)


"""One named export entry."""
ExportEntry: typing.TypeAlias = ExportEntryLocal | ExportEntryIndirect


def encode_export_entry(writer: BinaryWriter, value: ExportEntry) -> None:
    """Encode one ExportEntry."""
    if value.kind == "local":
        writer.write_unsigned(0)
        encode_local_export_entry(writer, value.local)
    elif value.kind == "indirect":
        writer.write_unsigned(1)
        encode_indirect_export_entry(writer, value.indirect)
    else:
        raise SerdeError("unknown enum variant")


def decode_export_entry(reader: BinaryReader) -> ExportEntry:
    """Decode one ExportEntry."""
    variant = reader.read_number()

    if variant == 0:
        local = decode_local_export_entry(reader)

        return ExportEntryLocal(local=local)
    elif variant == 1:
        indirect = decode_indirect_export_entry(reader)

        return ExportEntryIndirect(indirect=indirect)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_export_entry(value: ExportEntry) -> Json:
    """Return one JSON value for one ExportEntry."""
    if value.kind == "local":
        return {
            "kind": "local",
            "local": to_json_local_export_entry(value.local),
        }
    elif value.kind == "indirect":
        return {
            "kind": "indirect",
            "indirect": to_json_indirect_export_entry(value.indirect),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_export_entry(value: Json) -> ExportEntry:
    """Return one ExportEntry from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "local":
        return ExportEntryLocal(
            local=from_json_local_export_entry(json_field(object_, "local"))
        )
    elif kind == "indirect":
        return ExportEntryIndirect(
            indirect=from_json_indirect_export_entry(json_field(object_, "indirect"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class LocalExportEntry:
    """One local export from a symbol declared in the current module."""

    # the exported name
    key: ExportKey
    # the local symbol exposed by the export
    source: destack._generated.dir.symbol.symbol.LocalSymbolId
    # the export clause item that declared this export
    item: destack._generated.dir.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_local_export_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LocalExportEntry:
        """Decode one LocalExportEntry."""
        return decode_local_export_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_local_export_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> LocalExportEntry:
        """Return one LocalExportEntry from one JSON value."""
        return from_json_local_export_entry(value)


def encode_local_export_entry(writer: BinaryWriter, value: LocalExportEntry) -> None:
    """Encode one LocalExportEntry."""
    encode_export_key(writer, value.key)
    destack._generated.dir.symbol.symbol.encode_local_symbol_id(writer, value.source)
    if value.item is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.node.encode_local_node_id(writer, value.item)


def decode_local_export_entry(reader: BinaryReader) -> LocalExportEntry:
    """Decode one LocalExportEntry."""
    key = decode_export_key(reader)
    source = destack._generated.dir.symbol.symbol.decode_local_symbol_id(reader)
    item = reader.read_option(
        lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
    )

    return LocalExportEntry(
        key=key,
        source=source,
        item=item,
    )


def to_json_local_export_entry(value: LocalExportEntry) -> Json:
    """Return one JSON value for one LocalExportEntry."""
    return {
        "key": to_json_export_key(value.key),
        "source": destack._generated.dir.symbol.symbol.to_json_local_symbol_id(
            value.source
        ),
        **(
            {}
            if value.item is None
            else {
                "item": destack._generated.dir.tree.node.to_json_local_node_id(
                    value.item
                )
            }
        ),
    }


def from_json_local_export_entry(value: Json) -> LocalExportEntry:
    """Return one LocalExportEntry from one JSON value."""
    object_ = json_object(value)

    return LocalExportEntry(
        key=from_json_export_key(json_field(object_, "key")),
        source=destack._generated.dir.symbol.symbol.from_json_local_symbol_id(
            json_field(object_, "source")
        ),
        item=json_optional(
            object_,
            "item",
            lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


@dataclass(frozen=True, slots=True)
class IndirectExportEntry:
    """One named re-export from another module."""

    # the exported name in the current module
    key: ExportKey
    # the dependency item that declared the export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None
    # the export selected from the target module
    imported: ExportSelector

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_indirect_export_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndirectExportEntry:
        """Decode one IndirectExportEntry."""
        return decode_indirect_export_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_indirect_export_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> IndirectExportEntry:
        """Return one IndirectExportEntry from one JSON value."""
        return from_json_indirect_export_entry(value)


def encode_indirect_export_entry(
    writer: BinaryWriter, value: IndirectExportEntry
) -> None:
    """Encode one IndirectExportEntry."""
    encode_export_key(writer, value.key)
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.item)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target
        )
    encode_export_selector(writer, value.imported)


def decode_indirect_export_entry(reader: BinaryReader) -> IndirectExportEntry:
    """Decode one IndirectExportEntry."""
    key = decode_export_key(reader)
    item = destack._generated.dir.tree.node.decode_local_node_id(reader)
    target = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )
    imported = decode_export_selector(reader)

    return IndirectExportEntry(
        key=key,
        item=item,
        target=target,
        imported=imported,
    )


def to_json_indirect_export_entry(value: IndirectExportEntry) -> Json:
    """Return one JSON value for one IndirectExportEntry."""
    return {
        "key": to_json_export_key(value.key),
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
        "imported": to_json_export_selector(value.imported),
    }


def from_json_indirect_export_entry(value: Json) -> IndirectExportEntry:
    """Return one IndirectExportEntry from one JSON value."""
    object_ = json_object(value)

    return IndirectExportEntry(
        key=from_json_export_key(json_field(object_, "key")),
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
        imported=from_json_export_selector(json_field(object_, "imported")),
    )


@dataclass(frozen=True, slots=True)
class ExportSelectorNamed:
    """A named target export."""

    named: destack._generated.dir.symbol.key.StaticKey
    kind: typing.Literal["named"] = "named"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_selector(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_selector(self)


@dataclass(frozen=True, slots=True)
class ExportSelectorDefault:
    """The target module default export."""

    kind: typing.Literal["default"] = "default"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_selector(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_selector(self)


@dataclass(frozen=True, slots=True)
class ExportSelectorNamespace:
    """The target module namespace object."""

    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_selector(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_selector(self)


"""Which binding a re-export selects from the target module."""
ExportSelector: typing.TypeAlias = (
    ExportSelectorNamed | ExportSelectorDefault | ExportSelectorNamespace
)


def encode_export_selector(writer: BinaryWriter, value: ExportSelector) -> None:
    """Encode one ExportSelector."""
    if value.kind == "named":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.key.encode_static_key(writer, value.named)
    elif value.kind == "default":
        writer.write_unsigned(1)
    elif value.kind == "namespace":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_export_selector(reader: BinaryReader) -> ExportSelector:
    """Decode one ExportSelector."""
    variant = reader.read_number()

    if variant == 0:
        named = destack._generated.dir.symbol.key.decode_static_key(reader)

        return ExportSelectorNamed(named=named)
    elif variant == 1:
        return ExportSelectorDefault()
    elif variant == 2:
        return ExportSelectorNamespace()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_export_selector(value: ExportSelector) -> Json:
    """Return one JSON value for one ExportSelector."""
    if value.kind == "named":
        return {
            "kind": "named",
            "named": destack._generated.dir.symbol.key.to_json_static_key(value.named),
        }
    elif value.kind == "default":
        return {
            "kind": "default",
        }
    elif value.kind == "namespace":
        return {
            "kind": "namespace",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_export_selector(value: Json) -> ExportSelector:
    """Return one ExportSelector from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "named":
        return ExportSelectorNamed(
            named=destack._generated.dir.symbol.key.from_json_static_key(
                json_field(object_, "named")
            )
        )
    elif kind == "default":
        return ExportSelectorDefault()
    elif kind == "namespace":
        return ExportSelectorNamespace()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class StarExportEntry:
    """One `export * from` edge."""

    # the dependency item that declared the star export
    item: destack._generated.dir.tree.node.LocalNodeId
    # the target module selected by the export
    target: destack._generated.source.file.model.module.ModuleId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_star_export_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StarExportEntry:
        """Decode one StarExportEntry."""
        return decode_star_export_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_star_export_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> StarExportEntry:
        """Return one StarExportEntry from one JSON value."""
        return from_json_star_export_entry(value)


def encode_star_export_entry(writer: BinaryWriter, value: StarExportEntry) -> None:
    """Encode one StarExportEntry."""
    destack._generated.dir.tree.node.encode_local_node_id(writer, value.item)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target
        )


def decode_star_export_entry(reader: BinaryReader) -> StarExportEntry:
    """Decode one StarExportEntry."""
    item = destack._generated.dir.tree.node.decode_local_node_id(reader)
    target = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )

    return StarExportEntry(
        item=item,
        target=target,
    )


def to_json_star_export_entry(value: StarExportEntry) -> Json:
    """Return one JSON value for one StarExportEntry."""
    return {
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
    }


def from_json_star_export_entry(value: Json) -> StarExportEntry:
    """Return one StarExportEntry from one JSON value."""
    object_ = json_object(value)

    return StarExportEntry(
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
    )


__all__ = [
    "ExportKey",
    "encode_export_key",
    "decode_export_key",
    "to_json_export_key",
    "from_json_export_key",
    "ExportKeyDefault",
    "ExportKeyNamed",
    "ExportEntry",
    "encode_export_entry",
    "decode_export_entry",
    "to_json_export_entry",
    "from_json_export_entry",
    "ExportEntryLocal",
    "ExportEntryIndirect",
    "LocalExportEntry",
    "encode_local_export_entry",
    "decode_local_export_entry",
    "to_json_local_export_entry",
    "from_json_local_export_entry",
    "IndirectExportEntry",
    "encode_indirect_export_entry",
    "decode_indirect_export_entry",
    "to_json_indirect_export_entry",
    "from_json_indirect_export_entry",
    "ExportSelector",
    "encode_export_selector",
    "decode_export_selector",
    "to_json_export_selector",
    "from_json_export_selector",
    "ExportSelectorNamed",
    "ExportSelectorDefault",
    "ExportSelectorNamespace",
    "StarExportEntry",
    "encode_star_export_entry",
    "decode_star_export_entry",
    "to_json_star_export_entry",
    "from_json_star_export_entry",
]
