# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes


@dataclass(frozen=True, slots=True)
class ProfileId:
    """Unique identifier for profiles."""

    field_0: int


def encode_profile_id(writer: Writer, value: ProfileId) -> None:
    writer.write_unsigned(value.field_0)


def decode_profile_id(reader: Reader) -> ProfileId:
    field_0 = reader.read_unsigned()

    return ProfileId(
        field_0=field_0,
    )


__all__ = [
    "ProfileId",
    "encode_profile_id",
    "decode_profile_id",
]
