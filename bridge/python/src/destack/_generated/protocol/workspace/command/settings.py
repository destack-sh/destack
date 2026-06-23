# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
from typing import TYPE_CHECKING, Any, Literal, TypeAlias

from destack.protocol.serde import Reader, SerdeError, Writer, nested_bytes

import destack._generated.protocol.workspace.command.common

if TYPE_CHECKING:
    from destack._generated.protocol.workspace.command.common import (
        CommandEnvVar,
        CommandInput,
        CommandRevision,
        CommandTargetOverrides,
        ManifestOverride,
    )


@dataclass(frozen=True, slots=True)
class SettingsInput:
    """Request to return resolved settings."""

    """Revision selected for this settings request."""
    revision: CommandRevision
    """Input sources for the command."""
    inputs: Sequence[CommandInput]
    """Whether destack.json should resolve inputs when none are provided."""
    config_inputs: bool
    """Optional working directory for this command."""
    cwd: str | None
    """Optional Destack manifest path override."""
    manifest: str | None
    """Optional target name override."""
    target: str | None
    """Optional target overrides."""
    target_overrides: CommandTargetOverrides | None
    """Optional profile name override."""
    profile: str | None
    """Optional environment overrides."""
    env: Sequence[CommandEnvVar]
    """Optional manifest overrides."""
    overrides: Sequence[ManifestOverride]
    """Whether the command should watch for changes."""
    watch: bool
    """Whether the command should skip writes."""
    dry_run: bool


def encode_settings_input(writer: Writer, value: SettingsInput) -> None:
    destack._generated.protocol.workspace.command.common.encode_command_revision(
        writer, value.revision
    )
    writer.write_unsigned(len(value.inputs))
    for item_0 in value.inputs:
        destack._generated.protocol.workspace.command.common.encode_command_input(
            writer, item_0
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
    for item_0 in value.env:
        destack._generated.protocol.workspace.command.common.encode_command_env_var(
            writer, item_0
        )
    writer.write_unsigned(len(value.overrides))
    for item_0 in value.overrides:
        destack._generated.protocol.workspace.command.common.encode_manifest_override(
            writer, item_0
        )
    writer.write_bool(value.watch)
    writer.write_bool(value.dry_run)


def decode_settings_input(reader: Reader) -> SettingsInput:
    field_0 = (
        destack._generated.protocol.workspace.command.common.decode_command_revision(
            reader
        )
    )
    field_1 = [
        destack._generated.protocol.workspace.command.common.decode_command_input(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_2 = reader.read_bool()
    field_3 = reader.read_option(lambda: reader.read_string())
    field_4 = reader.read_option(lambda: reader.read_string())
    field_5 = reader.read_option(lambda: reader.read_string())
    field_6 = reader.read_option(
        lambda: (
            destack._generated.protocol.workspace.command.common.decode_command_target_overrides(
                reader
            )
        )
    )
    field_7 = reader.read_option(lambda: reader.read_string())
    field_8 = [
        destack._generated.protocol.workspace.command.common.decode_command_env_var(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_9 = [
        destack._generated.protocol.workspace.command.common.decode_manifest_override(
            reader
        )
        for _ in range(reader.read_number())
    ]
    field_10 = reader.read_bool()
    field_11 = reader.read_bool()

    return SettingsInput(
        revision=field_0,
        inputs=field_1,
        config_inputs=field_2,
        cwd=field_3,
        manifest=field_4,
        target=field_5,
        target_overrides=field_6,
        profile=field_7,
        env=field_8,
        overrides=field_9,
        watch=field_10,
        dry_run=field_11,
    )


@dataclass(frozen=True, slots=True)
class SettingsPayload:
    """Payload for settings command output."""

    """Machine-local Destack home."""
    home: str
    """Package directory."""
    packages: str
    """Maximum package directory size in bytes before pruning is requested."""
    package_maximum_bytes: int | None
    """Workspace-local cache and session directory."""
    workspace_cache: str
    """Maximum cache size in bytes before pruning is requested."""
    cache_maximum_bytes: int | None
    """Workspace-owned vendor directory."""
    vendor: str
    """Default registry name."""
    registry: str | None
    """Known registries."""
    registries: Sequence[SettingsRegistry]
    """Network settings for package and update commands."""
    network: SettingsNetwork


def encode_settings_payload(writer: Writer, value: SettingsPayload) -> None:
    writer.write_string(value.home)
    writer.write_string(value.packages)
    if value.package_maximum_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.package_maximum_bytes)
    writer.write_string(value.workspace_cache)
    if value.cache_maximum_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.cache_maximum_bytes)
    writer.write_string(value.vendor)
    if value.registry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.registry)
    writer.write_unsigned(len(value.registries))
    for item_0 in value.registries:
        encode_settings_registry(writer, item_0)
    encode_settings_network(writer, value.network)


def decode_settings_payload(reader: Reader) -> SettingsPayload:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = reader.read_option(lambda: reader.read_number())
    field_3 = reader.read_string()
    field_4 = reader.read_option(lambda: reader.read_number())
    field_5 = reader.read_string()
    field_6 = reader.read_option(lambda: reader.read_string())
    field_7 = [decode_settings_registry(reader) for _ in range(reader.read_number())]
    field_8 = decode_settings_network(reader)

    return SettingsPayload(
        home=field_0,
        packages=field_1,
        package_maximum_bytes=field_2,
        workspace_cache=field_3,
        cache_maximum_bytes=field_4,
        vendor=field_5,
        registry=field_6,
        registries=field_7,
        network=field_8,
    )


@dataclass(frozen=True, slots=True)
class SettingsRegistry:
    """Registry settings for command output."""

    """Registry name."""
    name: str
    """Registry URL."""
    url: str
    """Redacted authentication shape."""
    authentication: SettingsRegistryAuthentication


def encode_settings_registry(writer: Writer, value: SettingsRegistry) -> None:
    writer.write_string(value.name)
    writer.write_string(value.url)
    encode_settings_registry_authentication(writer, value.authentication)


def decode_settings_registry(reader: Reader) -> SettingsRegistry:
    field_0 = reader.read_string()
    field_1 = reader.read_string()
    field_2 = decode_settings_registry_authentication(reader)

    return SettingsRegistry(
        name=field_0,
        url=field_1,
        authentication=field_2,
    )


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationNone:
    """No registry authentication."""

    kind: Literal["none"] = "none"


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationToken:
    """Inline token authentication."""

    kind: Literal["token"] = "token"


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationTokenFromEnvironment:
    """Token read from one environment variable."""

    """Environment variable name."""
    variable: str
    kind: Literal["tokenFromEnvironment"] = "tokenFromEnvironment"


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationCommand:
    """Credentials printed by one process command."""

    """Program to run."""
    program: str
    kind: Literal["command"] = "command"


"""Redacted registry authentication shape."""
SettingsRegistryAuthentication: TypeAlias = (
    SettingsRegistryAuthenticationNone
    | SettingsRegistryAuthenticationToken
    | SettingsRegistryAuthenticationTokenFromEnvironment
    | SettingsRegistryAuthenticationCommand
)


def encode_settings_registry_authentication(
    writer: Writer, value: SettingsRegistryAuthentication
) -> None:
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "token":
        writer.write_unsigned(1)
    elif value.kind == "tokenFromEnvironment":
        writer.write_unsigned(2)
        writer.write_string(value.variable)
    elif value.kind == "command":
        writer.write_unsigned(3)
        writer.write_string(value.program)
    else:
        raise SerdeError("unknown enum variant")


def decode_settings_registry_authentication(
    reader: Reader,
) -> SettingsRegistryAuthentication:
    variant = reader.read_number()

    if variant == 0:
        return SettingsRegistryAuthenticationNone()
    elif variant == 1:
        return SettingsRegistryAuthenticationToken()
    elif variant == 2:
        field_0 = reader.read_string()

        return SettingsRegistryAuthenticationTokenFromEnvironment(
            variable=field_0,
        )
    elif variant == 3:
        field_0 = reader.read_string()

        return SettingsRegistryAuthenticationCommand(
            program=field_0,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


@dataclass(frozen=True, slots=True)
class SettingsNetwork:
    """Network settings for command output."""

    """Whether network access should be disabled by default."""
    offline: bool
    """Whether a proxy is configured."""
    has_proxy: bool
    """Request timeout in milliseconds."""
    timeout_milliseconds: int | None
    """Number of retries for transient network failures."""
    retry_count: int | None
    """Maximum concurrent network requests."""
    concurrency: int | None


def encode_settings_network(writer: Writer, value: SettingsNetwork) -> None:
    writer.write_bool(value.offline)
    writer.write_bool(value.has_proxy)
    if value.timeout_milliseconds is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.timeout_milliseconds)
    if value.retry_count is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.retry_count)
    if value.concurrency is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.concurrency)


def decode_settings_network(reader: Reader) -> SettingsNetwork:
    field_0 = reader.read_bool()
    field_1 = reader.read_bool()
    field_2 = reader.read_option(lambda: reader.read_number())
    field_3 = reader.read_option(lambda: reader.read_number())
    field_4 = reader.read_option(lambda: reader.read_number())

    return SettingsNetwork(
        offline=field_0,
        has_proxy=field_1,
        timeout_milliseconds=field_2,
        retry_count=field_3,
        concurrency=field_4,
    )


__all__ = [
    "SettingsInput",
    "encode_settings_input",
    "decode_settings_input",
    "SettingsPayload",
    "encode_settings_payload",
    "decode_settings_payload",
    "SettingsRegistry",
    "encode_settings_registry",
    "decode_settings_registry",
    "SettingsRegistryAuthentication",
    "encode_settings_registry_authentication",
    "decode_settings_registry_authentication",
    "SettingsRegistryAuthenticationNone",
    "SettingsRegistryAuthenticationToken",
    "SettingsRegistryAuthenticationTokenFromEnvironment",
    "SettingsRegistryAuthenticationCommand",
    "SettingsNetwork",
    "encode_settings_network",
    "decode_settings_network",
]
