import dataclasses
import enum
import textwrap
from dataclasses import dataclass
from typing import ClassVar, Union


class ProtoStrEnum(enum.StrEnum):
    """
    enum.StrEnum with an additional id per value.
    TODO @Cleanup: convert ProtoStrEnum to 'regular' int enum
     (keep this class, but stop specifying name for everything and store all enums as int)
    """

    _ignore_ = ["__RESERVED_NAMES__", "__RESERVED_IDS__"]
    __RESERVED_NAMES__: ClassVar[set[str]] = set()
    __RESERVED_IDS__: ClassVar[set[int]] = set()

    def __new__(cls, value: str, id: int):
        """Create a new instance."""
        obj = str.__new__(cls, value)
        obj._value_ = value
        assert id > 0 or value == "UNSPECIFIED" and id == 0, f"invalid id {id} for {value}"
        obj.id = id
        return obj


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
