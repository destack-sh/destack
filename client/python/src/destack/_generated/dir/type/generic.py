# generated client target, do not edit

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
    json_bool,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.tree.property
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class GlobalGenericParameterId:
    """Global generic parameter id across modules."""

    # the module id of the global generic parameter
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local generic parameter id
    local_id: LocalGenericParameterId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_generic_parameter_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalGenericParameterId:
        """Decode one GlobalGenericParameterId."""
        return decode_global_generic_parameter_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_generic_parameter_id(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalGenericParameterId:
        """Return one GlobalGenericParameterId from one JSON value."""
        return from_json_global_generic_parameter_id(value)


def encode_global_generic_parameter_id(
    writer: BinaryWriter, value: GlobalGenericParameterId
) -> None:
    """Encode one GlobalGenericParameterId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_generic_parameter_id(writer, value.local_id)


def decode_global_generic_parameter_id(
    reader: BinaryReader,
) -> GlobalGenericParameterId:
    """Decode one GlobalGenericParameterId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = decode_local_generic_parameter_id(reader)

    return GlobalGenericParameterId(
        module_id=module_id,
        local_id=local_id,
    )


def to_json_global_generic_parameter_id(value: GlobalGenericParameterId) -> Json:
    """Return one JSON value for one GlobalGenericParameterId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": to_json_local_generic_parameter_id(value.local_id),
    }


def from_json_global_generic_parameter_id(value: Json) -> GlobalGenericParameterId:
    """Return one GlobalGenericParameterId from one JSON value."""
    object_ = json_object(value)

    return GlobalGenericParameterId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=from_json_local_generic_parameter_id(json_field(object_, "localId")),
    )


"""Unique identifier for generic parameters."""
LocalGenericParameterId: typing.TypeAlias = int


def encode_local_generic_parameter_id(
    writer: BinaryWriter, value: LocalGenericParameterId
) -> None:
    """Encode one LocalGenericParameterId."""
    writer.write_unsigned(value)


def decode_local_generic_parameter_id(reader: BinaryReader) -> LocalGenericParameterId:
    """Decode one LocalGenericParameterId."""
    return reader.read_number()


def to_json_local_generic_parameter_id(value: LocalGenericParameterId) -> Json:
    """Return one JSON value for one LocalGenericParameterId."""
    return value


def from_json_local_generic_parameter_id(value: Json) -> LocalGenericParameterId:
    """Return one LocalGenericParameterId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class GenericTemplate:
    """One declaration of generic parameters."""

    # the source node that declares this template
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the declaration symbol this template belongs to
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId | None
    # the immediately enclosing generic template
    parent: LocalGenericTemplateId | None
    # the generic parameters in declaration order
    parameters: Sequence[LocalGenericParameterId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_template(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericTemplate:
        """Decode one GenericTemplate."""
        return decode_generic_template(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_template(self)

    @classmethod
    def from_json(cls, value: Json) -> GenericTemplate:
        """Return one GenericTemplate from one JSON value."""
        return from_json_generic_template(value)


def encode_generic_template(writer: BinaryWriter, value: GenericTemplate) -> None:
    """Encode one GenericTemplate."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    if value.symbol is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    if value.parent is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_local_generic_template_id(writer, value.parent)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        encode_local_generic_parameter_id(writer, item_value_parameters_0)


def decode_generic_template(reader: BinaryReader) -> GenericTemplate:
    """Decode one GenericTemplate."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    symbol = reader.read_option(
        lambda: destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    )
    parent = reader.read_option(lambda: decode_local_generic_template_id(reader))
    parameters = [
        decode_local_generic_parameter_id(reader) for _ in range(reader.read_number())
    ]

    return GenericTemplate(
        source=source,
        symbol=symbol,
        parent=parent,
        parameters=parameters,
    )


def to_json_generic_template(value: GenericTemplate) -> Json:
    """Return one JSON value for one GenericTemplate."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        **(
            {}
            if value.symbol is None
            else {
                "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                    value.symbol
                )
            }
        ),
        **(
            {}
            if value.parent is None
            else {"parent": to_json_local_generic_template_id(value.parent)}
        ),
        "parameters": [
            to_json_local_generic_parameter_id(item_0) for item_0 in value.parameters
        ],
    }


def from_json_generic_template(value: Json) -> GenericTemplate:
    """Return one GenericTemplate from one JSON value."""
    object_ = json_object(value)

    return GenericTemplate(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        symbol=json_optional(
            object_,
            "symbol",
            lambda value: (
                destack._generated.dir.symbol.symbol.from_json_global_symbol_id(value)
            ),
        ),
        parent=json_optional(
            object_, "parent", lambda value: from_json_local_generic_template_id(value)
        ),
        parameters=[
            from_json_local_generic_parameter_id(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
    )


"""Unique identifier for generic templates."""
LocalGenericTemplateId: typing.TypeAlias = int


def encode_local_generic_template_id(
    writer: BinaryWriter, value: LocalGenericTemplateId
) -> None:
    """Encode one LocalGenericTemplateId."""
    writer.write_unsigned(value)


def decode_local_generic_template_id(reader: BinaryReader) -> LocalGenericTemplateId:
    """Decode one LocalGenericTemplateId."""
    return reader.read_number()


def to_json_local_generic_template_id(value: LocalGenericTemplateId) -> Json:
    """Return one JSON value for one LocalGenericTemplateId."""
    return value


def from_json_local_generic_template_id(value: Json) -> LocalGenericTemplateId:
    """Return one LocalGenericTemplateId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class GenericParameterBinding:
    """One declaration-side generic parameter."""

    # the generic template that owns this parameter
    template: LocalGenericTemplateId
    # the parameter key
    key: GenericParameterKey
    # the parameter variance, rejected on comptime parameters
    variance: destack._generated.dir.tree.property.VarianceModifier | None
    # the optional constraint
    constraint: destack._generated.dir.type.type.GlobalTypeId | None
    # the optional default
    default: destack._generated.dir.type.type.GlobalTypeId | None
    # the parameter origin
    origin: GenericParameterOrigin
    # whether the parameter captures remaining arguments
    is_variadic: bool
    # whether arguments must solve to singleton types
    is_comptime: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter_binding(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericParameterBinding:
        """Decode one GenericParameterBinding."""
        return decode_generic_parameter_binding(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter_binding(self)

    @classmethod
    def from_json(cls, value: Json) -> GenericParameterBinding:
        """Return one GenericParameterBinding from one JSON value."""
        return from_json_generic_parameter_binding(value)


def encode_generic_parameter_binding(
    writer: BinaryWriter, value: GenericParameterBinding
) -> None:
    """Encode one GenericParameterBinding."""
    encode_local_generic_template_id(writer, value.template)
    encode_generic_parameter_key(writer, value.key)
    if value.variance is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.property.encode_variance_modifier(
            writer, value.variance
        )
    if value.constraint is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.constraint)
    if value.default is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.type.encode_global_type_id(writer, value.default)
    encode_generic_parameter_origin(writer, value.origin)
    writer.write_bool(value.is_variadic)
    writer.write_bool(value.is_comptime)


def decode_generic_parameter_binding(reader: BinaryReader) -> GenericParameterBinding:
    """Decode one GenericParameterBinding."""
    template = decode_local_generic_template_id(reader)
    key = decode_generic_parameter_key(reader)
    variance = reader.read_option(
        lambda: destack._generated.dir.tree.property.decode_variance_modifier(reader)
    )
    constraint = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    default = reader.read_option(
        lambda: destack._generated.dir.type.type.decode_global_type_id(reader)
    )
    origin = decode_generic_parameter_origin(reader)
    is_variadic = reader.read_bool()
    is_comptime = reader.read_bool()

    return GenericParameterBinding(
        template=template,
        key=key,
        variance=variance,
        constraint=constraint,
        default=default,
        origin=origin,
        is_variadic=is_variadic,
        is_comptime=is_comptime,
    )


def to_json_generic_parameter_binding(value: GenericParameterBinding) -> Json:
    """Return one JSON value for one GenericParameterBinding."""
    return {
        "template": to_json_local_generic_template_id(value.template),
        "key": to_json_generic_parameter_key(value.key),
        **(
            {}
            if value.variance is None
            else {
                "variance": destack._generated.dir.tree.property.to_json_variance_modifier(
                    value.variance
                )
            }
        ),
        **(
            {}
            if value.constraint is None
            else {
                "constraint": destack._generated.dir.type.type.to_json_global_type_id(
                    value.constraint
                )
            }
        ),
        **(
            {}
            if value.default is None
            else {
                "default": destack._generated.dir.type.type.to_json_global_type_id(
                    value.default
                )
            }
        ),
        "origin": to_json_generic_parameter_origin(value.origin),
        "isVariadic": value.is_variadic,
        "isComptime": value.is_comptime,
    }


def from_json_generic_parameter_binding(value: Json) -> GenericParameterBinding:
    """Return one GenericParameterBinding from one JSON value."""
    object_ = json_object(value)

    return GenericParameterBinding(
        template=from_json_local_generic_template_id(json_field(object_, "template")),
        key=from_json_generic_parameter_key(json_field(object_, "key")),
        variance=json_optional(
            object_,
            "variance",
            lambda value: (
                destack._generated.dir.tree.property.from_json_variance_modifier(value)
            ),
        ),
        constraint=json_optional(
            object_,
            "constraint",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        default=json_optional(
            object_,
            "default",
            lambda value: destack._generated.dir.type.type.from_json_global_type_id(
                value
            ),
        ),
        origin=from_json_generic_parameter_origin(json_field(object_, "origin")),
        is_variadic=json_bool(json_field(object_, "isVariadic")),
        is_comptime=json_bool(json_field(object_, "isComptime")),
    )


@dataclass(frozen=True, slots=True)
class GenericParameterKeySymbol:
    """Explicit source symbol, like the `T` in `<T>`."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter_key(self)


@dataclass(frozen=True, slots=True)
class GenericParameterKeyGenerated:
    """Generated checked parameter key, like the induced `T0` or `L0`."""

    generated: destack._generated.core.string.StringId
    kind: typing.Literal["generated"] = "generated"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter_key(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter_key(self)


"""User-visible key of one generic parameter."""
GenericParameterKey: typing.TypeAlias = (
    GenericParameterKeySymbol | GenericParameterKeyGenerated
)


def encode_generic_parameter_key(
    writer: BinaryWriter, value: GenericParameterKey
) -> None:
    """Encode one GenericParameterKey."""
    if value.kind == "symbol":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    elif value.kind == "generated":
        writer.write_unsigned(1)
        destack._generated.core.string.encode_string_id(writer, value.generated)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_parameter_key(reader: BinaryReader) -> GenericParameterKey:
    """Decode one GenericParameterKey."""
    variant = reader.read_number()

    if variant == 0:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return GenericParameterKeySymbol(symbol=symbol)
    elif variant == 1:
        generated = destack._generated.core.string.decode_string_id(reader)

        return GenericParameterKeyGenerated(generated=generated)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_parameter_key(value: GenericParameterKey) -> Json:
    """Return one JSON value for one GenericParameterKey."""
    if value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    elif value.kind == "generated":
        return {
            "kind": "generated",
            "generated": destack._generated.core.string.to_json_string_id(
                value.generated
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_generic_parameter_key(value: Json) -> GenericParameterKey:
    """Return one GenericParameterKey from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "symbol":
        return GenericParameterKeySymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    elif kind == "generated":
        return GenericParameterKeyGenerated(
            generated=destack._generated.core.string.from_json_string_id(
                json_field(object_, "generated")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GenericParameterOriginExplicit:
    """The parameter was written in source, like the `T` in `<T extends Clone>`."""

    kind: typing.Literal["explicit"] = "explicit"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter_origin(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter_origin(self)


@dataclass(frozen=True, slots=True)
class GenericParameterOriginInduced:
    """The parameter was induced by check, like the hidden `T0` a"""

    induced: GenericParameterInduction
    kind: typing.Literal["induced"] = "induced"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_parameter_origin(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_parameter_origin(self)


"""Source that introduced one generic parameter."""
GenericParameterOrigin: typing.TypeAlias = (
    GenericParameterOriginExplicit | GenericParameterOriginInduced
)


def encode_generic_parameter_origin(
    writer: BinaryWriter, value: GenericParameterOrigin
) -> None:
    """Encode one GenericParameterOrigin."""
    if value.kind == "explicit":
        writer.write_unsigned(0)
    elif value.kind == "induced":
        writer.write_unsigned(1)
        encode_generic_parameter_induction(writer, value.induced)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_parameter_origin(reader: BinaryReader) -> GenericParameterOrigin:
    """Decode one GenericParameterOrigin."""
    variant = reader.read_number()

    if variant == 0:
        return GenericParameterOriginExplicit()
    elif variant == 1:
        induced = decode_generic_parameter_induction(reader)

        return GenericParameterOriginInduced(induced=induced)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_parameter_origin(value: GenericParameterOrigin) -> Json:
    """Return one JSON value for one GenericParameterOrigin."""
    if value.kind == "explicit":
        return {
            "kind": "explicit",
        }
    elif value.kind == "induced":
        return {
            "kind": "induced",
            "induced": to_json_generic_parameter_induction(value.induced),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_generic_parameter_origin(value: Json) -> GenericParameterOrigin:
    """Return one GenericParameterOrigin from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "explicit":
        return GenericParameterOriginExplicit()
    elif kind == "induced":
        return GenericParameterOriginInduced(
            induced=from_json_generic_parameter_induction(
                json_field(object_, "induced")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Reason one generic parameter was induced."""
GenericParameterInduction: typing.TypeAlias = (
    typing.Literal["parameterConstraint"]
    | typing.Literal["storageConstraint"]
    | typing.Literal["form"]
    | typing.Literal["comptime"]
)


def encode_generic_parameter_induction(
    writer: BinaryWriter, value: GenericParameterInduction
) -> None:
    """Encode one GenericParameterInduction."""
    if value == "parameterConstraint":
        writer.write_unsigned(0)
    elif value == "storageConstraint":
        writer.write_unsigned(1)
    elif value == "form":
        writer.write_unsigned(2)
    elif value == "comptime":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_generic_parameter_induction(
    reader: BinaryReader,
) -> GenericParameterInduction:
    """Decode one GenericParameterInduction."""
    variant = reader.read_number()

    if variant == 0:
        return "parameterConstraint"
    elif variant == 1:
        return "storageConstraint"
    elif variant == 2:
        return "form"
    elif variant == 3:
        return "comptime"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_generic_parameter_induction(value: GenericParameterInduction) -> Json:
    """Return one JSON value for one GenericParameterInduction."""
    return value


def from_json_generic_parameter_induction(value: Json) -> GenericParameterInduction:
    """Return one GenericParameterInduction from one JSON value."""
    variant = json_string(value)

    if variant == "parameterConstraint":
        return "parameterConstraint"
    elif variant == "storageConstraint":
        return "storageConstraint"
    elif variant == "form":
        return "form"
    elif variant == "comptime":
        return "comptime"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "GlobalGenericParameterId",
    "encode_global_generic_parameter_id",
    "decode_global_generic_parameter_id",
    "to_json_global_generic_parameter_id",
    "from_json_global_generic_parameter_id",
    "LocalGenericParameterId",
    "encode_local_generic_parameter_id",
    "decode_local_generic_parameter_id",
    "to_json_local_generic_parameter_id",
    "from_json_local_generic_parameter_id",
    "GenericTemplate",
    "encode_generic_template",
    "decode_generic_template",
    "to_json_generic_template",
    "from_json_generic_template",
    "LocalGenericTemplateId",
    "encode_local_generic_template_id",
    "decode_local_generic_template_id",
    "to_json_local_generic_template_id",
    "from_json_local_generic_template_id",
    "GenericParameterBinding",
    "encode_generic_parameter_binding",
    "decode_generic_parameter_binding",
    "to_json_generic_parameter_binding",
    "from_json_generic_parameter_binding",
    "GenericParameterKey",
    "encode_generic_parameter_key",
    "decode_generic_parameter_key",
    "to_json_generic_parameter_key",
    "from_json_generic_parameter_key",
    "GenericParameterKeySymbol",
    "GenericParameterKeyGenerated",
    "GenericParameterOrigin",
    "encode_generic_parameter_origin",
    "decode_generic_parameter_origin",
    "to_json_generic_parameter_origin",
    "from_json_generic_parameter_origin",
    "GenericParameterOriginExplicit",
    "GenericParameterOriginInduced",
    "GenericParameterInduction",
    "encode_generic_parameter_induction",
    "decode_generic_parameter_induction",
    "to_json_generic_parameter_induction",
    "from_json_generic_parameter_induction",
]
