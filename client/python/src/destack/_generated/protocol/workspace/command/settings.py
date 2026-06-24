# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
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
)

import destack._generated.protocol.workspace.command.common


@dataclass(frozen=True, slots=True)
class SettingsInput:
    """Request to return resolved settings."""

    # revision selected for this settings request
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
        encode_settings_input(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SettingsInput:
        """Decode one SettingsInput."""
        return decode_settings_input(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_input(self)

    @classmethod
    def from_json(cls, value: Json) -> SettingsInput:
        """Return one SettingsInput from one JSON value."""
        return from_json_settings_input(value)


def encode_settings_input(writer: BinaryWriter, value: SettingsInput) -> None:
    """Encode one SettingsInput."""
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


def decode_settings_input(reader: BinaryReader) -> SettingsInput:
    """Decode one SettingsInput."""
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

    return SettingsInput(
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


def to_json_settings_input(value: SettingsInput) -> Json:
    """Return one JSON value for one SettingsInput."""
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


def from_json_settings_input(value: Json) -> SettingsInput:
    """Return one SettingsInput from one JSON value."""
    object_ = json_object(value)

    return SettingsInput(
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
class SettingsPayload:
    """Payload for settings command output."""

    # machine-local Destack home
    home: str
    # package directory
    packages: str
    # maximum package directory size in bytes before pruning is requested
    package_maximum_bytes: int | None
    # workspace-local cache and session directory
    workspace_cache: str
    # maximum cache size in bytes before pruning is requested
    cache_maximum_bytes: int | None
    # workspace-owned vendor directory
    vendor: str
    # default registry name
    registry: str | None
    # known registries
    registries: Sequence[SettingsRegistry]
    # network settings for package and update commands
    network: SettingsNetwork

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_payload(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SettingsPayload:
        """Decode one SettingsPayload."""
        return decode_settings_payload(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_payload(self)

    @classmethod
    def from_json(cls, value: Json) -> SettingsPayload:
        """Return one SettingsPayload from one JSON value."""
        return from_json_settings_payload(value)


def encode_settings_payload(writer: BinaryWriter, value: SettingsPayload) -> None:
    """Encode one SettingsPayload."""
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
    for item_value_registries_0 in value.registries:
        encode_settings_registry(writer, item_value_registries_0)
    encode_settings_network(writer, value.network)


def decode_settings_payload(reader: BinaryReader) -> SettingsPayload:
    """Decode one SettingsPayload."""
    home = reader.read_string()
    packages = reader.read_string()
    package_maximum_bytes = reader.read_option(lambda: reader.read_number())
    workspace_cache = reader.read_string()
    cache_maximum_bytes = reader.read_option(lambda: reader.read_number())
    vendor = reader.read_string()
    registry = reader.read_option(lambda: reader.read_string())
    registries = [decode_settings_registry(reader) for _ in range(reader.read_number())]
    network = decode_settings_network(reader)

    return SettingsPayload(
        home=home,
        packages=packages,
        package_maximum_bytes=package_maximum_bytes,
        workspace_cache=workspace_cache,
        cache_maximum_bytes=cache_maximum_bytes,
        vendor=vendor,
        registry=registry,
        registries=registries,
        network=network,
    )


def to_json_settings_payload(value: SettingsPayload) -> Json:
    """Return one JSON value for one SettingsPayload."""
    return {
        "home": value.home,
        "packages": value.packages,
        **(
            {}
            if value.package_maximum_bytes is None
            else {"packageMaximumBytes": value.package_maximum_bytes}
        ),
        "workspaceCache": value.workspace_cache,
        **(
            {}
            if value.cache_maximum_bytes is None
            else {"cacheMaximumBytes": value.cache_maximum_bytes}
        ),
        "vendor": value.vendor,
        **({} if value.registry is None else {"registry": value.registry}),
        "registries": [
            to_json_settings_registry(item_0) for item_0 in value.registries
        ],
        "network": to_json_settings_network(value.network),
    }


def from_json_settings_payload(value: Json) -> SettingsPayload:
    """Return one SettingsPayload from one JSON value."""
    object_ = json_object(value)

    return SettingsPayload(
        home=json_string(json_field(object_, "home")),
        packages=json_string(json_field(object_, "packages")),
        package_maximum_bytes=json_optional(
            object_, "packageMaximumBytes", lambda value: json_int(value)
        ),
        workspace_cache=json_string(json_field(object_, "workspaceCache")),
        cache_maximum_bytes=json_optional(
            object_, "cacheMaximumBytes", lambda value: json_int(value)
        ),
        vendor=json_string(json_field(object_, "vendor")),
        registry=json_optional(object_, "registry", lambda value: json_string(value)),
        registries=[
            from_json_settings_registry(item_0)
            for item_0 in json_array(json_field(object_, "registries"))
        ],
        network=from_json_settings_network(json_field(object_, "network")),
    )


@dataclass(frozen=True, slots=True)
class SettingsRegistry:
    """Registry settings for command output."""

    # registry name
    name: str
    # registry URL
    url: str
    # redacted authentication shape
    authentication: SettingsRegistryAuthentication

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_registry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SettingsRegistry:
        """Decode one SettingsRegistry."""
        return decode_settings_registry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_registry(self)

    @classmethod
    def from_json(cls, value: Json) -> SettingsRegistry:
        """Return one SettingsRegistry from one JSON value."""
        return from_json_settings_registry(value)


def encode_settings_registry(writer: BinaryWriter, value: SettingsRegistry) -> None:
    """Encode one SettingsRegistry."""
    writer.write_string(value.name)
    writer.write_string(value.url)
    encode_settings_registry_authentication(writer, value.authentication)


def decode_settings_registry(reader: BinaryReader) -> SettingsRegistry:
    """Decode one SettingsRegistry."""
    name = reader.read_string()
    url = reader.read_string()
    authentication = decode_settings_registry_authentication(reader)

    return SettingsRegistry(
        name=name,
        url=url,
        authentication=authentication,
    )


def to_json_settings_registry(value: SettingsRegistry) -> Json:
    """Return one JSON value for one SettingsRegistry."""
    return {
        "name": value.name,
        "url": value.url,
        "authentication": to_json_settings_registry_authentication(
            value.authentication
        ),
    }


def from_json_settings_registry(value: Json) -> SettingsRegistry:
    """Return one SettingsRegistry from one JSON value."""
    object_ = json_object(value)

    return SettingsRegistry(
        name=json_string(json_field(object_, "name")),
        url=json_string(json_field(object_, "url")),
        authentication=from_json_settings_registry_authentication(
            json_field(object_, "authentication")
        ),
    )


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationNone:
    """No registry authentication."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationToken:
    """Inline token authentication."""

    kind: typing.Literal["token"] = "token"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationTokenFromEnvironment:
    """Token read from one environment variable."""

    # environment variable name
    variable: str
    kind: typing.Literal["tokenFromEnvironment"] = "tokenFromEnvironment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class SettingsRegistryAuthenticationCommand:
    """Credentials printed by one process command."""

    # program to run
    program: str
    kind: typing.Literal["command"] = "command"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_registry_authentication(self)


"""Redacted registry authentication shape."""
SettingsRegistryAuthentication: typing.TypeAlias = (
    SettingsRegistryAuthenticationNone
    | SettingsRegistryAuthenticationToken
    | SettingsRegistryAuthenticationTokenFromEnvironment
    | SettingsRegistryAuthenticationCommand
)


def encode_settings_registry_authentication(
    writer: BinaryWriter, value: SettingsRegistryAuthentication
) -> None:
    """Encode one SettingsRegistryAuthentication."""
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
    reader: BinaryReader,
) -> SettingsRegistryAuthentication:
    """Decode one SettingsRegistryAuthentication."""
    variant = reader.read_number()

    if variant == 0:
        return SettingsRegistryAuthenticationNone()
    elif variant == 1:
        return SettingsRegistryAuthenticationToken()
    elif variant == 2:
        variable = reader.read_string()

        return SettingsRegistryAuthenticationTokenFromEnvironment(
            variable=variable,
        )
    elif variant == 3:
        program = reader.read_string()

        return SettingsRegistryAuthenticationCommand(
            program=program,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_settings_registry_authentication(
    value: SettingsRegistryAuthentication,
) -> Json:
    """Return one JSON value for one SettingsRegistryAuthentication."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "token":
        return {
            "kind": "token",
        }
    elif value.kind == "tokenFromEnvironment":
        return {
            "kind": "tokenFromEnvironment",
            "variable": value.variable,
        }
    elif value.kind == "command":
        return {
            "kind": "command",
            "program": value.program,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_settings_registry_authentication(
    value: Json,
) -> SettingsRegistryAuthentication:
    """Return one SettingsRegistryAuthentication from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return SettingsRegistryAuthenticationNone()
    elif kind == "token":
        return SettingsRegistryAuthenticationToken()
    elif kind == "tokenFromEnvironment":
        return SettingsRegistryAuthenticationTokenFromEnvironment(
            variable=json_string(json_field(object_, "variable")),
        )
    elif kind == "command":
        return SettingsRegistryAuthenticationCommand(
            program=json_string(json_field(object_, "program")),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class SettingsNetwork:
    """Network settings for command output."""

    # whether network access should be disabled by default
    offline: bool
    # whether a proxy is configured
    has_proxy: bool
    # request timeout in milliseconds
    timeout_milliseconds: int | None
    # number of retries for transient network failures
    retry_count: int | None
    # maximum concurrent network requests
    concurrency: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings_network(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SettingsNetwork:
        """Decode one SettingsNetwork."""
        return decode_settings_network(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings_network(self)

    @classmethod
    def from_json(cls, value: Json) -> SettingsNetwork:
        """Return one SettingsNetwork from one JSON value."""
        return from_json_settings_network(value)


def encode_settings_network(writer: BinaryWriter, value: SettingsNetwork) -> None:
    """Encode one SettingsNetwork."""
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


def decode_settings_network(reader: BinaryReader) -> SettingsNetwork:
    """Decode one SettingsNetwork."""
    offline = reader.read_bool()
    has_proxy = reader.read_bool()
    timeout_milliseconds = reader.read_option(lambda: reader.read_number())
    retry_count = reader.read_option(lambda: reader.read_number())
    concurrency = reader.read_option(lambda: reader.read_number())

    return SettingsNetwork(
        offline=offline,
        has_proxy=has_proxy,
        timeout_milliseconds=timeout_milliseconds,
        retry_count=retry_count,
        concurrency=concurrency,
    )


def to_json_settings_network(value: SettingsNetwork) -> Json:
    """Return one JSON value for one SettingsNetwork."""
    return {
        "offline": value.offline,
        "hasProxy": value.has_proxy,
        **(
            {}
            if value.timeout_milliseconds is None
            else {"timeoutMilliseconds": value.timeout_milliseconds}
        ),
        **({} if value.retry_count is None else {"retryCount": value.retry_count}),
        **({} if value.concurrency is None else {"concurrency": value.concurrency}),
    }


def from_json_settings_network(value: Json) -> SettingsNetwork:
    """Return one SettingsNetwork from one JSON value."""
    object_ = json_object(value)

    return SettingsNetwork(
        offline=json_bool(json_field(object_, "offline")),
        has_proxy=json_bool(json_field(object_, "hasProxy")),
        timeout_milliseconds=json_optional(
            object_, "timeoutMilliseconds", lambda value: json_int(value)
        ),
        retry_count=json_optional(object_, "retryCount", lambda value: json_int(value)),
        concurrency=json_optional(
            object_, "concurrency", lambda value: json_int(value)
        ),
    )


__all__ = [
    "SettingsInput",
    "encode_settings_input",
    "decode_settings_input",
    "to_json_settings_input",
    "from_json_settings_input",
    "SettingsPayload",
    "encode_settings_payload",
    "decode_settings_payload",
    "to_json_settings_payload",
    "from_json_settings_payload",
    "SettingsRegistry",
    "encode_settings_registry",
    "decode_settings_registry",
    "to_json_settings_registry",
    "from_json_settings_registry",
    "SettingsRegistryAuthentication",
    "encode_settings_registry_authentication",
    "decode_settings_registry_authentication",
    "to_json_settings_registry_authentication",
    "from_json_settings_registry_authentication",
    "SettingsRegistryAuthenticationNone",
    "SettingsRegistryAuthenticationToken",
    "SettingsRegistryAuthenticationTokenFromEnvironment",
    "SettingsRegistryAuthenticationCommand",
    "SettingsNetwork",
    "encode_settings_network",
    "decode_settings_network",
    "to_json_settings_network",
    "from_json_settings_network",
]
