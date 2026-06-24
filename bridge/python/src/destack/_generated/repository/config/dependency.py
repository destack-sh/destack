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
    json_field,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.repository.config.condition


@dataclass(frozen=True, slots=True)
class DependencyWorkspace:
    """Package resolved from the current workspace."""

    kind: typing.Literal["workspace"] = "workspace"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency(self)


@dataclass(frozen=True, slots=True)
class DependencyRegistry:
    """Package resolved from the configured package registry."""

    # registry name
    registry: str | None
    # exact package version
    version: str
    kind: typing.Literal["registry"] = "registry"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency(self)


@dataclass(frozen=True, slots=True)
class DependencyPath:
    """Package resolved from one local package path."""

    # package path
    path: str
    kind: typing.Literal["path"] = "path"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency(self)


@dataclass(frozen=True, slots=True)
class DependencyGit:
    """Package resolved from one Git repository."""

    # repository URL
    url: str
    # repository subdirectory containing the package
    path: str | None
    # exact commit revision
    rev: str | None
    # git tag selector
    tag: str | None
    # git branch selector
    branch: str | None
    kind: typing.Literal["git"] = "git"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_dependency(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_dependency(self)


"""Package dependency declaration."""
Dependency: typing.TypeAlias = (
    DependencyWorkspace | DependencyRegistry | DependencyPath | DependencyGit
)


def encode_dependency(writer: BinaryWriter, value: Dependency) -> None:
    """Encode one Dependency."""
    if value.kind == "workspace":
        writer.write_unsigned(0)
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
        if value.path is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.path)
        if value.rev is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.rev)
        if value.tag is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.tag)
        if value.branch is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_string(value.branch)
    else:
        raise SerdeError("unknown enum variant")


def decode_dependency(reader: BinaryReader) -> Dependency:
    """Decode one Dependency."""
    variant = reader.read_number()

    if variant == 0:
        return DependencyWorkspace()
    elif variant == 1:
        registry = reader.read_option(lambda: reader.read_string())
        version = reader.read_string()

        return DependencyRegistry(
            registry=registry,
            version=version,
        )
    elif variant == 2:
        path = reader.read_string()

        return DependencyPath(
            path=path,
        )
    elif variant == 3:
        url = reader.read_string()
        path = reader.read_option(lambda: reader.read_string())
        rev = reader.read_option(lambda: reader.read_string())
        tag = reader.read_option(lambda: reader.read_string())
        branch = reader.read_option(lambda: reader.read_string())

        return DependencyGit(
            url=url,
            path=path,
            rev=rev,
            tag=tag,
            branch=branch,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_dependency(value: Dependency) -> Json:
    """Return one JSON value for one Dependency."""
    if value.kind == "workspace":
        return {
            "kind": "workspace",
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
            **({} if value.path is None else {"path": value.path}),
            **({} if value.rev is None else {"rev": value.rev}),
            **({} if value.tag is None else {"tag": value.tag}),
            **({} if value.branch is None else {"branch": value.branch}),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_dependency(value: Json) -> Dependency:
    """Return one Dependency from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "workspace":
        return DependencyWorkspace()
    elif kind == "registry":
        return DependencyRegistry(
            registry=json_optional(
                object_, "registry", lambda value: json_string(value)
            ),
            version=json_string(json_field(object_, "version")),
        )
    elif kind == "path":
        return DependencyPath(
            path=json_string(json_field(object_, "path")),
        )
    elif kind == "git":
        return DependencyGit(
            url=json_string(json_field(object_, "url")),
            path=json_optional(object_, "path", lambda value: json_string(value)),
            rev=json_optional(object_, "rev", lambda value: json_string(value)),
            tag=json_optional(object_, "tag", lambda value: json_string(value)),
            branch=json_optional(object_, "branch", lambda value: json_string(value)),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class ConditionalDependencies:
    """Dependency declarations guarded by one active condition predicate."""

    # condition predicate enabling these dependencies
    when: destack._generated.repository.config.condition.ConditionRef
    # dependency declarations enabled when the predicate matches
    dependencies: Mapping[str, Dependency]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_conditional_dependencies(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ConditionalDependencies:
        """Decode one ConditionalDependencies."""
        return decode_conditional_dependencies(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_conditional_dependencies(self)

    @classmethod
    def from_json(cls, value: Json) -> ConditionalDependencies:
        """Return one ConditionalDependencies from one JSON value."""
        return from_json_conditional_dependencies(value)


def encode_conditional_dependencies(
    writer: BinaryWriter, value: ConditionalDependencies
) -> None:
    """Encode one ConditionalDependencies."""
    destack._generated.repository.config.condition.encode_condition_ref(
        writer, value.when
    )
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
        encode_dependency(writer, entry_value_dependencies_0[1])


def decode_conditional_dependencies(reader: BinaryReader) -> ConditionalDependencies:
    """Decode one ConditionalDependencies."""
    when = destack._generated.repository.config.condition.decode_condition_ref(reader)
    dependencies = {
        reader.read_string(): decode_dependency(reader)
        for _ in range(reader.read_number())
    }

    return ConditionalDependencies(
        when=when,
        dependencies=dependencies,
    )


def to_json_conditional_dependencies(value: ConditionalDependencies) -> Json:
    """Return one JSON value for one ConditionalDependencies."""
    return {
        "when": destack._generated.repository.config.condition.to_json_condition_ref(
            value.when
        ),
        "dependencies": {
            key_0: to_json_dependency(item_0)
            for key_0, item_0 in value.dependencies.items()
        },
    }


def from_json_conditional_dependencies(value: Json) -> ConditionalDependencies:
    """Return one ConditionalDependencies from one JSON value."""
    object_ = json_object(value)

    return ConditionalDependencies(
        when=destack._generated.repository.config.condition.from_json_condition_ref(
            json_field(object_, "when")
        ),
        dependencies={
            key_0: from_json_dependency(item_0)
            for key_0, item_0 in json_object(
                json_field(object_, "dependencies")
            ).items()
        },
    )


@dataclass(frozen=True, slots=True)
class PackagePatch:
    """Patch file applied to one resolved package."""

    # patch file path
    path: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_patch(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackagePatch:
        """Decode one PackagePatch."""
        return decode_package_patch(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_patch(self)

    @classmethod
    def from_json(cls, value: Json) -> PackagePatch:
        """Return one PackagePatch from one JSON value."""
        return from_json_package_patch(value)


def encode_package_patch(writer: BinaryWriter, value: PackagePatch) -> None:
    """Encode one PackagePatch."""
    writer.write_string(value.path)


def decode_package_patch(reader: BinaryReader) -> PackagePatch:
    """Decode one PackagePatch."""
    path = reader.read_string()

    return PackagePatch(
        path=path,
    )


def to_json_package_patch(value: PackagePatch) -> Json:
    """Return one JSON value for one PackagePatch."""
    return {
        "path": value.path,
    }


def from_json_package_patch(value: Json) -> PackagePatch:
    """Return one PackagePatch from one JSON value."""
    object_ = json_object(value)

    return PackagePatch(
        path=json_string(json_field(object_, "path")),
    )


__all__ = [
    "Dependency",
    "encode_dependency",
    "decode_dependency",
    "to_json_dependency",
    "from_json_dependency",
    "DependencyWorkspace",
    "DependencyRegistry",
    "DependencyPath",
    "DependencyGit",
    "ConditionalDependencies",
    "encode_conditional_dependencies",
    "decode_conditional_dependencies",
    "to_json_conditional_dependencies",
    "from_json_conditional_dependencies",
    "PackagePatch",
    "encode_package_patch",
    "decode_package_patch",
    "to_json_package_patch",
    "from_json_package_patch",
]
