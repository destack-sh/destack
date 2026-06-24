# generated bridge target, do not edit

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
    json_string,
    nested_bytes,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ReferenceTable:
    """Name resolutions for one module, keyed by the reference node."""

    # the module id of the reference table
    module_id: destack._generated.source.file.model.module.ModuleId
    # resolved references keyed by their source node
    entries: Mapping[destack._generated.dir.tree.node.GlobalNodeIdAny, Reference]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ReferenceTable:
        """Decode one ReferenceTable."""
        return decode_reference_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ReferenceTable:
        """Return one ReferenceTable from one JSON value."""
        return from_json_reference_table(value)


def encode_reference_table(writer: BinaryWriter, value: ReferenceTable) -> None:
    """Encode one ReferenceTable."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    entries_value_entries_0 = []
    for key_value_entries_0, item_value_entries_0 in value.entries.items():

        def write_key_value_entries_0(writer: BinaryWriter) -> None:
            destack._generated.dir.tree.node.encode_global_node_id_any(
                writer, key_value_entries_0
            )

        key_bytes = nested_bytes(write_key_value_entries_0)
        entries_value_entries_0.append(
            (key_value_entries_0, item_value_entries_0, key_bytes)
        )
    entries_value_entries_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_entries_0))
    for entry_value_entries_0 in entries_value_entries_0:
        destack._generated.dir.tree.node.encode_global_node_id_any(
            writer, entry_value_entries_0[0]
        )
        encode_reference(writer, entry_value_entries_0[1])


def decode_reference_table(reader: BinaryReader) -> ReferenceTable:
    """Decode one ReferenceTable."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    entries = {
        destack._generated.dir.tree.node.decode_global_node_id_any(
            reader
        ): decode_reference(reader)
        for _ in range(reader.read_number())
    }

    return ReferenceTable(
        module_id=module_id,
        entries=entries,
    )


def to_json_reference_table(value: ReferenceTable) -> Json:
    """Return one JSON value for one ReferenceTable."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "entries": [
            [
                destack._generated.dir.tree.node.to_json_global_node_id_any(key_0),
                to_json_reference(item_0),
            ]
            for key_0, item_0 in value.entries.items()
        ],
    }


def from_json_reference_table(value: Json) -> ReferenceTable:
    """Return one ReferenceTable from one JSON value."""
    object_ = json_object(value)

    return ReferenceTable(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        entries={
            destack._generated.dir.tree.node.from_json_global_node_id_any(
                key_0
            ): from_json_reference(item_0)
            for key_0, item_0 in json_array(json_field(object_, "entries"))
        },
    )


@dataclass(frozen=True, slots=True)
class ReferenceBound:
    """Resolved to declarations by name: lexical scope or a full namespace path."""

    bound: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]
    kind: typing.Literal["bound"] = "bound"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)


@dataclass(frozen=True, slots=True)
class ReferenceNamespace:
    """Resolved to a namespace: a prefix awaiting a further segment, or a bare namespace value."""

    namespace: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["namespace"] = "namespace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)


@dataclass(frozen=True, slots=True)
class ReferenceProjected:
    """A flat path named through its first segments; `segments[from..]` project as members off `base`."""

    # the declaration the leading segments name
    base: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the segment index where member projection begins
    from_: int
    kind: typing.Literal["projected"] = "projected"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)


@dataclass(frozen=True, slots=True)
class ReferenceAmbiguous:
    """Conflicting bindings with no single winner."""

    ambiguous: Sequence[destack._generated.dir.symbol.symbol.GlobalSymbolId]
    kind: typing.Literal["ambiguous"] = "ambiguous"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)


@dataclass(frozen=True, slots=True)
class ReferenceMissing:
    """No binding by name."""

    kind: typing.Literal["missing"] = "missing"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_reference(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_reference(self)


"""How one source reference resolves by name, before types and conditions apply."""
Reference: typing.TypeAlias = (
    ReferenceBound
    | ReferenceNamespace
    | ReferenceProjected
    | ReferenceAmbiguous
    | ReferenceMissing
)


def encode_reference(writer: BinaryWriter, value: Reference) -> None:
    """Encode one Reference."""
    if value.kind == "bound":
        writer.write_unsigned(0)
        writer.write_unsigned(len(value.bound))
        for item_value_bound_0 in value.bound:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, item_value_bound_0
            )
    elif value.kind == "namespace":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.namespace
        )
    elif value.kind == "projected":
        writer.write_unsigned(2)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.base)
        writer.write_unsigned(value.from_)
    elif value.kind == "ambiguous":
        writer.write_unsigned(3)
        writer.write_unsigned(len(value.ambiguous))
        for item_value_ambiguous_0 in value.ambiguous:
            destack._generated.dir.symbol.symbol.encode_global_symbol_id(
                writer, item_value_ambiguous_0
            )
    elif value.kind == "missing":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_reference(reader: BinaryReader) -> Reference:
    """Decode one Reference."""
    variant = reader.read_number()

    if variant == 0:
        bound = [
            destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
            for _ in range(reader.read_number())
        ]

        return ReferenceBound(bound=bound)
    elif variant == 1:
        namespace = destack._generated.source.file.model.module.decode_module_id(reader)

        return ReferenceNamespace(namespace=namespace)
    elif variant == 2:
        base = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        from_ = reader.read_number()

        return ReferenceProjected(
            base=base,
            from_=from_,
        )
    elif variant == 3:
        ambiguous = [
            destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
            for _ in range(reader.read_number())
        ]

        return ReferenceAmbiguous(ambiguous=ambiguous)
    elif variant == 4:
        return ReferenceMissing()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_reference(value: Reference) -> Json:
    """Return one JSON value for one Reference."""
    if value.kind == "bound":
        return {
            "kind": "bound",
            "bound": [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0)
                for item_0 in value.bound
            ],
        }
    elif value.kind == "namespace":
        return {
            "kind": "namespace",
            "namespace": destack._generated.source.file.model.module.to_json_module_id(
                value.namespace
            ),
        }
    elif value.kind == "projected":
        return {
            "kind": "projected",
            "base": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.base
            ),
            "from": value.from_,
        }
    elif value.kind == "ambiguous":
        return {
            "kind": "ambiguous",
            "ambiguous": [
                destack._generated.dir.symbol.symbol.to_json_global_symbol_id(item_0)
                for item_0 in value.ambiguous
            ],
        }
    elif value.kind == "missing":
        return {
            "kind": "missing",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_reference(value: Json) -> Reference:
    """Return one Reference from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "bound":
        return ReferenceBound(
            bound=[
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
                for item_0 in json_array(json_field(object_, "bound"))
            ]
        )
    elif kind == "namespace":
        return ReferenceNamespace(
            namespace=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "namespace")
            )
        )
    elif kind == "projected":
        return ReferenceProjected(
            base=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "base")
            ),
            from_=json_int(json_field(object_, "from")),
        )
    elif kind == "ambiguous":
        return ReferenceAmbiguous(
            ambiguous=[
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(item_0)
                for item_0 in json_array(json_field(object_, "ambiguous"))
            ]
        )
    elif kind == "missing":
        return ReferenceMissing()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "ReferenceTable",
    "encode_reference_table",
    "decode_reference_table",
    "to_json_reference_table",
    "from_json_reference_table",
    "Reference",
    "encode_reference",
    "decode_reference",
    "to_json_reference",
    "from_json_reference",
    "ReferenceBound",
    "ReferenceNamespace",
    "ReferenceProjected",
    "ReferenceAmbiguous",
    "ReferenceMissing",
]
