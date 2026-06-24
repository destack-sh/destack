# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json, json_object


@dataclass(frozen=True, slots=True)
class ModuleLinted:
    """The realized module lint surface for one module profile."""

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_module_linted(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleLinted:
        """Decode one ModuleLinted."""
        return decode_module_linted(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_module_linted(self)

    @classmethod
    def from_json(cls, value: Json) -> ModuleLinted:
        """Return one ModuleLinted from one JSON value."""
        return from_json_module_linted(value)


def encode_module_linted(writer: BinaryWriter, value: ModuleLinted) -> None:
    """Encode one ModuleLinted."""
    pass


def decode_module_linted(reader: BinaryReader) -> ModuleLinted:
    """Decode one ModuleLinted."""
    return ModuleLinted()


def to_json_module_linted(value: ModuleLinted) -> Json:
    """Return one JSON value for one ModuleLinted."""
    return {}


def from_json_module_linted(value: Json) -> ModuleLinted:
    """Return one ModuleLinted from one JSON value."""
    object_ = json_object(value)
    return ModuleLinted()


@dataclass(frozen=True, slots=True)
class PackageLinted:
    """The realized package lint surface for one package."""

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_linted(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageLinted:
        """Decode one PackageLinted."""
        return decode_package_linted(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_linted(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageLinted:
        """Return one PackageLinted from one JSON value."""
        return from_json_package_linted(value)


def encode_package_linted(writer: BinaryWriter, value: PackageLinted) -> None:
    """Encode one PackageLinted."""
    pass


def decode_package_linted(reader: BinaryReader) -> PackageLinted:
    """Decode one PackageLinted."""
    return PackageLinted()


def to_json_package_linted(value: PackageLinted) -> Json:
    """Return one JSON value for one PackageLinted."""
    return {}


def from_json_package_linted(value: Json) -> PackageLinted:
    """Return one PackageLinted from one JSON value."""
    object_ = json_object(value)
    return PackageLinted()


@dataclass(frozen=True, slots=True)
class WorkspaceLinted:
    """The realized workspace lint surface."""

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_linted(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceLinted:
        """Decode one WorkspaceLinted."""
        return decode_workspace_linted(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_linted(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceLinted:
        """Return one WorkspaceLinted from one JSON value."""
        return from_json_workspace_linted(value)


def encode_workspace_linted(writer: BinaryWriter, value: WorkspaceLinted) -> None:
    """Encode one WorkspaceLinted."""
    pass


def decode_workspace_linted(reader: BinaryReader) -> WorkspaceLinted:
    """Decode one WorkspaceLinted."""
    return WorkspaceLinted()


def to_json_workspace_linted(value: WorkspaceLinted) -> Json:
    """Return one JSON value for one WorkspaceLinted."""
    return {}


def from_json_workspace_linted(value: Json) -> WorkspaceLinted:
    """Return one WorkspaceLinted from one JSON value."""
    object_ = json_object(value)
    return WorkspaceLinted()


__all__ = [
    "ModuleLinted",
    "encode_module_linted",
    "decode_module_linted",
    "to_json_module_linted",
    "from_json_module_linted",
    "PackageLinted",
    "encode_package_linted",
    "decode_package_linted",
    "to_json_package_linted",
    "from_json_package_linted",
    "WorkspaceLinted",
    "encode_workspace_linted",
    "decode_workspace_linted",
    "to_json_workspace_linted",
    "from_json_workspace_linted",
]
