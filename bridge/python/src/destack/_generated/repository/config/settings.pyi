# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class PackageSettings:
    """Package directory settings."""

    # package directory
    path: str | None
    # maximum package directory size in bytes before pruning is requested
    maximum_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> PackageSettings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> PackageSettings: ...

def encode_package_settings(writer: BinaryWriter, value: PackageSettings) -> None: ...
def decode_package_settings(reader: BinaryReader) -> PackageSettings: ...
def to_json_package_settings(value: PackageSettings) -> Json: ...
def from_json_package_settings(value: Json) -> PackageSettings: ...

@dataclass(frozen=True, slots=True)
class CacheSettings:
    """Workspace cache settings."""

    # cache directory
    path: str | None
    # maximum cache size in bytes before pruning is requested
    maximum_bytes: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CacheSettings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CacheSettings: ...

def encode_cache_settings(writer: BinaryWriter, value: CacheSettings) -> None: ...
def decode_cache_settings(reader: BinaryReader) -> CacheSettings: ...
def to_json_cache_settings(value: CacheSettings) -> Json: ...
def from_json_cache_settings(value: Json) -> CacheSettings: ...

@dataclass(frozen=True, slots=True)
class RegistrySettings:
    """Registry settings."""

    # registry URL
    url: str
    # authentication used for this registry
    authentication: RegistryAuthentication

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RegistrySettings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RegistrySettings: ...

def encode_registry_settings(writer: BinaryWriter, value: RegistrySettings) -> None: ...
def decode_registry_settings(reader: BinaryReader) -> RegistrySettings: ...
def to_json_registry_settings(value: RegistrySettings) -> Json: ...
def from_json_registry_settings(value: Json) -> RegistrySettings: ...

@dataclass(frozen=True, slots=True)
class RegistryAuthenticationNone:
    """No registry authentication."""

    kind: typing.Literal["none"] = "none"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class RegistryAuthenticationToken:
    """Inline token stored in the machine-local settings file."""

    # registry token
    token: str
    kind: typing.Literal["token"] = "token"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class RegistryAuthenticationTokenFromEnvironment:
    """Token read from one environment variable."""

    # environment variable name
    variable: str
    kind: typing.Literal["tokenFromEnvironment"] = "tokenFromEnvironment"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class RegistryAuthenticationCommand:
    """Token or credential material printed by one command."""

    # program to run
    program: str
    # program arguments
    args: Sequence[str]
    kind: typing.Literal["command"] = "command"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Registry authentication settings."""
RegistryAuthentication: typing.TypeAlias = (
    RegistryAuthenticationNone
    | RegistryAuthenticationToken
    | RegistryAuthenticationTokenFromEnvironment
    | RegistryAuthenticationCommand
)

def encode_registry_authentication(
    writer: BinaryWriter, value: RegistryAuthentication
) -> None: ...
def decode_registry_authentication(reader: BinaryReader) -> RegistryAuthentication: ...
def to_json_registry_authentication(value: RegistryAuthentication) -> Json: ...
def from_json_registry_authentication(value: Json) -> RegistryAuthentication: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NetworkSettings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NetworkSettings: ...

def encode_network_settings(writer: BinaryWriter, value: NetworkSettings) -> None: ...
def decode_network_settings(reader: BinaryReader) -> NetworkSettings: ...
def to_json_network_settings(value: NetworkSettings) -> Json: ...
def from_json_network_settings(value: Json) -> NetworkSettings: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Settings: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Settings: ...

def encode_settings(writer: BinaryWriter, value: Settings) -> None: ...
def decode_settings(reader: BinaryReader) -> Settings: ...
def to_json_settings(value: Settings) -> Json: ...
def from_json_settings(value: Json) -> Settings: ...

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
