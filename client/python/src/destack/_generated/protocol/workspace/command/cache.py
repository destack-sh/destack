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
class CacheInput:
    """Request to return cache locations."""

    # revision selected for this cache request
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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cache_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CacheInput:
        """Decode one CacheInput."""
        return decode_cache_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cache_input(self)

    @classmethod
    def from_json(cls, value: Json) -> CacheInput:
        """Return one CacheInput from one JSON value."""
        return from_json_cache_input(value)


def encode_cache_input(writer: BinaryWriter, value: CacheInput) -> None:
    """Encode one CacheInput."""
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


def decode_cache_input(reader: BinaryReader) -> CacheInput:
    """Decode one CacheInput."""
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

    return CacheInput(
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
    )


def to_json_cache_input(value: CacheInput) -> Json:
    """Return one JSON value for one CacheInput."""
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
    }


def from_json_cache_input(value: Json) -> CacheInput:
    """Return one CacheInput from one JSON value."""
    object_ = json_object(value)

    return CacheInput(
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
    )


@dataclass(frozen=True, slots=True)
class CachePayload:
    """Cache payload for cache command output."""

    # cache entries for the workspace
    caches: Sequence[CacheEntry]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cache_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CachePayload:
        """Decode one CachePayload."""
        return decode_cache_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cache_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> CachePayload:
        """Return one CachePayload from one JSON value."""
        return from_json_cache_payload(value)


def encode_cache_payload(writer: BinaryWriter, value: CachePayload) -> None:
    """Encode one CachePayload."""
    writer.write_unsigned(len(value.caches))
    for item_value_caches_0 in value.caches:
        encode_cache_entry(writer, item_value_caches_0)


def decode_cache_payload(reader: BinaryReader) -> CachePayload:
    """Decode one CachePayload."""
    caches = [decode_cache_entry(reader) for _ in range(reader.read_number())]

    return CachePayload(
        caches=caches,
    )


def to_json_cache_payload(value: CachePayload) -> Json:
    """Return one JSON value for one CachePayload."""
    return {
        "caches": [to_json_cache_entry(item_0) for item_0 in value.caches],
    }


def from_json_cache_payload(value: Json) -> CachePayload:
    """Return one CachePayload from one JSON value."""
    object_ = json_object(value)

    return CachePayload(
        caches=[
            from_json_cache_entry(item_0)
            for item_0 in json_array(json_field(object_, "caches"))
        ],
    )


@dataclass(frozen=True, slots=True)
class CacheEntry:
    """Cache entry payload for cache command output."""

    # cache directory path
    directory: str
    # cache kind
    kind: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cache_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CacheEntry:
        """Decode one CacheEntry."""
        return decode_cache_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cache_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> CacheEntry:
        """Return one CacheEntry from one JSON value."""
        return from_json_cache_entry(value)


def encode_cache_entry(writer: BinaryWriter, value: CacheEntry) -> None:
    """Encode one CacheEntry."""
    writer.write_string(value.directory)
    writer.write_string(value.kind)


def decode_cache_entry(reader: BinaryReader) -> CacheEntry:
    """Decode one CacheEntry."""
    directory = reader.read_string()
    kind = reader.read_string()

    return CacheEntry(
        directory=directory,
        kind=kind,
    )


def to_json_cache_entry(value: CacheEntry) -> Json:
    """Return one JSON value for one CacheEntry."""
    return {
        "directory": value.directory,
        "kind": value.kind,
    }


def from_json_cache_entry(value: Json) -> CacheEntry:
    """Return one CacheEntry from one JSON value."""
    object_ = json_object(value)

    return CacheEntry(
        directory=json_string(json_field(object_, "directory")),
        kind=json_string(json_field(object_, "kind")),
    )


__all__ = [
    "CacheInput",
    "encode_cache_input",
    "decode_cache_input",
    "to_json_cache_input",
    "from_json_cache_input",
    "CachePayload",
    "encode_cache_payload",
    "decode_cache_payload",
    "to_json_cache_payload",
    "from_json_cache_payload",
    "CacheEntry",
    "encode_cache_entry",
    "decode_cache_entry",
    "to_json_cache_entry",
    "from_json_cache_entry",
]
