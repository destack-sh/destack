import enum
from dataclasses import dataclass
from itertools import chain
from typing import TYPE_CHECKING, Union

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
    enums: list["Enum"]
    messages: list["Message"]

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = ""
        for import_ in self.imports:
            source += f"import {import_};\n"
        source += "\n"
        for thing in chain(self.enums, self.messages):
            source += f"{thing.to_proto_source()}\n\n"
        return source


@dataclass
class Message(ProtoThing):
    """Proto message."""

    name: str
    reserved_names: list[str]
    reserved_ids: list[int]
    fields: list["Field"]

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f"message {self.name} {{\n"
        if self.reserved_names:
            source += f"  reserved {', '.join(self.reserved_names)};\n"
        if self.reserved_ids:
            source += f"  reserved {', '.join(str(id) for id in self.reserved_ids)};\n"
        for field in self.fields:
            source += f"  {field.to_proto_source()}\n"
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


@dataclass
class Enum(ProtoThing):
    """Proto enum."""

    name: str
    values: list["EnumValue"]

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f"enum {self.name} {{\n"
        for value in self.values:
            source += f"  {value.to_proto_source()}\n"
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
    id: int
    name: str
    type: FieldType | Enum | Message
    repeated: bool
    key_type: FieldType | None  # for map
    value_type: FieldType | None  # for map
    sub_fields: list["Field"] | None  # for one of

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
                type += f"  {sub_field.to_proto_source()}\n"
            type += "}"
        else:
            type = f"{repeated}{self.type}"
        return f"{type} {self.name} = {self.id}"


#
# Map Bench types to Proto types
# We map and walk at the same type for simplicity (using the cache)
#

_BenchType = Union["Node", "Struct", "Property", enum.StrEnum]


def _bench_property_to_proto(bench_t: "Property", cache: dict[_BenchType, ProtoThing]) -> Field:
    assert not bench_t.is_runtime, f"shouldn't map runtime property: {bench_t!r}"
    raise NotImplementedError("nocheckin")


def _bench_node_to_proto(bench_t: "Struct", cache: dict[_BenchType, ProtoThing]) -> Message:
    struct = Message(name=bench_t.__name__, reserved_names=[], reserved_ids=[], fields=[])
    for prop in bench_t.__properties__.values():
        field = _bench_property_to_proto(prop, cache)
        struct.fields.append(field)
    return struct


def _bench_enum_to_proto(bench_t: enum.StrEnum, cache: dict[_BenchType, ProtoThing]) -> Enum:
    enum_values = [
        EnumValue(id=id_, name=name) for id_, name in enumerate(bench_t.__members__.keys())
    ]
    return Enum(name=bench_t.__name__, values=enum_values)


def bench_to_proto(bench_t: _BenchType, cache: dict[_BenchType, ProtoThing]) -> ProtoThing:
    """Maps a Bench type to a Proto type. If not yet mapped, adds it to the cache."""
    if bench_t in cache:
        return cache[bench_t]
    if isinstance(bench_t, (Node, Struct)):
        ret = _bench_node_to_proto(bench_t, cache)
    elif isinstance(bench_t, enum.StrEnum):
        ret = _bench_enum_to_proto(bench_t, cache)
    else:
        raise TypeError(f"invalid type: {bench_t!r}")
    cache[bench_t] = ret
    return ret


def generate_proto_source(
    node_types: list[type["Node"]],
    struct_types: list[type["Struct"]],
    enum_types: list[type[enum.StrEnum]],
) -> Proto:
    """Maps a collection of Bench types to a Proto schema."""
    proto_types: dict[type[_BenchType], ProtoThing] = {}
    for thing in chain(node_types, struct_types, enum_types):
        _ = bench_to_proto(thing, proto_types)  # added to proto_types
    enums = [n for n in proto_types.values() if isinstance(n, Enum)]
    messages = [n for n in proto_types.values() if isinstance(n, Message)]
    return Proto("bench", [], enums, messages)
