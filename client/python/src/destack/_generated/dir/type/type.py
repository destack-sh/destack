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
import destack._generated.dir.symbol.key
import destack._generated.dir.symbol.symbol
import destack._generated.dir.tree.literal
import destack._generated.dir.tree.node
import destack._generated.dir.tree.static
import destack._generated.dir.tree.type
import destack._generated.dir.type.generic
import destack._generated.dir.type.primitive
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class TypeVariableId:
    """Identifier for one open inference variable inside a checked component."""

    # the module that allocated the variable
    module_id: destack._generated.source.file.model.module.ModuleId
    # the variable index inside the module
    index: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_variable_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeVariableId:
        """Decode one TypeVariableId."""
        return decode_type_variable_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_variable_id(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeVariableId:
        """Return one TypeVariableId from one JSON value."""
        return from_json_type_variable_id(value)


def encode_type_variable_id(writer: BinaryWriter, value: TypeVariableId) -> None:
    """Encode one TypeVariableId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.index)


def decode_type_variable_id(reader: BinaryReader) -> TypeVariableId:
    """Decode one TypeVariableId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    index = reader.read_number()

    return TypeVariableId(
        module_id=module_id,
        index=index,
    )


def to_json_type_variable_id(value: TypeVariableId) -> Json:
    """Return one JSON value for one TypeVariableId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "index": value.index,
    }


def from_json_type_variable_id(value: Json) -> TypeVariableId:
    """Return one TypeVariableId from one JSON value."""
    object_ = json_object(value)

    return TypeVariableId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        index=json_int(json_field(object_, "index")),
    )


@dataclass(frozen=True, slots=True)
class MemoryLiteralAccess:
    """Memory access singleton, like `"readonly"` or `"exclusive"`."""

    access: Access
    kind: typing.Literal["access"] = "access"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_literal(self)


@dataclass(frozen=True, slots=True)
class MemoryLiteralSpace:
    """Storage space singleton, like `"local"` or `"shared"`."""

    space: Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_literal(self)


@dataclass(frozen=True, slots=True)
class MemoryLiteralPlace:
    """Placement singleton, like `"ambient"` or a concrete space."""

    place: Place
    kind: typing.Literal["place"] = "place"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_literal(self)


@dataclass(frozen=True, slots=True)
class MemoryLiteralLifetime:
    """Lifetime singleton, like `"static"` or a lifetime parameter."""

    lifetime: Lifetime
    kind: typing.Literal["lifetime"] = "lifetime"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_memory_literal(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_memory_literal(self)


"""Singleton type of one normalized memory value."""
MemoryLiteral: typing.TypeAlias = (
    MemoryLiteralAccess
    | MemoryLiteralSpace
    | MemoryLiteralPlace
    | MemoryLiteralLifetime
)


def encode_memory_literal(writer: BinaryWriter, value: MemoryLiteral) -> None:
    """Encode one MemoryLiteral."""
    if value.kind == "access":
        writer.write_unsigned(0)
        encode_access(writer, value.access)
    elif value.kind == "space":
        writer.write_unsigned(1)
        encode_space(writer, value.space)
    elif value.kind == "place":
        writer.write_unsigned(2)
        encode_place(writer, value.place)
    elif value.kind == "lifetime":
        writer.write_unsigned(3)
        encode_lifetime(writer, value.lifetime)
    else:
        raise SerdeError("unknown enum variant")


def decode_memory_literal(reader: BinaryReader) -> MemoryLiteral:
    """Decode one MemoryLiteral."""
    variant = reader.read_number()

    if variant == 0:
        access = decode_access(reader)

        return MemoryLiteralAccess(access=access)
    elif variant == 1:
        space = decode_space(reader)

        return MemoryLiteralSpace(space=space)
    elif variant == 2:
        place = decode_place(reader)

        return MemoryLiteralPlace(place=place)
    elif variant == 3:
        lifetime = decode_lifetime(reader)

        return MemoryLiteralLifetime(lifetime=lifetime)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_memory_literal(value: MemoryLiteral) -> Json:
    """Return one JSON value for one MemoryLiteral."""
    if value.kind == "access":
        return {
            "kind": "access",
            "access": to_json_access(value.access),
        }
    elif value.kind == "space":
        return {
            "kind": "space",
            "space": to_json_space(value.space),
        }
    elif value.kind == "place":
        return {
            "kind": "place",
            "place": to_json_place(value.place),
        }
    elif value.kind == "lifetime":
        return {
            "kind": "lifetime",
            "lifetime": to_json_lifetime(value.lifetime),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_memory_literal(value: Json) -> MemoryLiteral:
    """Return one MemoryLiteral from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "access":
        return MemoryLiteralAccess(
            access=from_json_access(json_field(object_, "access"))
        )
    elif kind == "space":
        return MemoryLiteralSpace(space=from_json_space(json_field(object_, "space")))
    elif kind == "place":
        return MemoryLiteralPlace(place=from_json_place(json_field(object_, "place")))
    elif kind == "lifetime":
        return MemoryLiteralLifetime(
            lifetime=from_json_lifetime(json_field(object_, "lifetime"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Normalized memory access value."""
Access: typing.TypeAlias = (
    typing.Literal["readonly"] | typing.Literal["mutable"] | typing.Literal["exclusive"]
)


def encode_access(writer: BinaryWriter, value: Access) -> None:
    """Encode one Access."""
    if value == "readonly":
        writer.write_unsigned(0)
    elif value == "mutable":
        writer.write_unsigned(1)
    elif value == "exclusive":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_access(reader: BinaryReader) -> Access:
    """Decode one Access."""
    variant = reader.read_number()

    if variant == 0:
        return "readonly"
    elif variant == 1:
        return "mutable"
    elif variant == 2:
        return "exclusive"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_access(value: Access) -> Json:
    """Return one JSON value for one Access."""
    return value


def from_json_access(value: Json) -> Access:
    """Return one Access from one JSON value."""
    variant = json_string(value)

    if variant == "readonly":
        return "readonly"
    elif variant == "mutable":
        return "mutable"
    elif variant == "exclusive":
        return "exclusive"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Normalized storage space value."""
Space: typing.TypeAlias = (
    typing.Literal["local"]
    | typing.Literal["shared"]
    | typing.Literal["static"]
    | typing.Literal["frame"]
)


def encode_space(writer: BinaryWriter, value: Space) -> None:
    """Encode one Space."""
    if value == "local":
        writer.write_unsigned(0)
    elif value == "shared":
        writer.write_unsigned(1)
    elif value == "static":
        writer.write_unsigned(2)
    elif value == "frame":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_space(reader: BinaryReader) -> Space:
    """Decode one Space."""
    variant = reader.read_number()

    if variant == 0:
        return "local"
    elif variant == 1:
        return "shared"
    elif variant == 2:
        return "static"
    elif variant == 3:
        return "frame"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_space(value: Space) -> Json:
    """Return one JSON value for one Space."""
    return value


def from_json_space(value: Json) -> Space:
    """Return one Space from one JSON value."""
    variant = json_string(value)

    if variant == "local":
        return "local"
    elif variant == "shared":
        return "shared"
    elif variant == "static":
        return "static"
    elif variant == "frame":
        return "frame"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class PlaceAmbient:
    """Ambient placement."""

    kind: typing.Literal["ambient"] = "ambient"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_place(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_place(self)


@dataclass(frozen=True, slots=True)
class PlaceSpace:
    """Concrete storage space."""

    space: Space
    kind: typing.Literal["space"] = "space"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_place(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_place(self)


"""Normalized place value."""
Place: typing.TypeAlias = PlaceAmbient | PlaceSpace


def encode_place(writer: BinaryWriter, value: Place) -> None:
    """Encode one Place."""
    if value.kind == "ambient":
        writer.write_unsigned(0)
    elif value.kind == "space":
        writer.write_unsigned(1)
        encode_space(writer, value.space)
    else:
        raise SerdeError("unknown enum variant")


def decode_place(reader: BinaryReader) -> Place:
    """Decode one Place."""
    variant = reader.read_number()

    if variant == 0:
        return PlaceAmbient()
    elif variant == 1:
        space = decode_space(reader)

        return PlaceSpace(space=space)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_place(value: Place) -> Json:
    """Return one JSON value for one Place."""
    if value.kind == "ambient":
        return {
            "kind": "ambient",
        }
    elif value.kind == "space":
        return {
            "kind": "space",
            "space": to_json_space(value.space),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_place(value: Json) -> Place:
    """Return one Place from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "ambient":
        return PlaceAmbient()
    elif kind == "space":
        return PlaceSpace(space=from_json_space(json_field(object_, "space")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class LifetimeStatic:
    """Static lifetime."""

    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime(self)


@dataclass(frozen=True, slots=True)
class LifetimeFrame:
    """The enclosing frame's lifetime."""

    kind: typing.Literal["frame"] = "frame"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime(self)


@dataclass(frozen=True, slots=True)
class LifetimeSymbol:
    """Symbolic lifetime parameter or associated constant."""

    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime(self)


"""Normalized lifetime value."""
Lifetime: typing.TypeAlias = LifetimeStatic | LifetimeFrame | LifetimeSymbol


def encode_lifetime(writer: BinaryWriter, value: Lifetime) -> None:
    """Encode one Lifetime."""
    if value.kind == "static":
        writer.write_unsigned(0)
    elif value.kind == "frame":
        writer.write_unsigned(1)
    elif value.kind == "symbol":
        writer.write_unsigned(2)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.symbol
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_lifetime(reader: BinaryReader) -> Lifetime:
    """Decode one Lifetime."""
    variant = reader.read_number()

    if variant == 0:
        return LifetimeStatic()
    elif variant == 1:
        return LifetimeFrame()
    elif variant == 2:
        symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return LifetimeSymbol(symbol=symbol)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lifetime(value: Lifetime) -> Json:
    """Return one JSON value for one Lifetime."""
    if value.kind == "static":
        return {
            "kind": "static",
        }
    elif value.kind == "frame":
        return {
            "kind": "frame",
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.symbol
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_lifetime(value: Json) -> Lifetime:
    """Return one Lifetime from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "static":
        return LifetimeStatic()
    elif kind == "frame":
        return LifetimeFrame()
    elif kind == "symbol":
        return LifetimeSymbol(
            symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "symbol")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class GenericInstance:
    """One declaration applied to its complete positional arguments."""

    # the referenced declaration symbol
    symbol: destack._generated.dir.symbol.symbol.GlobalSymbolId
    # the complete positional arguments in declaration order
    arguments: Sequence[GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_generic_instance(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GenericInstance:
        """Decode one GenericInstance."""
        return decode_generic_instance(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_generic_instance(self)

    @classmethod
    def from_json(cls, value: Json) -> GenericInstance:
        """Return one GenericInstance from one JSON value."""
        return from_json_generic_instance(value)


def encode_generic_instance(writer: BinaryWriter, value: GenericInstance) -> None:
    """Encode one GenericInstance."""
    destack._generated.dir.symbol.symbol.encode_global_symbol_id(writer, value.symbol)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        encode_global_type_id(writer, item_value_arguments_0)


def decode_generic_instance(reader: BinaryReader) -> GenericInstance:
    """Decode one GenericInstance."""
    symbol = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)
    arguments = [decode_global_type_id(reader) for _ in range(reader.read_number())]

    return GenericInstance(
        symbol=symbol,
        arguments=arguments,
    )


def to_json_generic_instance(value: GenericInstance) -> Json:
    """Return one JSON value for one GenericInstance."""
    return {
        "symbol": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
            value.symbol
        ),
        "arguments": [to_json_global_type_id(item_0) for item_0 in value.arguments],
    }


def from_json_generic_instance(value: Json) -> GenericInstance:
    """Return one GenericInstance from one JSON value."""
    object_ = json_object(value)

    return GenericInstance(
        symbol=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
            json_field(object_, "symbol")
        ),
        arguments=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class GlobalTypeId:
    """Global type id across modules."""

    # the module id of the global type
    module_id: destack._generated.source.file.model.module.ModuleId
    # the local id of the global type
    local_id: LocalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_global_type_id(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> GlobalTypeId:
        """Decode one GlobalTypeId."""
        return decode_global_type_id(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_global_type_id(self)

    @classmethod
    def from_json(cls, value: Json) -> GlobalTypeId:
        """Return one GlobalTypeId from one JSON value."""
        return from_json_global_type_id(value)


def encode_global_type_id(writer: BinaryWriter, value: GlobalTypeId) -> None:
    """Encode one GlobalTypeId."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    encode_local_type_id(writer, value.local_id)


def decode_global_type_id(reader: BinaryReader) -> GlobalTypeId:
    """Decode one GlobalTypeId."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    local_id = decode_local_type_id(reader)

    return GlobalTypeId(
        module_id=module_id,
        local_id=local_id,
    )


def to_json_global_type_id(value: GlobalTypeId) -> Json:
    """Return one JSON value for one GlobalTypeId."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "localId": to_json_local_type_id(value.local_id),
    }


def from_json_global_type_id(value: Json) -> GlobalTypeId:
    """Return one GlobalTypeId from one JSON value."""
    object_ = json_object(value)

    return GlobalTypeId(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        local_id=from_json_local_type_id(json_field(object_, "localId")),
    )


"""Unique identifier for a local type."""
LocalTypeId: typing.TypeAlias = int


def encode_local_type_id(writer: BinaryWriter, value: LocalTypeId) -> None:
    """Encode one LocalTypeId."""
    writer.write_unsigned(value)


def decode_local_type_id(reader: BinaryReader) -> LocalTypeId:
    """Decode one LocalTypeId."""
    return reader.read_number()


def to_json_local_type_id(value: LocalTypeId) -> Json:
    """Return one JSON value for one LocalTypeId."""
    return value


def from_json_local_type_id(value: Json) -> LocalTypeId:
    """Return one LocalTypeId from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class MemberType:
    """Member type selected from an owner type."""

    # the owner type
    owner: GlobalTypeId
    # the selected member key
    key: destack._generated.dir.symbol.key.StaticKey
    # the complete positional arguments applied to the member
    arguments: Sequence[GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_member_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MemberType:
        """Decode one MemberType."""
        return decode_member_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_member_type(self)

    @classmethod
    def from_json(cls, value: Json) -> MemberType:
        """Return one MemberType from one JSON value."""
        return from_json_member_type(value)


def encode_member_type(writer: BinaryWriter, value: MemberType) -> None:
    """Encode one MemberType."""
    encode_global_type_id(writer, value.owner)
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        encode_global_type_id(writer, item_value_arguments_0)


def decode_member_type(reader: BinaryReader) -> MemberType:
    """Decode one MemberType."""
    owner = decode_global_type_id(reader)
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    arguments = [decode_global_type_id(reader) for _ in range(reader.read_number())]

    return MemberType(
        owner=owner,
        key=key,
        arguments=arguments,
    )


def to_json_member_type(value: MemberType) -> Json:
    """Return one JSON value for one MemberType."""
    return {
        "owner": to_json_global_type_id(value.owner),
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "arguments": [to_json_global_type_id(item_0) for item_0 in value.arguments],
    }


def from_json_member_type(value: Json) -> MemberType:
    """Return one MemberType from one JSON value."""
    object_ = json_object(value)

    return MemberType(
        owner=from_json_global_type_id(json_field(object_, "owner")),
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        arguments=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
    )


@dataclass(frozen=True, slots=True)
class FormType:
    """Canonical memory or access form."""

    # the form constructor
    form: Form
    # the type carried by the form
    value: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FormType:
        """Decode one FormType."""
        return decode_form_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FormType:
        """Return one FormType from one JSON value."""
        return from_json_form_type(value)


def encode_form_type(writer: BinaryWriter, value: FormType) -> None:
    """Encode one FormType."""
    encode_form(writer, value.form)
    encode_global_type_id(writer, value.value)


def decode_form_type(reader: BinaryReader) -> FormType:
    """Decode one FormType."""
    form = decode_form(reader)
    value_ = decode_global_type_id(reader)

    return FormType(
        form=form,
        value=value_,
    )


def to_json_form_type(value: FormType) -> Json:
    """Return one JSON value for one FormType."""
    return {
        "form": to_json_form(value.form),
        "value": to_json_global_type_id(value.value),
    }


def from_json_form_type(value: Json) -> FormType:
    """Return one FormType from one JSON value."""
    object_ = json_object(value)

    return FormType(
        form=from_json_form(json_field(object_, "form")),
        value=from_json_global_type_id(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class FormManaged:
    """Automatically managed runtime value, the unqualified `User`."""

    kind: typing.Literal["managed"] = "managed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormOwned:
    """Owned value, like `^User`."""

    kind: typing.Literal["owned"] = "owned"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormBorrowed:
    """Borrowed value, like `&User`, `&readonly User`, or `&exclusive User`."""

    # the solved borrow lifetime singleton
    lifetime: GlobalTypeId
    # the solved borrow access singleton
    access: GlobalTypeId
    kind: typing.Literal["borrowed"] = "borrowed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormRaw:
    """Raw pointer value, like `*User`."""

    kind: typing.Literal["raw"] = "raw"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormPlaced:
    """Placed value, like `local User` or `shared User`."""

    # the solved concrete or ambient place singleton
    place: GlobalTypeId
    kind: typing.Literal["placed"] = "placed"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


@dataclass(frozen=True, slots=True)
class FormReadonly:
    """Readonly view, like `readonly User`."""

    kind: typing.Literal["readonly"] = "readonly"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_form(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_form(self)


"""Canonical memory or access form constructor."""
Form: typing.TypeAlias = (
    FormManaged | FormOwned | FormBorrowed | FormRaw | FormPlaced | FormReadonly
)


def encode_form(writer: BinaryWriter, value: Form) -> None:
    """Encode one Form."""
    if value.kind == "managed":
        writer.write_unsigned(0)
    elif value.kind == "owned":
        writer.write_unsigned(1)
    elif value.kind == "borrowed":
        writer.write_unsigned(2)
        encode_global_type_id(writer, value.lifetime)
        encode_global_type_id(writer, value.access)
    elif value.kind == "raw":
        writer.write_unsigned(3)
    elif value.kind == "placed":
        writer.write_unsigned(4)
        encode_global_type_id(writer, value.place)
    elif value.kind == "readonly":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_form(reader: BinaryReader) -> Form:
    """Decode one Form."""
    variant = reader.read_number()

    if variant == 0:
        return FormManaged()
    elif variant == 1:
        return FormOwned()
    elif variant == 2:
        lifetime = decode_global_type_id(reader)
        access = decode_global_type_id(reader)

        return FormBorrowed(
            lifetime=lifetime,
            access=access,
        )
    elif variant == 3:
        return FormRaw()
    elif variant == 4:
        place = decode_global_type_id(reader)

        return FormPlaced(
            place=place,
        )
    elif variant == 5:
        return FormReadonly()
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_form(value: Form) -> Json:
    """Return one JSON value for one Form."""
    if value.kind == "managed":
        return {
            "kind": "managed",
        }
    elif value.kind == "owned":
        return {
            "kind": "owned",
        }
    elif value.kind == "borrowed":
        return {
            "kind": "borrowed",
            "lifetime": to_json_global_type_id(value.lifetime),
            "access": to_json_global_type_id(value.access),
        }
    elif value.kind == "raw":
        return {
            "kind": "raw",
        }
    elif value.kind == "placed":
        return {
            "kind": "placed",
            "place": to_json_global_type_id(value.place),
        }
    elif value.kind == "readonly":
        return {
            "kind": "readonly",
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_form(value: Json) -> Form:
    """Return one Form from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "managed":
        return FormManaged()
    elif kind == "owned":
        return FormOwned()
    elif kind == "borrowed":
        return FormBorrowed(
            lifetime=from_json_global_type_id(json_field(object_, "lifetime")),
            access=from_json_global_type_id(json_field(object_, "access")),
        )
    elif kind == "raw":
        return FormRaw()
    elif kind == "placed":
        return FormPlaced(
            place=from_json_global_type_id(json_field(object_, "place")),
        )
    elif kind == "readonly":
        return FormReadonly()
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DynamicType:
    """Explicit runtime `Dynamic<T>` representation."""

    # the `Dynamic<T>` constraint
    constraint: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dynamic_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DynamicType:
        """Decode one DynamicType."""
        return decode_dynamic_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dynamic_type(self)

    @classmethod
    def from_json(cls, value: Json) -> DynamicType:
        """Return one DynamicType from one JSON value."""
        return from_json_dynamic_type(value)


def encode_dynamic_type(writer: BinaryWriter, value: DynamicType) -> None:
    """Encode one DynamicType."""
    encode_global_type_id(writer, value.constraint)


def decode_dynamic_type(reader: BinaryReader) -> DynamicType:
    """Decode one DynamicType."""
    constraint = decode_global_type_id(reader)

    return DynamicType(
        constraint=constraint,
    )


def to_json_dynamic_type(value: DynamicType) -> Json:
    """Return one JSON value for one DynamicType."""
    return {
        "constraint": to_json_global_type_id(value.constraint),
    }


def from_json_dynamic_type(value: Json) -> DynamicType:
    """Return one DynamicType from one JSON value."""
    object_ = json_object(value)

    return DynamicType(
        constraint=from_json_global_type_id(json_field(object_, "constraint")),
    )


@dataclass(frozen=True, slots=True)
class TypeOperationStringMapping:
    """Compiler-known string mapping type, like `Uppercase<S>`."""

    # the string mapping operation
    mapping: StringMapping
    # the mapped string type
    target: GlobalTypeId
    kind: typing.Literal["stringMapping"] = "stringMapping"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationConditional:
    """Conditional type expression, like `T extends string ? A : B`."""

    conditional: ConditionalType
    kind: typing.Literal["conditional"] = "conditional"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationMapped:
    """Mapped type expression, like `{ [K in keyof T]: T[K] }`."""

    mapped: MappedType
    kind: typing.Literal["mapped"] = "mapped"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationIndex:
    """Indexed access type expression, like `User["name"]`."""

    index: IndexType
    kind: typing.Literal["index"] = "index"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationTemplateLiteral:
    """Template literal type expression, like `` `get${Name}` ``."""

    template_literal: TemplateLiteralType
    kind: typing.Literal["templateLiteral"] = "templateLiteral"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationInfer:
    """Type infer binding in a conditional type pattern, like `infer E`."""

    infer: InferType
    kind: typing.Literal["infer"] = "infer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationKeyOf:
    """`keyof T`."""

    key_of: UnaryType
    kind: typing.Literal["keyOf"] = "keyOf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationTryOutput:
    """Try success projection like `value?` continuing evaluation."""

    # the tried value type
    value: GlobalTypeId
    kind: typing.Literal["tryOutput"] = "tryOutput"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationTryResidual:
    """Try failure projection like `value?` propagating its residual."""

    # the tried value type
    value: GlobalTypeId
    kind: typing.Literal["tryResidual"] = "tryResidual"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationStaticBinary:
    """Static binary operation like `N * 2` or `Mode == "inline"`."""

    static_binary: StaticBinaryType
    kind: typing.Literal["staticBinary"] = "staticBinary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


@dataclass(frozen=True, slots=True)
class TypeOperationStaticUnary:
    """Static unary operation like `!Wide`."""

    static_unary: StaticUnaryType
    kind: typing.Literal["staticUnary"] = "staticUnary"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_operation(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_operation(self)


"""Type-level operation preserved by check."""
TypeOperation: typing.TypeAlias = (
    TypeOperationStringMapping
    | TypeOperationConditional
    | TypeOperationMapped
    | TypeOperationIndex
    | TypeOperationTemplateLiteral
    | TypeOperationInfer
    | TypeOperationKeyOf
    | TypeOperationTryOutput
    | TypeOperationTryResidual
    | TypeOperationStaticBinary
    | TypeOperationStaticUnary
)


def encode_type_operation(writer: BinaryWriter, value: TypeOperation) -> None:
    """Encode one TypeOperation."""
    if value.kind == "stringMapping":
        writer.write_unsigned(0)
        encode_string_mapping(writer, value.mapping)
        encode_global_type_id(writer, value.target)
    elif value.kind == "conditional":
        writer.write_unsigned(1)
        encode_conditional_type(writer, value.conditional)
    elif value.kind == "mapped":
        writer.write_unsigned(2)
        encode_mapped_type(writer, value.mapped)
    elif value.kind == "index":
        writer.write_unsigned(3)
        encode_index_type(writer, value.index)
    elif value.kind == "templateLiteral":
        writer.write_unsigned(4)
        encode_template_literal_type(writer, value.template_literal)
    elif value.kind == "infer":
        writer.write_unsigned(5)
        encode_infer_type(writer, value.infer)
    elif value.kind == "keyOf":
        writer.write_unsigned(6)
        encode_unary_type(writer, value.key_of)
    elif value.kind == "tryOutput":
        writer.write_unsigned(7)
        encode_global_type_id(writer, value.value)
    elif value.kind == "tryResidual":
        writer.write_unsigned(8)
        encode_global_type_id(writer, value.value)
    elif value.kind == "staticBinary":
        writer.write_unsigned(9)
        encode_static_binary_type(writer, value.static_binary)
    elif value.kind == "staticUnary":
        writer.write_unsigned(10)
        encode_static_unary_type(writer, value.static_unary)
    else:
        raise SerdeError("unknown enum variant")


def decode_type_operation(reader: BinaryReader) -> TypeOperation:
    """Decode one TypeOperation."""
    variant = reader.read_number()

    if variant == 0:
        mapping = decode_string_mapping(reader)
        target = decode_global_type_id(reader)

        return TypeOperationStringMapping(
            mapping=mapping,
            target=target,
        )
    elif variant == 1:
        conditional = decode_conditional_type(reader)

        return TypeOperationConditional(conditional=conditional)
    elif variant == 2:
        mapped = decode_mapped_type(reader)

        return TypeOperationMapped(mapped=mapped)
    elif variant == 3:
        index = decode_index_type(reader)

        return TypeOperationIndex(index=index)
    elif variant == 4:
        template_literal = decode_template_literal_type(reader)

        return TypeOperationTemplateLiteral(template_literal=template_literal)
    elif variant == 5:
        infer = decode_infer_type(reader)

        return TypeOperationInfer(infer=infer)
    elif variant == 6:
        key_of = decode_unary_type(reader)

        return TypeOperationKeyOf(key_of=key_of)
    elif variant == 7:
        value_ = decode_global_type_id(reader)

        return TypeOperationTryOutput(
            value=value_,
        )
    elif variant == 8:
        value_ = decode_global_type_id(reader)

        return TypeOperationTryResidual(
            value=value_,
        )
    elif variant == 9:
        static_binary = decode_static_binary_type(reader)

        return TypeOperationStaticBinary(static_binary=static_binary)
    elif variant == 10:
        static_unary = decode_static_unary_type(reader)

        return TypeOperationStaticUnary(static_unary=static_unary)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type_operation(value: TypeOperation) -> Json:
    """Return one JSON value for one TypeOperation."""
    if value.kind == "stringMapping":
        return {
            "kind": "stringMapping",
            "mapping": to_json_string_mapping(value.mapping),
            "target": to_json_global_type_id(value.target),
        }
    elif value.kind == "conditional":
        return {
            "kind": "conditional",
            "conditional": to_json_conditional_type(value.conditional),
        }
    elif value.kind == "mapped":
        return {
            "kind": "mapped",
            "mapped": to_json_mapped_type(value.mapped),
        }
    elif value.kind == "index":
        return {
            "kind": "index",
            "index": to_json_index_type(value.index),
        }
    elif value.kind == "templateLiteral":
        return {
            "kind": "templateLiteral",
            "template_literal": to_json_template_literal_type(value.template_literal),
        }
    elif value.kind == "infer":
        return {
            "kind": "infer",
            "infer": to_json_infer_type(value.infer),
        }
    elif value.kind == "keyOf":
        return {
            "kind": "keyOf",
            "key_of": to_json_unary_type(value.key_of),
        }
    elif value.kind == "tryOutput":
        return {
            "kind": "tryOutput",
            "value": to_json_global_type_id(value.value),
        }
    elif value.kind == "tryResidual":
        return {
            "kind": "tryResidual",
            "value": to_json_global_type_id(value.value),
        }
    elif value.kind == "staticBinary":
        return {
            "kind": "staticBinary",
            "static_binary": to_json_static_binary_type(value.static_binary),
        }
    elif value.kind == "staticUnary":
        return {
            "kind": "staticUnary",
            "static_unary": to_json_static_unary_type(value.static_unary),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type_operation(value: Json) -> TypeOperation:
    """Return one TypeOperation from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "stringMapping":
        return TypeOperationStringMapping(
            mapping=from_json_string_mapping(json_field(object_, "mapping")),
            target=from_json_global_type_id(json_field(object_, "target")),
        )
    elif kind == "conditional":
        return TypeOperationConditional(
            conditional=from_json_conditional_type(json_field(object_, "conditional"))
        )
    elif kind == "mapped":
        return TypeOperationMapped(
            mapped=from_json_mapped_type(json_field(object_, "mapped"))
        )
    elif kind == "index":
        return TypeOperationIndex(
            index=from_json_index_type(json_field(object_, "index"))
        )
    elif kind == "templateLiteral":
        return TypeOperationTemplateLiteral(
            template_literal=from_json_template_literal_type(
                json_field(object_, "template_literal")
            )
        )
    elif kind == "infer":
        return TypeOperationInfer(
            infer=from_json_infer_type(json_field(object_, "infer"))
        )
    elif kind == "keyOf":
        return TypeOperationKeyOf(
            key_of=from_json_unary_type(json_field(object_, "key_of"))
        )
    elif kind == "tryOutput":
        return TypeOperationTryOutput(
            value=from_json_global_type_id(json_field(object_, "value")),
        )
    elif kind == "tryResidual":
        return TypeOperationTryResidual(
            value=from_json_global_type_id(json_field(object_, "value")),
        )
    elif kind == "staticBinary":
        return TypeOperationStaticBinary(
            static_binary=from_json_static_binary_type(
                json_field(object_, "static_binary")
            )
        )
    elif kind == "staticUnary":
        return TypeOperationStaticUnary(
            static_unary=from_json_static_unary_type(
                json_field(object_, "static_unary")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Compiler-provided string mapping."""
StringMapping: typing.TypeAlias = (
    typing.Literal["uppercase"]
    | typing.Literal["lowercase"]
    | typing.Literal["capitalize"]
    | typing.Literal["uncapitalize"]
)


def encode_string_mapping(writer: BinaryWriter, value: StringMapping) -> None:
    """Encode one StringMapping."""
    if value == "uppercase":
        writer.write_unsigned(0)
    elif value == "lowercase":
        writer.write_unsigned(1)
    elif value == "capitalize":
        writer.write_unsigned(2)
    elif value == "uncapitalize":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_string_mapping(reader: BinaryReader) -> StringMapping:
    """Decode one StringMapping."""
    variant = reader.read_number()

    if variant == 0:
        return "uppercase"
    elif variant == 1:
        return "lowercase"
    elif variant == 2:
        return "capitalize"
    elif variant == 3:
        return "uncapitalize"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_string_mapping(value: StringMapping) -> Json:
    """Return one JSON value for one StringMapping."""
    return value


def from_json_string_mapping(value: Json) -> StringMapping:
    """Return one StringMapping from one JSON value."""
    variant = json_string(value)

    if variant == "uppercase":
        return "uppercase"
    elif variant == "lowercase":
        return "lowercase"
    elif variant == "capitalize":
        return "capitalize"
    elif variant == "uncapitalize":
        return "uncapitalize"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ConditionalType:
    """A conditional type."""

    # the left operand
    left: GlobalTypeId
    # the right operand
    right: GlobalTypeId
    # the type selected when the condition holds
    then_type: GlobalTypeId
    # the type selected when the condition does not hold
    else_type: GlobalTypeId
    # whether the conditional distributes over union-valued left operands
    is_distributive: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_conditional_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionalType:
        """Decode one ConditionalType."""
        return decode_conditional_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_conditional_type(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionalType:
        """Return one ConditionalType from one JSON value."""
        return from_json_conditional_type(value)


def encode_conditional_type(writer: BinaryWriter, value: ConditionalType) -> None:
    """Encode one ConditionalType."""
    encode_global_type_id(writer, value.left)
    encode_global_type_id(writer, value.right)
    encode_global_type_id(writer, value.then_type)
    encode_global_type_id(writer, value.else_type)
    writer.write_bool(value.is_distributive)


def decode_conditional_type(reader: BinaryReader) -> ConditionalType:
    """Decode one ConditionalType."""
    left = decode_global_type_id(reader)
    right = decode_global_type_id(reader)
    then_type = decode_global_type_id(reader)
    else_type = decode_global_type_id(reader)
    is_distributive = reader.read_bool()

    return ConditionalType(
        left=left,
        right=right,
        then_type=then_type,
        else_type=else_type,
        is_distributive=is_distributive,
    )


def to_json_conditional_type(value: ConditionalType) -> Json:
    """Return one JSON value for one ConditionalType."""
    return {
        "left": to_json_global_type_id(value.left),
        "right": to_json_global_type_id(value.right),
        "thenType": to_json_global_type_id(value.then_type),
        "elseType": to_json_global_type_id(value.else_type),
        "isDistributive": value.is_distributive,
    }


def from_json_conditional_type(value: Json) -> ConditionalType:
    """Return one ConditionalType from one JSON value."""
    object_ = json_object(value)

    return ConditionalType(
        left=from_json_global_type_id(json_field(object_, "left")),
        right=from_json_global_type_id(json_field(object_, "right")),
        then_type=from_json_global_type_id(json_field(object_, "thenType")),
        else_type=from_json_global_type_id(json_field(object_, "elseType")),
        is_distributive=json_bool(json_field(object_, "isDistributive")),
    )


@dataclass(frozen=True, slots=True)
class MappedType:
    """A mapped type."""

    # the mapped parameter
    parameter: MappedTypeParameter
    # the mapped modifiers
    modifiers: MappedTypeModifiers
    # the mapped value type
    value: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mapped_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedType:
        """Decode one MappedType."""
        return decode_mapped_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mapped_type(self)

    @classmethod
    def from_json(cls, value: Json) -> MappedType:
        """Return one MappedType from one JSON value."""
        return from_json_mapped_type(value)


def encode_mapped_type(writer: BinaryWriter, value: MappedType) -> None:
    """Encode one MappedType."""
    encode_mapped_type_parameter(writer, value.parameter)
    encode_mapped_type_modifiers(writer, value.modifiers)
    encode_global_type_id(writer, value.value)


def decode_mapped_type(reader: BinaryReader) -> MappedType:
    """Decode one MappedType."""
    parameter = decode_mapped_type_parameter(reader)
    modifiers = decode_mapped_type_modifiers(reader)
    value_ = decode_global_type_id(reader)

    return MappedType(
        parameter=parameter,
        modifiers=modifiers,
        value=value_,
    )


def to_json_mapped_type(value: MappedType) -> Json:
    """Return one JSON value for one MappedType."""
    return {
        "parameter": to_json_mapped_type_parameter(value.parameter),
        "modifiers": to_json_mapped_type_modifiers(value.modifiers),
        "value": to_json_global_type_id(value.value),
    }


def from_json_mapped_type(value: Json) -> MappedType:
    """Return one MappedType from one JSON value."""
    object_ = json_object(value)

    return MappedType(
        parameter=from_json_mapped_type_parameter(json_field(object_, "parameter")),
        modifiers=from_json_mapped_type_modifiers(json_field(object_, "modifiers")),
        value=from_json_global_type_id(json_field(object_, "value")),
    )


@dataclass(frozen=True, slots=True)
class MappedTypeParameter:
    """A mapped-type parameter."""

    # the parameter name like `K`
    name: destack._generated.core.string.StringId
    # the binder's generic parameter
    parameter: destack._generated.dir.type.generic.GlobalGenericParameterId
    # the constraint type like `keyof T`
    constraint: GlobalTypeId
    # the optional key remap like `as Foo<K>`
    key_remap: GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mapped_type_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedTypeParameter:
        """Decode one MappedTypeParameter."""
        return decode_mapped_type_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mapped_type_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> MappedTypeParameter:
        """Return one MappedTypeParameter from one JSON value."""
        return from_json_mapped_type_parameter(value)


def encode_mapped_type_parameter(
    writer: BinaryWriter, value: MappedTypeParameter
) -> None:
    """Encode one MappedTypeParameter."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    destack._generated.dir.type.generic.encode_global_generic_parameter_id(
        writer, value.parameter
    )
    encode_global_type_id(writer, value.constraint)
    if value.key_remap is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_global_type_id(writer, value.key_remap)


def decode_mapped_type_parameter(reader: BinaryReader) -> MappedTypeParameter:
    """Decode one MappedTypeParameter."""
    name = destack._generated.core.string.decode_string_id(reader)
    parameter = destack._generated.dir.type.generic.decode_global_generic_parameter_id(
        reader
    )
    constraint = decode_global_type_id(reader)
    key_remap = reader.read_option(lambda: decode_global_type_id(reader))

    return MappedTypeParameter(
        name=name,
        parameter=parameter,
        constraint=constraint,
        key_remap=key_remap,
    )


def to_json_mapped_type_parameter(value: MappedTypeParameter) -> Json:
    """Return one JSON value for one MappedTypeParameter."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "parameter": destack._generated.dir.type.generic.to_json_global_generic_parameter_id(
            value.parameter
        ),
        "constraint": to_json_global_type_id(value.constraint),
        **(
            {}
            if value.key_remap is None
            else {"keyRemap": to_json_global_type_id(value.key_remap)}
        ),
    }


def from_json_mapped_type_parameter(value: Json) -> MappedTypeParameter:
    """Return one MappedTypeParameter from one JSON value."""
    object_ = json_object(value)

    return MappedTypeParameter(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        parameter=destack._generated.dir.type.generic.from_json_global_generic_parameter_id(
            json_field(object_, "parameter")
        ),
        constraint=from_json_global_type_id(json_field(object_, "constraint")),
        key_remap=json_optional(
            object_, "keyRemap", lambda value: from_json_global_type_id(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class MappedTypeModifiers:
    """Mapped-type modifiers."""

    # the readonly modifier
    readonly: destack._generated.dir.tree.type.MappedTypeModifier
    # the optional modifier
    optional: destack._generated.dir.tree.type.MappedTypeModifier

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mapped_type_modifiers(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MappedTypeModifiers:
        """Decode one MappedTypeModifiers."""
        return decode_mapped_type_modifiers(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mapped_type_modifiers(self)

    @classmethod
    def from_json(cls, value: Json) -> MappedTypeModifiers:
        """Return one MappedTypeModifiers from one JSON value."""
        return from_json_mapped_type_modifiers(value)


def encode_mapped_type_modifiers(
    writer: BinaryWriter, value: MappedTypeModifiers
) -> None:
    """Encode one MappedTypeModifiers."""
    destack._generated.dir.tree.type.encode_mapped_type_modifier(writer, value.readonly)
    destack._generated.dir.tree.type.encode_mapped_type_modifier(writer, value.optional)


def decode_mapped_type_modifiers(reader: BinaryReader) -> MappedTypeModifiers:
    """Decode one MappedTypeModifiers."""
    readonly = destack._generated.dir.tree.type.decode_mapped_type_modifier(reader)
    optional = destack._generated.dir.tree.type.decode_mapped_type_modifier(reader)

    return MappedTypeModifiers(
        readonly=readonly,
        optional=optional,
    )


def to_json_mapped_type_modifiers(value: MappedTypeModifiers) -> Json:
    """Return one JSON value for one MappedTypeModifiers."""
    return {
        "readonly": destack._generated.dir.tree.type.to_json_mapped_type_modifier(
            value.readonly
        ),
        "optional": destack._generated.dir.tree.type.to_json_mapped_type_modifier(
            value.optional
        ),
    }


def from_json_mapped_type_modifiers(value: Json) -> MappedTypeModifiers:
    """Return one MappedTypeModifiers from one JSON value."""
    object_ = json_object(value)

    return MappedTypeModifiers(
        readonly=destack._generated.dir.tree.type.from_json_mapped_type_modifier(
            json_field(object_, "readonly")
        ),
        optional=destack._generated.dir.tree.type.from_json_mapped_type_modifier(
            json_field(object_, "optional")
        ),
    )


@dataclass(frozen=True, slots=True)
class IndexType:
    """Indexed access type."""

    # the indexed type
    left: GlobalTypeId
    # the index type
    index: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_index_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IndexType:
        """Decode one IndexType."""
        return decode_index_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_index_type(self)

    @classmethod
    def from_json(cls, value: Json) -> IndexType:
        """Return one IndexType from one JSON value."""
        return from_json_index_type(value)


def encode_index_type(writer: BinaryWriter, value: IndexType) -> None:
    """Encode one IndexType."""
    encode_global_type_id(writer, value.left)
    encode_global_type_id(writer, value.index)


def decode_index_type(reader: BinaryReader) -> IndexType:
    """Decode one IndexType."""
    left = decode_global_type_id(reader)
    index = decode_global_type_id(reader)

    return IndexType(
        left=left,
        index=index,
    )


def to_json_index_type(value: IndexType) -> Json:
    """Return one JSON value for one IndexType."""
    return {
        "left": to_json_global_type_id(value.left),
        "index": to_json_global_type_id(value.index),
    }


def from_json_index_type(value: Json) -> IndexType:
    """Return one IndexType from one JSON value."""
    object_ = json_object(value)

    return IndexType(
        left=from_json_global_type_id(json_field(object_, "left")),
        index=from_json_global_type_id(json_field(object_, "index")),
    )


@dataclass(frozen=True, slots=True)
class TemplateLiteralType:
    """A template literal type."""

    # the literal string segments
    strings: Sequence[destack._generated.core.string.StringId]
    # the interpolated type spans
    spans: Sequence[GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_template_literal_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TemplateLiteralType:
        """Decode one TemplateLiteralType."""
        return decode_template_literal_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_template_literal_type(self)

    @classmethod
    def from_json(cls, value: Json) -> TemplateLiteralType:
        """Return one TemplateLiteralType from one JSON value."""
        return from_json_template_literal_type(value)


def encode_template_literal_type(
    writer: BinaryWriter, value: TemplateLiteralType
) -> None:
    """Encode one TemplateLiteralType."""
    writer.write_unsigned(len(value.strings))
    for item_value_strings_0 in value.strings:
        destack._generated.core.string.encode_string_id(writer, item_value_strings_0)
    writer.write_unsigned(len(value.spans))
    for item_value_spans_0 in value.spans:
        encode_global_type_id(writer, item_value_spans_0)


def decode_template_literal_type(reader: BinaryReader) -> TemplateLiteralType:
    """Decode one TemplateLiteralType."""
    strings = [
        destack._generated.core.string.decode_string_id(reader)
        for _ in range(reader.read_number())
    ]
    spans = [decode_global_type_id(reader) for _ in range(reader.read_number())]

    return TemplateLiteralType(
        strings=strings,
        spans=spans,
    )


def to_json_template_literal_type(value: TemplateLiteralType) -> Json:
    """Return one JSON value for one TemplateLiteralType."""
    return {
        "strings": [
            destack._generated.core.string.to_json_string_id(item_0)
            for item_0 in value.strings
        ],
        "spans": [to_json_global_type_id(item_0) for item_0 in value.spans],
    }


def from_json_template_literal_type(value: Json) -> TemplateLiteralType:
    """Return one TemplateLiteralType from one JSON value."""
    object_ = json_object(value)

    return TemplateLiteralType(
        strings=[
            destack._generated.core.string.from_json_string_id(item_0)
            for item_0 in json_array(json_field(object_, "strings"))
        ],
        spans=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "spans"))
        ],
    )


@dataclass(frozen=True, slots=True)
class InferType:
    """An infer binding inside a conditional type pattern."""

    # the inferred binding name
    name: destack._generated.core.string.StringId | None
    # the optional inferred constraint
    constraint: GlobalTypeId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_infer_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InferType:
        """Decode one InferType."""
        return decode_infer_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_infer_type(self)

    @classmethod
    def from_json(cls, value: Json) -> InferType:
        """Return one InferType from one JSON value."""
        return from_json_infer_type(value)


def encode_infer_type(writer: BinaryWriter, value: InferType) -> None:
    """Encode one InferType."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)
    if value.constraint is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_global_type_id(writer, value.constraint)


def decode_infer_type(reader: BinaryReader) -> InferType:
    """Decode one InferType."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    constraint = reader.read_option(lambda: decode_global_type_id(reader))

    return InferType(
        name=name,
        constraint=constraint,
    )


def to_json_infer_type(value: InferType) -> Json:
    """Return one JSON value for one InferType."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
        **(
            {}
            if value.constraint is None
            else {"constraint": to_json_global_type_id(value.constraint)}
        ),
    }


def from_json_infer_type(value: Json) -> InferType:
    """Return one InferType from one JSON value."""
    object_ = json_object(value)

    return InferType(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        constraint=json_optional(
            object_, "constraint", lambda value: from_json_global_type_id(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class UnaryType:
    """A unary type operator."""

    # the target type
    target: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_unary_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> UnaryType:
        """Decode one UnaryType."""
        return decode_unary_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_unary_type(self)

    @classmethod
    def from_json(cls, value: Json) -> UnaryType:
        """Return one UnaryType from one JSON value."""
        return from_json_unary_type(value)


def encode_unary_type(writer: BinaryWriter, value: UnaryType) -> None:
    """Encode one UnaryType."""
    encode_global_type_id(writer, value.target)


def decode_unary_type(reader: BinaryReader) -> UnaryType:
    """Decode one UnaryType."""
    target = decode_global_type_id(reader)

    return UnaryType(
        target=target,
    )


def to_json_unary_type(value: UnaryType) -> Json:
    """Return one JSON value for one UnaryType."""
    return {
        "target": to_json_global_type_id(value.target),
    }


def from_json_unary_type(value: Json) -> UnaryType:
    """Return one UnaryType from one JSON value."""
    object_ = json_object(value)

    return UnaryType(
        target=from_json_global_type_id(json_field(object_, "target")),
    )


@dataclass(frozen=True, slots=True)
class StaticBinaryType:
    """One static binary operation over singleton operands."""

    # the applied operator
    operator: StaticBinaryOperator
    # the left operand
    left: GlobalTypeId
    # the right operand
    right: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_binary_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticBinaryType:
        """Decode one StaticBinaryType."""
        return decode_static_binary_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_binary_type(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticBinaryType:
        """Return one StaticBinaryType from one JSON value."""
        return from_json_static_binary_type(value)


def encode_static_binary_type(writer: BinaryWriter, value: StaticBinaryType) -> None:
    """Encode one StaticBinaryType."""
    encode_static_binary_operator(writer, value.operator)
    encode_global_type_id(writer, value.left)
    encode_global_type_id(writer, value.right)


def decode_static_binary_type(reader: BinaryReader) -> StaticBinaryType:
    """Decode one StaticBinaryType."""
    operator = decode_static_binary_operator(reader)
    left = decode_global_type_id(reader)
    right = decode_global_type_id(reader)

    return StaticBinaryType(
        operator=operator,
        left=left,
        right=right,
    )


def to_json_static_binary_type(value: StaticBinaryType) -> Json:
    """Return one JSON value for one StaticBinaryType."""
    return {
        "operator": to_json_static_binary_operator(value.operator),
        "left": to_json_global_type_id(value.left),
        "right": to_json_global_type_id(value.right),
    }


def from_json_static_binary_type(value: Json) -> StaticBinaryType:
    """Return one StaticBinaryType from one JSON value."""
    object_ = json_object(value)

    return StaticBinaryType(
        operator=from_json_static_binary_operator(json_field(object_, "operator")),
        left=from_json_global_type_id(json_field(object_, "left")),
        right=from_json_global_type_id(json_field(object_, "right")),
    )


"""One static binary operator."""
StaticBinaryOperator: typing.TypeAlias = (
    typing.Literal["add"]
    | typing.Literal["subtract"]
    | typing.Literal["multiply"]
    | typing.Literal["divide"]
    | typing.Literal["remainder"]
    | typing.Literal["exponent"]
    | typing.Literal["shiftLeft"]
    | typing.Literal["shiftRight"]
    | typing.Literal["unsignedShiftRight"]
    | typing.Literal["bitwiseAnd"]
    | typing.Literal["bitwiseXor"]
    | typing.Literal["bitwiseOr"]
    | typing.Literal["equal"]
    | typing.Literal["equalStrict"]
    | typing.Literal["notEqual"]
    | typing.Literal["notEqualStrict"]
    | typing.Literal["lessThan"]
    | typing.Literal["lessThanOrEqual"]
    | typing.Literal["greaterThan"]
    | typing.Literal["greaterThanOrEqual"]
    | typing.Literal["and"]
    | typing.Literal["or"]
)


def encode_static_binary_operator(
    writer: BinaryWriter, value: StaticBinaryOperator
) -> None:
    """Encode one StaticBinaryOperator."""
    if value == "add":
        writer.write_unsigned(0)
    elif value == "subtract":
        writer.write_unsigned(1)
    elif value == "multiply":
        writer.write_unsigned(2)
    elif value == "divide":
        writer.write_unsigned(3)
    elif value == "remainder":
        writer.write_unsigned(4)
    elif value == "exponent":
        writer.write_unsigned(5)
    elif value == "shiftLeft":
        writer.write_unsigned(6)
    elif value == "shiftRight":
        writer.write_unsigned(7)
    elif value == "unsignedShiftRight":
        writer.write_unsigned(8)
    elif value == "bitwiseAnd":
        writer.write_unsigned(9)
    elif value == "bitwiseXor":
        writer.write_unsigned(10)
    elif value == "bitwiseOr":
        writer.write_unsigned(11)
    elif value == "equal":
        writer.write_unsigned(12)
    elif value == "equalStrict":
        writer.write_unsigned(13)
    elif value == "notEqual":
        writer.write_unsigned(14)
    elif value == "notEqualStrict":
        writer.write_unsigned(15)
    elif value == "lessThan":
        writer.write_unsigned(16)
    elif value == "lessThanOrEqual":
        writer.write_unsigned(17)
    elif value == "greaterThan":
        writer.write_unsigned(18)
    elif value == "greaterThanOrEqual":
        writer.write_unsigned(19)
    elif value == "and":
        writer.write_unsigned(20)
    elif value == "or":
        writer.write_unsigned(21)
    else:
        raise SerdeError("unknown enum variant")


def decode_static_binary_operator(reader: BinaryReader) -> StaticBinaryOperator:
    """Decode one StaticBinaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "add"
    elif variant == 1:
        return "subtract"
    elif variant == 2:
        return "multiply"
    elif variant == 3:
        return "divide"
    elif variant == 4:
        return "remainder"
    elif variant == 5:
        return "exponent"
    elif variant == 6:
        return "shiftLeft"
    elif variant == 7:
        return "shiftRight"
    elif variant == 8:
        return "unsignedShiftRight"
    elif variant == 9:
        return "bitwiseAnd"
    elif variant == 10:
        return "bitwiseXor"
    elif variant == 11:
        return "bitwiseOr"
    elif variant == 12:
        return "equal"
    elif variant == 13:
        return "equalStrict"
    elif variant == 14:
        return "notEqual"
    elif variant == 15:
        return "notEqualStrict"
    elif variant == 16:
        return "lessThan"
    elif variant == 17:
        return "lessThanOrEqual"
    elif variant == 18:
        return "greaterThan"
    elif variant == 19:
        return "greaterThanOrEqual"
    elif variant == 20:
        return "and"
    elif variant == 21:
        return "or"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_static_binary_operator(value: StaticBinaryOperator) -> Json:
    """Return one JSON value for one StaticBinaryOperator."""
    return value


def from_json_static_binary_operator(value: Json) -> StaticBinaryOperator:
    """Return one StaticBinaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "add":
        return "add"
    elif variant == "subtract":
        return "subtract"
    elif variant == "multiply":
        return "multiply"
    elif variant == "divide":
        return "divide"
    elif variant == "remainder":
        return "remainder"
    elif variant == "exponent":
        return "exponent"
    elif variant == "shiftLeft":
        return "shiftLeft"
    elif variant == "shiftRight":
        return "shiftRight"
    elif variant == "unsignedShiftRight":
        return "unsignedShiftRight"
    elif variant == "bitwiseAnd":
        return "bitwiseAnd"
    elif variant == "bitwiseXor":
        return "bitwiseXor"
    elif variant == "bitwiseOr":
        return "bitwiseOr"
    elif variant == "equal":
        return "equal"
    elif variant == "equalStrict":
        return "equalStrict"
    elif variant == "notEqual":
        return "notEqual"
    elif variant == "notEqualStrict":
        return "notEqualStrict"
    elif variant == "lessThan":
        return "lessThan"
    elif variant == "lessThanOrEqual":
        return "lessThanOrEqual"
    elif variant == "greaterThan":
        return "greaterThan"
    elif variant == "greaterThanOrEqual":
        return "greaterThanOrEqual"
    elif variant == "and":
        return "and"
    elif variant == "or":
        return "or"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class StaticUnaryType:
    """One static unary operation over one singleton operand."""

    # the applied operator
    operator: StaticUnaryOperator
    # the operand
    target: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_static_unary_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> StaticUnaryType:
        """Decode one StaticUnaryType."""
        return decode_static_unary_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_static_unary_type(self)

    @classmethod
    def from_json(cls, value: Json) -> StaticUnaryType:
        """Return one StaticUnaryType from one JSON value."""
        return from_json_static_unary_type(value)


def encode_static_unary_type(writer: BinaryWriter, value: StaticUnaryType) -> None:
    """Encode one StaticUnaryType."""
    encode_static_unary_operator(writer, value.operator)
    encode_global_type_id(writer, value.target)


def decode_static_unary_type(reader: BinaryReader) -> StaticUnaryType:
    """Decode one StaticUnaryType."""
    operator = decode_static_unary_operator(reader)
    target = decode_global_type_id(reader)

    return StaticUnaryType(
        operator=operator,
        target=target,
    )


def to_json_static_unary_type(value: StaticUnaryType) -> Json:
    """Return one JSON value for one StaticUnaryType."""
    return {
        "operator": to_json_static_unary_operator(value.operator),
        "target": to_json_global_type_id(value.target),
    }


def from_json_static_unary_type(value: Json) -> StaticUnaryType:
    """Return one StaticUnaryType from one JSON value."""
    object_ = json_object(value)

    return StaticUnaryType(
        operator=from_json_static_unary_operator(json_field(object_, "operator")),
        target=from_json_global_type_id(json_field(object_, "target")),
    )


"""One static unary operator."""
StaticUnaryOperator: typing.TypeAlias = (
    typing.Literal["not"] | typing.Literal["negate"] | typing.Literal["bitwiseNot"]
)


def encode_static_unary_operator(
    writer: BinaryWriter, value: StaticUnaryOperator
) -> None:
    """Encode one StaticUnaryOperator."""
    if value == "not":
        writer.write_unsigned(0)
    elif value == "negate":
        writer.write_unsigned(1)
    elif value == "bitwiseNot":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_static_unary_operator(reader: BinaryReader) -> StaticUnaryOperator:
    """Decode one StaticUnaryOperator."""
    variant = reader.read_number()

    if variant == 0:
        return "not"
    elif variant == 1:
        return "negate"
    elif variant == 2:
        return "bitwiseNot"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_static_unary_operator(value: StaticUnaryOperator) -> Json:
    """Return one JSON value for one StaticUnaryOperator."""
    return value


def from_json_static_unary_operator(value: Json) -> StaticUnaryOperator:
    """Return one StaticUnaryOperator from one JSON value."""
    variant = json_string(value)

    if variant == "not":
        return "not"
    elif variant == "negate":
        return "negate"
    elif variant == "bitwiseNot":
        return "bitwiseNot"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class ArrayType:
    """Homogeneous array type."""

    # the element type
    element: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_array_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ArrayType:
        """Decode one ArrayType."""
        return decode_array_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_array_type(self)

    @classmethod
    def from_json(cls, value: Json) -> ArrayType:
        """Return one ArrayType from one JSON value."""
        return from_json_array_type(value)


def encode_array_type(writer: BinaryWriter, value: ArrayType) -> None:
    """Encode one ArrayType."""
    encode_global_type_id(writer, value.element)


def decode_array_type(reader: BinaryReader) -> ArrayType:
    """Decode one ArrayType."""
    element = decode_global_type_id(reader)

    return ArrayType(
        element=element,
    )


def to_json_array_type(value: ArrayType) -> Json:
    """Return one JSON value for one ArrayType."""
    return {
        "element": to_json_global_type_id(value.element),
    }


def from_json_array_type(value: Json) -> ArrayType:
    """Return one ArrayType from one JSON value."""
    object_ = json_object(value)

    return ArrayType(
        element=from_json_global_type_id(json_field(object_, "element")),
    )


@dataclass(frozen=True, slots=True)
class FixedArrayType:
    """A fixed-length array type."""

    # the element type
    element: GlobalTypeId
    # the static array length singleton
    count: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_fixed_array_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FixedArrayType:
        """Decode one FixedArrayType."""
        return decode_fixed_array_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_fixed_array_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FixedArrayType:
        """Return one FixedArrayType from one JSON value."""
        return from_json_fixed_array_type(value)


def encode_fixed_array_type(writer: BinaryWriter, value: FixedArrayType) -> None:
    """Encode one FixedArrayType."""
    encode_global_type_id(writer, value.element)
    encode_global_type_id(writer, value.count)


def decode_fixed_array_type(reader: BinaryReader) -> FixedArrayType:
    """Decode one FixedArrayType."""
    element = decode_global_type_id(reader)
    count = decode_global_type_id(reader)

    return FixedArrayType(
        element=element,
        count=count,
    )


def to_json_fixed_array_type(value: FixedArrayType) -> Json:
    """Return one JSON value for one FixedArrayType."""
    return {
        "element": to_json_global_type_id(value.element),
        "count": to_json_global_type_id(value.count),
    }


def from_json_fixed_array_type(value: Json) -> FixedArrayType:
    """Return one FixedArrayType from one JSON value."""
    object_ = json_object(value)

    return FixedArrayType(
        element=from_json_global_type_id(json_field(object_, "element")),
        count=from_json_global_type_id(json_field(object_, "count")),
    )


@dataclass(frozen=True, slots=True)
class RangeType:
    """Compact scalar interval type."""

    # the inclusive lower bound
    start: destack._generated.dir.tree.literal.ScalarLiteral | None
    # the upper bound
    end: destack._generated.dir.tree.literal.ScalarLiteral | None
    # whether the upper bound is included
    is_inclusive: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_range_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RangeType:
        """Decode one RangeType."""
        return decode_range_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_range_type(self)

    @classmethod
    def from_json(cls, value: Json) -> RangeType:
        """Return one RangeType from one JSON value."""
        return from_json_range_type(value)


def encode_range_type(writer: BinaryWriter, value: RangeType) -> None:
    """Encode one RangeType."""
    if value.start is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.start)
    if value.end is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.end)
    writer.write_bool(value.is_inclusive)


def decode_range_type(reader: BinaryReader) -> RangeType:
    """Decode one RangeType."""
    start = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    end = reader.read_option(
        lambda: destack._generated.dir.tree.literal.decode_scalar_literal(reader)
    )
    is_inclusive = reader.read_bool()

    return RangeType(
        start=start,
        end=end,
        is_inclusive=is_inclusive,
    )


def to_json_range_type(value: RangeType) -> Json:
    """Return one JSON value for one RangeType."""
    return {
        **(
            {}
            if value.start is None
            else {
                "start": destack._generated.dir.tree.literal.to_json_scalar_literal(
                    value.start
                )
            }
        ),
        **(
            {}
            if value.end is None
            else {
                "end": destack._generated.dir.tree.literal.to_json_scalar_literal(
                    value.end
                )
            }
        ),
        "isInclusive": value.is_inclusive,
    }


def from_json_range_type(value: Json) -> RangeType:
    """Return one RangeType from one JSON value."""
    object_ = json_object(value)

    return RangeType(
        start=json_optional(
            object_,
            "start",
            lambda value: destack._generated.dir.tree.literal.from_json_scalar_literal(
                value
            ),
        ),
        end=json_optional(
            object_,
            "end",
            lambda value: destack._generated.dir.tree.literal.from_json_scalar_literal(
                value
            ),
        ),
        is_inclusive=json_bool(json_field(object_, "isInclusive")),
    )


@dataclass(frozen=True, slots=True)
class SliceType:
    """Runtime-length homogeneous view type."""

    # the element type
    element: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_slice_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SliceType:
        """Decode one SliceType."""
        return decode_slice_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_slice_type(self)

    @classmethod
    def from_json(cls, value: Json) -> SliceType:
        """Return one SliceType from one JSON value."""
        return from_json_slice_type(value)


def encode_slice_type(writer: BinaryWriter, value: SliceType) -> None:
    """Encode one SliceType."""
    encode_global_type_id(writer, value.element)


def decode_slice_type(reader: BinaryReader) -> SliceType:
    """Decode one SliceType."""
    element = decode_global_type_id(reader)

    return SliceType(
        element=element,
    )


def to_json_slice_type(value: SliceType) -> Json:
    """Return one JSON value for one SliceType."""
    return {
        "element": to_json_global_type_id(value.element),
    }


def from_json_slice_type(value: Json) -> SliceType:
    """Return one SliceType from one JSON value."""
    object_ = json_object(value)

    return SliceType(
        element=from_json_global_type_id(json_field(object_, "element")),
    )


@dataclass(frozen=True, slots=True)
class TupleType:
    """A tuple type."""

    # the tuple source form
    form: TupleForm
    # the tuple elements
    elements: Sequence[TypeElement]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tuple_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TupleType:
        """Decode one TupleType."""
        return decode_tuple_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tuple_type(self)

    @classmethod
    def from_json(cls, value: Json) -> TupleType:
        """Return one TupleType from one JSON value."""
        return from_json_tuple_type(value)


def encode_tuple_type(writer: BinaryWriter, value: TupleType) -> None:
    """Encode one TupleType."""
    encode_tuple_form(writer, value.form)
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        encode_type_element(writer, item_value_elements_0)


def decode_tuple_type(reader: BinaryReader) -> TupleType:
    """Decode one TupleType."""
    form = decode_tuple_form(reader)
    elements = [decode_type_element(reader) for _ in range(reader.read_number())]

    return TupleType(
        form=form,
        elements=elements,
    )


def to_json_tuple_type(value: TupleType) -> Json:
    """Return one JSON value for one TupleType."""
    return {
        "form": to_json_tuple_form(value.form),
        "elements": [to_json_type_element(item_0) for item_0 in value.elements],
    }


def from_json_tuple_type(value: Json) -> TupleType:
    """Return one TupleType from one JSON value."""
    object_ = json_object(value)

    return TupleType(
        form=from_json_tuple_form(json_field(object_, "form")),
        elements=[
            from_json_type_element(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


"""The source form of a tuple type."""
TupleForm: typing.TypeAlias = typing.Literal["tuple"] | typing.Literal["array"]


def encode_tuple_form(writer: BinaryWriter, value: TupleForm) -> None:
    """Encode one TupleForm."""
    if value == "tuple":
        writer.write_unsigned(0)
    elif value == "array":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_tuple_form(reader: BinaryReader) -> TupleForm:
    """Decode one TupleForm."""
    variant = reader.read_number()

    if variant == 0:
        return "tuple"
    elif variant == 1:
        return "array"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_tuple_form(value: TupleForm) -> Json:
    """Return one JSON value for one TupleForm."""
    return value


def from_json_tuple_form(value: Json) -> TupleForm:
    """Return one TupleForm from one JSON value."""
    variant = json_string(value)

    if variant == "tuple":
        return "tuple"
    elif variant == "array":
        return "array"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TypeElement:
    """An element in a tuple type."""

    # the optional label for the element
    label: destack._generated.core.string.StringId | None
    # the element type
    ty: GlobalTypeId
    # whether the element is optional
    is_optional: bool
    # whether the element is readonly
    is_readonly: bool
    # whether the element is a rest element
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_element(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeElement:
        """Decode one TypeElement."""
        return decode_type_element(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_element(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeElement:
        """Return one TypeElement from one JSON value."""
        return from_json_type_element(value)


def encode_type_element(writer: BinaryWriter, value: TypeElement) -> None:
    """Encode one TypeElement."""
    if value.label is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.label)
    encode_global_type_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)
    writer.write_bool(value.is_rest)


def decode_type_element(reader: BinaryReader) -> TypeElement:
    """Decode one TypeElement."""
    label = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )
    ty = decode_global_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()
    is_rest = reader.read_bool()

    return TypeElement(
        label=label,
        ty=ty,
        is_optional=is_optional,
        is_readonly=is_readonly,
        is_rest=is_rest,
    )


def to_json_type_element(value: TypeElement) -> Json:
    """Return one JSON value for one TypeElement."""
    return {
        **(
            {}
            if value.label is None
            else {
                "label": destack._generated.core.string.to_json_string_id(value.label)
            }
        ),
        "ty": to_json_global_type_id(value.ty),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
        "isRest": value.is_rest,
    }


def from_json_type_element(value: Json) -> TypeElement:
    """Return one TypeElement from one JSON value."""
    object_ = json_object(value)

    return TypeElement(
        label=json_optional(
            object_,
            "label",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
        ty=from_json_global_type_id(json_field(object_, "ty")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
        is_rest=json_bool(json_field(object_, "isRest")),
    )


@dataclass(frozen=True, slots=True)
class ShapeType:
    """A structural object shape type."""

    # the shape fields
    fields: Sequence[TypeField]
    # the call signatures
    call_signatures: Sequence[GlobalTypeId]
    # the construct signatures
    construct_signatures: Sequence[GlobalTypeId]
    # the index signatures
    index_signatures: Sequence[TypeIndexSignature]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_shape_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ShapeType:
        """Decode one ShapeType."""
        return decode_shape_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_shape_type(self)

    @classmethod
    def from_json(cls, value: Json) -> ShapeType:
        """Return one ShapeType from one JSON value."""
        return from_json_shape_type(value)


def encode_shape_type(writer: BinaryWriter, value: ShapeType) -> None:
    """Encode one ShapeType."""
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        encode_type_field(writer, item_value_fields_0)
    writer.write_unsigned(len(value.call_signatures))
    for item_value_call_signatures_0 in value.call_signatures:
        encode_global_type_id(writer, item_value_call_signatures_0)
    writer.write_unsigned(len(value.construct_signatures))
    for item_value_construct_signatures_0 in value.construct_signatures:
        encode_global_type_id(writer, item_value_construct_signatures_0)
    writer.write_unsigned(len(value.index_signatures))
    for item_value_index_signatures_0 in value.index_signatures:
        encode_type_index_signature(writer, item_value_index_signatures_0)


def decode_shape_type(reader: BinaryReader) -> ShapeType:
    """Decode one ShapeType."""
    fields = [decode_type_field(reader) for _ in range(reader.read_number())]
    call_signatures = [
        decode_global_type_id(reader) for _ in range(reader.read_number())
    ]
    construct_signatures = [
        decode_global_type_id(reader) for _ in range(reader.read_number())
    ]
    index_signatures = [
        decode_type_index_signature(reader) for _ in range(reader.read_number())
    ]

    return ShapeType(
        fields=fields,
        call_signatures=call_signatures,
        construct_signatures=construct_signatures,
        index_signatures=index_signatures,
    )


def to_json_shape_type(value: ShapeType) -> Json:
    """Return one JSON value for one ShapeType."""
    return {
        "fields": [to_json_type_field(item_0) for item_0 in value.fields],
        "callSignatures": [
            to_json_global_type_id(item_0) for item_0 in value.call_signatures
        ],
        "constructSignatures": [
            to_json_global_type_id(item_0) for item_0 in value.construct_signatures
        ],
        "indexSignatures": [
            to_json_type_index_signature(item_0) for item_0 in value.index_signatures
        ],
    }


def from_json_shape_type(value: Json) -> ShapeType:
    """Return one ShapeType from one JSON value."""
    object_ = json_object(value)

    return ShapeType(
        fields=[
            from_json_type_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        call_signatures=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "callSignatures"))
        ],
        construct_signatures=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "constructSignatures"))
        ],
        index_signatures=[
            from_json_type_index_signature(item_0)
            for item_0 in json_array(json_field(object_, "indexSignatures"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TypeField:
    """A field in an object-like type."""

    # the key of the field
    key: destack._generated.dir.symbol.key.StaticKey
    # the type of the field
    ty: GlobalTypeId
    # whether the field is optional
    is_optional: bool
    # whether the field is readonly
    is_readonly: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_field(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeField:
        """Decode one TypeField."""
        return decode_type_field(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_field(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeField:
        """Return one TypeField from one JSON value."""
        return from_json_type_field(value)


def encode_type_field(writer: BinaryWriter, value: TypeField) -> None:
    """Encode one TypeField."""
    destack._generated.dir.symbol.key.encode_static_key(writer, value.key)
    encode_global_type_id(writer, value.ty)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)


def decode_type_field(reader: BinaryReader) -> TypeField:
    """Decode one TypeField."""
    key = destack._generated.dir.symbol.key.decode_static_key(reader)
    ty = decode_global_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()

    return TypeField(
        key=key,
        ty=ty,
        is_optional=is_optional,
        is_readonly=is_readonly,
    )


def to_json_type_field(value: TypeField) -> Json:
    """Return one JSON value for one TypeField."""
    return {
        "key": destack._generated.dir.symbol.key.to_json_static_key(value.key),
        "ty": to_json_global_type_id(value.ty),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
    }


def from_json_type_field(value: Json) -> TypeField:
    """Return one TypeField from one JSON value."""
    object_ = json_object(value)

    return TypeField(
        key=destack._generated.dir.symbol.key.from_json_static_key(
            json_field(object_, "key")
        ),
        ty=from_json_global_type_id(json_field(object_, "ty")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
    )


@dataclass(frozen=True, slots=True)
class TypeIndexSignature:
    """An index signature in an object type."""

    # the parameter name like `K`
    name: destack._generated.core.string.StringId
    # the key type
    key_type: GlobalTypeId
    # the value type
    value_type: GlobalTypeId
    # whether the index signature is optional
    is_optional: bool
    # whether the index signature is readonly
    is_readonly: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type_index_signature(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TypeIndexSignature:
        """Decode one TypeIndexSignature."""
        return decode_type_index_signature(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type_index_signature(self)

    @classmethod
    def from_json(cls, value: Json) -> TypeIndexSignature:
        """Return one TypeIndexSignature from one JSON value."""
        return from_json_type_index_signature(value)


def encode_type_index_signature(
    writer: BinaryWriter, value: TypeIndexSignature
) -> None:
    """Encode one TypeIndexSignature."""
    destack._generated.core.string.encode_string_id(writer, value.name)
    encode_global_type_id(writer, value.key_type)
    encode_global_type_id(writer, value.value_type)
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_readonly)


def decode_type_index_signature(reader: BinaryReader) -> TypeIndexSignature:
    """Decode one TypeIndexSignature."""
    name = destack._generated.core.string.decode_string_id(reader)
    key_type = decode_global_type_id(reader)
    value_type = decode_global_type_id(reader)
    is_optional = reader.read_bool()
    is_readonly = reader.read_bool()

    return TypeIndexSignature(
        name=name,
        key_type=key_type,
        value_type=value_type,
        is_optional=is_optional,
        is_readonly=is_readonly,
    )


def to_json_type_index_signature(value: TypeIndexSignature) -> Json:
    """Return one JSON value for one TypeIndexSignature."""
    return {
        "name": destack._generated.core.string.to_json_string_id(value.name),
        "keyType": to_json_global_type_id(value.key_type),
        "valueType": to_json_global_type_id(value.value_type),
        "isOptional": value.is_optional,
        "isReadonly": value.is_readonly,
    }


def from_json_type_index_signature(value: Json) -> TypeIndexSignature:
    """Return one TypeIndexSignature from one JSON value."""
    object_ = json_object(value)

    return TypeIndexSignature(
        name=destack._generated.core.string.from_json_string_id(
            json_field(object_, "name")
        ),
        key_type=from_json_global_type_id(json_field(object_, "keyType")),
        value_type=from_json_global_type_id(json_field(object_, "valueType")),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_readonly=json_bool(json_field(object_, "isReadonly")),
    )


@dataclass(frozen=True, slots=True)
class FunctionSignatureType:
    """A function signature type."""

    # the function asynchrony
    asynchrony: destack._generated.dir.tree.node.Asynchrony
    # the generic parameter types
    generic_parameters: Sequence[GlobalTypeId]
    # the optional `this` parameter type
    this_parameter: GlobalTypeId | None
    # the runtime parameters
    parameters: Sequence[FunctionParameterType]
    # the optional return type
    return_type: GlobalTypeId | None
    # whether this is a generator function
    is_generator: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_signature_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionSignatureType:
        """Decode one FunctionSignatureType."""
        return decode_function_signature_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_signature_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionSignatureType:
        """Return one FunctionSignatureType from one JSON value."""
        return from_json_function_signature_type(value)


def encode_function_signature_type(
    writer: BinaryWriter, value: FunctionSignatureType
) -> None:
    """Encode one FunctionSignatureType."""
    destack._generated.dir.tree.node.encode_asynchrony(writer, value.asynchrony)
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        encode_global_type_id(writer, item_value_generic_parameters_0)
    if value.this_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_global_type_id(writer, value.this_parameter)
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        encode_function_parameter_type(writer, item_value_parameters_0)
    if value.return_type is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_global_type_id(writer, value.return_type)
    writer.write_bool(value.is_generator)


def decode_function_signature_type(reader: BinaryReader) -> FunctionSignatureType:
    """Decode one FunctionSignatureType."""
    asynchrony = destack._generated.dir.tree.node.decode_asynchrony(reader)
    generic_parameters = [
        decode_global_type_id(reader) for _ in range(reader.read_number())
    ]
    this_parameter = reader.read_option(lambda: decode_global_type_id(reader))
    parameters = [
        decode_function_parameter_type(reader) for _ in range(reader.read_number())
    ]
    return_type = reader.read_option(lambda: decode_global_type_id(reader))
    is_generator = reader.read_bool()

    return FunctionSignatureType(
        asynchrony=asynchrony,
        generic_parameters=generic_parameters,
        this_parameter=this_parameter,
        parameters=parameters,
        return_type=return_type,
        is_generator=is_generator,
    )


def to_json_function_signature_type(value: FunctionSignatureType) -> Json:
    """Return one JSON value for one FunctionSignatureType."""
    return {
        "asynchrony": destack._generated.dir.tree.node.to_json_asynchrony(
            value.asynchrony
        ),
        "genericParameters": [
            to_json_global_type_id(item_0) for item_0 in value.generic_parameters
        ],
        **(
            {}
            if value.this_parameter is None
            else {"thisParameter": to_json_global_type_id(value.this_parameter)}
        ),
        "parameters": [
            to_json_function_parameter_type(item_0) for item_0 in value.parameters
        ],
        **(
            {}
            if value.return_type is None
            else {"returnType": to_json_global_type_id(value.return_type)}
        ),
        "isGenerator": value.is_generator,
    }


def from_json_function_signature_type(value: Json) -> FunctionSignatureType:
    """Return one FunctionSignatureType from one JSON value."""
    object_ = json_object(value)

    return FunctionSignatureType(
        asynchrony=destack._generated.dir.tree.node.from_json_asynchrony(
            json_field(object_, "asynchrony")
        ),
        generic_parameters=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        this_parameter=json_optional(
            object_, "thisParameter", lambda value: from_json_global_type_id(value)
        ),
        parameters=[
            from_json_function_parameter_type(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        return_type=json_optional(
            object_, "returnType", lambda value: from_json_global_type_id(value)
        ),
        is_generator=json_bool(json_field(object_, "isGenerator")),
    )


@dataclass(frozen=True, slots=True)
class FunctionParameterType:
    """A runtime parameter in a function type."""

    # the parameter type
    ty: GlobalTypeId
    # the static generic parameter supplied by this runtime argument
    static_parameter: (
        destack._generated.dir.type.generic.GlobalGenericParameterId | None
    )
    # whether the parameter may be omitted at the call site
    is_optional: bool
    # whether the parameter captures remaining call arguments
    is_rest: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_parameter_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionParameterType:
        """Decode one FunctionParameterType."""
        return decode_function_parameter_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_parameter_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionParameterType:
        """Return one FunctionParameterType from one JSON value."""
        return from_json_function_parameter_type(value)


def encode_function_parameter_type(
    writer: BinaryWriter, value: FunctionParameterType
) -> None:
    """Encode one FunctionParameterType."""
    encode_global_type_id(writer, value.ty)
    if value.static_parameter is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.dir.type.generic.encode_global_generic_parameter_id(
            writer, value.static_parameter
        )
    writer.write_bool(value.is_optional)
    writer.write_bool(value.is_rest)


def decode_function_parameter_type(reader: BinaryReader) -> FunctionParameterType:
    """Decode one FunctionParameterType."""
    ty = decode_global_type_id(reader)
    static_parameter = reader.read_option(
        lambda: destack._generated.dir.type.generic.decode_global_generic_parameter_id(
            reader
        )
    )
    is_optional = reader.read_bool()
    is_rest = reader.read_bool()

    return FunctionParameterType(
        ty=ty,
        static_parameter=static_parameter,
        is_optional=is_optional,
        is_rest=is_rest,
    )


def to_json_function_parameter_type(value: FunctionParameterType) -> Json:
    """Return one JSON value for one FunctionParameterType."""
    return {
        "ty": to_json_global_type_id(value.ty),
        **(
            {}
            if value.static_parameter is None
            else {
                "staticParameter": destack._generated.dir.type.generic.to_json_global_generic_parameter_id(
                    value.static_parameter
                )
            }
        ),
        "isOptional": value.is_optional,
        "isRest": value.is_rest,
    }


def from_json_function_parameter_type(value: Json) -> FunctionParameterType:
    """Return one FunctionParameterType from one JSON value."""
    object_ = json_object(value)

    return FunctionParameterType(
        ty=from_json_global_type_id(json_field(object_, "ty")),
        static_parameter=json_optional(
            object_,
            "staticParameter",
            lambda value: (
                destack._generated.dir.type.generic.from_json_global_generic_parameter_id(
                    value
                )
            ),
        ),
        is_optional=json_bool(json_field(object_, "isOptional")),
        is_rest=json_bool(json_field(object_, "isRest")),
    )


@dataclass(frozen=True, slots=True)
class FunctionType:
    """A fat callable value with a function signature and captured environment."""

    # the function signature
    signature: GlobalTypeId
    # the captured environment type
    environment: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionType:
        """Decode one FunctionType."""
        return decode_function_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionType:
        """Return one FunctionType from one JSON value."""
        return from_json_function_type(value)


def encode_function_type(writer: BinaryWriter, value: FunctionType) -> None:
    """Encode one FunctionType."""
    encode_global_type_id(writer, value.signature)
    encode_global_type_id(writer, value.environment)


def decode_function_type(reader: BinaryReader) -> FunctionType:
    """Decode one FunctionType."""
    signature = decode_global_type_id(reader)
    environment = decode_global_type_id(reader)

    return FunctionType(
        signature=signature,
        environment=environment,
    )


def to_json_function_type(value: FunctionType) -> Json:
    """Return one JSON value for one FunctionType."""
    return {
        "signature": to_json_global_type_id(value.signature),
        "environment": to_json_global_type_id(value.environment),
    }


def from_json_function_type(value: Json) -> FunctionType:
    """Return one FunctionType from one JSON value."""
    object_ = json_object(value)

    return FunctionType(
        signature=from_json_global_type_id(json_field(object_, "signature")),
        environment=from_json_global_type_id(json_field(object_, "environment")),
    )


@dataclass(frozen=True, slots=True)
class FunctionPointerType:
    """A thin callable value with no captured environment."""

    # the function signature
    signature: GlobalTypeId

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_function_pointer_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> FunctionPointerType:
        """Decode one FunctionPointerType."""
        return decode_function_pointer_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_function_pointer_type(self)

    @classmethod
    def from_json(cls, value: Json) -> FunctionPointerType:
        """Return one FunctionPointerType from one JSON value."""
        return from_json_function_pointer_type(value)


def encode_function_pointer_type(
    writer: BinaryWriter, value: FunctionPointerType
) -> None:
    """Encode one FunctionPointerType."""
    encode_global_type_id(writer, value.signature)


def decode_function_pointer_type(reader: BinaryReader) -> FunctionPointerType:
    """Decode one FunctionPointerType."""
    signature = decode_global_type_id(reader)

    return FunctionPointerType(
        signature=signature,
    )


def to_json_function_pointer_type(value: FunctionPointerType) -> Json:
    """Return one JSON value for one FunctionPointerType."""
    return {
        "signature": to_json_global_type_id(value.signature),
    }


def from_json_function_pointer_type(value: Json) -> FunctionPointerType:
    """Return one FunctionPointerType from one JSON value."""
    object_ = json_object(value)

    return FunctionPointerType(
        signature=from_json_global_type_id(json_field(object_, "signature")),
    )


@dataclass(frozen=True, slots=True)
class UnionType:
    """A union type."""

    # the union elements
    elements: Sequence[GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_union_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> UnionType:
        """Decode one UnionType."""
        return decode_union_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_union_type(self)

    @classmethod
    def from_json(cls, value: Json) -> UnionType:
        """Return one UnionType from one JSON value."""
        return from_json_union_type(value)


def encode_union_type(writer: BinaryWriter, value: UnionType) -> None:
    """Encode one UnionType."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        encode_global_type_id(writer, item_value_elements_0)


def decode_union_type(reader: BinaryReader) -> UnionType:
    """Decode one UnionType."""
    elements = [decode_global_type_id(reader) for _ in range(reader.read_number())]

    return UnionType(
        elements=elements,
    )


def to_json_union_type(value: UnionType) -> Json:
    """Return one JSON value for one UnionType."""
    return {
        "elements": [to_json_global_type_id(item_0) for item_0 in value.elements],
    }


def from_json_union_type(value: Json) -> UnionType:
    """Return one UnionType from one JSON value."""
    object_ = json_object(value)

    return UnionType(
        elements=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class IntersectionType:
    """An intersection type."""

    # the intersection elements
    elements: Sequence[GlobalTypeId]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_intersection_type(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> IntersectionType:
        """Decode one IntersectionType."""
        return decode_intersection_type(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_intersection_type(self)

    @classmethod
    def from_json(cls, value: Json) -> IntersectionType:
        """Return one IntersectionType from one JSON value."""
        return from_json_intersection_type(value)


def encode_intersection_type(writer: BinaryWriter, value: IntersectionType) -> None:
    """Encode one IntersectionType."""
    writer.write_unsigned(len(value.elements))
    for item_value_elements_0 in value.elements:
        encode_global_type_id(writer, item_value_elements_0)


def decode_intersection_type(reader: BinaryReader) -> IntersectionType:
    """Decode one IntersectionType."""
    elements = [decode_global_type_id(reader) for _ in range(reader.read_number())]

    return IntersectionType(
        elements=elements,
    )


def to_json_intersection_type(value: IntersectionType) -> Json:
    """Return one JSON value for one IntersectionType."""
    return {
        "elements": [to_json_global_type_id(item_0) for item_0 in value.elements],
    }


def from_json_intersection_type(value: Json) -> IntersectionType:
    """Return one IntersectionType from one JSON value."""
    object_ = json_object(value)

    return IntersectionType(
        elements=[
            from_json_global_type_id(item_0)
            for item_0 in json_array(json_field(object_, "elements"))
        ],
    )


@dataclass(frozen=True, slots=True)
class TypeVariable:
    """One open inference variable."""

    variable: TypeVariableId
    kind: typing.Literal["variable"] = "variable"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeError:
    """Error type that could not be resolved."""

    kind: typing.Literal["error"] = "error"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeNever:
    """Never type `never`."""

    kind: typing.Literal["never"] = "never"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeAny:
    """TypeScript `any` compatibility marker."""

    kind: typing.Literal["any"] = "any"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeUnknown:
    """Unknown type."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeVoid:
    """Void type."""

    kind: typing.Literal["void"] = "void"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeNull:
    """Null type and value."""

    kind: typing.Literal["null"] = "null"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeUndefined:
    """Undefined type and value."""

    kind: typing.Literal["undefined"] = "undefined"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeObject:
    """TypeScript `object` constraint."""

    kind: typing.Literal["object"] = "object"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypePrimitive:
    """Primitive type, like `string` or `int32`."""

    primitive: destack._generated.dir.type.primitive.PrimitiveType
    kind: typing.Literal["primitive"] = "primitive"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeLiteral:
    """Scalar literal type, like `"id"` or `42`."""

    literal: destack._generated.dir.tree.literal.ScalarLiteral
    kind: typing.Literal["literal"] = "literal"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeMemory:
    """Singleton type of one normalized memory value."""

    memory: MemoryLiteral
    kind: typing.Literal["memory"] = "memory"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeStatic:
    """Singleton type of one committed static value."""

    static: destack._generated.dir.tree.static.GlobalStaticId
    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeIntrinsic:
    """Compiler intrinsic type body."""

    kind: typing.Literal["intrinsic"] = "intrinsic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeParameter:
    """Generic parameter, like the `T` in `class Box<T>`."""

    parameter: destack._generated.dir.type.generic.GlobalGenericParameterId
    kind: typing.Literal["parameter"] = "parameter"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeReference:
    """Type declaration reference, like `User` or `Map<string, User>`."""

    reference: GenericInstance
    kind: typing.Literal["reference"] = "reference"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeThis:
    """This type in a method signature, like `this` in `clone(): this`."""

    kind: typing.Literal["this"] = "this"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeMember:
    """Member type selected from an owner type, like `T.Output`."""

    member: MemberType
    kind: typing.Literal["member"] = "member"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeForm:
    """Canonical memory or access form, like `^User` or `&exclusive User`."""

    form: FormType
    kind: typing.Literal["form"] = "form"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeDynamic:
    """Explicit runtime `Dynamic<T>` representation, like `Dynamic<Printable>`."""

    dynamic: DynamicType
    kind: typing.Literal["dynamic"] = "dynamic"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeOperationVariant:
    """Type-level operation preserved by check."""

    operation: TypeOperation
    kind: typing.Literal["operation"] = "operation"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeArray:
    """Homogeneous array type, like `int32[]`."""

    array: ArrayType
    kind: typing.Literal["array"] = "array"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFixedArray:
    """Fixed-length array type, like `[uint8; 4]`."""

    fixed_array: FixedArrayType
    kind: typing.Literal["fixedArray"] = "fixedArray"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeRange:
    """Compact scalar interval type, like `0..10`."""

    range: RangeType
    kind: typing.Literal["range"] = "range"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeSlice:
    """Runtime-length homogeneous view type, like `[uint8]`."""

    slice: SliceType
    kind: typing.Literal["slice"] = "slice"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeTuple:
    """Tuple type, like `(string, int32)`."""

    tuple: TupleType
    kind: typing.Literal["tuple"] = "tuple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeShape:
    """Structural object shape type, like `{ name: string }`."""

    shape: ShapeType
    kind: typing.Literal["shape"] = "shape"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunctionSignature:
    """Function signature type, like `(value: int32) => string`."""

    function_signature: FunctionSignatureType
    kind: typing.Literal["functionSignature"] = "functionSignature"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunction:
    """Fat callable value with an explicit captured environment."""

    function: FunctionType
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeFunctionPointer:
    """Thin callable value with no captured environment."""

    function_pointer: FunctionPointerType
    kind: typing.Literal["functionPointer"] = "functionPointer"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeUnion:
    """Union type `A | B | C`."""

    union: UnionType
    kind: typing.Literal["union"] = "union"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


@dataclass(frozen=True, slots=True)
class TypeIntersection:
    """Intersection type `A & B & C`."""

    intersection: IntersectionType
    kind: typing.Literal["intersection"] = "intersection"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_type(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_type(self)


"""A canonical solved type."""
Type: typing.TypeAlias = (
    TypeVariable
    | TypeError
    | TypeNever
    | TypeAny
    | TypeUnknown
    | TypeVoid
    | TypeNull
    | TypeUndefined
    | TypeObject
    | TypePrimitive
    | TypeLiteral
    | TypeMemory
    | TypeStatic
    | TypeIntrinsic
    | TypeParameter
    | TypeReference
    | TypeThis
    | TypeMember
    | TypeForm
    | TypeDynamic
    | TypeOperationVariant
    | TypeArray
    | TypeFixedArray
    | TypeRange
    | TypeSlice
    | TypeTuple
    | TypeShape
    | TypeFunctionSignature
    | TypeFunction
    | TypeFunctionPointer
    | TypeUnion
    | TypeIntersection
)


def encode_type(writer: BinaryWriter, value: Type) -> None:
    """Encode one Type."""
    if value.kind == "variable":
        writer.write_unsigned(0)
        encode_type_variable_id(writer, value.variable)
    elif value.kind == "error":
        writer.write_unsigned(1)
    elif value.kind == "never":
        writer.write_unsigned(2)
    elif value.kind == "any":
        writer.write_unsigned(3)
    elif value.kind == "unknown":
        writer.write_unsigned(4)
    elif value.kind == "void":
        writer.write_unsigned(5)
    elif value.kind == "null":
        writer.write_unsigned(6)
    elif value.kind == "undefined":
        writer.write_unsigned(7)
    elif value.kind == "object":
        writer.write_unsigned(8)
    elif value.kind == "primitive":
        writer.write_unsigned(9)
        destack._generated.dir.type.primitive.encode_primitive_type(
            writer, value.primitive
        )
    elif value.kind == "literal":
        writer.write_unsigned(10)
        destack._generated.dir.tree.literal.encode_scalar_literal(writer, value.literal)
    elif value.kind == "memory":
        writer.write_unsigned(11)
        encode_memory_literal(writer, value.memory)
    elif value.kind == "static":
        writer.write_unsigned(12)
        destack._generated.dir.tree.static.encode_global_static_id(writer, value.static)
    elif value.kind == "intrinsic":
        writer.write_unsigned(13)
    elif value.kind == "parameter":
        writer.write_unsigned(14)
        destack._generated.dir.type.generic.encode_global_generic_parameter_id(
            writer, value.parameter
        )
    elif value.kind == "reference":
        writer.write_unsigned(15)
        encode_generic_instance(writer, value.reference)
    elif value.kind == "this":
        writer.write_unsigned(16)
    elif value.kind == "member":
        writer.write_unsigned(17)
        encode_member_type(writer, value.member)
    elif value.kind == "form":
        writer.write_unsigned(18)
        encode_form_type(writer, value.form)
    elif value.kind == "dynamic":
        writer.write_unsigned(19)
        encode_dynamic_type(writer, value.dynamic)
    elif value.kind == "operation":
        writer.write_unsigned(20)
        encode_type_operation(writer, value.operation)
    elif value.kind == "array":
        writer.write_unsigned(21)
        encode_array_type(writer, value.array)
    elif value.kind == "fixedArray":
        writer.write_unsigned(22)
        encode_fixed_array_type(writer, value.fixed_array)
    elif value.kind == "range":
        writer.write_unsigned(23)
        encode_range_type(writer, value.range)
    elif value.kind == "slice":
        writer.write_unsigned(24)
        encode_slice_type(writer, value.slice)
    elif value.kind == "tuple":
        writer.write_unsigned(25)
        encode_tuple_type(writer, value.tuple)
    elif value.kind == "shape":
        writer.write_unsigned(26)
        encode_shape_type(writer, value.shape)
    elif value.kind == "functionSignature":
        writer.write_unsigned(27)
        encode_function_signature_type(writer, value.function_signature)
    elif value.kind == "function":
        writer.write_unsigned(28)
        encode_function_type(writer, value.function)
    elif value.kind == "functionPointer":
        writer.write_unsigned(29)
        encode_function_pointer_type(writer, value.function_pointer)
    elif value.kind == "union":
        writer.write_unsigned(30)
        encode_union_type(writer, value.union)
    elif value.kind == "intersection":
        writer.write_unsigned(31)
        encode_intersection_type(writer, value.intersection)
    else:
        raise SerdeError("unknown enum variant")


def decode_type(reader: BinaryReader) -> Type:
    """Decode one Type."""
    variant = reader.read_number()

    if variant == 0:
        variable = decode_type_variable_id(reader)

        return TypeVariable(variable=variable)
    elif variant == 1:
        return TypeError()
    elif variant == 2:
        return TypeNever()
    elif variant == 3:
        return TypeAny()
    elif variant == 4:
        return TypeUnknown()
    elif variant == 5:
        return TypeVoid()
    elif variant == 6:
        return TypeNull()
    elif variant == 7:
        return TypeUndefined()
    elif variant == 8:
        return TypeObject()
    elif variant == 9:
        primitive = destack._generated.dir.type.primitive.decode_primitive_type(reader)

        return TypePrimitive(primitive=primitive)
    elif variant == 10:
        literal = destack._generated.dir.tree.literal.decode_scalar_literal(reader)

        return TypeLiteral(literal=literal)
    elif variant == 11:
        memory = decode_memory_literal(reader)

        return TypeMemory(memory=memory)
    elif variant == 12:
        static = destack._generated.dir.tree.static.decode_global_static_id(reader)

        return TypeStatic(static=static)
    elif variant == 13:
        return TypeIntrinsic()
    elif variant == 14:
        parameter = (
            destack._generated.dir.type.generic.decode_global_generic_parameter_id(
                reader
            )
        )

        return TypeParameter(parameter=parameter)
    elif variant == 15:
        reference = decode_generic_instance(reader)

        return TypeReference(reference=reference)
    elif variant == 16:
        return TypeThis()
    elif variant == 17:
        member = decode_member_type(reader)

        return TypeMember(member=member)
    elif variant == 18:
        form = decode_form_type(reader)

        return TypeForm(form=form)
    elif variant == 19:
        dynamic = decode_dynamic_type(reader)

        return TypeDynamic(dynamic=dynamic)
    elif variant == 20:
        operation = decode_type_operation(reader)

        return TypeOperationVariant(operation=operation)
    elif variant == 21:
        array = decode_array_type(reader)

        return TypeArray(array=array)
    elif variant == 22:
        fixed_array = decode_fixed_array_type(reader)

        return TypeFixedArray(fixed_array=fixed_array)
    elif variant == 23:
        range_ = decode_range_type(reader)

        return TypeRange(range=range_)
    elif variant == 24:
        slice = decode_slice_type(reader)

        return TypeSlice(slice=slice)
    elif variant == 25:
        tuple = decode_tuple_type(reader)

        return TypeTuple(tuple=tuple)
    elif variant == 26:
        shape = decode_shape_type(reader)

        return TypeShape(shape=shape)
    elif variant == 27:
        function_signature = decode_function_signature_type(reader)

        return TypeFunctionSignature(function_signature=function_signature)
    elif variant == 28:
        function = decode_function_type(reader)

        return TypeFunction(function=function)
    elif variant == 29:
        function_pointer = decode_function_pointer_type(reader)

        return TypeFunctionPointer(function_pointer=function_pointer)
    elif variant == 30:
        union = decode_union_type(reader)

        return TypeUnion(union=union)
    elif variant == 31:
        intersection = decode_intersection_type(reader)

        return TypeIntersection(intersection=intersection)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_type(value: Type) -> Json:
    """Return one JSON value for one Type."""
    if value.kind == "variable":
        return {
            "kind": "variable",
            "variable": to_json_type_variable_id(value.variable),
        }
    elif value.kind == "error":
        return {
            "kind": "error",
        }
    elif value.kind == "never":
        return {
            "kind": "never",
        }
    elif value.kind == "any":
        return {
            "kind": "any",
        }
    elif value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    elif value.kind == "void":
        return {
            "kind": "void",
        }
    elif value.kind == "null":
        return {
            "kind": "null",
        }
    elif value.kind == "undefined":
        return {
            "kind": "undefined",
        }
    elif value.kind == "object":
        return {
            "kind": "object",
        }
    elif value.kind == "primitive":
        return {
            "kind": "primitive",
            "primitive": destack._generated.dir.type.primitive.to_json_primitive_type(
                value.primitive
            ),
        }
    elif value.kind == "literal":
        return {
            "kind": "literal",
            "literal": destack._generated.dir.tree.literal.to_json_scalar_literal(
                value.literal
            ),
        }
    elif value.kind == "memory":
        return {
            "kind": "memory",
            "memory": to_json_memory_literal(value.memory),
        }
    elif value.kind == "static":
        return {
            "kind": "static",
            "static": destack._generated.dir.tree.static.to_json_global_static_id(
                value.static
            ),
        }
    elif value.kind == "intrinsic":
        return {
            "kind": "intrinsic",
        }
    elif value.kind == "parameter":
        return {
            "kind": "parameter",
            "parameter": destack._generated.dir.type.generic.to_json_global_generic_parameter_id(
                value.parameter
            ),
        }
    elif value.kind == "reference":
        return {
            "kind": "reference",
            "reference": to_json_generic_instance(value.reference),
        }
    elif value.kind == "this":
        return {
            "kind": "this",
        }
    elif value.kind == "member":
        return {
            "kind": "member",
            "member": to_json_member_type(value.member),
        }
    elif value.kind == "form":
        return {
            "kind": "form",
            "form": to_json_form_type(value.form),
        }
    elif value.kind == "dynamic":
        return {
            "kind": "dynamic",
            "dynamic": to_json_dynamic_type(value.dynamic),
        }
    elif value.kind == "operation":
        return {
            "kind": "operation",
            "operation": to_json_type_operation(value.operation),
        }
    elif value.kind == "array":
        return {
            "kind": "array",
            "array": to_json_array_type(value.array),
        }
    elif value.kind == "fixedArray":
        return {
            "kind": "fixedArray",
            "fixed_array": to_json_fixed_array_type(value.fixed_array),
        }
    elif value.kind == "range":
        return {
            "kind": "range",
            "range": to_json_range_type(value.range),
        }
    elif value.kind == "slice":
        return {
            "kind": "slice",
            "slice": to_json_slice_type(value.slice),
        }
    elif value.kind == "tuple":
        return {
            "kind": "tuple",
            "tuple": to_json_tuple_type(value.tuple),
        }
    elif value.kind == "shape":
        return {
            "kind": "shape",
            "shape": to_json_shape_type(value.shape),
        }
    elif value.kind == "functionSignature":
        return {
            "kind": "functionSignature",
            "function_signature": to_json_function_signature_type(
                value.function_signature
            ),
        }
    elif value.kind == "function":
        return {
            "kind": "function",
            "function": to_json_function_type(value.function),
        }
    elif value.kind == "functionPointer":
        return {
            "kind": "functionPointer",
            "function_pointer": to_json_function_pointer_type(value.function_pointer),
        }
    elif value.kind == "union":
        return {
            "kind": "union",
            "union": to_json_union_type(value.union),
        }
    elif value.kind == "intersection":
        return {
            "kind": "intersection",
            "intersection": to_json_intersection_type(value.intersection),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_type(value: Json) -> Type:
    """Return one Type from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "variable":
        return TypeVariable(
            variable=from_json_type_variable_id(json_field(object_, "variable"))
        )
    elif kind == "error":
        return TypeError()
    elif kind == "never":
        return TypeNever()
    elif kind == "any":
        return TypeAny()
    elif kind == "unknown":
        return TypeUnknown()
    elif kind == "void":
        return TypeVoid()
    elif kind == "null":
        return TypeNull()
    elif kind == "undefined":
        return TypeUndefined()
    elif kind == "object":
        return TypeObject()
    elif kind == "primitive":
        return TypePrimitive(
            primitive=destack._generated.dir.type.primitive.from_json_primitive_type(
                json_field(object_, "primitive")
            )
        )
    elif kind == "literal":
        return TypeLiteral(
            literal=destack._generated.dir.tree.literal.from_json_scalar_literal(
                json_field(object_, "literal")
            )
        )
    elif kind == "memory":
        return TypeMemory(
            memory=from_json_memory_literal(json_field(object_, "memory"))
        )
    elif kind == "static":
        return TypeStatic(
            static=destack._generated.dir.tree.static.from_json_global_static_id(
                json_field(object_, "static")
            )
        )
    elif kind == "intrinsic":
        return TypeIntrinsic()
    elif kind == "parameter":
        return TypeParameter(
            parameter=destack._generated.dir.type.generic.from_json_global_generic_parameter_id(
                json_field(object_, "parameter")
            )
        )
    elif kind == "reference":
        return TypeReference(
            reference=from_json_generic_instance(json_field(object_, "reference"))
        )
    elif kind == "this":
        return TypeThis()
    elif kind == "member":
        return TypeMember(member=from_json_member_type(json_field(object_, "member")))
    elif kind == "form":
        return TypeForm(form=from_json_form_type(json_field(object_, "form")))
    elif kind == "dynamic":
        return TypeDynamic(
            dynamic=from_json_dynamic_type(json_field(object_, "dynamic"))
        )
    elif kind == "operation":
        return TypeOperationVariant(
            operation=from_json_type_operation(json_field(object_, "operation"))
        )
    elif kind == "array":
        return TypeArray(array=from_json_array_type(json_field(object_, "array")))
    elif kind == "fixedArray":
        return TypeFixedArray(
            fixed_array=from_json_fixed_array_type(json_field(object_, "fixed_array"))
        )
    elif kind == "range":
        return TypeRange(range=from_json_range_type(json_field(object_, "range")))
    elif kind == "slice":
        return TypeSlice(slice=from_json_slice_type(json_field(object_, "slice")))
    elif kind == "tuple":
        return TypeTuple(tuple=from_json_tuple_type(json_field(object_, "tuple")))
    elif kind == "shape":
        return TypeShape(shape=from_json_shape_type(json_field(object_, "shape")))
    elif kind == "functionSignature":
        return TypeFunctionSignature(
            function_signature=from_json_function_signature_type(
                json_field(object_, "function_signature")
            )
        )
    elif kind == "function":
        return TypeFunction(
            function=from_json_function_type(json_field(object_, "function"))
        )
    elif kind == "functionPointer":
        return TypeFunctionPointer(
            function_pointer=from_json_function_pointer_type(
                json_field(object_, "function_pointer")
            )
        )
    elif kind == "union":
        return TypeUnion(union=from_json_union_type(json_field(object_, "union")))
    elif kind == "intersection":
        return TypeIntersection(
            intersection=from_json_intersection_type(
                json_field(object_, "intersection")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "TypeVariableId",
    "encode_type_variable_id",
    "decode_type_variable_id",
    "to_json_type_variable_id",
    "from_json_type_variable_id",
    "MemoryLiteral",
    "encode_memory_literal",
    "decode_memory_literal",
    "to_json_memory_literal",
    "from_json_memory_literal",
    "MemoryLiteralAccess",
    "MemoryLiteralSpace",
    "MemoryLiteralPlace",
    "MemoryLiteralLifetime",
    "Access",
    "encode_access",
    "decode_access",
    "to_json_access",
    "from_json_access",
    "Space",
    "encode_space",
    "decode_space",
    "to_json_space",
    "from_json_space",
    "Place",
    "encode_place",
    "decode_place",
    "to_json_place",
    "from_json_place",
    "PlaceAmbient",
    "PlaceSpace",
    "Lifetime",
    "encode_lifetime",
    "decode_lifetime",
    "to_json_lifetime",
    "from_json_lifetime",
    "LifetimeStatic",
    "LifetimeFrame",
    "LifetimeSymbol",
    "GenericInstance",
    "encode_generic_instance",
    "decode_generic_instance",
    "to_json_generic_instance",
    "from_json_generic_instance",
    "GlobalTypeId",
    "encode_global_type_id",
    "decode_global_type_id",
    "to_json_global_type_id",
    "from_json_global_type_id",
    "LocalTypeId",
    "encode_local_type_id",
    "decode_local_type_id",
    "to_json_local_type_id",
    "from_json_local_type_id",
    "MemberType",
    "encode_member_type",
    "decode_member_type",
    "to_json_member_type",
    "from_json_member_type",
    "FormType",
    "encode_form_type",
    "decode_form_type",
    "to_json_form_type",
    "from_json_form_type",
    "Form",
    "encode_form",
    "decode_form",
    "to_json_form",
    "from_json_form",
    "FormManaged",
    "FormOwned",
    "FormBorrowed",
    "FormRaw",
    "FormPlaced",
    "FormReadonly",
    "DynamicType",
    "encode_dynamic_type",
    "decode_dynamic_type",
    "to_json_dynamic_type",
    "from_json_dynamic_type",
    "TypeOperation",
    "encode_type_operation",
    "decode_type_operation",
    "to_json_type_operation",
    "from_json_type_operation",
    "TypeOperationStringMapping",
    "TypeOperationConditional",
    "TypeOperationMapped",
    "TypeOperationIndex",
    "TypeOperationTemplateLiteral",
    "TypeOperationInfer",
    "TypeOperationKeyOf",
    "TypeOperationTryOutput",
    "TypeOperationTryResidual",
    "TypeOperationStaticBinary",
    "TypeOperationStaticUnary",
    "StringMapping",
    "encode_string_mapping",
    "decode_string_mapping",
    "to_json_string_mapping",
    "from_json_string_mapping",
    "ConditionalType",
    "encode_conditional_type",
    "decode_conditional_type",
    "to_json_conditional_type",
    "from_json_conditional_type",
    "MappedType",
    "encode_mapped_type",
    "decode_mapped_type",
    "to_json_mapped_type",
    "from_json_mapped_type",
    "MappedTypeParameter",
    "encode_mapped_type_parameter",
    "decode_mapped_type_parameter",
    "to_json_mapped_type_parameter",
    "from_json_mapped_type_parameter",
    "MappedTypeModifiers",
    "encode_mapped_type_modifiers",
    "decode_mapped_type_modifiers",
    "to_json_mapped_type_modifiers",
    "from_json_mapped_type_modifiers",
    "IndexType",
    "encode_index_type",
    "decode_index_type",
    "to_json_index_type",
    "from_json_index_type",
    "TemplateLiteralType",
    "encode_template_literal_type",
    "decode_template_literal_type",
    "to_json_template_literal_type",
    "from_json_template_literal_type",
    "InferType",
    "encode_infer_type",
    "decode_infer_type",
    "to_json_infer_type",
    "from_json_infer_type",
    "UnaryType",
    "encode_unary_type",
    "decode_unary_type",
    "to_json_unary_type",
    "from_json_unary_type",
    "StaticBinaryType",
    "encode_static_binary_type",
    "decode_static_binary_type",
    "to_json_static_binary_type",
    "from_json_static_binary_type",
    "StaticBinaryOperator",
    "encode_static_binary_operator",
    "decode_static_binary_operator",
    "to_json_static_binary_operator",
    "from_json_static_binary_operator",
    "StaticUnaryType",
    "encode_static_unary_type",
    "decode_static_unary_type",
    "to_json_static_unary_type",
    "from_json_static_unary_type",
    "StaticUnaryOperator",
    "encode_static_unary_operator",
    "decode_static_unary_operator",
    "to_json_static_unary_operator",
    "from_json_static_unary_operator",
    "ArrayType",
    "encode_array_type",
    "decode_array_type",
    "to_json_array_type",
    "from_json_array_type",
    "FixedArrayType",
    "encode_fixed_array_type",
    "decode_fixed_array_type",
    "to_json_fixed_array_type",
    "from_json_fixed_array_type",
    "RangeType",
    "encode_range_type",
    "decode_range_type",
    "to_json_range_type",
    "from_json_range_type",
    "SliceType",
    "encode_slice_type",
    "decode_slice_type",
    "to_json_slice_type",
    "from_json_slice_type",
    "TupleType",
    "encode_tuple_type",
    "decode_tuple_type",
    "to_json_tuple_type",
    "from_json_tuple_type",
    "TupleForm",
    "encode_tuple_form",
    "decode_tuple_form",
    "to_json_tuple_form",
    "from_json_tuple_form",
    "TypeElement",
    "encode_type_element",
    "decode_type_element",
    "to_json_type_element",
    "from_json_type_element",
    "ShapeType",
    "encode_shape_type",
    "decode_shape_type",
    "to_json_shape_type",
    "from_json_shape_type",
    "TypeField",
    "encode_type_field",
    "decode_type_field",
    "to_json_type_field",
    "from_json_type_field",
    "TypeIndexSignature",
    "encode_type_index_signature",
    "decode_type_index_signature",
    "to_json_type_index_signature",
    "from_json_type_index_signature",
    "FunctionSignatureType",
    "encode_function_signature_type",
    "decode_function_signature_type",
    "to_json_function_signature_type",
    "from_json_function_signature_type",
    "FunctionParameterType",
    "encode_function_parameter_type",
    "decode_function_parameter_type",
    "to_json_function_parameter_type",
    "from_json_function_parameter_type",
    "FunctionType",
    "encode_function_type",
    "decode_function_type",
    "to_json_function_type",
    "from_json_function_type",
    "FunctionPointerType",
    "encode_function_pointer_type",
    "decode_function_pointer_type",
    "to_json_function_pointer_type",
    "from_json_function_pointer_type",
    "UnionType",
    "encode_union_type",
    "decode_union_type",
    "to_json_union_type",
    "from_json_union_type",
    "IntersectionType",
    "encode_intersection_type",
    "decode_intersection_type",
    "to_json_intersection_type",
    "from_json_intersection_type",
    "Type",
    "encode_type",
    "decode_type",
    "to_json_type",
    "from_json_type",
    "TypeVariable",
    "TypeError",
    "TypeNever",
    "TypeAny",
    "TypeUnknown",
    "TypeVoid",
    "TypeNull",
    "TypeUndefined",
    "TypeObject",
    "TypePrimitive",
    "TypeLiteral",
    "TypeMemory",
    "TypeStatic",
    "TypeIntrinsic",
    "TypeParameter",
    "TypeReference",
    "TypeThis",
    "TypeMember",
    "TypeForm",
    "TypeDynamic",
    "TypeOperationVariant",
    "TypeArray",
    "TypeFixedArray",
    "TypeRange",
    "TypeSlice",
    "TypeTuple",
    "TypeShape",
    "TypeFunctionSignature",
    "TypeFunction",
    "TypeFunctionPointer",
    "TypeUnion",
    "TypeIntersection",
]
