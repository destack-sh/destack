# generated bridge target, do not edit

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
import destack._generated.protocol.workspace.command.targets


@dataclass(frozen=True, slots=True)
class InfoInput:
    """Request to return workspace information."""

    # revision selected for this info request
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
    # whether to include all workspace packages
    all: bool

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_info_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InfoInput:
        """Decode one InfoInput."""
        return decode_info_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_info_input(self)

    @classmethod
    def from_json(cls, value: Json) -> InfoInput:
        """Return one InfoInput from one JSON value."""
        return from_json_info_input(value)


def encode_info_input(writer: BinaryWriter, value: InfoInput) -> None:
    """Encode one InfoInput."""
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
    writer.write_bool(value.all)


def decode_info_input(reader: BinaryReader) -> InfoInput:
    """Decode one InfoInput."""
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
    all = reader.read_bool()

    return InfoInput(
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
        all=all,
    )


def to_json_info_input(value: InfoInput) -> Json:
    """Return one JSON value for one InfoInput."""
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
        "all": value.all,
    }


def from_json_info_input(value: Json) -> InfoInput:
    """Return one InfoInput from one JSON value."""
    object_ = json_object(value)

    return InfoInput(
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
        all=json_bool(json_field(object_, "all")),
    )


@dataclass(frozen=True, slots=True)
class InfoPayload:
    """Payload for info command output."""

    # workspace metadata
    workspace: InfoWorkspace
    # resolved destack.json path
    manifest: str | None
    # targets for the active package
    targets: (
        Sequence[destack._generated.protocol.workspace.command.targets.TargetEntry]
        | None
    )
    # targets for all workspace packages
    workspace_targets: (
        Sequence[destack._generated.protocol.workspace.command.targets.TargetEntry]
        | None
    )

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_info_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InfoPayload:
        """Decode one InfoPayload."""
        return decode_info_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_info_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> InfoPayload:
        """Return one InfoPayload from one JSON value."""
        return from_json_info_payload(value)


def encode_info_payload(writer: BinaryWriter, value: InfoPayload) -> None:
    """Encode one InfoPayload."""
    encode_info_workspace(writer, value.workspace)
    if value.manifest is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.manifest)
    if value.targets is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.targets))
        for item_value_targets_1 in value.targets:
            destack._generated.protocol.workspace.command.targets.encode_target_entry(
                writer, item_value_targets_1
            )
    if value.workspace_targets is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(len(value.workspace_targets))
        for item_value_workspace_targets_1 in value.workspace_targets:
            destack._generated.protocol.workspace.command.targets.encode_target_entry(
                writer, item_value_workspace_targets_1
            )


def decode_info_payload(reader: BinaryReader) -> InfoPayload:
    """Decode one InfoPayload."""
    workspace = decode_info_workspace(reader)
    manifest = reader.read_option(lambda: reader.read_string())
    targets = reader.read_option(
        lambda: [
            destack._generated.protocol.workspace.command.targets.decode_target_entry(
                reader
            )
            for _ in range(reader.read_number())
        ]
    )
    workspace_targets = reader.read_option(
        lambda: [
            destack._generated.protocol.workspace.command.targets.decode_target_entry(
                reader
            )
            for _ in range(reader.read_number())
        ]
    )

    return InfoPayload(
        workspace=workspace,
        manifest=manifest,
        targets=targets,
        workspace_targets=workspace_targets,
    )


def to_json_info_payload(value: InfoPayload) -> Json:
    """Return one JSON value for one InfoPayload."""
    return {
        "workspace": to_json_info_workspace(value.workspace),
        **({} if value.manifest is None else {"manifest": value.manifest}),
        **(
            {}
            if value.targets is None
            else {
                "targets": [
                    destack._generated.protocol.workspace.command.targets.to_json_target_entry(
                        item_0
                    )
                    for item_0 in value.targets
                ]
            }
        ),
        **(
            {}
            if value.workspace_targets is None
            else {
                "workspaceTargets": [
                    destack._generated.protocol.workspace.command.targets.to_json_target_entry(
                        item_0
                    )
                    for item_0 in value.workspace_targets
                ]
            }
        ),
    }


def from_json_info_payload(value: Json) -> InfoPayload:
    """Return one InfoPayload from one JSON value."""
    object_ = json_object(value)

    return InfoPayload(
        workspace=from_json_info_workspace(json_field(object_, "workspace")),
        manifest=json_optional(object_, "manifest", lambda value: json_string(value)),
        targets=json_optional(
            object_,
            "targets",
            lambda value: [
                destack._generated.protocol.workspace.command.targets.from_json_target_entry(
                    item_0
                )
                for item_0 in json_array(value)
            ],
        ),
        workspace_targets=json_optional(
            object_,
            "workspaceTargets",
            lambda value: [
                destack._generated.protocol.workspace.command.targets.from_json_target_entry(
                    item_0
                )
                for item_0 in json_array(value)
            ],
        ),
    )


@dataclass(frozen=True, slots=True)
class InfoWorkspace:
    """Workspace info for info command output."""

    # workspace root path
    root: str
    # workspace kind label
    kind: str
    # workspace package paths
    packages: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_info_workspace(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> InfoWorkspace:
        """Decode one InfoWorkspace."""
        return decode_info_workspace(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_info_workspace(self)

    @classmethod
    def from_json(cls, value: Json) -> InfoWorkspace:
        """Return one InfoWorkspace from one JSON value."""
        return from_json_info_workspace(value)


def encode_info_workspace(writer: BinaryWriter, value: InfoWorkspace) -> None:
    """Encode one InfoWorkspace."""
    writer.write_string(value.root)
    writer.write_string(value.kind)
    writer.write_unsigned(len(value.packages))
    for item_value_packages_0 in value.packages:
        writer.write_string(item_value_packages_0)


def decode_info_workspace(reader: BinaryReader) -> InfoWorkspace:
    """Decode one InfoWorkspace."""
    root = reader.read_string()
    kind = reader.read_string()
    packages = [reader.read_string() for _ in range(reader.read_number())]

    return InfoWorkspace(
        root=root,
        kind=kind,
        packages=packages,
    )


def to_json_info_workspace(value: InfoWorkspace) -> Json:
    """Return one JSON value for one InfoWorkspace."""
    return {
        "root": value.root,
        "kind": value.kind,
        "packages": [item_0 for item_0 in value.packages],
    }


def from_json_info_workspace(value: Json) -> InfoWorkspace:
    """Return one InfoWorkspace from one JSON value."""
    object_ = json_object(value)

    return InfoWorkspace(
        root=json_string(json_field(object_, "root")),
        kind=json_string(json_field(object_, "kind")),
        packages=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "packages"))
        ],
    )


__all__ = [
    "InfoInput",
    "encode_info_input",
    "decode_info_input",
    "to_json_info_input",
    "from_json_info_input",
    "InfoPayload",
    "encode_info_payload",
    "decode_info_payload",
    "to_json_info_payload",
    "from_json_info_payload",
    "InfoWorkspace",
    "encode_info_workspace",
    "decode_info_workspace",
    "to_json_info_workspace",
    "from_json_info_workspace",
]
