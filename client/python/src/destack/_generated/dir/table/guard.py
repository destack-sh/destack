# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping
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
    json_object,
    json_string,
    nested_bytes,
)

import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GuardTable:
    """Elaborated type guard checks."""

    # the module id of the guard table
    module_id: destack._generated.source.file.model.module.ModuleId
    # runtime guard entry by guard node
    entry_by_node: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, GuardEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GuardTable:
        """Decode one GuardTable."""
        return decode_guard_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_table(self)

    @classmethod
    def from_json(cls, value: Json) -> GuardTable:
        """Return one GuardTable from one JSON value."""
        return from_json_guard_table(value)


def encode_guard_table(writer: BinaryWriter, value: GuardTable) -> None:
    """Encode one GuardTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_entry_by_node_0 = []
    for (
        key_value_entry_by_node_0,
        item_value_entry_by_node_0,
    ) in value.entry_by_node.items():

        def write_key_value_entry_by_node_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_entry_by_node_0
            )

        key_bytes = nested_bytes(write_key_value_entry_by_node_0)
        entries_value_entry_by_node_0.append(
            (key_value_entry_by_node_0, item_value_entry_by_node_0, key_bytes)
        )
    entries_value_entry_by_node_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_entry_by_node_0))
    for entry_value_entry_by_node_0 in entries_value_entry_by_node_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_entry_by_node_0[0]
        )
        encode_guard_entry(writer, entry_value_entry_by_node_0[1])


def decode_guard_table(reader: BinaryReader) -> GuardTable:
    """Decode one GuardTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    entry_by_node = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): decode_guard_entry(reader)
        for _ in range(reader.read_number())
    }

    return GuardTable(
        module_id=module_id,
        entry_by_node=entry_by_node,
    )


def to_json_guard_table(value: GuardTable) -> Json:
    """Return one JSON value for one GuardTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "entryByNode": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                to_json_guard_entry(item_0),
            ]
            for key_0, item_0 in value.entry_by_node.items()
        ],
    }


def from_json_guard_table(value: Json) -> GuardTable:
    """Return one GuardTable from one JSON value."""
    object_ = json_object(value)

    return GuardTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        entry_by_node={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): from_json_guard_entry(item_0)
            for key_0, item_0 in json_array(json_field(object_, "entryByNode"))
        },
    )


@dataclass(frozen=True, slots=True)
class GuardEntryConstant:
    """The runtime check was reduced to a constant."""

    constant: bool
    kind: typing.Literal["constant"] = "constant"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_entry(self)


@dataclass(frozen=True, slots=True)
class GuardEntryUnionTag:
    """The runtime check uses a union tag."""

    kind: typing.Literal["unionTag"] = "unionTag"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_entry(self)


@dataclass(frozen=True, slots=True)
class GuardEntryTypeDescriptor:
    """The runtime check compares type identities."""

    kind: typing.Literal["typeDescriptor"] = "typeDescriptor"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_guard_entry(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_guard_entry(self)


"""Runtime check selected for one elaborated type guard."""
GuardEntry: typing.TypeAlias = (
    GuardEntryConstant | GuardEntryUnionTag | GuardEntryTypeDescriptor
)


def encode_guard_entry(writer: BinaryWriter, value: GuardEntry) -> None:
    """Encode one GuardEntry."""
    if value.kind == "constant":
        writer.write_unsigned(0)
        writer.write_bool(value.constant)
    elif value.kind == "unionTag":
        writer.write_unsigned(1)
    elif value.kind == "typeDescriptor":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_guard_entry(reader: BinaryReader) -> GuardEntry:
    """Decode one GuardEntry."""
    variant = reader.read_number()

    if variant == 0:
        constant = reader.read_bool()

        return GuardEntryConstant(constant=constant)
    elif variant == 1:
        return GuardEntryUnionTag()
    elif variant == 2:
        return GuardEntryTypeDescriptor()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_guard_entry(value: GuardEntry) -> Json:
    """Return one JSON value for one GuardEntry."""
    if value.kind == "constant":
        return {
            "kind": "constant",
            "constant": value.constant,
        }
    elif value.kind == "unionTag":
        return {
            "kind": "unionTag",
        }
    elif value.kind == "typeDescriptor":
        return {
            "kind": "typeDescriptor",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_guard_entry(value: Json) -> GuardEntry:
    """Return one GuardEntry from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "constant":
        return GuardEntryConstant(constant=json_bool(json_field(object_, "constant")))
    elif kind == "unionTag":
        return GuardEntryUnionTag()
    elif kind == "typeDescriptor":
        return GuardEntryTypeDescriptor()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "GuardTable",
    "encode_guard_table",
    "decode_guard_table",
    "to_json_guard_table",
    "from_json_guard_table",
    "GuardEntry",
    "encode_guard_entry",
    "decode_guard_entry",
    "to_json_guard_entry",
    "from_json_guard_entry",
    "GuardEntryConstant",
    "GuardEntryUnionTag",
    "GuardEntryTypeDescriptor",
]
