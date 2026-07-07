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
    json_field,
    json_object,
    json_string,
)

import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.node
import destack._generated.dir.type.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class AutoSegment:
    """Auto-derived implementations added by one DIR phase."""

    # the module id of the auto segment
    module_id: destack._generated.source.file.model.module.ModuleId
    # auto-derived implementations in emission order
    implementations: Sequence[AutoDerivedImplementation]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_auto_segment(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoSegment:
        """Decode one AutoSegment."""
        return decode_auto_segment(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_auto_segment(self)

    @classmethod
    def from_json(cls, value: Json) -> AutoSegment:
        """Return one AutoSegment from one JSON value."""
        return from_json_auto_segment(value)


def encode_auto_segment(writer: BinaryWriter, value: AutoSegment) -> None:
    """Encode one AutoSegment."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(len(value.implementations))
    for item_value_implementations_0 in value.implementations:
        encode_auto_derived_implementation(writer, item_value_implementations_0)


def decode_auto_segment(reader: BinaryReader) -> AutoSegment:
    """Decode one AutoSegment."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    implementations = [
        decode_auto_derived_implementation(reader) for _ in range(reader.read_number())
    ]

    return AutoSegment(
        module_id=module_id,
        implementations=implementations,
    )


def to_json_auto_segment(value: AutoSegment) -> Json:
    """Return one JSON value for one AutoSegment."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "implementations": [
            to_json_auto_derived_implementation(item_0)
            for item_0 in value.implementations
        ],
    }


def from_json_auto_segment(value: Json) -> AutoSegment:
    """Return one AutoSegment from one JSON value."""
    object_ = json_object(value)

    return AutoSegment(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        implementations=[
            from_json_auto_derived_implementation(item_0)
            for item_0 in json_array(json_field(object_, "implementations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class AutoDerivedImplementation:
    """Generated implementation for one auto interface."""

    # the source node that requested this implementation
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generated interface
    interface: AutoInterface
    # the implemented type
    target: destack._generated.dir.type.type.GlobalTypeId
    # the generated members
    members: Sequence[AutoImplementationMember]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_auto_derived_implementation(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoDerivedImplementation:
        """Decode one AutoDerivedImplementation."""
        return decode_auto_derived_implementation(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_auto_derived_implementation(self)

    @classmethod
    def from_json(cls, value: Json) -> AutoDerivedImplementation:
        """Return one AutoDerivedImplementation from one JSON value."""
        return from_json_auto_derived_implementation(value)


def encode_auto_derived_implementation(
    writer: BinaryWriter, value: AutoDerivedImplementation
) -> None:
    """Encode one AutoDerivedImplementation."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    encode_auto_interface(writer, value.interface)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.target)
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        encode_auto_implementation_member(writer, item_value_members_0)


def decode_auto_derived_implementation(
    reader: BinaryReader,
) -> AutoDerivedImplementation:
    """Decode one AutoDerivedImplementation."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    interface = decode_auto_interface(reader)
    target = destack._generated.dir.type.type.decode_global_type_id(reader)
    members = [
        decode_auto_implementation_member(reader) for _ in range(reader.read_number())
    ]

    return AutoDerivedImplementation(
        source=source,
        interface=interface,
        target=target,
        members=members,
    )


def to_json_auto_derived_implementation(value: AutoDerivedImplementation) -> Json:
    """Return one JSON value for one AutoDerivedImplementation."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "interface": to_json_auto_interface(value.interface),
        "target": destack._generated.dir.type.type.to_json_global_type_id(value.target),
        "members": [
            to_json_auto_implementation_member(item_0) for item_0 in value.members
        ],
    }


def from_json_auto_derived_implementation(value: Json) -> AutoDerivedImplementation:
    """Return one AutoDerivedImplementation from one JSON value."""
    object_ = json_object(value)

    return AutoDerivedImplementation(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        interface=from_json_auto_interface(json_field(object_, "interface")),
        target=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "target")
        ),
        members=[
            from_json_auto_implementation_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
    )


"""Interface whose implementation can be provided by compiler rules."""
AutoInterface: typing.TypeAlias = (
    typing.Literal["compare"]
    | typing.Literal["concrete"]
    | typing.Literal["copy"]
    | typing.Literal["clone"]
    | typing.Literal["debug"]
    | typing.Literal["default"]
    | typing.Literal["deserialize"]
    | typing.Literal["dynamicSafe"]
    | typing.Literal["equal"]
    | typing.Literal["float"]
    | typing.Literal["hash"]
    | typing.Literal["integer"]
    | typing.Literal["overwriteStable"]
    | typing.Literal["partialCompare"]
    | typing.Literal["partialEqual"]
    | typing.Literal["serialize"]
    | typing.Literal["send"]
    | typing.Literal["sync"]
    | typing.Literal["unpin"]
    | typing.Literal["zeroable"]
)


def encode_auto_interface(writer: BinaryWriter, value: AutoInterface) -> None:
    """Encode one AutoInterface."""
    if value == "compare":
        writer.write_unsigned(0)
    elif value == "concrete":
        writer.write_unsigned(1)
    elif value == "copy":
        writer.write_unsigned(2)
    elif value == "clone":
        writer.write_unsigned(3)
    elif value == "debug":
        writer.write_unsigned(4)
    elif value == "default":
        writer.write_unsigned(5)
    elif value == "deserialize":
        writer.write_unsigned(6)
    elif value == "dynamicSafe":
        writer.write_unsigned(7)
    elif value == "equal":
        writer.write_unsigned(8)
    elif value == "float":
        writer.write_unsigned(9)
    elif value == "hash":
        writer.write_unsigned(10)
    elif value == "integer":
        writer.write_unsigned(11)
    elif value == "overwriteStable":
        writer.write_unsigned(12)
    elif value == "partialCompare":
        writer.write_unsigned(13)
    elif value == "partialEqual":
        writer.write_unsigned(14)
    elif value == "serialize":
        writer.write_unsigned(15)
    elif value == "send":
        writer.write_unsigned(16)
    elif value == "sync":
        writer.write_unsigned(17)
    elif value == "unpin":
        writer.write_unsigned(18)
    elif value == "zeroable":
        writer.write_unsigned(19)
    else:
        raise SerdeError("unknown enum variant")


def decode_auto_interface(reader: BinaryReader) -> AutoInterface:
    """Decode one AutoInterface."""
    variant = reader.read_number()

    if variant == 0:
        return "compare"
    elif variant == 1:
        return "concrete"
    elif variant == 2:
        return "copy"
    elif variant == 3:
        return "clone"
    elif variant == 4:
        return "debug"
    elif variant == 5:
        return "default"
    elif variant == 6:
        return "deserialize"
    elif variant == 7:
        return "dynamicSafe"
    elif variant == 8:
        return "equal"
    elif variant == 9:
        return "float"
    elif variant == 10:
        return "hash"
    elif variant == 11:
        return "integer"
    elif variant == 12:
        return "overwriteStable"
    elif variant == 13:
        return "partialCompare"
    elif variant == 14:
        return "partialEqual"
    elif variant == 15:
        return "serialize"
    elif variant == 16:
        return "send"
    elif variant == 17:
        return "sync"
    elif variant == 18:
        return "unpin"
    elif variant == 19:
        return "zeroable"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_auto_interface(value: AutoInterface) -> Json:
    """Return one JSON value for one AutoInterface."""
    return value


def from_json_auto_interface(value: Json) -> AutoInterface:
    """Return one AutoInterface from one JSON value."""
    variant = json_string(value)

    if variant == "compare":
        return "compare"
    elif variant == "concrete":
        return "concrete"
    elif variant == "copy":
        return "copy"
    elif variant == "clone":
        return "clone"
    elif variant == "debug":
        return "debug"
    elif variant == "default":
        return "default"
    elif variant == "deserialize":
        return "deserialize"
    elif variant == "dynamicSafe":
        return "dynamicSafe"
    elif variant == "equal":
        return "equal"
    elif variant == "float":
        return "float"
    elif variant == "hash":
        return "hash"
    elif variant == "integer":
        return "integer"
    elif variant == "overwriteStable":
        return "overwriteStable"
    elif variant == "partialCompare":
        return "partialCompare"
    elif variant == "partialEqual":
        return "partialEqual"
    elif variant == "serialize":
        return "serialize"
    elif variant == "send":
        return "send"
    elif variant == "sync":
        return "sync"
    elif variant == "unpin":
        return "unpin"
    elif variant == "zeroable":
        return "zeroable"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class AutoImplementationMember:
    """Member generated for one auto-derived implementation."""

    # the source node that owns the generated member
    source: destack._generated.dir.tree.node.GlobalNodeIdAny
    # the generated member symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the generated member type
    ty: destack._generated.dir.type.type.GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_auto_implementation_member(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> AutoImplementationMember:
        """Decode one AutoImplementationMember."""
        return decode_auto_implementation_member(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_auto_implementation_member(self)

    @classmethod
    def from_json(cls, value: Json) -> AutoImplementationMember:
        """Return one AutoImplementationMember from one JSON value."""
        return from_json_auto_implementation_member(value)


def encode_auto_implementation_member(
    writer: BinaryWriter, value: AutoImplementationMember
) -> None:
    """Encode one AutoImplementationMember."""
    destack._generated.dir.tree.node.encode_global_node_id_any(writer, value.source)
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    destack._generated.dir.type.type.encode_global_type_id(writer, value.ty)


def decode_auto_implementation_member(reader: BinaryReader) -> AutoImplementationMember:
    """Decode one AutoImplementationMember."""
    source = destack._generated.dir.tree.node.decode_global_node_id_any(reader)
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    ty = destack._generated.dir.type.type.decode_global_type_id(reader)

    return AutoImplementationMember(
        source=source,
        symbol=symbol,
        ty=ty,
    )


def to_json_auto_implementation_member(value: AutoImplementationMember) -> Json:
    """Return one JSON value for one AutoImplementationMember."""
    return {
        "source": destack._generated.dir.tree.node.to_json_global_node_id_any(
            value.source
        ),
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "ty": destack._generated.dir.type.type.to_json_global_type_id(value.ty),
    }


def from_json_auto_implementation_member(value: Json) -> AutoImplementationMember:
    """Return one AutoImplementationMember from one JSON value."""
    object_ = json_object(value)

    return AutoImplementationMember(
        source=destack._generated.dir.tree.node.from_json_global_node_id_any(
            json_field(object_, "source")
        ),
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        ty=destack._generated.dir.type.type.from_json_global_type_id(
            json_field(object_, "ty")
        ),
    )


__all__ = [
    "AutoSegment",
    "encode_auto_segment",
    "decode_auto_segment",
    "to_json_auto_segment",
    "from_json_auto_segment",
    "AutoDerivedImplementation",
    "encode_auto_derived_implementation",
    "decode_auto_derived_implementation",
    "to_json_auto_derived_implementation",
    "from_json_auto_derived_implementation",
    "AutoInterface",
    "encode_auto_interface",
    "decode_auto_interface",
    "to_json_auto_interface",
    "from_json_auto_interface",
    "AutoImplementationMember",
    "encode_auto_implementation_member",
    "decode_auto_implementation_member",
    "to_json_auto_implementation_member",
    "from_json_auto_implementation_member",
]
