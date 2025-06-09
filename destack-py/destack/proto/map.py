from collections.abc import Collection, Sequence
from itertools import chain
from typing import TYPE_CHECKING, Any, cast

from destack.language import (
    BuiltinEnum,
    BuiltinObjectBase,
    EnumType,
    Node,
    NodeType,
    PrimitiveType,
    ScalarType,
    StructType,
    TypeCardinality,
)
from destack.language.registry import (
    ENUM_CLASS_BY_TYPE,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    get_builtin_class,
)
from destack.utils.string import Casing, to_casing

from .core import (
    ProtoEnum,
    ProtoEnumValue,
    ProtoField,
    ProtoFieldType,
    ProtoMessage,
    ProtoSchema,
    ProtoThing,
)

if TYPE_CHECKING:
    from destack.language import Property

#
# Map Destack types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

VARIABLE_PROPERTY_OFFSET = 17000

PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, ProtoFieldType] = {
    PrimitiveType.BOOLEAN: ProtoFieldType.BOOL,
    PrimitiveType.INT16: ProtoFieldType.INT32,
    PrimitiveType.INT32: ProtoFieldType.INT32,
    PrimitiveType.INT64: ProtoFieldType.INT64,
    PrimitiveType.FLOAT32: ProtoFieldType.FLOAT,
    PrimitiveType.FLOAT64: ProtoFieldType.DOUBLE,
    PrimitiveType.STRING: ProtoFieldType.STRING,
    PrimitiveType.BYTES: ProtoFieldType.BYTES,
    PrimitiveType.DATETIME: ProtoFieldType.TIMESTAMP,
    PrimitiveType.DATE: ProtoFieldType.DATE,
    PrimitiveType.DURATION: ProtoFieldType.DURATION,
    PrimitiveType.UUID: ProtoFieldType.STRING,  # see https://stackoverflow.com/q/36344826/3375858
    PrimitiveType.JSON: ProtoFieldType.VALUE,
}


def _map_property_to_proto_field(
    prop: "Property", cache: dict[type[BuiltinObjectBase] | type[BuiltinEnum], ProtoThing]
) -> ProtoField:
    assert prop.id == 1 or prop.is_wired, f"not a wired property: {prop!r}"
    assert isinstance(prop.id, int), f"invalid id: {prop!r}"

    # base field
    if prop.scalar_type == ScalarType.NODE_REFERENCE:
        field = ProtoField(
            id=prop.id,
            name=prop.name,
            type="NodeReferenceData",
            optional=prop.is_optional,
            repeated=prop.cardinality == TypeCardinality.LIST,
        )
    elif prop.scalar_type == ScalarType.STRUCT:
        assert prop.struct_type is not None, f"invalid struct: {prop!r}"
        struct_cls = STRUCT_CLASS_BY_TYPE[prop.struct_type]
        proto_t = _map_object_type_to_proto(struct_cls, cache)
        assert isinstance(proto_t, ProtoMessage), f"unexpected property type: {proto_t!r}"
        field = ProtoField(
            id=prop.id,
            name=prop.name,
            type=proto_t,
            optional=prop.is_optional,
            repeated=prop.cardinality == TypeCardinality.LIST,
        )
    elif prop.scalar_type == ScalarType.ENUM:
        assert prop.enum_type is not None, f"invalid enum: {prop!r}"
        enum_cls = ENUM_CLASS_BY_TYPE[prop.enum_type]
        proto_t = _map_object_type_to_proto(enum_cls, cache)
        assert isinstance(proto_t, ProtoEnum), f"unexpected property type: {proto_t!r}"
        field = ProtoField(
            id=prop.id,
            name=prop.name,
            type=proto_t,
            optional=prop.is_optional,
            repeated=prop.cardinality == TypeCardinality.LIST,
        )
    elif prop.scalar_type == ScalarType.PRIMITIVE:
        assert prop.primitive_type is not None, f"invalid primitive: {prop!r}"
        field_type = PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE.get(prop.primitive_type)
        assert field_type is not None, f"invalid primitive type: {prop.primitive_type!r}"
        field = ProtoField(
            id=prop.id,
            name=prop.name,
            type=field_type,
            optional=prop.is_optional,
            repeated=prop.cardinality == TypeCardinality.LIST,
        )
    else:
        raise TypeError(f"cannot map to proto type: {prop!r}")

    # map
    if prop.cardinality == TypeCardinality.MAP:
        assert prop.key_type is not None, f"invalid map: {prop!r}"
        assert prop.key_type.cardinality == TypeCardinality.SCALAR, (
            f"invalid key: {prop.key_type!r}"
        )
        value_field = field
        if prop.key_type.scalar_type == ScalarType.ENUM:
            key_field = ProtoFieldType.INT32
        elif prop.key_type.scalar_type == ScalarType.PRIMITIVE:
            assert prop.key_type.primitive_type is not None, f"invalid key type: {prop.key_type!r}"
            key_field = PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE.get(prop.key_type.primitive_type)
            assert key_field in (
                ProtoFieldType.INT32,
                ProtoFieldType.INT64,
                ProtoFieldType.STRING,
            ), f"invalid key type: {prop.key_type!r}"
        else:
            raise TypeError(f"invalid key type: {prop.key_type!r}")
        wrapper_field = ProtoField(
            id=prop.id,
            name=field.name,
            type=ProtoFieldType.MAP,
            key_type=key_field,
            value_type=value_field.type,
        )
        return wrapper_field

    # variable
    if prop.is_variable:
        field.optional = False  # proto fields in oneof cannot have labels
        variable_field = ProtoField(
            id=prop.id + VARIABLE_PROPERTY_OFFSET,
            name=f"{prop.name}_variable",
            type="VariableData",
            repeated=prop.cardinality == TypeCardinality.LIST,
        )
        wrapper_field = ProtoField(
            id=prop.id,
            name=field.name,
            type=ProtoFieldType.ONE_OF,
            sub_fields=(field, variable_field),
        )
        field.name = f"{field.name}_value"
        return wrapper_field

    return field


def _map_builtin_object_to_proto_message(
    cls: type[BuiltinObjectBase],
    cache: dict[type[BuiltinObjectBase] | type[BuiltinEnum], ProtoThing],
    alias: str | None = None,
    properties: Sequence["Property"] | None = None,
) -> ProtoMessage:
    if cls in cache:
        message = cache[cls]
        assert isinstance(message, ProtoMessage), f"unexpected cached {message!r} for {cls!r}"
        return message
    message = ProtoMessage(
        name=alias or cls.__name__, reserved_names=[], reserved_ids=[], fields=[]
    )
    doc = cls.__doc__ if issubclass(cls, Node) else None
    message.comment = (doc or "").strip()
    cache[cls] = message  # to solve recursive references
    for prop in properties if properties is not None else cls.__properties__.values():
        if not prop.is_wired or prop.ptr_prop is not None:
            continue
        field = _map_property_to_proto_field(prop, cache)
        message.fields.append(field)
    message.fields.sort(key=lambda f: cast(int, f.id))
    return message


def _map_builtin_enum_to_proto_enum(
    destack_t: type[BuiltinEnum],
    alias: str | None = None,
) -> ProtoEnum:
    assert issubclass(destack_t, BuiltinEnum), f"invalid enum: {destack_t!r}"
    enum_prefix = to_casing(alias or destack_t.__name__, Casing.ALL_CAPS) + "_"
    enum_values = [
        ProtoEnumValue(id=member.id, name=enum_prefix + member.name) for member in destack_t
    ]
    # add unset if not already present
    if not any(v.id == 0 for v in enum_values):
        enum_values = [ProtoEnumValue(id=0, name=enum_prefix + "UNSPECIFIED"), *enum_values]
    has_duplicates = len(enum_values) != len({v.id for v in enum_values})
    proto_t = ProtoEnum(
        name=alias or destack_t.__name__, values=enum_values, allow_alias=has_duplicates
    )
    if destack_t.__doc__:
        proto_t.comment = destack_t.__doc__.strip()
    return proto_t


def _map_object_type_to_proto(
    destack_cls: type[BuiltinObjectBase] | type[BuiltinEnum],
    cache: dict[type[BuiltinObjectBase] | type[BuiltinEnum], ProtoThing],
    alias: str | None = None,
) -> ProtoThing:
    """Maps a Destack type to a Proto type. If not yet mapped, adds it to the cache."""
    from destack.language import BuiltinObjectBase

    if destack_cls in cache:
        return cache[destack_cls]
    if issubclass(destack_cls, BuiltinObjectBase):
        ret = _map_builtin_object_to_proto_message(destack_cls, cache, alias=alias)
    elif issubclass(destack_cls, BuiltinEnum):
        ret = _map_builtin_enum_to_proto_enum(destack_cls, alias=alias)
    else:
        raise TypeError(f"invalid destack type: {destack_cls!r}")
    cache[destack_cls] = ret
    return ret


def generate_proto_schema(
    name: str,
    unions: dict[str, tuple[str, Collection[EnumType | NodeType | StructType]]],
    extras: list[ProtoEnum | ProtoMessage],
    message_postfix: str,
) -> ProtoSchema:
    # walk all destack types to populate the cache
    cache: dict[type[BuiltinObjectBase] | type[BuiltinEnum], ProtoThing] = {}
    for destack_cls in chain(
        NODE_CLASS_BY_TYPE.values(), STRUCT_CLASS_BY_TYPE.values(), ENUM_CLASS_BY_TYPE.values()
    ):
        cache[destack_cls] = _map_object_type_to_proto(destack_cls, cache)

    # collect all proto types
    proto_types: list[ProtoEnum | ProtoMessage] = []
    for proto_type in cache.values():
        if isinstance(proto_type, (ProtoEnum, ProtoMessage)):
            proto_types.append(proto_type)
    # sort: enum -> struct -> node (and by name within each group)
    proto_types.sort(key=lambda t: (isinstance(t, ProtoMessage), isinstance(t, ProtoEnum), t.name))

    # add custom union types
    for union_name, (wrapper_field_name, unioned_types) in unions.items():
        sub_fields = [
            ProtoField(
                id=t.value,
                name=to_casing(t.name, Casing.SNAKE),
                type=cast(Any, _map_object_type_to_proto(get_builtin_class(t), cache)),
            )
            for t in unioned_types
        ]
        wrapper_field = ProtoField(
            id=None, name=wrapper_field_name, type=ProtoFieldType.ONE_OF, sub_fields=sub_fields
        )
        wrapper_message = ProtoMessage(
            name=union_name, reserved_names=[], reserved_ids=[], fields=[wrapper_field]
        )
        proto_types.append(wrapper_message)
    proto_types.extend(extras)

    # apply postfix to messages
    if message_postfix:
        for proto_type in proto_types:
            if isinstance(proto_type, ProtoMessage) and proto_type not in extras:
                proto_type.name += message_postfix

    return ProtoSchema.from_types(name, proto_types)
