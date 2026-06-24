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
    json_int,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.core.string


@dataclass(frozen=True, slots=True)
class Lifetime:
    """The boundary lifetime for an escaping borrowed value."""

    # terms the borrowed value may depend on
    terms: Sequence[LifetimeTerm]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Lifetime:
        """Decode one Lifetime."""
        return decode_lifetime(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime(self)

    @classmethod
    def from_json(cls, value: Json) -> Lifetime:
        """Return one Lifetime from one JSON value."""
        return from_json_lifetime(value)


def encode_lifetime(writer: BinaryWriter, value: Lifetime) -> None:
    """Encode one Lifetime."""
    writer.write_unsigned(len(value.terms))
    for item_value_terms_0 in value.terms:
        encode_lifetime_term(writer, item_value_terms_0)


def decode_lifetime(reader: BinaryReader) -> Lifetime:
    """Decode one Lifetime."""
    terms = [decode_lifetime_term(reader) for _ in range(reader.read_number())]

    return Lifetime(
        terms=terms,
    )


def to_json_lifetime(value: Lifetime) -> Json:
    """Return one JSON value for one Lifetime."""
    return {
        "terms": [to_json_lifetime_term(item_0) for item_0 in value.terms],
    }


def from_json_lifetime(value: Json) -> Lifetime:
    """Return one Lifetime from one JSON value."""
    object_ = json_object(value)

    return Lifetime(
        terms=[
            from_json_lifetime_term(item_0)
            for item_0 in json_array(json_field(object_, "terms"))
        ],
    )


@dataclass(frozen=True, slots=True)
class LifetimeTermStatic:
    """Global or static storage."""

    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime_term(self)


@dataclass(frozen=True, slots=True)
class LifetimeTermSlot:
    """A lifetime slot in the current lifetime environment."""

    slot: LifetimeSlot
    kind: typing.Literal["slot"] = "slot"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime_term(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime_term(self)


"""One term in a MIR lifetime."""
LifetimeTerm: typing.TypeAlias = LifetimeTermStatic | LifetimeTermSlot


def encode_lifetime_term(writer: BinaryWriter, value: LifetimeTerm) -> None:
    """Encode one LifetimeTerm."""
    if value.kind == "static":
        writer.write_unsigned(0)
    elif value.kind == "slot":
        writer.write_unsigned(1)
        encode_lifetime_slot(writer, value.slot)
    else:
        raise SerdeError("unknown enum variant")


def decode_lifetime_term(reader: BinaryReader) -> LifetimeTerm:
    """Decode one LifetimeTerm."""
    variant = reader.read_number()

    if variant == 0:
        return LifetimeTermStatic()
    elif variant == 1:
        slot = decode_lifetime_slot(reader)

        return LifetimeTermSlot(slot=slot)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_lifetime_term(value: LifetimeTerm) -> Json:
    """Return one JSON value for one LifetimeTerm."""
    if value.kind == "static":
        return {
            "kind": "static",
        }
    elif value.kind == "slot":
        return {
            "kind": "slot",
            "slot": to_json_lifetime_slot(value.slot),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_lifetime_term(value: Json) -> LifetimeTerm:
    """Return one LifetimeTerm from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "static":
        return LifetimeTermStatic()
    elif kind == "slot":
        return LifetimeTermSlot(
            slot=from_json_lifetime_slot(json_field(object_, "slot"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""One lifetime slot in a MIR lifetime environment."""
LifetimeSlot: typing.TypeAlias = int


def encode_lifetime_slot(writer: BinaryWriter, value: LifetimeSlot) -> None:
    """Encode one LifetimeSlot."""
    writer.write_unsigned(value)


def decode_lifetime_slot(reader: BinaryReader) -> LifetimeSlot:
    """Decode one LifetimeSlot."""
    return reader.read_number()


def to_json_lifetime_slot(value: LifetimeSlot) -> Json:
    """Return one JSON value for one LifetimeSlot."""
    return value


def from_json_lifetime_slot(value: Json) -> LifetimeSlot:
    """Return one LifetimeSlot from one JSON value."""
    return json_int(value)


@dataclass(frozen=True, slots=True)
class LifetimeParameter:
    """One declared lifetime parameter."""

    # the source or generated parameter name
    name: destack._generated.core.string.StringId | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_lifetime_parameter(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LifetimeParameter:
        """Decode one LifetimeParameter."""
        return decode_lifetime_parameter(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_lifetime_parameter(self)

    @classmethod
    def from_json(cls, value: Json) -> LifetimeParameter:
        """Return one LifetimeParameter from one JSON value."""
        return from_json_lifetime_parameter(value)


def encode_lifetime_parameter(writer: BinaryWriter, value: LifetimeParameter) -> None:
    """Encode one LifetimeParameter."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.core.string.encode_string_id(writer, value.name)


def decode_lifetime_parameter(reader: BinaryReader) -> LifetimeParameter:
    """Decode one LifetimeParameter."""
    name = reader.read_option(
        lambda: destack._generated.core.string.decode_string_id(reader)
    )

    return LifetimeParameter(
        name=name,
    )


def to_json_lifetime_parameter(value: LifetimeParameter) -> Json:
    """Return one JSON value for one LifetimeParameter."""
    return {
        **(
            {}
            if value.name is None
            else {"name": destack._generated.core.string.to_json_string_id(value.name)}
        ),
    }


def from_json_lifetime_parameter(value: Json) -> LifetimeParameter:
    """Return one LifetimeParameter from one JSON value."""
    object_ = json_object(value)

    return LifetimeParameter(
        name=json_optional(
            object_,
            "name",
            lambda value: destack._generated.core.string.from_json_string_id(value),
        ),
    )


__all__ = [
    "Lifetime",
    "encode_lifetime",
    "decode_lifetime",
    "to_json_lifetime",
    "from_json_lifetime",
    "LifetimeTerm",
    "encode_lifetime_term",
    "decode_lifetime_term",
    "to_json_lifetime_term",
    "from_json_lifetime_term",
    "LifetimeTermStatic",
    "LifetimeTermSlot",
    "LifetimeSlot",
    "encode_lifetime_slot",
    "decode_lifetime_slot",
    "to_json_lifetime_slot",
    "from_json_lifetime_slot",
    "LifetimeParameter",
    "encode_lifetime_parameter",
    "decode_lifetime_parameter",
    "to_json_lifetime_parameter",
    "from_json_lifetime_parameter",
]
