# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
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


@dataclass(frozen=True, slots=True)
class PackageSettings:
    """Package directory settings."""

    # package directory
    path: str | None
    # maximum package directory size in bytes before pruning is requested
    maximum_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_package_settings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageSettings:
        """Decode one PackageSettings."""
        return decode_package_settings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_package_settings(self)

    @classmethod
    def from_json(cls, value: Json) -> PackageSettings:
        """Return one PackageSettings from one JSON value."""
        return from_json_package_settings(value)


def encode_package_settings(writer: BinaryWriter, value: PackageSettings) -> None:
    """Encode one PackageSettings."""
    if value.path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.path)
    if value.maximum_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.maximum_bytes)


def decode_package_settings(reader: BinaryReader) -> PackageSettings:
    """Decode one PackageSettings."""
    path = reader.read_option(lambda: reader.read_string())
    maximum_bytes = reader.read_option(lambda: reader.read_number())

    return PackageSettings(
        path=path,
        maximum_bytes=maximum_bytes,
    )


def to_json_package_settings(value: PackageSettings) -> Json:
    """Return one JSON value for one PackageSettings."""
    return {
        **({} if value.path is None else {"path": value.path}),
        **(
            {} if value.maximum_bytes is None else {"maximumBytes": value.maximum_bytes}
        ),
    }


def from_json_package_settings(value: Json) -> PackageSettings:
    """Return one PackageSettings from one JSON value."""
    object_ = json_object(value)

    return PackageSettings(
        path=json_optional(object_, "path", lambda value: json_string(value)),
        maximum_bytes=json_optional(
            object_, "maximumBytes", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class CacheSettings:
    """Workspace cache settings."""

    # cache directory
    path: str | None
    # maximum cache size in bytes before pruning is requested
    maximum_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_cache_settings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CacheSettings:
        """Decode one CacheSettings."""
        return decode_cache_settings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_cache_settings(self)

    @classmethod
    def from_json(cls, value: Json) -> CacheSettings:
        """Return one CacheSettings from one JSON value."""
        return from_json_cache_settings(value)


def encode_cache_settings(writer: BinaryWriter, value: CacheSettings) -> None:
    """Encode one CacheSettings."""
    if value.path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.path)
    if value.maximum_bytes is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_unsigned(value.maximum_bytes)


def decode_cache_settings(reader: BinaryReader) -> CacheSettings:
    """Decode one CacheSettings."""
    path = reader.read_option(lambda: reader.read_string())
    maximum_bytes = reader.read_option(lambda: reader.read_number())

    return CacheSettings(
        path=path,
        maximum_bytes=maximum_bytes,
    )


def to_json_cache_settings(value: CacheSettings) -> Json:
    """Return one JSON value for one CacheSettings."""
    return {
        **({} if value.path is None else {"path": value.path}),
        **(
            {} if value.maximum_bytes is None else {"maximumBytes": value.maximum_bytes}
        ),
    }


def from_json_cache_settings(value: Json) -> CacheSettings:
    """Return one CacheSettings from one JSON value."""
    object_ = json_object(value)

    return CacheSettings(
        path=json_optional(object_, "path", lambda value: json_string(value)),
        maximum_bytes=json_optional(
            object_, "maximumBytes", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class RegistrySettings:
    """Registry settings."""

    # registry URL
    url: str
    # authentication used for this registry
    authentication: RegistryAuthentication

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_registry_settings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> RegistrySettings:
        """Decode one RegistrySettings."""
        return decode_registry_settings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_registry_settings(self)

    @classmethod
    def from_json(cls, value: Json) -> RegistrySettings:
        """Return one RegistrySettings from one JSON value."""
        return from_json_registry_settings(value)


def encode_registry_settings(writer: BinaryWriter, value: RegistrySettings) -> None:
    """Encode one RegistrySettings."""
    writer.write_string(value.url)
    encode_registry_authentication(writer, value.authentication)


def decode_registry_settings(reader: BinaryReader) -> RegistrySettings:
    """Decode one RegistrySettings."""
    url = reader.read_string()
    authentication = decode_registry_authentication(reader)

    return RegistrySettings(
        url=url,
        authentication=authentication,
    )


def to_json_registry_settings(value: RegistrySettings) -> Json:
    """Return one JSON value for one RegistrySettings."""
    return {
        "url": value.url,
        "authentication": to_json_registry_authentication(value.authentication),
    }


def from_json_registry_settings(value: Json) -> RegistrySettings:
    """Return one RegistrySettings from one JSON value."""
    object_ = json_object(value)

    return RegistrySettings(
        url=json_string(json_field(object_, "url")),
        authentication=from_json_registry_authentication(
            json_field(object_, "authentication")
        ),
    )


@dataclass(frozen=True, slots=True)
class RegistryAuthenticationNone:
    """No registry authentication."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class RegistryAuthenticationToken:
    """Inline token stored in the machine-local settings file."""

    # registry token
    token: str
    kind: typing.Literal["token"] = "token"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class RegistryAuthenticationTokenFromEnvironment:
    """Token read from one environment variable."""

    # environment variable name
    variable: str
    kind: typing.Literal["tokenFromEnvironment"] = "tokenFromEnvironment"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_registry_authentication(self)


@dataclass(frozen=True, slots=True)
class RegistryAuthenticationCommand:
    """Token or credential material printed by one command."""

    # program to run
    program: str
    # program arguments
    args: Sequence[str]
    kind: typing.Literal["command"] = "command"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_registry_authentication(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_registry_authentication(self)


"""Registry authentication settings."""
RegistryAuthentication: typing.TypeAlias = (
    RegistryAuthenticationNone
    | RegistryAuthenticationToken
    | RegistryAuthenticationTokenFromEnvironment
    | RegistryAuthenticationCommand
)


def encode_registry_authentication(
    writer: BinaryWriter, value: RegistryAuthentication
) -> None:
    """Encode one RegistryAuthentication."""
    if value.kind == "none":
        writer.write_unsigned(0)
    elif value.kind == "token":
        writer.write_unsigned(1)
        writer.write_string(value.token)
    elif value.kind == "tokenFromEnvironment":
        writer.write_unsigned(2)
        writer.write_string(value.variable)
    elif value.kind == "command":
        writer.write_unsigned(3)
        writer.write_string(value.program)
        writer.write_unsigned(len(value.args))
        for item_value_args_0 in value.args:
            writer.write_string(item_value_args_0)
    else:
        raise SerdeError("unknown enum variant")


def decode_registry_authentication(reader: BinaryReader) -> RegistryAuthentication:
    """Decode one RegistryAuthentication."""
    variant = reader.read_number()

    if variant == 0:
        return RegistryAuthenticationNone()
    elif variant == 1:
        token = reader.read_string()

        return RegistryAuthenticationToken(
            token=token,
        )
    elif variant == 2:
        variable = reader.read_string()

        return RegistryAuthenticationTokenFromEnvironment(
            variable=variable,
        )
    elif variant == 3:
        program = reader.read_string()
        args = [reader.read_string() for _ in range(reader.read_number())]

        return RegistryAuthenticationCommand(
            program=program,
            args=args,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_registry_authentication(value: RegistryAuthentication) -> Json:
    """Return one JSON value for one RegistryAuthentication."""
    if value.kind == "none":
        return {
            "kind": "none",
        }
    elif value.kind == "token":
        return {
            "kind": "token",
            "token": value.token,
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
            "args": [item_0 for item_0 in value.args],
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_registry_authentication(value: Json) -> RegistryAuthentication:
    """Return one RegistryAuthentication from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "none":
        return RegistryAuthenticationNone()
    elif kind == "token":
        return RegistryAuthenticationToken(
            token=json_string(json_field(object_, "token")),
        )
    elif kind == "tokenFromEnvironment":
        return RegistryAuthenticationTokenFromEnvironment(
            variable=json_string(json_field(object_, "variable")),
        )
    elif kind == "command":
        return RegistryAuthenticationCommand(
            program=json_string(json_field(object_, "program")),
            args=[
                json_string(item_0)
                for item_0 in json_array(json_field(object_, "args"))
            ],
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class NetworkSettings:
    """Network settings for package and update commands."""

    # whether network access should be disabled by default
    offline: bool
    # optional HTTP proxy URL
    proxy: str | None
    # request timeout in milliseconds
    timeout_milliseconds: int | None
    # number of retries for transient network failures
    retry_count: int | None
    # maximum concurrent network requests
    concurrency: int | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_network_settings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NetworkSettings:
        """Decode one NetworkSettings."""
        return decode_network_settings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_network_settings(self)

    @classmethod
    def from_json(cls, value: Json) -> NetworkSettings:
        """Return one NetworkSettings from one JSON value."""
        return from_json_network_settings(value)


def encode_network_settings(writer: BinaryWriter, value: NetworkSettings) -> None:
    """Encode one NetworkSettings."""
    writer.write_bool(value.offline)
    if value.proxy is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.proxy)
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


def decode_network_settings(reader: BinaryReader) -> NetworkSettings:
    """Decode one NetworkSettings."""
    offline = reader.read_bool()
    proxy = reader.read_option(lambda: reader.read_string())
    timeout_milliseconds = reader.read_option(lambda: reader.read_number())
    retry_count = reader.read_option(lambda: reader.read_number())
    concurrency = reader.read_option(lambda: reader.read_number())

    return NetworkSettings(
        offline=offline,
        proxy=proxy,
        timeout_milliseconds=timeout_milliseconds,
        retry_count=retry_count,
        concurrency=concurrency,
    )


def to_json_network_settings(value: NetworkSettings) -> Json:
    """Return one JSON value for one NetworkSettings."""
    return {
        "offline": value.offline,
        **({} if value.proxy is None else {"proxy": value.proxy}),
        **(
            {}
            if value.timeout_milliseconds is None
            else {"timeoutMilliseconds": value.timeout_milliseconds}
        ),
        **({} if value.retry_count is None else {"retryCount": value.retry_count}),
        **({} if value.concurrency is None else {"concurrency": value.concurrency}),
    }


def from_json_network_settings(value: Json) -> NetworkSettings:
    """Return one NetworkSettings from one JSON value."""
    object_ = json_object(value)

    return NetworkSettings(
        offline=json_bool(json_field(object_, "offline")),
        proxy=json_optional(object_, "proxy", lambda value: json_string(value)),
        timeout_milliseconds=json_optional(
            object_, "timeoutMilliseconds", lambda value: json_int(value)
        ),
        retry_count=json_optional(object_, "retryCount", lambda value: json_int(value)),
        concurrency=json_optional(
            object_, "concurrency", lambda value: json_int(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class Settings:
    """Machine-local Destack settings."""

    # package directory settings
    packages: PackageSettings
    # workspace cache settings
    cache: CacheSettings
    # registry settings keyed by registry name
    registries: Mapping[str, RegistrySettings]
    # default registry name
    registry: str | None
    # network settings for package and update commands
    network: NetworkSettings

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_settings(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Settings:
        """Decode one Settings."""
        return decode_settings(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_settings(self)

    @classmethod
    def from_json(cls, value: Json) -> Settings:
        """Return one Settings from one JSON value."""
        return from_json_settings(value)


def encode_settings(writer: BinaryWriter, value: Settings) -> None:
    """Encode one Settings."""
    encode_package_settings(writer, value.packages)
    encode_cache_settings(writer, value.cache)
    entries_value_registries_0 = []
    for key_value_registries_0, item_value_registries_0 in value.registries.items():

        def write_key_value_registries_0(writer: BinaryWriter) -> None:
            writer.write_string(key_value_registries_0)

        key_bytes = nested_bytes(write_key_value_registries_0)
        entries_value_registries_0.append(
            (key_value_registries_0, item_value_registries_0, key_bytes)
        )
    entries_value_registries_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_registries_0))
    for entry_value_registries_0 in entries_value_registries_0:
        writer.write_string(entry_value_registries_0[0])
        encode_registry_settings(writer, entry_value_registries_0[1])
    if value.registry is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.registry)
    encode_network_settings(writer, value.network)


def decode_settings(reader: BinaryReader) -> Settings:
    """Decode one Settings."""
    packages = decode_package_settings(reader)
    cache = decode_cache_settings(reader)
    registries = {
        reader.read_string(): decode_registry_settings(reader)
        for _ in range(reader.read_number())
    }
    registry = reader.read_option(lambda: reader.read_string())
    network = decode_network_settings(reader)

    return Settings(
        packages=packages,
        cache=cache,
        registries=registries,
        registry=registry,
        network=network,
    )


def to_json_settings(value: Settings) -> Json:
    """Return one JSON value for one Settings."""
    return {
        "packages": to_json_package_settings(value.packages),
        "cache": to_json_cache_settings(value.cache),
        "registries": {
            key_0: to_json_registry_settings(item_0)
            for key_0, item_0 in value.registries.items()
        },
        **({} if value.registry is None else {"registry": value.registry}),
        "network": to_json_network_settings(value.network),
    }


def from_json_settings(value: Json) -> Settings:
    """Return one Settings from one JSON value."""
    object_ = json_object(value)

    return Settings(
        packages=from_json_package_settings(json_field(object_, "packages")),
        cache=from_json_cache_settings(json_field(object_, "cache")),
        registries={
            key_0: from_json_registry_settings(item_0)
            for key_0, item_0 in json_object(json_field(object_, "registries")).items()
        },
        registry=json_optional(object_, "registry", lambda value: json_string(value)),
        network=from_json_network_settings(json_field(object_, "network")),
    )


__all__ = [
    "PackageSettings",
    "encode_package_settings",
    "decode_package_settings",
    "to_json_package_settings",
    "from_json_package_settings",
    "CacheSettings",
    "encode_cache_settings",
    "decode_cache_settings",
    "to_json_cache_settings",
    "from_json_cache_settings",
    "RegistrySettings",
    "encode_registry_settings",
    "decode_registry_settings",
    "to_json_registry_settings",
    "from_json_registry_settings",
    "RegistryAuthentication",
    "encode_registry_authentication",
    "decode_registry_authentication",
    "to_json_registry_authentication",
    "from_json_registry_authentication",
    "RegistryAuthenticationNone",
    "RegistryAuthenticationToken",
    "RegistryAuthenticationTokenFromEnvironment",
    "RegistryAuthenticationCommand",
    "NetworkSettings",
    "encode_network_settings",
    "decode_network_settings",
    "to_json_network_settings",
    "from_json_network_settings",
    "Settings",
    "encode_settings",
    "decode_settings",
    "to_json_settings",
    "from_json_settings",
]
