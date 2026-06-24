# generated bridge target, do not edit

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
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class Vendor:
    """Vendored dependency resolution options."""

    # vendored dependency resolution mode
    mode: VendorMode
    # project-owned vendor directory
    path: str
    # package patterns included in the vendor mirror
    include: Sequence[str]
    # package patterns excluded from the vendor mirror
    exclude: Sequence[str]
    # whether vendored packages must match the lock
    verify: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_vendor(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Vendor:
        """Decode one Vendor."""
        return decode_vendor(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_vendor(self)

    @classmethod
    def from_json(cls, value: Json) -> Vendor:
        """Return one Vendor from one JSON value."""
        return from_json_vendor(value)


def encode_vendor(writer: BinaryWriter, value: Vendor) -> None:
    """Encode one Vendor."""
    encode_vendor_mode(writer, value.mode)
    writer.write_string(value.path)
    writer.write_unsigned(len(value.include))
    for item_value_include_0 in value.include:
        writer.write_string(item_value_include_0)
    writer.write_unsigned(len(value.exclude))
    for item_value_exclude_0 in value.exclude:
        writer.write_string(item_value_exclude_0)
    writer.write_bool(value.verify)


def decode_vendor(reader: BinaryReader) -> Vendor:
    """Decode one Vendor."""
    mode = decode_vendor_mode(reader)
    path = reader.read_string()
    include = [reader.read_string() for _ in range(reader.read_number())]
    exclude = [reader.read_string() for _ in range(reader.read_number())]
    verify = reader.read_bool()

    return Vendor(
        mode=mode,
        path=path,
        include=include,
        exclude=exclude,
        verify=verify,
    )


def to_json_vendor(value: Vendor) -> Json:
    """Return one JSON value for one Vendor."""
    return {
        "mode": to_json_vendor_mode(value.mode),
        "path": value.path,
        "include": [item_0 for item_0 in value.include],
        "exclude": [item_0 for item_0 in value.exclude],
        "verify": value.verify,
    }


def from_json_vendor(value: Json) -> Vendor:
    """Return one Vendor from one JSON value."""
    object_ = json_object(value)

    return Vendor(
        mode=from_json_vendor_mode(json_field(object_, "mode")),
        path=json_string(json_field(object_, "path")),
        include=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "include"))
        ],
        exclude=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "exclude"))
        ],
        verify=json_bool(json_field(object_, "verify")),
    )


"""Vendored dependency resolution mode."""
VendorMode: typing.TypeAlias = (
    typing.Literal["auto"]
    | typing.Literal["prefer"]
    | typing.Literal["require"]
    | typing.Literal["ignore"]
)


def encode_vendor_mode(writer: BinaryWriter, value: VendorMode) -> None:
    """Encode one VendorMode."""
    if value == "auto":
        writer.write_unsigned(0)
    elif value == "prefer":
        writer.write_unsigned(1)
    elif value == "require":
        writer.write_unsigned(2)
    elif value == "ignore":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_vendor_mode(reader: BinaryReader) -> VendorMode:
    """Decode one VendorMode."""
    variant = reader.read_number()

    if variant == 0:
        return "auto"
    elif variant == 1:
        return "prefer"
    elif variant == 2:
        return "require"
    elif variant == 3:
        return "ignore"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_vendor_mode(value: VendorMode) -> Json:
    """Return one JSON value for one VendorMode."""
    return value


def from_json_vendor_mode(value: Json) -> VendorMode:
    """Return one VendorMode from one JSON value."""
    variant = json_string(value)

    if variant == "auto":
        return "auto"
    elif variant == "prefer":
        return "prefer"
    elif variant == "require":
        return "require"
    elif variant == "ignore":
        return "ignore"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "Vendor",
    "encode_vendor",
    "decode_vendor",
    "to_json_vendor",
    "from_json_vendor",
    "VendorMode",
    "encode_vendor_mode",
    "decode_vendor_mode",
    "to_json_vendor_mode",
    "from_json_vendor_mode",
]
