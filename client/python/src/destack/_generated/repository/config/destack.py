# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

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

import destack._generated.repository.config.compiler
import destack._generated.repository.config.condition
import destack._generated.repository.config.dependency
import destack._generated.repository.config.export
import destack._generated.repository.config.formatter
import destack._generated.repository.config.linter.core
import destack._generated.repository.config.policy
import destack._generated.repository.config.product.product
import destack._generated.repository.config.profile
import destack._generated.repository.config.runtime.runtime
import destack._generated.repository.config.stage
import destack._generated.repository.config.target.target
import destack._generated.repository.config.task
import destack._generated.repository.config.topology
import destack._generated.repository.config.vendor


@dataclass(frozen=True, slots=True)
class WorkspaceLayout:
    """Workspace package layout."""

    # package root glob patterns
    packages: Sequence[str] | None
    # named groups of package paths
    groups: Mapping[str, Sequence[str]] | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_workspace_layout(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> WorkspaceLayout:
        """Decode one WorkspaceLayout."""
        return decode_workspace_layout(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_workspace_layout(self)

    @classmethod
    def from_json(cls, value: Json) -> WorkspaceLayout:
        """Return one WorkspaceLayout from one JSON value."""
        return from_json_workspace_layout(value)


def encode_workspace_layout(writer: BinaryWriter, value: WorkspaceLayout) -> None:
    """Encode one WorkspaceLayout."""
    if value.packages is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.packages))
        for item_value_packages_1 in value.packages:
            writer.write_string(item_value_packages_1)
    if value.groups is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        entries_value_groups_1 = []
        for key_value_groups_1, item_value_groups_1 in value.groups.items():

            def write_key_value_groups_1(writer: BinaryWriter) -> None:
                writer.write_string(key_value_groups_1)

            key_bytes = nested_bytes(write_key_value_groups_1)
            entries_value_groups_1.append(
                (key_value_groups_1, item_value_groups_1, key_bytes)
            )
        entries_value_groups_1.sort(key=lambda entry: entry[2])
        writer.write_unsigned(len(entries_value_groups_1))
        for entry_value_groups_1 in entries_value_groups_1:
            writer.write_string(entry_value_groups_1[0])
            writer.write_unsigned(len(entry_value_groups_1[1]))
            for item_entry_value_groups_1_1_2 in entry_value_groups_1[1]:
                writer.write_string(item_entry_value_groups_1_1_2)


def decode_workspace_layout(reader: BinaryReader) -> WorkspaceLayout:
    """Decode one WorkspaceLayout."""
    packages = reader.read_option(
        lambda: [reader.read_string() for _ in range(reader.read_number())]
    )
    groups = reader.read_option(
        lambda: {
            reader.read_string(): [
                reader.read_string() for _ in range(reader.read_number())
            ]
            for _ in range(reader.read_number())
        }
    )

    return WorkspaceLayout(
        packages=packages,
        groups=groups,
    )


def to_json_workspace_layout(value: WorkspaceLayout) -> Json:
    """Return one JSON value for one WorkspaceLayout."""
    return {
        **(
            {}
            if value.packages is None
            else {"packages": [item_0 for item_0 in value.packages]}
        ),
        **(
            {}
            if value.groups is None
            else {
                "groups": {
                    key_0: [item_1 for item_1 in item_0]
                    for key_0, item_0 in value.groups.items()
                }
            }
        ),
    }


def from_json_workspace_layout(value: Json) -> WorkspaceLayout:
    """Return one WorkspaceLayout from one JSON value."""
    object_ = json_object(value)

    return WorkspaceLayout(
        packages=json_optional(
            object_,
            "packages",
            lambda value: [json_string(item_0) for item_0 in json_array(value)],
        ),
        groups=json_optional(
            object_,
            "groups",
            lambda value: {
                key_0: [json_string(item_1) for item_1 in json_array(item_0)]
                for key_0, item_0 in json_object(value).items()
            },
        ),
    )


@dataclass(frozen=True, slots=True)
class Destack:
    """Destack configuration document."""

    # package name
    name: str | None
    # package version
    version: str | None
    # package release stage
    stage: destack._generated.repository.config.stage.Stage | None
    # whether the package is private
    private: bool | None
    # package description
    description: str | None
    # package license identifier
    license: str | None
    # package repository metadata
    repository: typing.Any | None
    # package homepage
    homepage: str | None
    # package keywords
    keywords: Sequence[str]
    # repository wide workspace package and group configuration
    workspace: WorkspaceLayout | None
    # config path inherited before this config
    extends: str | None
    # specific files to include in the project
    files: Sequence[str]
    # glob patterns for files to include
    include: Sequence[str]
    # glob patterns for files to exclude
    exclude: Sequence[str]
    # public package exports
    exports: Mapping[str, destack._generated.repository.config.export.Export]
    # package dependencies
    dependencies: Mapping[
        str, destack._generated.repository.config.dependency.Dependency
    ]
    # dependencies enabled by condition predicates
    conditional_dependencies: Sequence[
        destack._generated.repository.config.dependency.ConditionalDependencies
    ]
    # package dependency overrides
    overrides: Mapping[str, destack._generated.repository.config.dependency.Dependency]
    # package patch files
    patches: Mapping[str, destack._generated.repository.config.dependency.PackagePatch]
    # vendored dependency resolution declaration
    vendor: destack._generated.repository.config.vendor.Vendor
    # package topology definition
    topology: destack._generated.repository.config.topology.Topology
    # compiler configuration
    compiler: destack._generated.repository.config.compiler.CompilerOptions
    # package policy declarations and rules
    policy: destack._generated.repository.config.policy.Policy
    # runtime configuration
    runtime: destack._generated.repository.config.runtime.runtime.RuntimeOptions
    # formatter configuration
    formatter: destack._generated.repository.config.formatter.FormatterOptions
    # linter configuration
    linter: destack._generated.repository.config.linter.core.LinterOptions
    # build targets
    targets: Mapping[str, destack._generated.repository.config.target.target.Target]
    # deliverable products
    products: Mapping[str, destack._generated.repository.config.product.product.Product]
    # named profiles for semantic configuration
    profiles: Mapping[str, destack._generated.repository.config.profile.ProfileOptions]
    # named source graph conditions
    conditions: destack._generated.repository.config.condition.ConditionCatalog
    # named toolchain and shell tasks
    tasks: Mapping[str, destack._generated.repository.config.task.Task]
    # default target for the package
    default_target: str | None
    # default product for the package
    default_product: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_destack(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Destack:
        """Decode one Destack."""
        return decode_destack(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_destack(self)

    @classmethod
    def from_json(cls, value: Json) -> Destack:
        """Return one Destack from one JSON value."""
        return from_json_destack(value)


def encode_destack(writer: BinaryWriter, value: Destack) -> None:
    """Encode one Destack."""
    if value.name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.name)
    if value.version is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.version)
    if value.stage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.repository.config.stage.encode_stage(writer, value.stage)
    if value.private is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.private)
    if value.description is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.description)
    if value.license is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.license)
    if value.repository is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_json(value.repository)
    if value.homepage is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.homepage)
    writer.write_unsigned(len(value.keywords))
    for item_value_keywords_0 in value.keywords:
        writer.write_string(item_value_keywords_0)
    if value.workspace is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        encode_workspace_layout(writer, value.workspace)
    if value.extends is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.extends)
    writer.write_unsigned(len(value.files))
    for item_value_files_0 in value.files:
        writer.write_string(item_value_files_0)
    writer.write_unsigned(len(value.include))
    for item_value_include_0 in value.include:
        writer.write_string(item_value_include_0)
    writer.write_unsigned(len(value.exclude))
    for item_value_exclude_0 in value.exclude:
        writer.write_string(item_value_exclude_0)
    entries_value_exports_0 = []
    for key_value_exports_0, item_value_exports_0 in value.exports.items():

        def write_key_value_exports_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_exports_0)

        key_bytes = nested_bytes(write_key_value_exports_0)
        entries_value_exports_0.append(
            (key_value_exports_0, item_value_exports_0, key_bytes)
        )
    entries_value_exports_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_exports_0))
    for entry_value_exports_0 in entries_value_exports_0:
        writer.write_string(entry_value_exports_0[0])
        destack._generated.repository.config.export.encode_export(
            writer, entry_value_exports_0[1]
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
        destack._generated.repository.config.dependency.encode_dependency(
            writer, entry_value_dependencies_0[1]
        )
    writer.write_unsigned(len(value.conditional_dependencies))
    for item_value_conditional_dependencies_0 in value.conditional_dependencies:
        destack._generated.repository.config.dependency.encode_conditional_dependencies(
            writer, item_value_conditional_dependencies_0
        )
    entries_value_overrides_0 = []
    for key_value_overrides_0, item_value_overrides_0 in value.overrides.items():

        def write_key_value_overrides_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_overrides_0)

        key_bytes = nested_bytes(write_key_value_overrides_0)
        entries_value_overrides_0.append(
            (key_value_overrides_0, item_value_overrides_0, key_bytes)
        )
    entries_value_overrides_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_overrides_0))
    for entry_value_overrides_0 in entries_value_overrides_0:
        writer.write_string(entry_value_overrides_0[0])
        destack._generated.repository.config.dependency.encode_dependency(
            writer, entry_value_overrides_0[1]
        )
    entries_value_patches_0 = []
    for key_value_patches_0, item_value_patches_0 in value.patches.items():

        def write_key_value_patches_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_patches_0)

        key_bytes = nested_bytes(write_key_value_patches_0)
        entries_value_patches_0.append(
            (key_value_patches_0, item_value_patches_0, key_bytes)
        )
    entries_value_patches_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_patches_0))
    for entry_value_patches_0 in entries_value_patches_0:
        writer.write_string(entry_value_patches_0[0])
        destack._generated.repository.config.dependency.encode_package_patch(
            writer, entry_value_patches_0[1]
        )
    destack._generated.repository.config.vendor.encode_vendor(writer, value.vendor)
    destack._generated.repository.config.topology.encode_topology(
        writer, value.topology
    )
    destack._generated.repository.config.compiler.encode_compiler_options(
        writer, value.compiler
    )
    destack._generated.repository.config.policy.encode_policy(writer, value.policy)
    destack._generated.repository.config.runtime.runtime.encode_runtime_options(
        writer, value.runtime
    )
    destack._generated.repository.config.formatter.encode_formatter_options(
        writer, value.formatter
    )
    destack._generated.repository.config.linter.core.encode_linter_options(
        writer, value.linter
    )
    entries_value_targets_0 = []
    for key_value_targets_0, item_value_targets_0 in value.targets.items():

        def write_key_value_targets_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_targets_0)

        key_bytes = nested_bytes(write_key_value_targets_0)
        entries_value_targets_0.append(
            (key_value_targets_0, item_value_targets_0, key_bytes)
        )
    entries_value_targets_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_targets_0))
    for entry_value_targets_0 in entries_value_targets_0:
        writer.write_string(entry_value_targets_0[0])
        destack._generated.repository.config.target.target.encode_target(
            writer, entry_value_targets_0[1]
        )
    entries_value_products_0 = []
    for key_value_products_0, item_value_products_0 in value.products.items():

        def write_key_value_products_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_products_0)

        key_bytes = nested_bytes(write_key_value_products_0)
        entries_value_products_0.append(
            (key_value_products_0, item_value_products_0, key_bytes)
        )
    entries_value_products_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_products_0))
    for entry_value_products_0 in entries_value_products_0:
        writer.write_string(entry_value_products_0[0])
        destack._generated.repository.config.product.product.encode_product(
            writer, entry_value_products_0[1]
        )
    entries_value_profiles_0 = []
    for key_value_profiles_0, item_value_profiles_0 in value.profiles.items():

        def write_key_value_profiles_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_profiles_0)

        key_bytes = nested_bytes(write_key_value_profiles_0)
        entries_value_profiles_0.append(
            (key_value_profiles_0, item_value_profiles_0, key_bytes)
        )
    entries_value_profiles_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_profiles_0))
    for entry_value_profiles_0 in entries_value_profiles_0:
        writer.write_string(entry_value_profiles_0[0])
        destack._generated.repository.config.profile.encode_profile_options(
            writer, entry_value_profiles_0[1]
        )
    destack._generated.repository.config.condition.encode_condition_catalog(
        writer, value.conditions
    )
    entries_value_tasks_0 = []
    for key_value_tasks_0, item_value_tasks_0 in value.tasks.items():

        def write_key_value_tasks_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_tasks_0)

        key_bytes = nested_bytes(write_key_value_tasks_0)
        entries_value_tasks_0.append((key_value_tasks_0, item_value_tasks_0, key_bytes))
    entries_value_tasks_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_tasks_0))
    for entry_value_tasks_0 in entries_value_tasks_0:
        writer.write_string(entry_value_tasks_0[0])
        destack._generated.repository.config.task.encode_task(
            writer, entry_value_tasks_0[1]
        )
    if value.default_target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.default_target)
    if value.default_product is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.default_product)


def decode_destack(reader: BinaryReader) -> Destack:
    """Decode one Destack."""
    name = reader.read_option(lambda: reader.read_string())
    version = reader.read_option(lambda: reader.read_string())
    stage = reader.read_option(
        lambda: destack._generated.repository.config.stage.decode_stage(reader)
    )
    private = reader.read_option(lambda: reader.read_bool())
    description = reader.read_option(lambda: reader.read_string())
    license = reader.read_option(lambda: reader.read_string())
    repository = reader.read_option(lambda: reader.read_json())
    homepage = reader.read_option(lambda: reader.read_string())
    keywords = [reader.read_string() for _ in range(reader.read_number())]
    workspace = reader.read_option(lambda: decode_workspace_layout(reader))
    extends = reader.read_option(lambda: reader.read_string())
    files = [reader.read_string() for _ in range(reader.read_number())]
    include = [reader.read_string() for _ in range(reader.read_number())]
    exclude = [reader.read_string() for _ in range(reader.read_number())]
    exports = {
        reader.read_string(): destack._generated.repository.config.export.decode_export(
            reader
        )
        for _ in range(reader.read_number())
    }
    dependencies = {
        reader.read_string(): destack._generated.repository.config.dependency.decode_dependency(
            reader
        )
        for _ in range(reader.read_number())
    }
    conditional_dependencies = [
        destack._generated.repository.config.dependency.decode_conditional_dependencies(
            reader
        )
        for _ in range(reader.read_number())
    ]
    overrides = {
        reader.read_string(): destack._generated.repository.config.dependency.decode_dependency(
            reader
        )
        for _ in range(reader.read_number())
    }
    patches = {
        reader.read_string(): destack._generated.repository.config.dependency.decode_package_patch(
            reader
        )
        for _ in range(reader.read_number())
    }
    vendor = destack._generated.repository.config.vendor.decode_vendor(reader)
    topology = destack._generated.repository.config.topology.decode_topology(reader)
    compiler = destack._generated.repository.config.compiler.decode_compiler_options(
        reader
    )
    policy = destack._generated.repository.config.policy.decode_policy(reader)
    runtime = (
        destack._generated.repository.config.runtime.runtime.decode_runtime_options(
            reader
        )
    )
    formatter = destack._generated.repository.config.formatter.decode_formatter_options(
        reader
    )
    linter = destack._generated.repository.config.linter.core.decode_linter_options(
        reader
    )
    targets = {
        reader.read_string(): destack._generated.repository.config.target.target.decode_target(
            reader
        )
        for _ in range(reader.read_number())
    }
    products = {
        reader.read_string(): destack._generated.repository.config.product.product.decode_product(
            reader
        )
        for _ in range(reader.read_number())
    }
    profiles = {
        reader.read_string(): destack._generated.repository.config.profile.decode_profile_options(
            reader
        )
        for _ in range(reader.read_number())
    }
    conditions = (
        destack._generated.repository.config.condition.decode_condition_catalog(reader)
    )
    tasks = {
        reader.read_string(): destack._generated.repository.config.task.decode_task(
            reader
        )
        for _ in range(reader.read_number())
    }
    default_target = reader.read_option(lambda: reader.read_string())
    default_product = reader.read_option(lambda: reader.read_string())

    return Destack(
        name=name,
        version=version,
        stage=stage,
        private=private,
        description=description,
        license=license,
        repository=repository,
        homepage=homepage,
        keywords=keywords,
        workspace=workspace,
        extends=extends,
        files=files,
        include=include,
        exclude=exclude,
        exports=exports,
        dependencies=dependencies,
        conditional_dependencies=conditional_dependencies,
        overrides=overrides,
        patches=patches,
        vendor=vendor,
        topology=topology,
        compiler=compiler,
        policy=policy,
        runtime=runtime,
        formatter=formatter,
        linter=linter,
        targets=targets,
        products=products,
        profiles=profiles,
        conditions=conditions,
        tasks=tasks,
        default_target=default_target,
        default_product=default_product,
    )


def to_json_destack(value: Destack) -> Json:
    """Return one JSON value for one Destack."""
    return {
        **({} if value.name is None else {"name": value.name}),
        **({} if value.version is None else {"version": value.version}),
        **(
            {}
            if value.stage is None
            else {
                "stage": destack._generated.repository.config.stage.to_json_stage(
                    value.stage
                )
            }
        ),
        **({} if value.private is None else {"private": value.private}),
        **({} if value.description is None else {"description": value.description}),
        **({} if value.license is None else {"license": value.license}),
        **({} if value.repository is None else {"repository": value.repository}),
        **({} if value.homepage is None else {"homepage": value.homepage}),
        "keywords": [item_0 for item_0 in value.keywords],
        **(
            {}
            if value.workspace is None
            else {"workspace": to_json_workspace_layout(value.workspace)}
        ),
        **({} if value.extends is None else {"extends": value.extends}),
        "files": [item_0 for item_0 in value.files],
        "include": [item_0 for item_0 in value.include],
        "exclude": [item_0 for item_0 in value.exclude],
        "exports": {
            key_0: destack._generated.repository.config.export.to_json_export(item_0)
            for key_0, item_0 in value.exports.items()
        },
        "dependencies": {
            key_0: destack._generated.repository.config.dependency.to_json_dependency(
                item_0
            )
            for key_0, item_0 in value.dependencies.items()
        },
        "conditionalDependencies": [
            destack._generated.repository.config.dependency.to_json_conditional_dependencies(
                item_0
            )
            for item_0 in value.conditional_dependencies
        ],
        "overrides": {
            key_0: destack._generated.repository.config.dependency.to_json_dependency(
                item_0
            )
            for key_0, item_0 in value.overrides.items()
        },
        "patches": {
            key_0: destack._generated.repository.config.dependency.to_json_package_patch(
                item_0
            )
            for key_0, item_0 in value.patches.items()
        },
        "vendor": destack._generated.repository.config.vendor.to_json_vendor(
            value.vendor
        ),
        "topology": destack._generated.repository.config.topology.to_json_topology(
            value.topology
        ),
        "compiler": destack._generated.repository.config.compiler.to_json_compiler_options(
            value.compiler
        ),
        "policy": destack._generated.repository.config.policy.to_json_policy(
            value.policy
        ),
        "runtime": destack._generated.repository.config.runtime.runtime.to_json_runtime_options(
            value.runtime
        ),
        "formatter": destack._generated.repository.config.formatter.to_json_formatter_options(
            value.formatter
        ),
        "linter": destack._generated.repository.config.linter.core.to_json_linter_options(
            value.linter
        ),
        "targets": {
            key_0: destack._generated.repository.config.target.target.to_json_target(
                item_0
            )
            for key_0, item_0 in value.targets.items()
        },
        "products": {
            key_0: destack._generated.repository.config.product.product.to_json_product(
                item_0
            )
            for key_0, item_0 in value.products.items()
        },
        "profiles": {
            key_0: destack._generated.repository.config.profile.to_json_profile_options(
                item_0
            )
            for key_0, item_0 in value.profiles.items()
        },
        "conditions": destack._generated.repository.config.condition.to_json_condition_catalog(
            value.conditions
        ),
        "tasks": {
            key_0: destack._generated.repository.config.task.to_json_task(item_0)
            for key_0, item_0 in value.tasks.items()
        },
        **(
            {}
            if value.default_target is None
            else {"defaultTarget": value.default_target}
        ),
        **(
            {}
            if value.default_product is None
            else {"defaultProduct": value.default_product}
        ),
    }


def from_json_destack(value: Json) -> Destack:
    """Return one Destack from one JSON value."""
    object_ = json_object(value)

    return Destack(
        name=json_optional(object_, "name", lambda value: json_string(value)),
        version=json_optional(object_, "version", lambda value: json_string(value)),
        stage=json_optional(
            object_,
            "stage",
            lambda value: destack._generated.repository.config.stage.from_json_stage(
                value
            ),
        ),
        private=json_optional(object_, "private", lambda value: json_bool(value)),
        description=json_optional(
            object_, "description", lambda value: json_string(value)
        ),
        license=json_optional(object_, "license", lambda value: json_string(value)),
        repository=json_optional(object_, "repository", lambda value: value),
        homepage=json_optional(object_, "homepage", lambda value: json_string(value)),
        keywords=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "keywords"))
        ],
        workspace=json_optional(
            object_, "workspace", lambda value: from_json_workspace_layout(value)
        ),
        extends=json_optional(object_, "extends", lambda value: json_string(value)),
        files=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "files"))
        ],
        include=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "include"))
        ],
        exclude=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "exclude"))
        ],
        exports={
            key_0: destack._generated.repository.config.export.from_json_export(item_0)
            for key_0, item_0 in json_object(json_field(object_, "exports")).items()
        },
        dependencies={
            key_0: destack._generated.repository.config.dependency.from_json_dependency(
                item_0
            )
            for key_0, item_0 in json_object(
                json_field(object_, "dependencies")
            ).items()
        },
        conditional_dependencies=[
            destack._generated.repository.config.dependency.from_json_conditional_dependencies(
                item_0
            )
            for item_0 in json_array(json_field(object_, "conditionalDependencies"))
        ],
        overrides={
            key_0: destack._generated.repository.config.dependency.from_json_dependency(
                item_0
            )
            for key_0, item_0 in json_object(json_field(object_, "overrides")).items()
        },
        patches={
            key_0: destack._generated.repository.config.dependency.from_json_package_patch(
                item_0
            )
            for key_0, item_0 in json_object(json_field(object_, "patches")).items()
        },
        vendor=destack._generated.repository.config.vendor.from_json_vendor(
            json_field(object_, "vendor")
        ),
        topology=destack._generated.repository.config.topology.from_json_topology(
            json_field(object_, "topology")
        ),
        compiler=destack._generated.repository.config.compiler.from_json_compiler_options(
            json_field(object_, "compiler")
        ),
        policy=destack._generated.repository.config.policy.from_json_policy(
            json_field(object_, "policy")
        ),
        runtime=destack._generated.repository.config.runtime.runtime.from_json_runtime_options(
            json_field(object_, "runtime")
        ),
        formatter=destack._generated.repository.config.formatter.from_json_formatter_options(
            json_field(object_, "formatter")
        ),
        linter=destack._generated.repository.config.linter.core.from_json_linter_options(
            json_field(object_, "linter")
        ),
        targets={
            key_0: destack._generated.repository.config.target.target.from_json_target(
                item_0
            )
            for key_0, item_0 in json_object(json_field(object_, "targets")).items()
        },
        products={
            key_0: destack._generated.repository.config.product.product.from_json_product(
                item_0
            )
            for key_0, item_0 in json_object(json_field(object_, "products")).items()
        },
        profiles={
            key_0: destack._generated.repository.config.profile.from_json_profile_options(
                item_0
            )
            for key_0, item_0 in json_object(json_field(object_, "profiles")).items()
        },
        conditions=destack._generated.repository.config.condition.from_json_condition_catalog(
            json_field(object_, "conditions")
        ),
        tasks={
            key_0: destack._generated.repository.config.task.from_json_task(item_0)
            for key_0, item_0 in json_object(json_field(object_, "tasks")).items()
        },
        default_target=json_optional(
            object_, "defaultTarget", lambda value: json_string(value)
        ),
        default_product=json_optional(
            object_, "defaultProduct", lambda value: json_string(value)
        ),
    )


__all__ = [
    "WorkspaceLayout",
    "encode_workspace_layout",
    "decode_workspace_layout",
    "to_json_workspace_layout",
    "from_json_workspace_layout",
    "Destack",
    "encode_destack",
    "decode_destack",
    "to_json_destack",
    "from_json_destack",
]
