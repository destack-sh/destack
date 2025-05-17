import enum
from typing import TYPE_CHECKING, Any, Collection, Sequence, Union, cast

from bench.language import (
    BENCH_CLASS_BY_TYPE,
    FINAL_BENCH_CLASSES,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
    BuiltinObject,
    EnumType,
    Node,
    PrimitiveType,
)
from bench.language.core import BuiltinEnum
from bench.utils.string import Casing, to_casing

from .core import (
    Message,
    ProtoEnum,
    ProtoEnumValue,
    ProtoField,
    ProtoFieldType,
    ProtoSchema,
    ProtoThing,
)

if TYPE_CHECKING:
    from bench.language import Property

#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

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

_ThingType = type[Union["BuiltinObject", "Property", BuiltinEnum, enum.IntFlag]]


def map_bench_property_to_proto(
    prop: "Property", cache: dict[_ThingType, ProtoThing]
) -> ProtoField | Sequence[ProtoField]:
    assert prop.id == 1 or not prop.is_ephemeral, f"shouldn't map runtime property: {prop!r}"
    assert isinstance(prop.id, int), f"stored properties need an id: {prop!r}"
    if prop.is_node_reference:
        return ProtoField(
            id=prop.id,
            name=prop.name,
            type="NodeReferenceData",
            optional=prop.is_optional or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.is_struct or prop.is_enum:
        proto_t = map_object_type_to_proto(prop.py_type, cache)
        assert isinstance(proto_t, (ProtoEnum, Message)), f"unexpected property type: {proto_t!r}"
        return ProtoField(
            id=prop.id,
            name=prop.name,
            type=proto_t,
            optional=prop.is_optional or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.primitive_type in PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE:
        field_type = PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
        return ProtoField(
            id=prop.id,
            name=prop.name,
            type=field_type,
            optional=prop.is_optional or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.is_node_data:
        return ProtoField(
            id=prop.id,
            name=prop.name,
            type="SomeNodeData",
            optional=prop.is_optional,
            repeated=prop.is_list,
        )
    else:
        raise TypeError(f"cannot map to proto type: {prop!r}")


def map_builtin_object_to_proto(
    cls: type[BuiltinObject],
    cache: dict[_ThingType, ProtoThing],
    alias: str | None = None,
    properties: Sequence["Property"] | None = None,
) -> Message:
    if cls in cache:
        message = cache[cls]
        assert isinstance(message, Message), f"unexpected cached {message!r} for {cls!r}"
        return message
    message = Message(name=alias or cls.__name__, reserved_names=[], reserved_ids=[], fields=[])
    doc = cls.__doc__ if issubclass(cls, Node) else None
    message.comment = (doc or "").strip()
    cache[cls] = message  # to solve recursive references
    for prop in properties if properties is not None else cls.__properties__.values():
        if not prop.is_proto:
            continue
        fields = map_bench_property_to_proto(prop, cache)
        if isinstance(fields, ProtoField):
            fields = [fields]
        message.fields.extend(fields)
    message.fields.sort(key=lambda f: cast(int, f.id))
    return message


def map_builtin_enum_to_proto(
    bench_t: type[BuiltinEnum] | type[enum.IntFlag],
    cache: dict[_ThingType, ProtoThing],
    alias: str | None = None,
) -> ProtoEnum:
    assert issubclass(
        bench_t, (BuiltinEnum, enum.IntEnum, enum.IntFlag)
    ), f"invalid enum: {bench_t!r}"
    enum_prefix = to_casing(alias or bench_t.__name__, Casing.ALL_CAPS) + "_"
    if issubclass(bench_t, BuiltinEnum):
        enum_values = [
            ProtoEnumValue(id=member.id, name=enum_prefix + member.name) for member in bench_t
        ]
    elif issubclass(bench_t, (enum.IntFlag, enum.IntEnum)):
        # use int values as ids
        enum_values = [
            ProtoEnumValue(id=name, name=enum_prefix + id_)
            for id_, name in bench_t.__members__.items()
        ]
    else:
        raise TypeError(f"invalid enum type: {bench_t!r}")
    # add unset if not already present
    if not any(v.id == 0 for v in enum_values):
        enum_values = [ProtoEnumValue(id=0, name=enum_prefix + "UNSPECIFIED"), *enum_values]
    has_duplicates = len(enum_values) != len({v.id for v in enum_values})
    proto_t = ProtoEnum(
        name=alias or bench_t.__name__, values=enum_values, allow_alias=has_duplicates
    )
    if bench_t.__doc__:
        proto_t.comment = bench_t.__doc__.strip()
    return proto_t


def map_object_type_to_proto(
    bench_t: _ThingType, cache: dict[_ThingType, ProtoThing], alias: str | None = None
) -> ProtoThing:
    """Maps a Bench type to a Proto type. If not yet mapped, adds it to the cache."""
    from bench.language import BuiltinObject

    assert isinstance(bench_t, type), f"invalid type: {bench_t!r}"
    if bench_t in cache:
        return cache[bench_t]
    if issubclass(bench_t, BuiltinObject):
        ret = map_builtin_object_to_proto(bench_t, cache, alias=alias)
    elif issubclass(bench_t, BuiltinEnum):
        ret = map_builtin_enum_to_proto(bench_t, cache, alias=alias)
    else:
        raise TypeError(f"invalid bench type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def generate_proto_schema(
    name: str,
    unions: dict[str, tuple[str, Collection[type[Union["BuiltinObject", BuiltinEnum]]]]],
    extras: list[ProtoEnum | Message],
    message_postfix: str,
) -> ProtoSchema:
    from bench.language import Node

    proto_types_cache: dict[_ThingType, ProtoThing] = {}
    proto_types: list[ProtoEnum | Message] = []
    # enums
    for enum_t in EnumType:
        enum_cls = cast(type[BuiltinEnum], BENCH_CLASS_BY_TYPE[enum_t])
        proto_types.append(map_builtin_enum_to_proto(enum_cls, proto_types_cache))
    # structs
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        proto_types.append(map_builtin_object_to_proto(struct_cls, proto_types_cache))
    # nodes
    proto_types.append(map_builtin_object_to_proto(Node, proto_types_cache, alias="BaseNode"))
    for node_cls in NODE_CLASS_BY_TYPE.values():
        proto_types.append(map_builtin_object_to_proto(node_cls, proto_types_cache))
    # additional types
    for cls in FINAL_BENCH_CLASSES:
        if cls not in proto_types_cache:
            _ = map_object_type_to_proto(cls, proto_types_cache)
    for proto_thing in proto_types_cache.values():
        if proto_thing not in proto_types:
            proto_types.append(proto_thing)  # type: ignore

    # add custom union types
    for union_name, (wrapper_field_name, unioned_types) in unions.items():
        sub_fields = [
            ProtoField(
                id=i + 1,
                name=to_casing(t.__name__, Casing.SNAKE),
                type=cast(Any, map_object_type_to_proto(t, proto_types_cache)),
            )
            for i, t in enumerate(unioned_types)
        ]
        wrapper_field = ProtoField(
            id=None, name=wrapper_field_name, type=ProtoFieldType.ONE_OF, sub_fields=sub_fields
        )
        wrapper_message = Message(
            name=union_name, reserved_names=[], reserved_ids=[], fields=[wrapper_field]
        )
        proto_types.append(wrapper_message)
    # and other extra types
    proto_types.extend(extras)

    # apply postfix to messages
    if message_postfix:
        for proto_type in proto_types:
            if isinstance(proto_type, Message) and proto_type not in extras:
                proto_type.name += message_postfix

    return ProtoSchema.from_types(name, proto_types)
