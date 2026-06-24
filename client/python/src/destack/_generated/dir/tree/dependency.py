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

from destack._impl.dir.tree.dependency import (
    DependencyItemImpl,
)

import destack._generated.core.string
import destack._generated.dir.tree.key
import destack._generated.dir.tree.node

"""The source form of one dependency declaration."""
DependencyForm: typing.TypeAlias = typing.Literal["plain"] | typing.Literal["type"]


def encode_dependency_form(writer: BinaryWriter, value: DependencyForm) -> None:
    """Encode one DependencyForm."""
    if value == "plain":
        writer.write_unsigned(0)
    elif value == "type":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency_form(reader: BinaryReader) -> DependencyForm:
    """Decode one DependencyForm."""
    variant = reader.read_number()

    if variant == 0:
        return "plain"
    elif variant == 1:
        return "type"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency_form(value: DependencyForm) -> Json:
    """Return one JSON value for one DependencyForm."""
    return value


def from_json_dependency_form(value: Json) -> DependencyForm:
    """Return one DependencyForm from one JSON value."""
    variant = json_string(value)

    if variant == "plain":
        return "plain"
    elif variant == "type":
        return "type"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""The export kind of a declaration or binding."""
ExportKind: typing.TypeAlias = typing.Literal["named"] | typing.Literal["default"]


def encode_export_kind(writer: BinaryWriter, value: ExportKind) -> None:
    """Encode one ExportKind."""
    if value == "named":
        writer.write_unsigned(0)
    elif value == "default":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_export_kind(reader: BinaryReader) -> ExportKind:
    """Decode one ExportKind."""
    variant = reader.read_number()

    if variant == 0:
        return "named"
    elif variant == 1:
        return "default"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_export_kind(value: ExportKind) -> Json:
    """Return one JSON value for one ExportKind."""
    return value


def from_json_export_kind(value: Json) -> ExportKind:
    """Return one ExportKind from one JSON value."""
    variant = json_string(value)

    if variant == "named":
        return "named"
    elif variant == "default":
        return "default"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class DependencyItemBinding(DependencyItemImpl):
    """One valid dependency binding."""

    # how the item binds into the local module
    binding: DependencyBinding
    # the source form of the item, when specified
    form: DependencyForm | None
    # the name of the item (like `foo` in `foo as bar`, None if default)
    name: destack._generated.dir.tree.key.Name | None
    # the alias to use for the item (like `bar` in `foo as bar`)
    alias: destack._generated.core.string.StringId | None
    # the value of the item (for namespace exports)
    value: destack._generated.dir.tree.node.LocalNodeId | None
    kind: typing.Literal["binding"] = "binding"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency_item(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency_item(self)


@dataclass(frozen=True, slots=True)
class DependencyItemError(DependencyItemImpl):
    """One malformed dependency item slot."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency_item(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency_item(self)


"""A dependency item imports or exports one binding from a target."""
DependencyItem: typing.TypeAlias = DependencyItemBinding | DependencyItemError


def encode_dependency_item(writer: BinaryWriter, value: DependencyItem) -> None:
    """Encode one DependencyItem."""
    if value.kind == "binding":
        writer.write_unsigned(0)
        encode_dependency_binding(writer, value.binding)
        if value.form is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_dependency_form(writer, value.form)
        if value.name is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.key.encode_name(writer, value.name)
        if value.alias is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.core.string.encode_string_id(writer, value.alias)
        if value.value is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.dir.tree.node.encode_local_node_id(writer, value.value)
    elif value.kind == "error":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency_item(reader: BinaryReader) -> DependencyItem:
    """Decode one DependencyItem."""
    variant = reader.read_number()

    if variant == 0:
        binding = decode_dependency_binding(reader)
        form = reader.read_option(lambda: decode_dependency_form(reader))
        name = reader.read_option(
            lambda: destack._generated.dir.tree.key.decode_name(reader)
        )
        alias = reader.read_option(
            lambda: destack._generated.core.string.decode_string_id(reader)
        )
        value_ = reader.read_option(
            lambda: destack._generated.dir.tree.node.decode_local_node_id(reader)
        )

        return DependencyItemBinding(
            binding=binding,
            form=form,
            name=name,
            alias=alias,
            value=value_,
        )
    elif variant == 1:
        return DependencyItemError()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency_item(value: DependencyItem) -> Json:
    """Return one JSON value for one DependencyItem."""
    if value.kind == "binding":
        return {
            "kind": "binding",
            "binding": to_json_dependency_binding(value.binding),
            **(
                {}
                if value.form is None
                else {"form": to_json_dependency_form(value.form)}
            ),
            **(
                {}
                if value.name is None
                else {"name": destack._generated.dir.tree.key.to_json_name(value.name)}
            ),
            **(
                {}
                if value.alias is None
                else {
                    "alias": destack._generated.core.string.to_json_string_id(
                        value.alias
                    )
                }
            ),
            **(
                {}
                if value.value is None
                else {
                    "value": destack._generated.dir.tree.node.to_json_local_node_id(
                        value.value
                    )
                }
            ),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_dependency_item(value: Json) -> DependencyItem:
    """Return one DependencyItem from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "binding":
        return DependencyItemBinding(
            binding=from_json_dependency_binding(json_field(object_, "binding")),
            form=json_optional(
                object_, "form", lambda value: from_json_dependency_form(value)
            ),
            name=json_optional(
                object_,
                "name",
                lambda value: destack._generated.dir.tree.key.from_json_name(value),
            ),
            alias=json_optional(
                object_,
                "alias",
                lambda value: destack._generated.core.string.from_json_string_id(value),
            ),
            value=json_optional(
                object_,
                "value",
                lambda value: destack._generated.dir.tree.node.from_json_local_node_id(
                    value
                ),
            ),
        )
    elif kind == "error":
        return DependencyItemError()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""How one dependency item binds into the local module."""
DependencyBinding: typing.TypeAlias = (
    typing.Literal["named"] | typing.Literal["default"] | typing.Literal["namespace"]
)


def encode_dependency_binding(writer: BinaryWriter, value: DependencyBinding) -> None:
    """Encode one DependencyBinding."""
    if value == "named":
        writer.write_unsigned(0)
    elif value == "default":
        writer.write_unsigned(1)
    elif value == "namespace":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency_binding(reader: BinaryReader) -> DependencyBinding:
    """Decode one DependencyBinding."""
    variant = reader.read_number()

    if variant == 0:
        return "named"
    elif variant == 1:
        return "default"
    elif variant == 2:
        return "namespace"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency_binding(value: DependencyBinding) -> Json:
    """Return one JSON value for one DependencyBinding."""
    return value


def from_json_dependency_binding(value: Json) -> DependencyBinding:
    """Return one DependencyBinding from one JSON value."""
    variant = json_string(value)

    if variant == "named":
        return "named"
    elif variant == "default":
        return "default"
    elif variant == "namespace":
        return "namespace"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "DependencyForm",
    "encode_dependency_form",
    "decode_dependency_form",
    "to_json_dependency_form",
    "from_json_dependency_form",
    "ExportKind",
    "encode_export_kind",
    "decode_export_kind",
    "to_json_export_kind",
    "from_json_export_kind",
    "DependencyItem",
    "encode_dependency_item",
    "decode_dependency_item",
    "to_json_dependency_item",
    "from_json_dependency_item",
    "DependencyItemBinding",
    "DependencyItemError",
    "DependencyBinding",
    "encode_dependency_binding",
    "decode_dependency_binding",
    "to_json_dependency_binding",
    "from_json_dependency_binding",
]
