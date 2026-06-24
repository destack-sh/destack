# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class HostOptions:
    """Runtime host module defaults."""

    # host poller defaults
    poller: HostPollerOptions
    # filesystem host defaults
    fs: HostFsOptions
    # network host defaults
    net: HostNetOptions
    # process host defaults
    process: HostProcessOptions
    # audio host defaults
    audio: HostAudioOptions
    # display host defaults
    display: HostDisplayOptions
    # input host defaults
    input: HostInputOptions
    # GPU host defaults
    gpu: HostGpuOptions
    # TLS host defaults
    tls: HostTlsOptions
    # OS host defaults
    os: HostOsOptions

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostOptions: ...

def encode_host_options(writer: BinaryWriter, value: HostOptions) -> None: ...
def decode_host_options(reader: BinaryReader) -> HostOptions: ...
def to_json_host_options(value: HostOptions) -> Json: ...
def from_json_host_options(value: Json) -> HostOptions: ...

@dataclass(frozen=True, slots=True)
class HostPollerOptions:
    """Runtime host-poller configuration."""

    # host poller backend selection
    backend: PollerBackend

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostPollerOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostPollerOptions: ...

def encode_host_poller_options(
    writer: BinaryWriter, value: HostPollerOptions
) -> None: ...
def decode_host_poller_options(reader: BinaryReader) -> HostPollerOptions: ...
def to_json_host_poller_options(value: HostPollerOptions) -> Json: ...
def from_json_host_poller_options(value: Json) -> HostPollerOptions: ...

"""Host poller backend selection."""
PollerBackend: typing.TypeAlias = (
    typing.Literal["auto"]
    | typing.Literal["ioUring"]
    | typing.Literal["epoll"]
    | typing.Literal["kqueue"]
    | typing.Literal["poll"]
    | typing.Literal["windows"]
)

def encode_poller_backend(writer: BinaryWriter, value: PollerBackend) -> None: ...
def decode_poller_backend(reader: BinaryReader) -> PollerBackend: ...
def to_json_poller_backend(value: PollerBackend) -> Json: ...
def from_json_poller_backend(value: Json) -> PollerBackend: ...

@dataclass(frozen=True, slots=True)
class HostFsOptions:
    """Filesystem host defaults."""

    # optional temporary directory override
    temporary_directory: str | None
    # optional cache directory override
    cache_directory: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostFsOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostFsOptions: ...

def encode_host_fs_options(writer: BinaryWriter, value: HostFsOptions) -> None: ...
def decode_host_fs_options(reader: BinaryReader) -> HostFsOptions: ...
def to_json_host_fs_options(value: HostFsOptions) -> Json: ...
def from_json_host_fs_options(value: Json) -> HostFsOptions: ...

@dataclass(frozen=True, slots=True)
class HostNetOptions:
    """Network host defaults."""

    # optional DNS server override list
    dns_servers: Sequence[str]
    # optional proxy URL override
    proxy_url: str | None
    # optional default egress interface binding
    bind_interface: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostNetOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostNetOptions: ...

def encode_host_net_options(writer: BinaryWriter, value: HostNetOptions) -> None: ...
def decode_host_net_options(reader: BinaryReader) -> HostNetOptions: ...
def to_json_host_net_options(value: HostNetOptions) -> Json: ...
def from_json_host_net_options(value: Json) -> HostNetOptions: ...

@dataclass(frozen=True, slots=True)
class HostProcessOptions:
    """Process host defaults."""

    # optional default working directory for spawned child processes
    default_working_directory: str | None
    # whether child processes inherit environment variables by default
    inherit_environment: bool | None
    # optional allow-list for inherited environment variables
    environment_allowlist: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostProcessOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostProcessOptions: ...

def encode_host_process_options(
    writer: BinaryWriter, value: HostProcessOptions
) -> None: ...
def decode_host_process_options(reader: BinaryReader) -> HostProcessOptions: ...
def to_json_host_process_options(value: HostProcessOptions) -> Json: ...
def from_json_host_process_options(value: Json) -> HostProcessOptions: ...

@dataclass(frozen=True, slots=True)
class HostAudioOptions:
    """Audio host defaults."""

    # optional preferred audio backend name
    backend: str | None
    # optional preferred output device identifier
    output_device: str | None
    # optional preferred input device identifier
    input_device: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostAudioOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostAudioOptions: ...

def encode_host_audio_options(
    writer: BinaryWriter, value: HostAudioOptions
) -> None: ...
def decode_host_audio_options(reader: BinaryReader) -> HostAudioOptions: ...
def to_json_host_audio_options(value: HostAudioOptions) -> Json: ...
def from_json_host_audio_options(value: Json) -> HostAudioOptions: ...

@dataclass(frozen=True, slots=True)
class HostDisplayOptions:
    """Display host defaults."""

    # optional preferred display backend name
    backend: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostDisplayOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostDisplayOptions: ...

def encode_host_display_options(
    writer: BinaryWriter, value: HostDisplayOptions
) -> None: ...
def decode_host_display_options(reader: BinaryReader) -> HostDisplayOptions: ...
def to_json_host_display_options(value: HostDisplayOptions) -> Json: ...
def from_json_host_display_options(value: Json) -> HostDisplayOptions: ...

@dataclass(frozen=True, slots=True)
class HostInputOptions:
    """Input host defaults."""

    # optional preferred input backend name
    backend: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostInputOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostInputOptions: ...

def encode_host_input_options(
    writer: BinaryWriter, value: HostInputOptions
) -> None: ...
def decode_host_input_options(reader: BinaryReader) -> HostInputOptions: ...
def to_json_host_input_options(value: HostInputOptions) -> Json: ...
def from_json_host_input_options(value: Json) -> HostInputOptions: ...

@dataclass(frozen=True, slots=True)
class HostGpuOptions:
    """GPU host defaults."""

    # optional preferred GPU backend name
    backend: str | None
    # optional preferred adapter name filter
    adapter_name: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostGpuOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostGpuOptions: ...

def encode_host_gpu_options(writer: BinaryWriter, value: HostGpuOptions) -> None: ...
def decode_host_gpu_options(reader: BinaryReader) -> HostGpuOptions: ...
def to_json_host_gpu_options(value: HostGpuOptions) -> Json: ...
def from_json_host_gpu_options(value: Json) -> HostGpuOptions: ...

@dataclass(frozen=True, slots=True)
class HostTlsOptions:
    """TLS host defaults."""

    # optional trust store path override
    trust_store_path: str | None
    # override list for Unix system certificate bundle files
    system_certificate_files: Sequence[str]
    # override list for Unix system certificate directories
    system_certificate_directories: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostTlsOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostTlsOptions: ...

def encode_host_tls_options(writer: BinaryWriter, value: HostTlsOptions) -> None: ...
def decode_host_tls_options(reader: BinaryReader) -> HostTlsOptions: ...
def to_json_host_tls_options(value: HostTlsOptions) -> Json: ...
def from_json_host_tls_options(value: Json) -> HostTlsOptions: ...

@dataclass(frozen=True, slots=True)
class HostOsOptions:
    """OS host defaults."""

    # optional default locale override
    default_locale: str | None
    # optional application data directory override
    data_directory: str | None
    # optional application state directory override
    state_directory: str | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> HostOsOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> HostOsOptions: ...

def encode_host_os_options(writer: BinaryWriter, value: HostOsOptions) -> None: ...
def decode_host_os_options(reader: BinaryReader) -> HostOsOptions: ...
def to_json_host_os_options(value: HostOsOptions) -> Json: ...
def from_json_host_os_options(value: Json) -> HostOsOptions: ...

__all__ = [
    "HostOptions",
    "encode_host_options",
    "decode_host_options",
    "to_json_host_options",
    "from_json_host_options",
    "HostPollerOptions",
    "encode_host_poller_options",
    "decode_host_poller_options",
    "to_json_host_poller_options",
    "from_json_host_poller_options",
    "PollerBackend",
    "encode_poller_backend",
    "decode_poller_backend",
    "to_json_poller_backend",
    "from_json_poller_backend",
    "HostFsOptions",
    "encode_host_fs_options",
    "decode_host_fs_options",
    "to_json_host_fs_options",
    "from_json_host_fs_options",
    "HostNetOptions",
    "encode_host_net_options",
    "decode_host_net_options",
    "to_json_host_net_options",
    "from_json_host_net_options",
    "HostProcessOptions",
    "encode_host_process_options",
    "decode_host_process_options",
    "to_json_host_process_options",
    "from_json_host_process_options",
    "HostAudioOptions",
    "encode_host_audio_options",
    "decode_host_audio_options",
    "to_json_host_audio_options",
    "from_json_host_audio_options",
    "HostDisplayOptions",
    "encode_host_display_options",
    "decode_host_display_options",
    "to_json_host_display_options",
    "from_json_host_display_options",
    "HostInputOptions",
    "encode_host_input_options",
    "decode_host_input_options",
    "to_json_host_input_options",
    "from_json_host_input_options",
    "HostGpuOptions",
    "encode_host_gpu_options",
    "decode_host_gpu_options",
    "to_json_host_gpu_options",
    "from_json_host_gpu_options",
    "HostTlsOptions",
    "encode_host_tls_options",
    "decode_host_tls_options",
    "to_json_host_tls_options",
    "from_json_host_tls_options",
    "HostOsOptions",
    "encode_host_os_options",
    "decode_host_os_options",
    "to_json_host_os_options",
    "from_json_host_os_options",
]
