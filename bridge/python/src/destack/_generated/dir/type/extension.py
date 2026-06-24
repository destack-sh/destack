# generated bridge target, do not edit

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
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.table.definition
import destack._generated.dir.tree.node
import destack._generated.dir.type.generic
import destack._generated.dir.type.type


@dataclass(frozen=True, slots=True)
class Extension:
    """A resolved extension declaration."""

    # the extension declaration's symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the extension declaration form
    form: ExtensionForm
    # the extension's generic template
    template: destack._generated.dir.type.generic.LocalGenericTemplateId | None
    # the checked receiver target
    target: ExtensionTarget
    # the implemented interfaces
    implements: Sequence[destack._generated.dir.table.definition.NominalHeritage]
    # the checked where clauses that gate this extension
    where_clauses: Sequence[ExtensionWhereClause]
    # the members in declaration order
    members: Sequence[destack._generated.dir.table.definition.DefinitionMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Extension:
        """Decode one Extension."""
        return decode_extension(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension(self)

    @classmethod
    def from_json(cls, value: Json) -> Extension:
        """Return one Extension from one JSON value."""
        return from_json_extension(value)


def encode_extension(writer: BinaryWriter, value: Extension) -> None:
    """Encode one Extension."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    encode_extension_form(writer, value.form)
    if value.template is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_local_generic_template_id(
            writer, value.template
        )
    encode_extension_target(writer, value.target)
    writer.write_unsigned(len(value.implements))
    for item_value_implements_0 in value.implements:
        destack._generated.dir.table.definition.encode_nominal_heritage(
            writer, item_value_implements_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        encode_extension_where_clause(writer, item_value_where_clauses_0)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.table.definition.encode_definition_member(
            writer, item_value_members_0
        )


def decode_extension(reader: BinaryReader) -> Extension:
    """Decode one Extension."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    form = decode_extension_form(reader)
    template = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_local_generic_template_id(
            reader
        )
    )
    target = decode_extension_target(reader)
    implements = [
        destack._generated.dir.table.definition.decode_nominal_heritage(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        decode_extension_where_clause(reader) for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.table.definition.decode_definition_member(reader)
        for _ in range(reader.read_number())
    ]

    return Extension(
        symbol=symbol,
        form=form,
        template=template,
        target=target,
        implements=implements,
        where_clauses=where_clauses,
        members=members,
    )


def to_json_extension(value: Extension) -> Json:
    """Return one JSON value for one Extension."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "form": to_json_extension_form(value.form),
        **(
            {}
            if value.template is None
            else {
                "template": destack._generated.dir.type.generic.to_json_local_generic_template_id(
                    value.template
                )
            }
        ),
        "target": to_json_extension_target(value.target),
        "implements": [
            destack._generated.dir.table.definition.to_json_nominal_heritage(item_0)
            for item_0 in value.implements
        ],
        "whereClauses": [
            to_json_extension_where_clause(item_0) for item_0 in value.where_clauses
        ],
        "members": [
            destack._generated.dir.table.definition.to_json_definition_member(item_0)
            for item_0 in value.members
        ],
    }


def from_json_extension(value: Json) -> Extension:
    """Return one Extension from one JSON value."""
    object_ = json_object(value)

    return Extension(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        form=from_json_extension_form(json_field(object_, "form")),
        template=json_optional(
            object_,
            "template",
            lambda value: (
                destack._generated.dir.type.generic.from_json_local_generic_template_id(
                    value
                )
            ),
        ),
        target=from_json_extension_target(json_field(object_, "target")),
        implements=[
            destack._generated.dir.table.definition.from_json_nominal_heritage(item_0)
            for item_0 in json_array(json_field(object_, "implements"))
        ],
        where_clauses=[
            from_json_extension_where_clause(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        members=[
            destack._generated.dir.table.definition.from_json_definition_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


"""How an extension declaration relates to its target type."""
ExtensionForm: typing.TypeAlias = (
    typing.Literal["inherent"] | typing.Literal["local"] | typing.Literal["named"]
)


def encode_extension_form(writer: BinaryWriter, value: ExtensionForm) -> None:
    """Encode one ExtensionForm."""
    if value == "inherent":
        writer.write_unsigned(0)
    elif value == "local":
        writer.write_unsigned(1)
    elif value == "named":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_extension_form(reader: BinaryReader) -> ExtensionForm:
    """Decode one ExtensionForm."""
    variant = reader.read_number()

    if variant == 0:
        return "inherent"
    elif variant == 1:
        return "local"
    elif variant == 2:
        return "named"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_extension_form(value: ExtensionForm) -> Json:
    """Return one JSON value for one ExtensionForm."""
    return value


def from_json_extension_form(value: Json) -> ExtensionForm:
    """Return one ExtensionForm from one JSON value."""
    variant = json_string(value)

    if variant == "inherent":
        return "inherent"
    elif variant == "local":
        return "local"
    elif variant == "named":
        return "named"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ExtensionTargetNominal:
    """Extension whose receiver type has a nominal root."""

    # the nominal root used for member lookup
    root: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["nominal"] = "nominal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_target(self)


@dataclass(frozen=True, slots=True)
class ExtensionTargetBlanket:
    """Extension over an open receiver type."""

    # the checked receiver type
    ty: destack._generated.dir.type.type.GlobalTypeId
    kind: typing.Literal["blanket"] = "blanket"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_target(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_target(self)


"""Extension lookup target."""
ExtensionTarget: typing.TypeAlias = ExtensionTargetNominal | ExtensionTargetBlanket


def encode_extension_target(writer: BinaryWriter, value: ExtensionTarget) -> None:
    """Encode one ExtensionTarget."""
    if value.kind == "nominal":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.root)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    elif value.kind == "blanket":
        writer.write_unsigned(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)
    else:
        raise SerdeError("unknown enum variant")


def decode_extension_target(reader: BinaryReader) -> ExtensionTarget:
    """Decode one ExtensionTarget."""
    variant = reader.read_number()

    if variant == 0:
        root = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ExtensionTargetNominal(
            root=root,
            ty=ty,
        )
    elif variant == 1:
        ty = destack._generated.dir.type.type.decode_global_type_id(reader)

        return ExtensionTargetBlanket(
            ty=ty,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_extension_target(value: ExtensionTarget) -> Json:
    """Return one JSON value for one ExtensionTarget."""
    if value.kind == "nominal":
        return {
            "kind": "nominal",
            "root": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.root
            ),
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    elif value.kind == "blanket":
        return {
            "kind": "blanket",
            "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_extension_target(value: Json) -> ExtensionTarget:
    """Return one ExtensionTarget from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "nominal":
        return ExtensionTargetNominal(
            root=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "root")
            ),
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    elif kind == "blanket":
        return ExtensionTargetBlanket(
            ty=destack._generated.dir.type.type.from_json_global_type_id(
                json_field(object_, "ty")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ExtensionWhereClause:
    """A checked where clause attached to one extension."""

    # the source where clause node
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the constrained type
    left: destack._generated.dir.type.type.GlobalTypeId
    # the required constraint type
    right: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_extension_where_clause(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExtensionWhereClause:
        """Decode one ExtensionWhereClause."""
        return decode_extension_where_clause(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_extension_where_clause(self)

    @classmethod
    def from_json(cls, value: Json) -> ExtensionWhereClause:
        """Return one ExtensionWhereClause from one JSON value."""
        return from_json_extension_where_clause(value)


def encode_extension_where_clause(
    writer: BinaryWriter, value: ExtensionWhereClause
) -> None:
    """Encode one ExtensionWhereClause."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.left)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.right)


def decode_extension_where_clause(reader: BinaryReader) -> ExtensionWhereClause:
    """Decode one ExtensionWhereClause."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    left = destack._generated.dir.type.type.decode_global_type_id(reader)
    right = destack._generated.dir.type.type.decode_global_type_id(reader)

    return ExtensionWhereClause(
        source=source,
        left=left,
        right=right,
    )


def to_json_extension_where_clause(value: ExtensionWhereClause) -> Json:
    """Return one JSON value for one ExtensionWhereClause."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "left": destack._generated.dir.type.type.to_json_global_type_id(value.left),
        "right": destack._generated.dir.type.type.to_json_global_type_id(value.right),
    }


def from_json_extension_where_clause(value: Json) -> ExtensionWhereClause:
    """Return one ExtensionWhereClause from one JSON value."""
    object_ = json_object(value)

    return ExtensionWhereClause(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        left=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "left")
        ),
        right=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "right")
        ),
    )


__all__ = [
    "Extension",
    "encode_extension",
    "decode_extension",
    "to_json_extension",
    "from_json_extension",
    "ExtensionForm",
    "encode_extension_form",
    "decode_extension_form",
    "to_json_extension_form",
    "from_json_extension_form",
    "ExtensionTarget",
    "encode_extension_target",
    "decode_extension_target",
    "to_json_extension_target",
    "from_json_extension_target",
    "ExtensionTargetNominal",
    "ExtensionTargetBlanket",
    "ExtensionWhereClause",
    "encode_extension_where_clause",
    "decode_extension_where_clause",
    "to_json_extension_where_clause",
    "from_json_extension_where_clause",
]
