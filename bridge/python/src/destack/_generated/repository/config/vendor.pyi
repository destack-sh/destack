# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Vendor: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Vendor: ...

def encode_vendor(writer: BinaryWriter, value: Vendor) -> None: ...
def decode_vendor(reader: BinaryReader) -> Vendor: ...
def to_json_vendor(value: Vendor) -> Json: ...
def from_json_vendor(value: Json) -> Vendor: ...

"""Vendored dependency resolution mode."""
VendorMode: typing.TypeAlias = (
    typing.Literal["auto"]
    | typing.Literal["prefer"]
    | typing.Literal["require"]
    | typing.Literal["ignore"]
)

def encode_vendor_mode(writer: BinaryWriter, value: VendorMode) -> None: ...
def decode_vendor_mode(reader: BinaryReader) -> VendorMode: ...
def to_json_vendor_mode(value: VendorMode) -> Json: ...
def from_json_vendor_mode(value: Json) -> VendorMode: ...

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
