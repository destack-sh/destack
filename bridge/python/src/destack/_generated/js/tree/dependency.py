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

import destack._generated.core.string
import destack._generated.js.tree.key
import destack._generated.js.tree.node

"""The source form of one dependency item."""
DependencyForm: typing.TypeAlias = typing.Literal["type"] | typing.Literal["plain"]


def encode_dependency_form(writer: BinaryWriter, value: DependencyForm) -> None:
    """Encode one DependencyForm."""
    if value == "type":
        writer.write_unsigned(0)
    elif value == "plain":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency_form(reader: BinaryReader) -> DependencyForm:
    """Decode one DependencyForm."""
    variant = reader.read_number()

    if variant == 0:
        return "type"
    elif variant == 1:
        return "plain"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency_form(value: DependencyForm) -> Json:
    """Return one JSON value for one DependencyForm."""
    return value


def from_json_dependency_form(value: Json) -> DependencyForm:
    """Return one DependencyForm from one JSON value."""
    variant = json_string(value)

    if variant == "type":
        return "type"
    elif variant == "plain":
        return "plain"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""How one dependency item binds into the local module or export surface."""
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


@dataclass(frozen=True, slots=True)
class DependencyItem:
    """One dependency binding in an import or export clause."""

    # how the item binds
    binding: DependencyBinding
    # the source form of the item, when specified
    form: DependencyForm | None
    # the name of the item (like `foo` in `foo as bar`)
    name: destack._generated.js.tree.key.Name | None
    # the alias to use for the item (like `bar` in `foo as bar`)
    alias: destack._generated.core.string.StringId | None
    # the value of the item (for `export = foo` style exports)
    value: destack._generated.js.tree.node.LocalNodeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DependencyItem:
        """Decode one DependencyItem."""
        return decode_dependency_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency_item(self)

    @classmethod
    def from_json(cls, value: Json) -> DependencyItem:
        """Return one DependencyItem from one JSON value."""
        return from_json_dependency_item(value)


def encode_dependency_item(writer: BinaryWriter, value: DependencyItem) -> None:
    """Encode one DependencyItem."""
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
        destack._generated.js.tree.key.encode_name(writer, value.name)
    if value.alias is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.alias)
    if value.value is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.js.tree.node.encode_local_node_id(writer, value.value)


def decode_dependency_item(reader: BinaryReader) -> DependencyItem:
    """Decode one DependencyItem."""
    binding = decode_dependency_binding(reader)
    form = reader.read_option(lambda: decode_dependency_form(reader))
    name = reader.read_option(
        lambda: destack._generated.js.tree.key.decode_name(reader)
    )
    alias = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    value_ = reader.read_option(
        lambda: destack._generated.js.tree.node.decode_local_node_id(reader)
    )

    return DependencyItem(
        binding=binding,
        form=form,
        name=name,
        alias=alias,
        value=value_,
    )


def to_json_dependency_item(value: DependencyItem) -> Json:
    """Return one JSON value for one DependencyItem."""
    return {
        "binding": to_json_dependency_binding(value.binding),
        **({} if value.form is None else {"form": to_json_dependency_form(value.form)}),
        **(
            {}
            if value.name is None
            else {"name": destack._generated.js.tree.key.to_json_name(value.name)}
        ),
        **(
            {}
            if value.alias is None
            else {
                "alias": destack._generated.core.string.to_json_string_id(value.alias)
            }
        ),
        **(
            {}
            if value.value is None
            else {
                "value": destack._generated.js.tree.node.to_json_local_node_id(
                    value.value
                )
            }
        ),
    }


def from_json_dependency_item(value: Json) -> DependencyItem:
    """Return one DependencyItem from one JSON value."""
    object_ = json_object(value)

    return DependencyItem(
        binding=from_json_dependency_binding(json_field(object_, "binding")),
        form=json_optional(
            object_, "form", lambda value: from_json_dependency_form(value)
        ),
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.js.tree.key.from_json_name(value),
        ),
        alias=json_optional(
            object_,
            "alias",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        value=json_optional(
            object_,
            "value",
            lambda value: destack._generated.js.tree.node.from_json_local_node_id(
                value
            ),
        ),
    )


__all__ = [
    "DependencyForm",
    "encode_dependency_form",
    "decode_dependency_form",
    "to_json_dependency_form",
    "from_json_dependency_form",
    "DependencyBinding",
    "encode_dependency_binding",
    "decode_dependency_binding",
    "to_json_dependency_binding",
    "from_json_dependency_binding",
    "DependencyItem",
    "encode_dependency_item",
    "decode_dependency_item",
    "to_json_dependency_item",
    "from_json_dependency_item",
]
