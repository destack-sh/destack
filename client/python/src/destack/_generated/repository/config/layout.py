# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_field,
    json_object,
    json_optional,
    json_string,
)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_destack_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLayout:
        """Decode one DestackLayout."""
        return decode_destack_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_destack_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> DestackLayout:
        """Return one DestackLayout from one JSON value."""
        return from_json_destack_layout(value)


def encode_destack_layout(writer: BinaryWriter, value: DestackLayout) -> None:
    """Encode one DestackLayout."""
    writer.write_string(value.home)
    writer.write_string(value.packages)
    writer.write_string(value.workspace_cache)
    writer.write_string(value.vendor)


def decode_destack_layout(reader: BinaryReader) -> DestackLayout:
    """Decode one DestackLayout."""
    home = reader.read_string()
    packages = reader.read_string()
    workspace_cache = reader.read_string()
    vendor = reader.read_string()

    return DestackLayout(
        home=home,
        packages=packages,
        workspace_cache=workspace_cache,
        vendor=vendor,
    )


def to_json_destack_layout(value: DestackLayout) -> Json:
    """Return one JSON value for one DestackLayout."""
    return {
        "home": value.home,
        "packages": value.packages,
        "workspaceCache": value.workspace_cache,
        "vendor": value.vendor,
    }


def from_json_destack_layout(value: Json) -> DestackLayout:
    """Return one DestackLayout from one JSON value."""
    object_ = json_object(value)

    return DestackLayout(
        home=json_string(json_field(object_, "home")),
        packages=json_string(json_field(object_, "packages")),
        workspace_cache=json_string(json_field(object_, "workspaceCache")),
        vendor=json_string(json_field(object_, "vendor")),
    )


@dataclass(frozen=True, slots=True)
class DestackLayoutOverride:
    """Invocation-level layout overrides."""

    # home directory override
    home: str | None
    # package directory override
    packages: str | None
    # workspace cache override
    workspace_cache: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_destack_layout_override(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLayoutOverride:
        """Decode one DestackLayoutOverride."""
        return decode_destack_layout_override(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_destack_layout_override(self)

    @classmethod
    def from_json(cls, value: Json) -> DestackLayoutOverride:
        """Return one DestackLayoutOverride from one JSON value."""
        return from_json_destack_layout_override(value)


def encode_destack_layout_override(
    writer: BinaryWriter, value: DestackLayoutOverride
) -> None:
    """Encode one DestackLayoutOverride."""
    if value.home is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.home)
    if value.packages is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.packages)
    if value.workspace_cache is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.workspace_cache)


def decode_destack_layout_override(reader: BinaryReader) -> DestackLayoutOverride:
    """Decode one DestackLayoutOverride."""
    home = reader.read_option(lambda: reader.read_string())
    packages = reader.read_option(lambda: reader.read_string())
    workspace_cache = reader.read_option(lambda: reader.read_string())

    return DestackLayoutOverride(
        home=home,
        packages=packages,
        workspace_cache=workspace_cache,
    )


def to_json_destack_layout_override(value: DestackLayoutOverride) -> Json:
    """Return one JSON value for one DestackLayoutOverride."""
    return {
        **({} if value.home is None else {"home": value.home}),
        **({} if value.packages is None else {"packages": value.packages}),
        **(
            {}
            if value.workspace_cache is None
            else {"workspaceCache": value.workspace_cache}
        ),
    }


def from_json_destack_layout_override(value: Json) -> DestackLayoutOverride:
    """Return one DestackLayoutOverride from one JSON value."""
    object_ = json_object(value)

    return DestackLayoutOverride(
        home=json_optional(object_, "home", lambda value: json_string(value)),
        packages=json_optional(object_, "packages", lambda value: json_string(value)),
        workspace_cache=json_optional(
            object_, "workspaceCache", lambda value: json_string(value)
        ),
    )


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
