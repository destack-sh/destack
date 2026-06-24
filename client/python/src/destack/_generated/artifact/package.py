# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_bool,
    json_field,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.source.file.model.package
import destack._generated.source.file.model.profile


@dataclass(frozen=True, slots=True)
class PackageImportIndex:
    """Active import index for one package."""

    # the package this index belongs to
    package: destack._generated.source.file.model.package.PackageId
    # package root path when filesystem backed
    root: str | None
    # active direct dependencies keyed by package specifier
    dependencies: Mapping[
        str, destack._generated.source.file.model.package.PackageId | None
    ]
    # active public exports
    exports: ExportIndex

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_import_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageImportIndex:
        """Decode one PackageImportIndex."""
        return decode_package_import_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_import_index(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageImportIndex:
        """Return one PackageImportIndex from one JSON value."""
        return from_json_package_import_index(value)


def encode_package_import_index(
    writer: BinaryWriter, value: PackageImportIndex
) -> None:
    """Encode one PackageImportIndex."""
    destack._generated.source.file.model.package.encode_package_id(
        writer, value.package
    )
    if value.root is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.root)
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
        if entry_value_dependencies_0[1] is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.package.encode_package_id(
                writer, entry_value_dependencies_0[1]
            )
    encode_export_index(writer, value.exports)


def decode_package_import_index(reader: BinaryReader) -> PackageImportIndex:
    """Decode one PackageImportIndex."""
    package = destack._generated.source.file.model.package.decode_package_id(reader)
    root = reader.read_option(lambda: reader.read_string())
    dependencies = {
        reader.read_string(): reader.read_option(
            lambda: destack._generated.source.file.model.package.decode_package_id(
                reader
            )
        )
        for _ in range(reader.read_number())
    }
    exports = decode_export_index(reader)

    return PackageImportIndex(
        package=package,
        root=root,
        dependencies=dependencies,
        exports=exports,
    )


def to_json_package_import_index(value: PackageImportIndex) -> Json:
    """Return one JSON value for one PackageImportIndex."""
    return {
        "package": destack._generated.source.file.model.package.to_json_package_id(
            value.package
        ),
        **({} if value.root is None else {"root": value.root}),
        "dependencies": {
            key_0: None
            if item_0 is None
            else destack._generated.source.file.model.package.to_json_package_id(item_0)
            for key_0, item_0 in value.dependencies.items()
        },
        "exports": to_json_export_index(value.exports),
    }


def from_json_package_import_index(value: Json) -> PackageImportIndex:
    """Return one PackageImportIndex from one JSON value."""
    object_ = json_object(value)

    return PackageImportIndex(
        package=destack._generated.source.file.model.package.from_json_package_id(
            json_field(object_, "package")
        ),
        root=json_optional(object_, "root", lambda value: json_string(value)),
        dependencies={
            key_0: None
            if item_0 is None
            else destack._generated.source.file.model.package.from_json_package_id(
                item_0
            )
            for key_0, item_0 in json_object(
                json_field(object_, "dependencies")
            ).items()
        },
        exports=from_json_export_index(json_field(object_, "exports")),
    )


@dataclass(frozen=True, slots=True)
class ExportIndex:
    """Active package exports indexed for module import resolution."""

    # the package this export index belongs to
    package: destack._generated.source.file.model.package.PackageId
    # exact exports keyed by export specifier
    exact: Mapping[str, ExportTarget]
    # pattern exports sorted from most specific to least specific
    patterns: Sequence[ExportPattern]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportIndex:
        """Decode one ExportIndex."""
        return decode_export_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_index(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportIndex:
        """Return one ExportIndex from one JSON value."""
        return from_json_export_index(value)


def encode_export_index(writer: BinaryWriter, value: ExportIndex) -> None:
    """Encode one ExportIndex."""
    destack._generated.source.file.model.package.encode_package_id(
        writer, value.package
    )
    entries_value_exact_0 = []
    for key_value_exact_0, item_value_exact_0 in value.exact.items():

        def write_key_value_exact_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_exact_0)

        key_bytes = nested_bytes(write_key_value_exact_0)
        entries_value_exact_0.append((key_value_exact_0, item_value_exact_0, key_bytes))
    entries_value_exact_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_exact_0))
    for entry_value_exact_0 in entries_value_exact_0:
        writer.write_string(entry_value_exact_0[0])
        encode_export_target(writer, entry_value_exact_0[1])
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        encode_export_pattern(writer, item_value_patterns_0)


def decode_export_index(reader: BinaryReader) -> ExportIndex:
    """Decode one ExportIndex."""
    package = destack._generated.source.file.model.package.decode_package_id(reader)
    exact = {
        reader.read_string(): decode_export_target(reader)
        for _ in range(reader.read_number())
    }
    patterns = [decode_export_pattern(reader) for _ in range(reader.read_number())]

    return ExportIndex(
        package=package,
        exact=exact,
        patterns=patterns,
    )


def to_json_export_index(value: ExportIndex) -> Json:
    """Return one JSON value for one ExportIndex."""
    return {
        "package": destack._generated.source.file.model.package.to_json_package_id(
            value.package
        ),
        "exact": {
            key_0: to_json_export_target(item_0)
            for key_0, item_0 in value.exact.items()
        },
        "patterns": [to_json_export_pattern(item_0) for item_0 in value.patterns],
    }


def from_json_export_index(value: Json) -> ExportIndex:
    """Return one ExportIndex from one JSON value."""
    object_ = json_object(value)

    return ExportIndex(
        package=destack._generated.source.file.model.package.from_json_package_id(
            json_field(object_, "package")
        ),
        exact={
            key_0: from_json_export_target(item_0)
            for key_0, item_0 in json_object(json_field(object_, "exact")).items()
        },
        patterns=[
            from_json_export_pattern(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ExportTarget:
    """Active package export target."""

    # package relative export path
    path: str
    # whether the export can be imported as a source module
    is_module: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_target(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportTarget:
        """Decode one ExportTarget."""
        return decode_export_target(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_target(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportTarget:
        """Return one ExportTarget from one JSON value."""
        return from_json_export_target(value)


def encode_export_target(writer: BinaryWriter, value: ExportTarget) -> None:
    """Encode one ExportTarget."""
    writer.write_string(value.path)
    writer.write_bool(value.is_module)


def decode_export_target(reader: BinaryReader) -> ExportTarget:
    """Decode one ExportTarget."""
    path = reader.read_string()
    is_module = reader.read_bool()

    return ExportTarget(
        path=path,
        is_module=is_module,
    )


def to_json_export_target(value: ExportTarget) -> Json:
    """Return one JSON value for one ExportTarget."""
    return {
        "path": value.path,
        "isModule": value.is_module,
    }


def from_json_export_target(value: Json) -> ExportTarget:
    """Return one ExportTarget from one JSON value."""
    object_ = json_object(value)

    return ExportTarget(
        path=json_string(json_field(object_, "path")),
        is_module=json_bool(json_field(object_, "isModule")),
    )


@dataclass(frozen=True, slots=True)
class ExportPattern:
    """Active pattern export target."""

    # export key prefix before `*`
    prefix: str
    # export key suffix after `*`
    suffix: str
    # pattern export target
    target: ExportTarget

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_export_pattern(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ExportPattern:
        """Decode one ExportPattern."""
        return decode_export_pattern(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_export_pattern(self)

    @classmethod
    def from_json(cls, value: Json) -> ExportPattern:
        """Return one ExportPattern from one JSON value."""
        return from_json_export_pattern(value)


def encode_export_pattern(writer: BinaryWriter, value: ExportPattern) -> None:
    """Encode one ExportPattern."""
    writer.write_string(value.prefix)
    writer.write_string(value.suffix)
    encode_export_target(writer, value.target)


def decode_export_pattern(reader: BinaryReader) -> ExportPattern:
    """Decode one ExportPattern."""
    prefix = reader.read_string()
    suffix = reader.read_string()
    target = decode_export_target(reader)

    return ExportPattern(
        prefix=prefix,
        suffix=suffix,
        target=target,
    )


def to_json_export_pattern(value: ExportPattern) -> Json:
    """Return one JSON value for one ExportPattern."""
    return {
        "prefix": value.prefix,
        "suffix": value.suffix,
        "target": to_json_export_target(value.target),
    }


def from_json_export_pattern(value: Json) -> ExportPattern:
    """Return one ExportPattern from one JSON value."""
    object_ = json_object(value)

    return ExportPattern(
        prefix=json_string(json_field(object_, "prefix")),
        suffix=json_string(json_field(object_, "suffix")),
        target=from_json_export_target(json_field(object_, "target")),
    )


@dataclass(frozen=True, slots=True)
class PackageIndex:
    """Active package dependency and export index for one profile."""

    # the profile this index belongs to
    profile: destack._generated.source.file.model.profile.ProfileId
    # indexed packages keyed by package id
    packages: Mapping[
        destack._generated.source.file.model.package.PackageId, PackageImportIndex
    ]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_index(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageIndex:
        """Decode one PackageIndex."""
        return decode_package_index(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_index(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageIndex:
        """Return one PackageIndex from one JSON value."""
        return from_json_package_index(value)


def encode_package_index(writer: BinaryWriter, value: PackageIndex) -> None:
    """Encode one PackageIndex."""
    destack._generated.source.file.model.profile.encode_profile_id(
        writer, value.profile
    )
    entries_value_packages_0 = []
    for key_value_packages_0, item_value_packages_0 in value.packages.items():

        def write_key_value_packages_0(writer: BinaryWriter) -> None:
            destack._generated.source.file.model.package.encode_package_id(
                writer, key_value_packages_0
            )

        key_bytes = nested_bytes(write_key_value_packages_0)
        entries_value_packages_0.append(
            (key_value_packages_0, item_value_packages_0, key_bytes)
        )
    entries_value_packages_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_packages_0))
    for entry_value_packages_0 in entries_value_packages_0:
        destack._generated.source.file.model.package.encode_package_id(
            writer, entry_value_packages_0[0]
        )
        encode_package_import_index(writer, entry_value_packages_0[1])


def decode_package_index(reader: BinaryReader) -> PackageIndex:
    """Decode one PackageIndex."""
    profile = destack._generated.source.file.model.profile.decode_profile_id(reader)
    packages = {
        destack._generated.source.file.model.package.decode_package_id(
            reader
        ): decode_package_import_index(reader)
        for _ in range(reader.read_number())
    }

    return PackageIndex(
        profile=profile,
        packages=packages,
    )


def to_json_package_index(value: PackageIndex) -> Json:
    """Return one JSON value for one PackageIndex."""
    return {
        "profile": destack._generated.source.file.model.profile.to_json_profile_id(
            value.profile
        ),
        "packages": [
            [
                destack._generated.source.file.model.package.to_json_package_id(key_0),
                to_json_package_import_index(item_0),
            ]
            for key_0, item_0 in value.packages.items()
        ],
    }


def from_json_package_index(value: Json) -> PackageIndex:
    """Return one PackageIndex from one JSON value."""
    object_ = json_object(value)

    return PackageIndex(
        profile=destack._generated.source.file.model.profile.from_json_profile_id(
            json_field(object_, "profile")
        ),
        packages={
            destack._generated.source.file.model.package.from_json_package_id(
                key_0
            ): from_json_package_import_index(item_0)
            for key_0, item_0 in json_array(json_field(object_, "packages"))
        },
    )


__all__ = [
    "PackageImportIndex",
    "encode_package_import_index",
    "decode_package_import_index",
    "to_json_package_import_index",
    "from_json_package_import_index",
    "ExportIndex",
    "encode_export_index",
    "decode_export_index",
    "to_json_export_index",
    "from_json_export_index",
    "ExportTarget",
    "encode_export_target",
    "decode_export_target",
    "to_json_export_target",
    "from_json_export_target",
    "ExportPattern",
    "encode_export_pattern",
    "decode_export_pattern",
    "to_json_export_pattern",
    "from_json_export_pattern",
    "PackageIndex",
    "encode_package_index",
    "decode_package_index",
    "to_json_package_index",
    "from_json_package_index",
]
