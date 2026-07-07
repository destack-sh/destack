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
    json_optional,
    json_string,
)

import destack._generated.query.protocol.target
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class DecoratorsRequest:
    """Request payload for decorator queries."""

    # the query scope
    scope: DecoratorScope
    # the decorator name filter
    name: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorators_request(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorsRequest:
        """Decode one DecoratorsRequest."""
        return decode_decorators_request(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorators_request(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorsRequest:
        """Return one DecoratorsRequest from one JSON value."""
        return from_json_decorators_request(value)


def encode_decorators_request(writer: BinaryWriter, value: DecoratorsRequest) -> None:
    """Encode one DecoratorsRequest."""
    encode_decorator_scope(writer, value.scope)
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)


def decode_decorators_request(reader: BinaryReader) -> DecoratorsRequest:
    """Decode one DecoratorsRequest."""
    scope = decode_decorator_scope(reader)
    name = reader.read_option(lambda: reader.read_string())

    return DecoratorsRequest(
        scope=scope,
        name=name,
    )


def to_json_decorators_request(value: DecoratorsRequest) -> Json:
    """Return one JSON value for one DecoratorsRequest."""
    return {
        "scope": to_json_decorator_scope(value.scope),
        **({} if value.name is None else {"name": value.name}),
    }


def from_json_decorators_request(value: Json) -> DecoratorsRequest:
    """Return one DecoratorsRequest from one JSON value."""
    object_ = json_object(value)

    return DecoratorsRequest(
        scope=from_json_decorator_scope(json_field(object_, "scope")),
        name=json_optional(object_, "name", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class DecoratorScopeModule:
    """One module."""

    module: destack._generated.query.protocol.target.Module
    kind: typing.Literal["module"] = "module"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_scope(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_scope(self)


@dataclass(frozen=True, slots=True)
class DecoratorScopeProgram:
    """Program profiles."""

    # the profiles to search
    profile_ids: Sequence[destack._generated.source.file.model.profile.ProfileId]
    kind: typing.Literal["program"] = "program"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_scope(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_scope(self)


"""Scope for decorator queries."""
DecoratorScope: typing.TypeAlias = DecoratorScopeModule | DecoratorScopeProgram


def encode_decorator_scope(writer: BinaryWriter, value: DecoratorScope) -> None:
    """Encode one DecoratorScope."""
    if value.kind == "module":
        writer.write_unsigned(0)
        destack._generated.query.protocol.target.encode_module(writer, value.module)
    elif value.kind == "program":
        writer.write_unsigned(1)
        writer.write_unsigned(len(value.profile_ids))
        for item_value_profile_ids_0 in value.profile_ids:
            destack._generated.source.file.model.profile.encode_profile_id(
                writer, item_value_profile_ids_0
            )
    else:
        raise SerdeError("unknown enum variant")


def decode_decorator_scope(reader: BinaryReader) -> DecoratorScope:
    """Decode one DecoratorScope."""
    variant = reader.read_number()

    if variant == 0:
        module = destack._generated.query.protocol.target.decode_module(reader)

        return DecoratorScopeModule(module=module)
    elif variant == 1:
        profile_ids = [
            destack._generated.source.file.model.profile.decode_profile_id(reader)
            for _ in range(reader.read_number())
        ]

        return DecoratorScopeProgram(
            profile_ids=profile_ids,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_decorator_scope(value: DecoratorScope) -> Json:
    """Return one JSON value for one DecoratorScope."""
    if value.kind == "module":
        return {
            "kind": "module",
            "module": destack._generated.query.protocol.target.to_json_module(
                value.module
            ),
        }
    elif value.kind == "program":
        return {
            "kind": "program",
            "profileIds": [
                destack._generated.source.file.model.profile.to_json_profile_id(item_0)
                for item_0 in value.profile_ids
            ],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_decorator_scope(value: Json) -> DecoratorScope:
    """Return one DecoratorScope from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "module":
        return DecoratorScopeModule(
            module=destack._generated.query.protocol.target.from_json_module(
                json_field(object_, "module")
            )
        )
    elif kind == "program":
        return DecoratorScopeProgram(
            profile_ids=[
                destack._generated.source.file.model.profile.from_json_profile_id(
                    item_0
                )
                for item_0 in json_array(json_field(object_, "profileIds"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DecoratorsResponse:
    """Response payload for decorator queries."""

    # matching decorators
    decorators: Sequence[DecoratorItem]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorators_response(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorsResponse:
        """Decode one DecoratorsResponse."""
        return decode_decorators_response(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorators_response(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorsResponse:
        """Return one DecoratorsResponse from one JSON value."""
        return from_json_decorators_response(value)


def encode_decorators_response(writer: BinaryWriter, value: DecoratorsResponse) -> None:
    """Encode one DecoratorsResponse."""
    writer.write_unsigned(len(value.decorators))
    for item_value_decorators_0 in value.decorators:
        encode_decorator_item(writer, item_value_decorators_0)


def decode_decorators_response(reader: BinaryReader) -> DecoratorsResponse:
    """Decode one DecoratorsResponse."""
    decorators = [decode_decorator_item(reader) for _ in range(reader.read_number())]

    return DecoratorsResponse(
        decorators=decorators,
    )


def to_json_decorators_response(value: DecoratorsResponse) -> Json:
    """Return one JSON value for one DecoratorsResponse."""
    return {
        "decorators": [to_json_decorator_item(item_0) for item_0 in value.decorators],
    }


def from_json_decorators_response(value: Json) -> DecoratorsResponse:
    """Return one DecoratorsResponse from one JSON value."""
    object_ = json_object(value)

    return DecoratorsResponse(
        decorators=[
            from_json_decorator_item(item_0)
            for item_0 in json_array(json_field(object_, "decorators"))
        ],
    )


@dataclass(frozen=True, slots=True)
class DecoratorItem:
    """One decorator query item."""

    # the decorator name when syntactically known
    name: str | None
    # the decorator expression target
    decorator: destack._generated.query.protocol.target.Target
    # the decorated target
    target: destack._generated.query.protocol.target.Target
    # the resolved decorator role
    role: DecoratorRole

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_decorator_item(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DecoratorItem:
        """Decode one DecoratorItem."""
        return decode_decorator_item(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_decorator_item(self)

    @classmethod
    def from_json(cls, value: Json) -> DecoratorItem:
        """Return one DecoratorItem from one JSON value."""
        return from_json_decorator_item(value)


def encode_decorator_item(writer: BinaryWriter, value: DecoratorItem) -> None:
    """Encode one DecoratorItem."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    destack._generated.query.protocol.target.encode_target(writer, value.decorator)
    destack._generated.query.protocol.target.encode_target(writer, value.target)
    encode_decorator_role(writer, value.role)


def decode_decorator_item(reader: BinaryReader) -> DecoratorItem:
    """Decode one DecoratorItem."""
    name = reader.read_option(lambda: reader.read_string())
    decorator = destack._generated.query.protocol.target.decode_target(reader)
    target = destack._generated.query.protocol.target.decode_target(reader)
    role = decode_decorator_role(reader)

    return DecoratorItem(
        name=name,
        decorator=decorator,
        target=target,
        role=role,
    )


def to_json_decorator_item(value: DecoratorItem) -> Json:
    """Return one JSON value for one DecoratorItem."""
    return {
        **({} if value.name is None else {"name": value.name}),
        "decorator": destack._generated.query.protocol.target.to_json_target(
            value.decorator
        ),
        "target": destack._generated.query.protocol.target.to_json_target(value.target),
        "role": to_json_decorator_role(value.role),
    }


def from_json_decorator_item(value: Json) -> DecoratorItem:
    """Return one DecoratorItem from one JSON value."""
    object_ = json_object(value)

    return DecoratorItem(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        decorator=destack._generated.query.protocol.target.from_json_target(
            json_field(object_, "decorator")
        ),
        target=destack._generated.query.protocol.target.from_json_target(
            json_field(object_, "target")
        ),
        role=from_json_decorator_role(json_field(object_, "role")),
    )


"""Role of one decorator expression."""
DecoratorRole: typing.TypeAlias = (
    typing.Literal["languageItem"]
    | typing.Literal["symbol"]
    | typing.Literal["unresolved"]
)


def encode_decorator_role(writer: BinaryWriter, value: DecoratorRole) -> None:
    """Encode one DecoratorRole."""
    if value == "languageItem":
        writer.write_unsigned(0)
    elif value == "symbol":
        writer.write_unsigned(1)
    elif value == "unresolved":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_decorator_role(reader: BinaryReader) -> DecoratorRole:
    """Decode one DecoratorRole."""
    variant = reader.read_number()

    if variant == 0:
        return "languageItem"
    elif variant == 1:
        return "symbol"
    elif variant == 2:
        return "unresolved"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_decorator_role(value: DecoratorRole) -> Json:
    """Return one JSON value for one DecoratorRole."""
    return value


def from_json_decorator_role(value: Json) -> DecoratorRole:
    """Return one DecoratorRole from one JSON value."""
    variant = json_string(value)

    if variant == "languageItem":
        return "languageItem"
    elif variant == "symbol":
        return "symbol"
    elif variant == "unresolved":
        return "unresolved"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "DecoratorsRequest",
    "encode_decorators_request",
    "decode_decorators_request",
    "to_json_decorators_request",
    "from_json_decorators_request",
    "DecoratorScope",
    "encode_decorator_scope",
    "decode_decorator_scope",
    "to_json_decorator_scope",
    "from_json_decorator_scope",
    "DecoratorScopeModule",
    "DecoratorScopeProgram",
    "DecoratorsResponse",
    "encode_decorators_response",
    "decode_decorators_response",
    "to_json_decorators_response",
    "from_json_decorators_response",
    "DecoratorItem",
    "encode_decorator_item",
    "decode_decorator_item",
    "to_json_decorator_item",
    "from_json_decorator_item",
    "DecoratorRole",
    "encode_decorator_role",
    "decode_decorator_role",
    "to_json_decorator_role",
    "from_json_decorator_role",
]
