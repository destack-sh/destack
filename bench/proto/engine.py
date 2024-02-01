import enum
from itertools import chain
from typing import TYPE_CHECKING, Collection, Union

from bench.proto.core import (
    Enum,
    EnumValue,
    Field,
    FieldType,
    Message,
    ProtoObject,
    ProtoSchema,
    ProtoStrEnum,
)
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing

if TYPE_CHECKING:
    from bench.language import Node, Property, Struct

#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE: dict[PrimitiveType, FieldType] = {
    PrimitiveType.BOOLEAN: FieldType.BOOL,
    PrimitiveType.INT32: FieldType.INT32,
    PrimitiveType.INT64: FieldType.INT64,
    PrimitiveType.FLOAT32: FieldType.FLOAT,
    PrimitiveType.FLOAT64: FieldType.DOUBLE,
    PrimitiveType.STRING: FieldType.STRING,
    PrimitiveType.BYTES: FieldType.BYTES,
    PrimitiveType.DATETIME: FieldType.TIMESTAMP,
    PrimitiveType.UUID: FieldType.STRING,  # see https://stackoverflow.com/q/36344826/3375858
    PrimitiveType.JSON: FieldType.STRUCT,
}

_BenchType = type[Union["Node", "Struct", "Property", enum.StrEnum, enum.IntFlag]]


def map_bench_property_to_proto(prop: "Property", cache: dict[_BenchType, ProtoObject]) -> Field:
    assert not prop.is_runtime_only, f"shouldn't map runtime property: {prop!r}"
    assert isinstance(prop.id, int), f"stored properties need an id: {prop!r}"
    # store typed enum/struct references (except for int/flag enums, which proto doesn't have)
    if prop.is_struct or prop.is_enum and prop.primitive_type == PrimitiveType.STRING:
        proto_t = map_bench_type_to_proto(prop.py_type_stripped, cache)
        return Field(
            id=prop.id,
            name=prop.name,
            type=proto_t,
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_array,
        )
    elif prop.primitive_type in PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE:
        field_type = PROTO_FIELD_TYPE_BY_PRIMITIVE_TYPE[prop.primitive_type]
        return Field(
            id=prop.id,
            name=prop.name,
            type=field_type,
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_array,
        )
    elif prop.reference_kind is not None:
        return Field(
            id=prop.id,
            name=prop.name,
            type="NodeReferenceData",
            optional=prop.is_optional or prop.is_deferred or prop.is_sensitive,
            repeated=prop.is_array,
        )
    else:
        raise TypeError(f"cannot map {prop.primitive_type} to proto type: {prop!r}")


def map_bench_struct_to_proto(
    struct: type["Struct"], cache: dict[_BenchType, ProtoObject], alias: str = None
) -> Message:
    message = Message(name=alias or struct.__name__, reserved_names=[], reserved_ids=[], fields=[])
    assert struct.__doc__, f"missing docstring for {struct!r}"
    message.comment = struct.__doc__.strip()
    cache[struct] = message  # to solve recursive references
    for prop in struct.__properties__.values():
        if not prop.is_wired:
            continue
        field = map_bench_property_to_proto(prop, cache)
        message.fields.append(field)
    for reserved in struct.__reserved_properties__:
        if isinstance(reserved, str):
            message.reserved_names.append(reserved)
        elif isinstance(reserved, int):
            message.reserved_ids.append(reserved)
        else:
            raise TypeError(f"invalid reserved property: {reserved!r}")
    message.fields.sort(key=lambda f: f.id)
    return message


def map_bench_enum_to_proto(
    bench_t: type[ProtoStrEnum] | type[enum.IntEnum] | type[enum.IntFlag],
    cache: dict[_BenchType, ProtoObject],
    alias: str = None,
) -> Enum:
    assert issubclass(
        bench_t, (ProtoStrEnum, enum.IntEnum, enum.IntFlag)
    ), f"invalid enum: {bench_t!r}"
    # TODO @Broken: assign static ids to enum values (or use int enums) for proto serialization
    enum_prefix = to_casing(alias or bench_t.__name__, Casing.ALL_CAPS) + "_"
    if issubclass(bench_t, ProtoStrEnum):
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
    has_duplicates = len(enum_values) != len(set(v.id for v in enum_values))
    proto_t = Enum(name=alias or bench_t.__name__, values=enum_values, allow_alias=has_duplicates)
    if bench_t.__doc__:
        proto_t.comment = bench_t.__doc__.strip()
    return proto_t


def map_bench_type_to_proto(
    bench_t: _BenchType, cache: dict[_BenchType, ProtoObject], alias: str = None
) -> ProtoObject:
    """Maps a Bench type to a Proto type. If not yet mapped, adds it to the cache."""
    from bench.language import Node, Struct

    assert isinstance(bench_t, type), f"invalid type: {bench_t!r}"
    if bench_t in cache:
        return cache[bench_t]
    if issubclass(bench_t, (Node, Struct)):
        ret = map_bench_struct_to_proto(bench_t, cache, alias=alias)
    elif issubclass(bench_t, enum.Enum):
        ret = map_bench_enum_to_proto(bench_t, cache, alias=alias)
    else:
        raise TypeError(f"invalid bench type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def generate_proto_schema(
    name: str,
    bench_classes: Collection[type[Union["Node", "Struct", enum.Enum]]],
    aliases: dict[type[Union["Node", "Struct", enum.Enum]], str],
    unions: dict[str, tuple[str, Collection[type[Union["Node", "Struct", enum.Enum]]]]],
    extras: list[Enum | Message],
    message_postfix: str = "",
) -> ProtoSchema:
    from bench.language import Node, Struct

    proto_types_cache: dict[type[_BenchType], ProtoObject] = {}
    for thing in bench_classes:
        _ = map_bench_type_to_proto(thing, proto_types_cache, alias=aliases.get(thing))

    collected_enums: list[type[enum.Enum]] = [
        t for t in proto_types_cache.keys() if issubclass(t, enum.Enum)
    ]
    collected_structs: list[type["Struct"]] = [
        t for t in bench_classes if issubclass(t, Struct) and not issubclass(t, Node)
    ]
    collected_nodes: list[type["Node"]] = [t for t in bench_classes if issubclass(t, Node)]
    collected_enums.sort(key=lambda t: t.__name__)
    collected_structs.sort(key=lambda t: t.__name__)
    collected_nodes.sort(key=lambda t: t.__name__)
    proto_types: list[Enum | Message] = [
        proto_types_cache[t] for t in chain(collected_enums, collected_structs, collected_nodes)
    ]

    # add custom union types
    for union_name, (wrapper_field_name, unioned_types) in unions.items():
        sub_fields = [
            Field(
                id=i + 1,
                name=to_casing(t.__name__, Casing.SNAKE),
                type=map_bench_type_to_proto(t, proto_types_cache),
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

    if message_postfix:  # apply postfix to messages
        for proto_type in proto_types:
            if isinstance(proto_type, Message) and proto_type not in extras:
                proto_type.name += message_postfix

    return ProtoSchema.from_types(name, proto_types)
