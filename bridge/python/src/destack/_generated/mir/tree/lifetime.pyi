# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string

@dataclass(frozen=True, slots=True)
class Lifetime:
    """The boundary lifetime for an escaping borrowed value."""

    # terms the borrowed value may depend on
    terms: Sequence[LifetimeTerm]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Lifetime: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Lifetime: ...

def encode_lifetime(writer: BinaryWriter, value: Lifetime) -> None: ...
def decode_lifetime(reader: BinaryReader) -> Lifetime: ...
def to_json_lifetime(value: Lifetime) -> Json: ...
def from_json_lifetime(value: Json) -> Lifetime: ...

@dataclass(frozen=True, slots=True)
class LifetimeTermStatic:
    """Global or static storage."""

    kind: typing.Literal["static"] = "static"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LifetimeTermSlot:
    """A lifetime slot in the current lifetime environment."""

    slot: LifetimeSlot
    kind: typing.Literal["slot"] = "slot"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One term in a MIR lifetime."""
LifetimeTerm: typing.TypeAlias = LifetimeTermStatic | LifetimeTermSlot

def encode_lifetime_term(writer: BinaryWriter, value: LifetimeTerm) -> None: ...
def decode_lifetime_term(reader: BinaryReader) -> LifetimeTerm: ...
def to_json_lifetime_term(value: LifetimeTerm) -> Json: ...
def from_json_lifetime_term(value: Json) -> LifetimeTerm: ...

"""One lifetime slot in a MIR lifetime environment."""
LifetimeSlot: typing.TypeAlias = int

def encode_lifetime_slot(writer: BinaryWriter, value: LifetimeSlot) -> None: ...
def decode_lifetime_slot(reader: BinaryReader) -> LifetimeSlot: ...
def to_json_lifetime_slot(value: LifetimeSlot) -> Json: ...
def from_json_lifetime_slot(value: Json) -> LifetimeSlot: ...

@dataclass(frozen=True, slots=True)
class LifetimeParameter:
    """One declared lifetime parameter."""

    # the source or generated parameter name
    name: destack._generated.core.string.StringId | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LifetimeParameter: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LifetimeParameter: ...

def encode_lifetime_parameter(
    writer: BinaryWriter, value: LifetimeParameter
) -> None: ...
def decode_lifetime_parameter(reader: BinaryReader) -> LifetimeParameter: ...
def to_json_lifetime_parameter(value: LifetimeParameter) -> Json: ...
def from_json_lifetime_parameter(value: Json) -> LifetimeParameter: ...

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
