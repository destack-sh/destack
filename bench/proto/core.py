import dataclasses
import enum
import textwrap
from dataclasses import dataclass
from typing import Union


class ProtoThing:
    """Proto thing."""

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        raise NotImplementedError


@dataclass
class ProtoSchema(ProtoThing):
    """Proto file."""

    name: str
    imports: list[str]
    types: list[Union["ProtoEnum", "Message"]]

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f'syntax = "proto3";\n\npackage {self.name};\n\n'
        for import_ in self.imports:
            source += f'import "{import_}";\n'
        source += "\n"
        for thing in self.types:
            source += f"{thing.to_proto_source()}\n\n"
        return source

    @staticmethod
    def from_types(name: str, types: list[Union["ProtoEnum", "Message"]]) -> "ProtoSchema":
        """Create a proto file from types. Figures out imports."""
        # just add default imports for all the well-known types we use
        imports = [
            "google/protobuf/timestamp.proto",
            "google/protobuf/duration.proto",
            "google/protobuf/struct.proto",
            "proto/google/type/datetime.proto",
            "proto/google/type/date.proto",
            "proto/google/type/timeofday.proto",
        ]
        return ProtoSchema(name=name, imports=imports, types=types)


def _to_multi_line_comment(comment: str) -> str:
    """Convert a single line comment to a multi line comment."""
    return "\n".join(f"// {line.strip()}" for line in comment.splitlines())


@dataclass
class Message(ProtoThing):
    """Proto message."""

    name: str
    fields: list["ProtoField"]
    reserved_names: list[str] = dataclasses.field(default_factory=list)
    reserved_ids: list[int] = dataclasses.field(default_factory=list)
    comment: str | None = None

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
        if self.comment:
            source = f"{_to_multi_line_comment(self.comment)}\n" + source
        return source


class ProtoFieldType(enum.StrEnum):
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
    DATETIME = "google.type.DateTime"
    DATE = "google.type.Date"
    TIME = "google.type.TimeOfDay"
    DURATION = "google.protobuf.Duration"
    EMPTY = "google.protobuf.Empty"
    STRUCT = "google.protobuf.Struct"
    VALUE = "google.protobuf.Value"
    UUID = "google.protobuf.UUID"


@dataclass
class ProtoEnum(ProtoThing):
    """Proto enum."""

    name: str
    values: list["ProtoEnumValue"]
    allow_alias: bool = False
    comment: str | None = None

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        source = f"enum {self.name} {{\n"
        if self.allow_alias:
            source += "  option allow_alias = true;\n"
        for value in self.values:
            source += f"  {value.to_proto_source()};\n"
        source += "}"
        if self.comment:
            source = f"{_to_multi_line_comment(self.comment)}\n" + source
        return source


@dataclass
class ProtoEnumValue(ProtoThing):
    """Proto enum value."""

    id: int
    name: str

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        return f"{self.name} = {self.id}"


@dataclass
class ProtoField(ProtoThing):
    id: int | None
    name: str
    type: ProtoFieldType | ProtoEnum | Message | str
    optional: bool = False
    repeated: bool = False
    key_type: ProtoFieldType | None = None  # for map
    value_type: ProtoFieldType | None = None  # for map
    sub_fields: list["ProtoField"] | None = None  # for one of

    def to_proto_source(self) -> str:
        """Convert to proto source."""
        prefix = ""
        if self.repeated:
            prefix += "repeated "
        elif self.optional:
            prefix += "optional "
        if self.type == ProtoFieldType.REPEATED:
            type = f"{prefix}{self.value_type}[]"
        elif self.type == ProtoFieldType.MAP:
            type = f"{prefix}map<{self.key_type}, {self.value_type}>"
        elif self.type == ProtoFieldType.ONE_OF:
            type = f"{prefix}oneof {self.name} {{\n"
            for sub_field in self.sub_fields or ():
                type += f"  {sub_field.to_proto_source()};\n"
            type += "}"
            return type  # no id for one of
        elif isinstance(self.type, (ProtoEnum, Message)):
            type = f"{prefix}{self.type.name}"
        elif isinstance(self.type, ProtoFieldType):
            type = f"{prefix}{self.type.value}"
        else:
            type = f"{prefix}{self.type}"
        return f"{type} {self.name} = {self.id}"
