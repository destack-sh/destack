# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
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
)

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class CleanInput:
    """Request to clean generated state."""

    # revision selected for this clean request
    revision: destack._generated.protocol.workspace.command.common.CommandRevision
    # input sources for the command
    inputs: Sequence[destack._generated.protocol.workspace.command.common.CommandInput]
    # whether destack.json should resolve inputs when none are provided
    config_inputs: bool
    # optional working directory for this command
    cwd: str | None
    # optional Destack manifest path override
    manifest: str | None
    # optional target name override
    target: str | None
    # optional target overrides
    target_overrides: (
        destack._generated.protocol.workspace.command.common.CommandTargetOverrides
        | None
    )
    # optional profile name override
    profile: str | None
    # optional environment overrides
    env: Sequence[destack._generated.protocol.workspace.command.common.CommandEnvVar]
    # optional manifest overrides
    overrides: Sequence[
        destack._generated.protocol.workspace.command.common.ManifestOverride
    ]
    # whether the command should watch for changes
    watch: bool
    # whether the command should skip writes
    dry_run: bool
    # optional directory override
    dir: str | None
    # remove build output directories
    dist: bool
    # remove cache directories
    cache: bool
    # remove all build outputs and caches
    all: bool
    # clean all packages in the workspace
    all_packages: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_clean_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CleanInput:
        """Decode one CleanInput."""
        return decode_clean_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_clean_input(self)

    @classmethod
    def from_json(cls, value: Json) -> CleanInput:
        """Return one CleanInput from one JSON value."""
        return from_json_clean_input(value)


def encode_clean_input(writer: BinaryWriter, value: CleanInput) -> None:
    """Encode one CleanInput."""
    destack._generated.protocol.workspace.command.common.encode_command_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.inputs))
    for item_value_inputs_0 in value.inputs:
        destack._generated.protocol.workspace.command.common.encode_command_input(
            writer, item_value_inputs_0
        )
    writer.write_bool(value.config_inputs)
    if value.cwd is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.cwd)
    if value.manifest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.manifest)
    if value.target is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.target)
    if value.target_overrides is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.protocol.workspace.command.common.encode_command_target_overrides(
            writer, value.target_overrides
        )
    if value.profile is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.profile)
    writer.write_unsigned(len(value.env))
    for item_value_env_0 in value.env:
        destack._generated.protocol.workspace.command.common.encode_command_env_var(
            writer, item_value_env_0
        )
    writer.write_unsigned(len(value.overrides))
    for item_value_overrides_0 in value.overrides:
        destack._generated.protocol.workspace.command.common.encode_manifest_override(
            writer, item_value_overrides_0
        )
    writer.write_bool(value.watch)
    writer.write_bool(value.dry_run)
    if value.dir is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.dir)
    writer.write_bool(value.dist)
    writer.write_bool(value.cache)
    writer.write_bool(value.all)
    writer.write_bool(value.all_packages)


def decode_clean_input(reader: BinaryReader) -> CleanInput:
    """Decode one CleanInput."""
    revision = (
        destack._generated.protocol.workspace.command.common.decode_command_revision(
            reader
        )
    )
    inputs = [
        destack._generated.protocol.workspace.command.common.decode_command_input(
            reader
        )
        for _ in range(reader.read_number())
    ]
    config_inputs = reader.read_bool()
    cwd = reader.read_option(lambda: reader.read_string())
    manifest = reader.read_option(lambda: reader.read_string())
    target = reader.read_option(lambda: reader.read_string())
    target_overrides = reader.read_option(
        lambda: (
            destack._generated.protocol.workspace.command.common.decode_command_target_overrides(
                reader
            )
        )
    )
    profile = reader.read_option(lambda: reader.read_string())
    env = [
        destack._generated.protocol.workspace.command.common.decode_command_env_var(
            reader
        )
        for _ in range(reader.read_number())
    ]
    overrides = [
        destack._generated.protocol.workspace.command.common.decode_manifest_override(
            reader
        )
        for _ in range(reader.read_number())
    ]
    watch = reader.read_bool()
    dry_run = reader.read_bool()
    dir = reader.read_option(lambda: reader.read_string())
    dist = reader.read_bool()
    cache = reader.read_bool()
    all = reader.read_bool()
    all_packages = reader.read_bool()

    return CleanInput(
        revision=revision,
        inputs=inputs,
        config_inputs=config_inputs,
        cwd=cwd,
        manifest=manifest,
        target=target,
        target_overrides=target_overrides,
        profile=profile,
        env=env,
        overrides=overrides,
        watch=watch,
        dry_run=dry_run,
        dir=dir,
        dist=dist,
        cache=cache,
        all=all,
        all_packages=all_packages,
    )


def to_json_clean_input(value: CleanInput) -> Json:
    """Return one JSON value for one CleanInput."""
    return {
        "revision": destack._generated.protocol.workspace.command.common.to_json_command_revision(
            value.revision
        ),
        "inputs": [
            destack._generated.protocol.workspace.command.common.to_json_command_input(
                item_0
            )
            for item_0 in value.inputs
        ],
        "configInputs": value.config_inputs,
        **({} if value.cwd is None else {"cwd": value.cwd}),
        **({} if value.manifest is None else {"manifest": value.manifest}),
        **({} if value.target is None else {"target": value.target}),
        **(
            {}
            if value.target_overrides is None
            else {
                "targetOverrides": destack._generated.protocol.workspace.command.common.to_json_command_target_overrides(
                    value.target_overrides
                )
            }
        ),
        **({} if value.profile is None else {"profile": value.profile}),
        "env": [
            destack._generated.protocol.workspace.command.common.to_json_command_env_var(
                item_0
            )
            for item_0 in value.env
        ],
        "overrides": [
            destack._generated.protocol.workspace.command.common.to_json_manifest_override(
                item_0
            )
            for item_0 in value.overrides
        ],
        "watch": value.watch,
        "dryRun": value.dry_run,
        **({} if value.dir is None else {"dir": value.dir}),
        "dist": value.dist,
        "cache": value.cache,
        "all": value.all,
        "allPackages": value.all_packages,
    }


def from_json_clean_input(value: Json) -> CleanInput:
    """Return one CleanInput from one JSON value."""
    object_ = json_object(value)

    return CleanInput(
        revision=destack._generated.protocol.workspace.command.common.from_json_command_revision(
            json_field(object_, "revision")
        ),
        inputs=[
            destack._generated.protocol.workspace.command.common.from_json_command_input(
                item_0
            )
            for item_0 in json_array(json_field(object_, "inputs"))
        ],
        config_inputs=json_bool(json_field(object_, "configInputs")),
        cwd=json_optional(object_, "cwd", lambda value: json_string(value)),
        manifest=json_optional(object_, "manifest", lambda value: json_string(value)),
        target=json_optional(object_, "target", lambda value: json_string(value)),
        target_overrides=json_optional(
            object_,
            "targetOverrides",
            lambda value: (
                destack._generated.protocol.workspace.command.common.from_json_command_target_overrides(
                    value
                )
            ),
        ),
        profile=json_optional(object_, "profile", lambda value: json_string(value)),
        env=[
            destack._generated.protocol.workspace.command.common.from_json_command_env_var(
                item_0
            )
            for item_0 in json_array(json_field(object_, "env"))
        ],
        overrides=[
            destack._generated.protocol.workspace.command.common.from_json_manifest_override(
                item_0
            )
            for item_0 in json_array(json_field(object_, "overrides"))
        ],
        watch=json_bool(json_field(object_, "watch")),
        dry_run=json_bool(json_field(object_, "dryRun")),
        dir=json_optional(object_, "dir", lambda value: json_string(value)),
        dist=json_bool(json_field(object_, "dist")),
        cache=json_bool(json_field(object_, "cache")),
        all=json_bool(json_field(object_, "all")),
        all_packages=json_bool(json_field(object_, "allPackages")),
    )


@dataclass(frozen=True, slots=True)
class CleanPayload:
    """Payload for clean command output."""

    # removed paths or candidate paths for dry runs
    removed: Sequence[str]
    # errors encountered during removal
    errors: Sequence[str]
    # whether this was a dry run
    dry_run: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_clean_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CleanPayload:
        """Decode one CleanPayload."""
        return decode_clean_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_clean_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> CleanPayload:
        """Return one CleanPayload from one JSON value."""
        return from_json_clean_payload(value)


def encode_clean_payload(writer: BinaryWriter, value: CleanPayload) -> None:
    """Encode one CleanPayload."""
    writer.write_unsigned(len(value.removed))
    for item_value_removed_0 in value.removed:
        writer.write_string(item_value_removed_0)
    writer.write_unsigned(len(value.errors))
    for item_value_errors_0 in value.errors:
        writer.write_string(item_value_errors_0)
    writer.write_bool(value.dry_run)


def decode_clean_payload(reader: BinaryReader) -> CleanPayload:
    """Decode one CleanPayload."""
    removed = [reader.read_string() for _ in range(reader.read_number())]
    errors = [reader.read_string() for _ in range(reader.read_number())]
    dry_run = reader.read_bool()

    return CleanPayload(
        removed=removed,
        errors=errors,
        dry_run=dry_run,
    )


def to_json_clean_payload(value: CleanPayload) -> Json:
    """Return one JSON value for one CleanPayload."""
    return {
        "removed": [item_0 for item_0 in value.removed],
        "errors": [item_0 for item_0 in value.errors],
        "dryRun": value.dry_run,
    }


def from_json_clean_payload(value: Json) -> CleanPayload:
    """Return one CleanPayload from one JSON value."""
    object_ = json_object(value)

    return CleanPayload(
        removed=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "removed"))
        ],
        errors=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "errors"))
        ],
        dry_run=json_bool(json_field(object_, "dryRun")),
    )


__all__ = [
    "CleanInput",
    "encode_clean_input",
    "decode_clean_input",
    "to_json_clean_input",
    "from_json_clean_input",
    "CleanPayload",
    "encode_clean_payload",
    "decode_clean_payload",
    "to_json_clean_payload",
    "from_json_clean_payload",
]
