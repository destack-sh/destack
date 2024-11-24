import enum
from typing import TYPE_CHECKING, Any, Collection, Sequence, Union, cast

from bench.language.const import EnumType, PrimitiveType
from bench.language.node import BuiltinObject, Node
from bench.language.setup import (
    BENCH_CLASS_BY_TYPE,
    FINAL_BENCH_CLASSES,
    NODE_CLASS_BY_TYPE,
    STRUCT_CLASS_BY_TYPE,
)
from bench.proto.core import Enum, EnumValue, Field, FieldType, Message, ProtoSchema, ProtoThing
from bench.utils.func import IdEnum
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    from bench.language import Property

#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, FieldType] = {
    PrimitiveType.BOOLEAN: FieldType.BOOL,
    PrimitiveType.INT16: FieldType.INT32,
    PrimitiveType.INT32: FieldType.INT32,
    PrimitiveType.INT64: FieldType.INT64,
    PrimitiveType.FLOAT32: FieldType.FLOAT,
    PrimitiveType.FLOAT64: FieldType.DOUBLE,
    PrimitiveType.STRING: FieldType.STRING,
    PrimitiveType.BYTES: FieldType.BYTES,
    PrimitiveType.DATETIME: FieldType.TIMESTAMP,
    PrimitiveType.DATE: FieldType.DATE,
    PrimitiveType.INTERVAL: FieldType.DURATION,
    PrimitiveType.UUID: FieldType.STRING,  # see https://stackoverflow.com/q/36344826/3375858
    PrimitiveType.JSON: FieldType.VALUE,
}

_ThingType = type[Union["BuiltinObject", "Property", IdEnum, enum.IntFlag]]


def map_bench_property_to_proto(
    prop: "Property", cache: dict[_ThingType, ProtoThing]
) -> Field | Sequence[Field]:
    assert prop.id == 1 or not prop.is_ephemeral, f"shouldn't map runtime property: {prop!r}"
    assert isinstance(prop.id, int), f"stored properties need an id: {prop!r}"
    if prop.is_node_reference:
        return Field(
            id=prop.id,
            name=prop.name,
            type="NodeReferenceData",
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.is_struct or prop.is_enum:
        proto_t = map_object_type_to_proto(prop.py_type_stripped, cache)
        assert isinstance(proto_t, (Enum, Message)), f"unexpected property type: {proto_t!r}"
        return Field(
            id=prop.id,
            name=prop.name,
            type=proto_t,
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.primitive_type in PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE:
        field_type = PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
        return Field(
            id=prop.id,
            name=prop.name,
            type=field_type,
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_list,
        )
    elif prop.reference_is_node_data:
        return Field(
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
    doc = (
        cls.__doc__ or cls.__base_class__.__doc__
        if issubclass(cls, Node) and cls.__base_class__
        else None
    )
    message.comment = (doc or "").strip()
    cache[cls] = message  # to solve recursive references
    for prop in properties if properties is not None else cls.__properties__.values():
        if not prop.is_wired:
            continue
        fields = map_bench_property_to_proto(prop, cache)
        if isinstance(fields, Field):
            fields = [fields]
        message.fields.extend(fields)
    message.fields.sort(key=lambda f: cast(int, f.id))
    return message


def map_builtin_enum_to_proto(
    bench_t: type[IdEnum] | type[enum.IntFlag],
    cache: dict[_ThingType, ProtoThing],
    alias: str | None = None,
) -> Enum:
    assert issubclass(bench_t, (IdEnum, enum.IntEnum, enum.IntFlag)), f"invalid enum: {bench_t!r}"
    enum_prefix = to_casing(alias or bench_t.__name__, Casing.ALL_CAPS) + "_"
    if issubclass(bench_t, IdEnum):
        enum_values = [
            EnumValue(id=member.id, name=enum_prefix + member.name) for member in bench_t
        ]
    elif issubclass(bench_t, (enum.IntFlag, enum.IntEnum)):
        # use int values as ids
        enum_values = [
            EnumValue(id=name, name=enum_prefix + id_) for id_, name in bench_t.__members__.items()
        ]
    else:
        raise TypeError(f"invalid enum type: {bench_t!r}")
    # add unset if not already present
    if not any(v.id == 0 for v in enum_values):
        enum_values = [EnumValue(id=0, name=enum_prefix + "UNSPECIFIED"), *enum_values]
    has_duplicates = len(enum_values) != len({v.id for v in enum_values})
    proto_t = Enum(name=alias or bench_t.__name__, values=enum_values, allow_alias=has_duplicates)
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
    elif issubclass(bench_t, (IdEnum, enum.IntFlag)):
        ret = map_builtin_enum_to_proto(bench_t, cache, alias=alias)
    else:
        raise TypeError(f"invalid bench type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def map_object_subtype_to_proto(
    node_t: type[Node],
    cache: dict[_ThingType, ProtoThing],
    subtype: int,
) -> Message:
    """Maps a Node subtype to a Proto type. Only includes the subtype properties not in base."""
    assert node_t.__base_class__ is not None, f"node is not a subtype: {node_t!r}"
    properties = [
        prop
        for prop in node_t.__properties__.values()
        if prop.name not in node_t.__base_class__.__properties__
    ]
    return map_builtin_object_to_proto(node_t, cache, properties=properties)


def generate_proto_schema(
    name: str,
    unions: dict[str, tuple[str, Collection[type[Union["BuiltinObject", IdEnum]]]]],
    extras: list[Enum | Message],
    message_postfix: str = "",
) -> ProtoSchema:
    from bench.language import Node

    proto_types_cache: dict[_ThingType, ProtoThing] = {}
    proto_types: list[Enum | Message] = []
    # enums
    for enum_t in EnumType:
        enum_cls = cast(type[IdEnum], BENCH_CLASS_BY_TYPE[enum_t])
        proto_types.append(map_builtin_enum_to_proto(enum_cls, proto_types_cache))
    # structs
    for struct_cls in STRUCT_CLASS_BY_TYPE.values():
        proto_types.append(map_builtin_object_to_proto(struct_cls, proto_types_cache))
    # nodes
    proto_types.append(map_builtin_object_to_proto(Node, proto_types_cache, alias="BaseNode"))
    for node_cls in NODE_CLASS_BY_TYPE.values():
        proto_types.append(map_builtin_object_to_proto(node_cls, proto_types_cache))
        if node_cls.__has_subtypes__:
            for subnode_type, subnode_cls in node_cls.__subclass_by_subtype__.items():
                proto_types.append(
                    map_object_subtype_to_proto(
                        subnode_cls, proto_types_cache, subtype=subnode_type
                    )
                )
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
            Field(
                id=i + 1,
                name=to_casing(t.__name__, Casing.SNAKE),
                type=cast(Any, map_object_type_to_proto(t, proto_types_cache)),
            )
            for i, t in enumerate(unioned_types)
        ]
        wrapper_field = Field(
            id=None, name=wrapper_field_name, type=FieldType.ONE_OF, sub_fields=sub_fields
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
