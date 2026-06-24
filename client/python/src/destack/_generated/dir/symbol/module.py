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

import destack._generated.core.string
import destack._generated.dir.tree.node
import destack._generated.source.file.model.loader
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class ModuleEdge:
    """One resolved module import edge."""

    # the DIR node that declared the dependency
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the static import specifier
    specifier: destack._generated.core.string.StringId
    # the module import relation
    relation: ModuleRelation
    # the loader override selected for the import
    loader: destack._generated.source.file.model.loader.Loader | None
    # the resolved module
    target: destack._generated.source.file.model.module.ModuleId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_edge(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleEdge:
        """Decode one ModuleEdge."""
        return decode_module_edge(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_edge(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleEdge:
        """Return one ModuleEdge from one JSON value."""
        return from_json_module_edge(value)


def encode_module_edge(writer: BinaryWriter, value: ModuleEdge) -> None:
    """Encode one ModuleEdge."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.core.string.encode_string_id(writer, value.specifier)
    encode_module_relation(writer, value.relation)
    if value.loader is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.loader.encode_loader(writer, value.loader)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.target
        )


def decode_module_edge(reader: BinaryReader) -> ModuleEdge:
    """Decode one ModuleEdge."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    specifier = destack._generated.core.string.decode_string_id(reader)
    relation = decode_module_relation(reader)
    loader = reader.read_option(
        lambda: destack._generated.source.file.model.loader.decode_loader(reader)
    )
    target = reader.read_option(
        lambda: destack._generated.source.file.model.module.decode_module_id(reader)
    )

    return ModuleEdge(
        source=source,
        specifier=specifier,
        relation=relation,
        loader=loader,
        target=target,
    )


def to_json_module_edge(value: ModuleEdge) -> Json:
    """Return one JSON value for one ModuleEdge."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "specifier": destack._generated.core.string.to_json_string_id(value.specifier),
        "relation": to_json_module_relation(value.relation),
        **(
            {}
            if value.loader is None
            else {
                "loader": destack._generated.source.file.model.loader.to_json_loader(
                    value.loader
                )
            }
        ),
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


def from_json_module_edge(value: Json) -> ModuleEdge:
    """Return one ModuleEdge from one JSON value."""
    object_ = json_object(value)

    return ModuleEdge(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        specifier=destack._generated.core.string.from_json_string_id(
            json_field(object_, "specifier")
        ),
        relation=from_json_module_relation(json_field(object_, "relation")),
        loader=json_optional(
            object_,
            "loader",
            lambda value: destack._generated.source.file.model.loader.from_json_loader(
                value
            ),
        ),
        target=json_optional(
            object_,
            "target",
            lambda value: (
                destack._generated.source.file.model.module.from_json_module_id(value)
            ),
        ),
    )


"""The relation declared by a resolved module import edge."""
ModuleRelation: typing.TypeAlias = typing.Literal["import"] | typing.Literal["reExport"]


def encode_module_relation(writer: BinaryWriter, value: ModuleRelation) -> None:
    """Encode one ModuleRelation."""
    if value == "import":
        writer.write_unsigned(0)
    elif value == "reExport":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_module_relation(reader: BinaryReader) -> ModuleRelation:
    """Decode one ModuleRelation."""
    variant = reader.read_number()

    if variant == 0:
        return "import"
    elif variant == 1:
        return "reExport"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_module_relation(value: ModuleRelation) -> Json:
    """Return one JSON value for one ModuleRelation."""
    return value


def from_json_module_relation(value: Json) -> ModuleRelation:
    """Return one ModuleRelation from one JSON value."""
    variant = json_string(value)

    if variant == "import":
        return "import"
    elif variant == "reExport":
        return "reExport"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "ModuleEdge",
    "encode_module_edge",
    "decode_module_edge",
    "to_json_module_edge",
    "from_json_module_edge",
    "ModuleRelation",
    "encode_module_relation",
    "decode_module_relation",
    "to_json_module_relation",
    "from_json_module_relation",
]
