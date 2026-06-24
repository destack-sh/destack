# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping
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
    nested_bytes,
)

import destack._generated.repository.config.dependency


@dataclass(frozen=True, slots=True)
class PackageLock:
    """Locked package source and dependency record."""

    # package name read from the locked manifest
    name: str
    # package version read from the locked manifest
    version: str | None
    # resolved source locator
    source: SourceLock
    # hash of the package source tree
    source_hash: str
    # hash of the package manifest
    manifest_hash: str
    # locked dependency declarations
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]
    # whether this package is an editable source
    editable: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_lock(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageLock:
        """Decode one PackageLock."""
        return decode_package_lock(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_lock(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageLock:
        """Return one PackageLock from one JSON value."""
        return from_json_package_lock(value)


def encode_package_lock(writer: BinaryWriter, value: PackageLock) -> None:
    """Encode one PackageLock."""
    writer.write_string(value.name)
    if value.version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.version)
    encode_source_lock(writer, value.source)
    writer.write_string(value.source_hash)
    writer.write_string(value.manifest_hash)
    entries_value_dependencies_0 = []
    for (
        key_value_dependencies_0,
        item_value_dependencies_0,
    ) in value.dependencies.items():

        def write_key_value_dependencies_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_dependencies_0)

        key_bytes = nested_bytes(write_key_value_dependencies_0)
        entries_value_dependencies_0.append(
            (key_value_dependencies_0, item_value_dependencies_0, key_bytes)
        )
    entries_value_dependencies_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_dependencies_0))
    for entry_value_dependencies_0 in entries_value_dependencies_0:
        writer.write_string(entry_value_dependencies_0[0])
        destack._generated.repository.config.dependency.encode_dependency(
            writer, entry_value_dependencies_0[1]
        )
    writer.write_bool(value.editable)


def decode_package_lock(reader: BinaryReader) -> PackageLock:
    """Decode one PackageLock."""
    name = reader.read_string()
    version = reader.read_option(lambda: reader.read_string())
    source = decode_source_lock(reader)
    source_hash = reader.read_string()
    manifest_hash = reader.read_string()
    dependencies = {
        reader.read_string(): destack._generated.repository.config.dependency.decode_dependency(
            reader
        )
        for _ in range(reader.read_number())
    }
    editable = reader.read_bool()

    return PackageLock(
        name=name,
        version=version,
        source=source,
        source_hash=source_hash,
        manifest_hash=manifest_hash,
        dependencies=dependencies,
        editable=editable,
    )


def to_json_package_lock(value: PackageLock) -> Json:
    """Return one JSON value for one PackageLock."""
    return {
        "name": value.name,
        **({} if value.version is None else {"version": value.version}),
        "source": to_json_source_lock(value.source),
        "sourceHash": value.source_hash,
        "manifestHash": value.manifest_hash,
        "dependencies": {
            key_0: destack._generated.repository.config.dependency.to_json_dependency(
                item_0
            )
            for key_0, item_0 in value.dependencies.items()
        },
        "editable": value.editable,
    }


def from_json_package_lock(value: Json) -> PackageLock:
    """Return one PackageLock from one JSON value."""
    object_ = json_object(value)

    return PackageLock(
        name=json_string(json_field(object_, "name")),
        version=json_optional(object_, "version", lambda value: json_string(value)),
        source=from_json_source_lock(json_field(object_, "source")),
        source_hash=json_string(json_field(object_, "sourceHash")),
        manifest_hash=json_string(json_field(object_, "manifestHash")),
        dependencies={
            key_0: destack._generated.repository.config.dependency.from_json_dependency(
                item_0
            )
            for key_0, item_0 in json_object(
                json_field(object_, "dependencies")
            ).items()
        },
        editable=json_bool(json_field(object_, "editable")),
    )


@dataclass(frozen=True, slots=True)
class SourceLockWorkspace:
    """Package resolved from a workspace member."""

    # workspace relative package path
    path: str
    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_lock(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_lock(self)


@dataclass(frozen=True, slots=True)
class SourceLockRegistry:
    """Package resolved from a registry."""

    # registry name
    registry: str | None
    # exact package version
    version: str
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_lock(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_lock(self)


@dataclass(frozen=True, slots=True)
class SourceLockPath:
    """Package resolved from a local path."""

    # package path
    path: str
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_lock(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_lock(self)


@dataclass(frozen=True, slots=True)
class SourceLockGit:
    """Package resolved from one Git repository."""

    # repository URL
    url: str
    # exact commit revision
    rev: str
    # repository subdirectory containing the package
    path: str | None
    kind: typing.Literal["git"] = "git"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_source_lock(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_source_lock(self)


"""Locked source locator."""
SourceLock: typing.TypeAlias = (
    SourceLockWorkspace | SourceLockRegistry | SourceLockPath | SourceLockGit
)


def encode_source_lock(writer: BinaryWriter, value: SourceLock) -> None:
    """Encode one SourceLock."""
    if value.kind == "workspace":
        writer.write_unsigned(0)
        writer.write_string(value.path)
    elif value.kind == "registry":
        writer.write_unsigned(1)
        if value.registry is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.registry)
        writer.write_string(value.version)
    elif value.kind == "path":
        writer.write_unsigned(2)
        writer.write_string(value.path)
    elif value.kind == "git":
        writer.write_unsigned(3)
        writer.write_string(value.url)
        writer.write_string(value.rev)
        if value.path is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.path)
    else:
        raise SerdeError("unknown enum variant")


def decode_source_lock(reader: BinaryReader) -> SourceLock:
    """Decode one SourceLock."""
    variant = reader.read_number()

    if variant == 0:
        path = reader.read_string()

        return SourceLockWorkspace(
            path=path,
        )
    elif variant == 1:
        registry = reader.read_option(lambda: reader.read_string())
        version = reader.read_string()

        return SourceLockRegistry(
            registry=registry,
            version=version,
        )
    elif variant == 2:
        path = reader.read_string()

        return SourceLockPath(
            path=path,
        )
    elif variant == 3:
        url = reader.read_string()
        rev = reader.read_string()
        path = reader.read_option(lambda: reader.read_string())

        return SourceLockGit(
            url=url,
            rev=rev,
            path=path,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_source_lock(value: SourceLock) -> Json:
    """Return one JSON value for one SourceLock."""
    if value.kind == "workspace":
        return {
            "kind": "workspace",
            "path": value.path,
        }
    elif value.kind == "registry":
        return {
            "kind": "registry",
            **({} if value.registry is None else {"registry": value.registry}),
            "version": value.version,
        }
    elif value.kind == "path":
        return {
            "kind": "path",
            "path": value.path,
        }
    elif value.kind == "git":
        return {
            "kind": "git",
            "url": value.url,
            "rev": value.rev,
            **({} if value.path is None else {"path": value.path}),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_source_lock(value: Json) -> SourceLock:
    """Return one SourceLock from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "workspace":
        return SourceLockWorkspace(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "registry":
        return SourceLockRegistry(
            registry=json_optional(
                object_, "registry", lambda value: json_string(value)
            ),
            version=json_string(json_field(object_, "version")),
        )
    elif kind == "path":
        return SourceLockPath(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "git":
        return SourceLockGit(
            url=json_string(json_field(object_, "url")),
            rev=json_string(json_field(object_, "rev")),
            path=json_optional(object_, "path", lambda value: json_string(value)),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class DestackLock:
    """Destack lock document."""

    # lock document format version
    version: int
    # root package name
    root: str | None
    # locked packages keyed by package import name
    packages: Mapping[str, PackageLock]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_destack_lock(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> DestackLock:
        """Decode one DestackLock."""
        return decode_destack_lock(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_destack_lock(self)

    @classmethod
    def from_json(cls, value: Json) -> DestackLock:
        """Return one DestackLock from one JSON value."""
        return from_json_destack_lock(value)


def encode_destack_lock(writer: BinaryWriter, value: DestackLock) -> None:
    """Encode one DestackLock."""
    writer.write_unsigned(value.version)
    if value.root is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.root)
    entries_value_packages_0 = []
    for key_value_packages_0, item_value_packages_0 in value.packages.items():

        def write_key_value_packages_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_packages_0)

        key_bytes = nested_bytes(write_key_value_packages_0)
        entries_value_packages_0.append(
            (key_value_packages_0, item_value_packages_0, key_bytes)
        )
    entries_value_packages_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_packages_0))
    for entry_value_packages_0 in entries_value_packages_0:
        writer.write_string(entry_value_packages_0[0])
        encode_package_lock(writer, entry_value_packages_0[1])


def decode_destack_lock(reader: BinaryReader) -> DestackLock:
    """Decode one DestackLock."""
    version = reader.read_number()
    root = reader.read_option(lambda: reader.read_string())
    packages = {
        reader.read_string(): decode_package_lock(reader)
        for _ in range(reader.read_number())
    }

    return DestackLock(
        version=version,
        root=root,
        packages=packages,
    )


def to_json_destack_lock(value: DestackLock) -> Json:
    """Return one JSON value for one DestackLock."""
    return {
        "version": value.version,
        **({} if value.root is None else {"root": value.root}),
        "packages": {
            key_0: to_json_package_lock(item_0)
            for key_0, item_0 in value.packages.items()
        },
    }


def from_json_destack_lock(value: Json) -> DestackLock:
    """Return one DestackLock from one JSON value."""
    object_ = json_object(value)

    return DestackLock(
        version=json_int(json_field(object_, "version")),
        root=json_optional(object_, "root", lambda value: json_string(value)),
        packages={
            key_0: from_json_package_lock(item_0)
            for key_0, item_0 in json_object(json_field(object_, "packages")).items()
        },
    )


__all__ = [
    "PackageLock",
    "encode_package_lock",
    "decode_package_lock",
    "to_json_package_lock",
    "from_json_package_lock",
    "SourceLock",
    "encode_source_lock",
    "decode_source_lock",
    "to_json_source_lock",
    "from_json_source_lock",
    "SourceLockWorkspace",
    "SourceLockRegistry",
    "SourceLockPath",
    "SourceLockGit",
    "DestackLock",
    "encode_destack_lock",
    "decode_destack_lock",
    "to_json_destack_lock",
    "from_json_destack_lock",
]
