# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class DestackLayout:
    """Resolved Destack storage layout for one invocation."""

    # machine-local Destack home
    home: str
    # machine-local package directory
    packages: str
    # workspace-local cache and session directory
    workspace_cache: str
    # workspace-owned vendor directory
    vendor: str

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLayout: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DestackLayout: ...

def encode_destack_layout(writer: BinaryWriter, value: DestackLayout) -> None: ...
def decode_destack_layout(reader: BinaryReader) -> DestackLayout: ...
def to_json_destack_layout(value: DestackLayout) -> Json: ...
def from_json_destack_layout(value: Json) -> DestackLayout: ...

@dataclass(frozen=True, slots=True)
class DestackLayoutOverride:
    """Invocation-level layout overrides."""

    # home directory override
    home: str | None
    # package directory override
    packages: str | None
    # workspace cache override
    workspace_cache: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLayoutOverride: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> DestackLayoutOverride: ...

def encode_destack_layout_override(
    writer: BinaryWriter, value: DestackLayoutOverride
) -> None: ...
def decode_destack_layout_override(reader: BinaryReader) -> DestackLayoutOverride: ...
def to_json_destack_layout_override(value: DestackLayoutOverride) -> Json: ...
def from_json_destack_layout_override(value: Json) -> DestackLayoutOverride: ...

__all__ = [
    "DestackLayout",
    "encode_destack_layout",
    "decode_destack_layout",
    "to_json_destack_layout",
    "from_json_destack_layout",
    "DestackLayoutOverride",
    "encode_destack_layout_override",
    "decode_destack_layout_override",
    "to_json_destack_layout_override",
    "from_json_destack_layout_override",
]
