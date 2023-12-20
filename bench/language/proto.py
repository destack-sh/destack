import dataclasses
import enum
import textwrap
from dataclasses import dataclass
from itertools import chain
from typing import TYPE_CHECKING, Collection, Union

from bench.sql.core import ColumnType
from bench.utils.utils import to_all_caps, to_snake_case

if TYPE_CHECKING:
    from bench.language import Node, Property, Struct


class ProtoThing:
    """Proto thing."""

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        raise NotImplementedError()


@dataclass
class Proto(ProtoThing):
    """Proto file."""

    name: str
    imports: list[str]
    types: list[Union["Enum", "Message"]]

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = 'syntax = "proto3";\n\n'
        for import_ in self.imports:
            source += f'import "{import_}";\n'
        source += "\n"
        for thing in self.types:
            source += f"{thing.to_proto_source()}\n\n"
        return source

    @staticmethod
    def from_types(name: str, types: list[Union["Enum", "Message"]]) -> "Proto":
        """Create a proto file from types. Figures out imports."""
        # just add default imports for all the well-known types we use
        imports = [
            "google/protobuf/timestamp.proto",
            "google/protobuf/struct.proto",
        ]
        return Proto(name=name, imports=imports, types=types)


@dataclass
class Message(ProtoThing):
    """Proto message."""

    name: str
    fields: list["Field"]
    reserved_names: list[str] = dataclasses.field(default_factory=list)
    reserved_ids: list[int] = dataclasses.field(default_factory=list)

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f"message {self.name} {{\n"
        if self.reserved_names:
            source += f"  reserved {', '.join(self.reserved_names)};\n"
        if self.reserved_ids:
            source += f"  reserved {', '.join(str(id) for id in self.reserved_ids)};\n"
        for field in self.fields:
            source += f"{textwrap.indent(field.to_proto_source(), '  ')};\n"
        source += "}"
        return source


class FieldType(enum.StrEnum):
    """Proto field type."""

    INT32 = "int32"
    INT64 = "int64"
    FLOAT = "float"
    DOUBLE = "double"
    BOOL = "bool"
    STRING = "string"
    BYTES = "bytes"
    ENUM = "enum"
    MESSAGE = "message"
    REPEATED = "repeated"
    MAP = "map"
    ONE_OF = "oneof"
    # well-known types
    ANY = "google.protobuf.Any"
    TIMESTAMP = "google.protobuf.Timestamp"
    DURATION = "google.protobuf.Duration"
    EMPTY = "google.protobuf.Empty"
    STRUCT = "google.protobuf.Struct"
    UUID = "google.protobuf.UUID"


@dataclass
class Enum(ProtoThing):
    """Proto enum."""

    name: str
    values: list["EnumValue"]
    allow_alias: bool = False

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f"enum {self.name} {{\n"
        if self.allow_alias:
            source += "  option allow_alias = true;\n"
        for value in self.values:
            source += f"  {value.to_proto_source()};\n"
        source += "}"
        return source


@dataclass
class EnumValue(ProtoThing):
    """Proto enum value."""

    id: int
    name: str

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        return f"{self.name} = {self.id}"


@dataclass
class Field(ProtoThing):
    id: int | None
    name: str
    type: FieldType | Enum | Message | str
    repeated: bool = False
    key_type: FieldType | None = None  # for map
    value_type: FieldType | None = None  # for map
    sub_fields: list["Field"] | None = None  # for one of

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        if self.repeated:
            repeated = "repeated "
        else:
            repeated = ""
        if self.type == FieldType.REPEATED:
            type = f"{repeated}{self.value_type}[]"
        elif self.type == FieldType.MAP:
            type = f"{repeated}map<{self.key_type}, {self.value_type}>"
        elif self.type == FieldType.ONE_OF:
            type = f"{repeated}oneof {self.name} {{\n"
            for sub_field in self.sub_fields:
                type += f"  {sub_field.to_proto_source()};\n"
            type += "}"
            return type  # no id for one of
        elif isinstance(self.type, (Enum, Message)):
            type = f"{repeated}{self.type.name}"
        elif isinstance(self.type, FieldType):
            type = f"{repeated}{self.type.value}"
        else:
            type = f"{repeated}{self.type}"
        return f"{type} {self.name} = {self.id}"


#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

PROTO_FIELD_TYPE_BY_COLUMN_TYPE: dict[ColumnType, FieldType] = {
    ColumnType.BOOLEAN: FieldType.BOOL,
    ColumnType.INT: FieldType.INT32,
    ColumnType.BIGINT: FieldType.INT64,
    ColumnType.FLOAT: FieldType.FLOAT,
    ColumnType.STRING: FieldType.STRING,
    ColumnType.BYTES: FieldType.BYTES,
    ColumnType.DATETIME: FieldType.TIMESTAMP,
    ColumnType.UUID: FieldType.STRING,  # see https://stackoverflow.com/q/36344826/3375858
    ColumnType.JSON: FieldType.STRUCT,
}

_BenchType = type[Union["Node", "Struct", "Property", enum.StrEnum, enum.IntFlag]]


def _bench_property_to_proto(prop: "Property", cache: dict[_BenchType, ProtoThing]) -> Field:
    assert not prop.is_runtime, f"shouldn't map runtime property: {prop!r}"
    assert isinstance(prop.id, int), f"stored properties need an id: {prop!r}"
    # store typed enum/struct references (except for int/flag enums, which proto doesn't have)
    if prop.is_struct or prop.is_enum and prop.store_as == ColumnType.STRING:
        struct_type = bench_to_proto(prop.py_type_stripped, cache)
        return Field(id=prop.id, name=prop.name, type=struct_type, repeated=prop.is_array)
    elif prop.store_as in PROTO_FIELD_TYPE_BY_COLUMN_TYPE:
        field_type = PROTO_FIELD_TYPE_BY_COLUMN_TYPE[prop.store_as]
        return Field(id=prop.id, name=prop.name, type=field_type, repeated=prop.is_array)
    else:
        raise TypeError(f"cannot map to proto type: {prop!r}")


def _bench_struct_to_proto(
    node: type["Struct"], cache: dict[_BenchType, ProtoThing], alias: str = None
) -> Message:
    struct = Message(name=alias or node.__name__, reserved_names=[], reserved_ids=[], fields=[])
    cache[node] = struct  # to solve recursive references
    for prop in node.__properties__.values():
        if not prop.is_stored:
            continue
        field = _bench_property_to_proto(prop, cache)
        struct.fields.append(field)
    for reserved in node.__reserved_properties__:
        if isinstance(reserved, str):
            struct.reserved_names.append(reserved)
        elif isinstance(reserved, int):
            struct.reserved_ids.append(reserved)
        else:
            raise TypeError(f"invalid reserved property: {reserved!r}")
    struct.fields.sort(key=lambda f: f.id)
    return struct


def _bench_enum_to_proto(
    bench_t: type[enum.StrEnum] | type[enum.IntFlag],
    cache: dict[_BenchType, ProtoThing],
    alias: str = None,
) -> Enum:
    # TODO @Broken: assign static ids to enum values (or use int enums) for proto serialization
    enum_prefix = to_all_caps(alias or bench_t.__name__) + "_"
    if issubclass(bench_t, enum.StrEnum):
        enum_values = [
            EnumValue(id=i + 1, name=enum_prefix + name)
            for i, name in enumerate(bench_t.__members__.keys())
        ]
    elif issubclass(bench_t, (enum.IntFlag, enum.IntEnum)):
        # use int values as ids
        enum_values = [
            EnumValue(id=name, name=enum_prefix + id_) for id_, name in bench_t.__members__.items()
        ]
    else:
        raise TypeError(f"invalid type: {bench_t!r}")
    # add unset if not already present
    if not any(v.id == 0 for v in enum_values):
        enum_values = [EnumValue(id=0, name=enum_prefix + "UNSPECIFIED"), *enum_values]
    has_duplicates = len(enum_values) != len(set(v.id for v in enum_values))
    return Enum(name=alias or bench_t.__name__, values=enum_values, allow_alias=has_duplicates)


def bench_to_proto(
    bench_t: _BenchType, cache: dict[_BenchType, ProtoThing], alias: str = None
) -> ProtoThing:
    """Maps a Bench type to a Proto type. If not yet mapped, adds it to the cache."""
    from bench.language import Node, Struct

    assert isinstance(bench_t, type), f"invalid type: {bench_t!r}"
    if bench_t in cache:
        return cache[bench_t]
    if issubclass(bench_t, (Node, Struct)):
        ret = _bench_struct_to_proto(bench_t, cache, alias=alias)
    elif issubclass(bench_t, enum.Enum):
        ret = _bench_enum_to_proto(bench_t, cache, alias=alias)
    else:
        raise TypeError(f"invalid type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def generate_proto_schema(
    bench_types: Collection[type[Union["Node", "Struct", enum.Enum]]],
    aliases: dict[type[Union["Node", "Struct", enum.Enum]], str],
    unions: dict[str, tuple[str, list[type[Union["Node", "Struct", enum.Enum]]]]],
    extras: list[Enum | Message],
    message_postfix: str = "",
) -> Proto:
    """Maps a collection of Bench types to a Proto schema :ProtoSchema."""
    from bench.language import Node, Struct

    proto_types_cache: dict[type[_BenchType], ProtoThing] = {}
    for thing in bench_types:
        _ = bench_to_proto(thing, proto_types_cache, alias=aliases.get(thing))

    # collect proto types
    collected_enums: list[type[enum.Enum]] = [t for t in bench_types if issubclass(t, enum.Enum)]
    collected_structs: list[type["Struct"]] = [
        t for t in bench_types if issubclass(t, Struct) and not issubclass(t, Node)
    ]
    collected_nodes: list[type["Node"]] = [t for t in bench_types if issubclass(t, Node)]
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
                id=i + 1, name=to_snake_case(t.__name__), type=bench_to_proto(t, proto_types_cache)
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
            if isinstance(proto_type, Message):
                proto_type.name += message_postfix

    return Proto.from_types("bench", proto_types)
