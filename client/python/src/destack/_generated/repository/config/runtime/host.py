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
    json_object,
    json_optional,
    json_string,
)


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostOptions:
        """Decode one HostOptions."""
        return decode_host_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostOptions:
        """Return one HostOptions from one JSON value."""
        return from_json_host_options(value)


def encode_host_options(writer: BinaryWriter, value: HostOptions) -> None:
    """Encode one HostOptions."""
    encode_host_poller_options(writer, value.poller)
    encode_host_fs_options(writer, value.fs)
    encode_host_net_options(writer, value.net)
    encode_host_process_options(writer, value.process)
    encode_host_audio_options(writer, value.audio)
    encode_host_display_options(writer, value.display)
    encode_host_input_options(writer, value.input)
    encode_host_gpu_options(writer, value.gpu)
    encode_host_tls_options(writer, value.tls)
    encode_host_os_options(writer, value.os)


def decode_host_options(reader: BinaryReader) -> HostOptions:
    """Decode one HostOptions."""
    poller = decode_host_poller_options(reader)
    fs = decode_host_fs_options(reader)
    net = decode_host_net_options(reader)
    process = decode_host_process_options(reader)
    audio = decode_host_audio_options(reader)
    display = decode_host_display_options(reader)
    input = decode_host_input_options(reader)
    gpu = decode_host_gpu_options(reader)
    tls = decode_host_tls_options(reader)
    os = decode_host_os_options(reader)

    return HostOptions(
        poller=poller,
        fs=fs,
        net=net,
        process=process,
        audio=audio,
        display=display,
        input=input,
        gpu=gpu,
        tls=tls,
        os=os,
    )


def to_json_host_options(value: HostOptions) -> Json:
    """Return one JSON value for one HostOptions."""
    return {
        "poller": to_json_host_poller_options(value.poller),
        "fs": to_json_host_fs_options(value.fs),
        "net": to_json_host_net_options(value.net),
        "process": to_json_host_process_options(value.process),
        "audio": to_json_host_audio_options(value.audio),
        "display": to_json_host_display_options(value.display),
        "input": to_json_host_input_options(value.input),
        "gpu": to_json_host_gpu_options(value.gpu),
        "tls": to_json_host_tls_options(value.tls),
        "os": to_json_host_os_options(value.os),
    }


def from_json_host_options(value: Json) -> HostOptions:
    """Return one HostOptions from one JSON value."""
    object_ = json_object(value)

    return HostOptions(
        poller=from_json_host_poller_options(json_field(object_, "poller")),
        fs=from_json_host_fs_options(json_field(object_, "fs")),
        net=from_json_host_net_options(json_field(object_, "net")),
        process=from_json_host_process_options(json_field(object_, "process")),
        audio=from_json_host_audio_options(json_field(object_, "audio")),
        display=from_json_host_display_options(json_field(object_, "display")),
        input=from_json_host_input_options(json_field(object_, "input")),
        gpu=from_json_host_gpu_options(json_field(object_, "gpu")),
        tls=from_json_host_tls_options(json_field(object_, "tls")),
        os=from_json_host_os_options(json_field(object_, "os")),
    )


@dataclass(frozen=True, slots=True)
class HostPollerOptions:
    """Runtime host-poller configuration."""

    # host poller backend selection
    backend: PollerBackend

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_poller_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostPollerOptions:
        """Decode one HostPollerOptions."""
        return decode_host_poller_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_poller_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostPollerOptions:
        """Return one HostPollerOptions from one JSON value."""
        return from_json_host_poller_options(value)


def encode_host_poller_options(writer: BinaryWriter, value: HostPollerOptions) -> None:
    """Encode one HostPollerOptions."""
    encode_poller_backend(writer, value.backend)


def decode_host_poller_options(reader: BinaryReader) -> HostPollerOptions:
    """Decode one HostPollerOptions."""
    backend = decode_poller_backend(reader)

    return HostPollerOptions(
        backend=backend,
    )


def to_json_host_poller_options(value: HostPollerOptions) -> Json:
    """Return one JSON value for one HostPollerOptions."""
    return {
        "backend": to_json_poller_backend(value.backend),
    }


def from_json_host_poller_options(value: Json) -> HostPollerOptions:
    """Return one HostPollerOptions from one JSON value."""
    object_ = json_object(value)

    return HostPollerOptions(
        backend=from_json_poller_backend(json_field(object_, "backend")),
    )


"""Host poller backend selection."""
PollerBackend: typing.TypeAlias = (
    typing.Literal["auto"]
    | typing.Literal["ioUring"]
    | typing.Literal["epoll"]
    | typing.Literal["kqueue"]
    | typing.Literal["poll"]
    | typing.Literal["windows"]
)


def encode_poller_backend(writer: BinaryWriter, value: PollerBackend) -> None:
    """Encode one PollerBackend."""
    if value == "auto":
        writer.write_unsigned(0)
    elif value == "ioUring":
        writer.write_unsigned(1)
    elif value == "epoll":
        writer.write_unsigned(2)
    elif value == "kqueue":
        writer.write_unsigned(3)
    elif value == "poll":
        writer.write_unsigned(4)
    elif value == "windows":
        writer.write_unsigned(5)
    else:
        raise SerdeError("unknown enum variant")


def decode_poller_backend(reader: BinaryReader) -> PollerBackend:
    """Decode one PollerBackend."""
    variant = reader.read_number()

    if variant == 0:
        return "auto"
    elif variant == 1:
        return "ioUring"
    elif variant == 2:
        return "epoll"
    elif variant == 3:
        return "kqueue"
    elif variant == 4:
        return "poll"
    elif variant == 5:
        return "windows"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_poller_backend(value: PollerBackend) -> Json:
    """Return one JSON value for one PollerBackend."""
    return value


def from_json_poller_backend(value: Json) -> PollerBackend:
    """Return one PollerBackend from one JSON value."""
    variant = json_string(value)

    if variant == "auto":
        return "auto"
    elif variant == "ioUring":
        return "ioUring"
    elif variant == "epoll":
        return "epoll"
    elif variant == "kqueue":
        return "kqueue"
    elif variant == "poll":
        return "poll"
    elif variant == "windows":
        return "windows"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class HostFsOptions:
    """Filesystem host defaults."""

    # optional temporary directory override
    temporary_directory: str | None
    # optional cache directory override
    cache_directory: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_fs_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostFsOptions:
        """Decode one HostFsOptions."""
        return decode_host_fs_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_fs_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostFsOptions:
        """Return one HostFsOptions from one JSON value."""
        return from_json_host_fs_options(value)


def encode_host_fs_options(writer: BinaryWriter, value: HostFsOptions) -> None:
    """Encode one HostFsOptions."""
    if value.temporary_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.temporary_directory)
    if value.cache_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.cache_directory)


def decode_host_fs_options(reader: BinaryReader) -> HostFsOptions:
    """Decode one HostFsOptions."""
    temporary_directory = reader.read_option(lambda: reader.read_string())
    cache_directory = reader.read_option(lambda: reader.read_string())

    return HostFsOptions(
        temporary_directory=temporary_directory,
        cache_directory=cache_directory,
    )


def to_json_host_fs_options(value: HostFsOptions) -> Json:
    """Return one JSON value for one HostFsOptions."""
    return {
        **(
            {}
            if value.temporary_directory is None
            else {"temporaryDirectory": value.temporary_directory}
        ),
        **(
            {}
            if value.cache_directory is None
            else {"cacheDirectory": value.cache_directory}
        ),
    }


def from_json_host_fs_options(value: Json) -> HostFsOptions:
    """Return one HostFsOptions from one JSON value."""
    object_ = json_object(value)

    return HostFsOptions(
        temporary_directory=json_optional(
            object_, "temporaryDirectory", lambda value: json_string(value)
        ),
        cache_directory=json_optional(
            object_, "cacheDirectory", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class HostNetOptions:
    """Network host defaults."""

    # optional DNS server override list
    dns_servers: Sequence[str]
    # optional proxy URL override
    proxy_url: str | None
    # optional default egress interface binding
    bind_interface: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_net_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostNetOptions:
        """Decode one HostNetOptions."""
        return decode_host_net_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_net_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostNetOptions:
        """Return one HostNetOptions from one JSON value."""
        return from_json_host_net_options(value)


def encode_host_net_options(writer: BinaryWriter, value: HostNetOptions) -> None:
    """Encode one HostNetOptions."""
    writer.write_unsigned(len(value.dns_servers))
    for item_value_dns_servers_0 in value.dns_servers:
        writer.write_string(item_value_dns_servers_0)
    if value.proxy_url is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.proxy_url)
    if value.bind_interface is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.bind_interface)


def decode_host_net_options(reader: BinaryReader) -> HostNetOptions:
    """Decode one HostNetOptions."""
    dns_servers = [reader.read_string() for _ in range(reader.read_number())]
    proxy_url = reader.read_option(lambda: reader.read_string())
    bind_interface = reader.read_option(lambda: reader.read_string())

    return HostNetOptions(
        dns_servers=dns_servers,
        proxy_url=proxy_url,
        bind_interface=bind_interface,
    )


def to_json_host_net_options(value: HostNetOptions) -> Json:
    """Return one JSON value for one HostNetOptions."""
    return {
        "dnsServers": [item_0 for item_0 in value.dns_servers],
        **({} if value.proxy_url is None else {"proxyUrl": value.proxy_url}),
        **(
            {}
            if value.bind_interface is None
            else {"bindInterface": value.bind_interface}
        ),
    }


def from_json_host_net_options(value: Json) -> HostNetOptions:
    """Return one HostNetOptions from one JSON value."""
    object_ = json_object(value)

    return HostNetOptions(
        dns_servers=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "dnsServers"))
        ],
        proxy_url=json_optional(object_, "proxyUrl", lambda value: json_string(value)),
        bind_interface=json_optional(
            object_, "bindInterface", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class HostProcessOptions:
    """Process host defaults."""

    # optional default working directory for spawned child processes
    default_working_directory: str | None
    # whether child processes inherit environment variables by default
    inherit_environment: bool | None
    # optional allow-list for inherited environment variables
    environment_allowlist: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_process_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostProcessOptions:
        """Decode one HostProcessOptions."""
        return decode_host_process_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_process_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostProcessOptions:
        """Return one HostProcessOptions from one JSON value."""
        return from_json_host_process_options(value)


def encode_host_process_options(
    writer: BinaryWriter, value: HostProcessOptions
) -> None:
    """Encode one HostProcessOptions."""
    if value.default_working_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.default_working_directory)
    if value.inherit_environment is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_bool(value.inherit_environment)
    writer.write_unsigned(len(value.environment_allowlist))
    for item_value_environment_allowlist_0 in value.environment_allowlist:
        writer.write_string(item_value_environment_allowlist_0)


def decode_host_process_options(reader: BinaryReader) -> HostProcessOptions:
    """Decode one HostProcessOptions."""
    default_working_directory = reader.read_option(lambda: reader.read_string())
    inherit_environment = reader.read_option(lambda: reader.read_bool())
    environment_allowlist = [reader.read_string() for _ in range(reader.read_number())]

    return HostProcessOptions(
        default_working_directory=default_working_directory,
        inherit_environment=inherit_environment,
        environment_allowlist=environment_allowlist,
    )


def to_json_host_process_options(value: HostProcessOptions) -> Json:
    """Return one JSON value for one HostProcessOptions."""
    return {
        **(
            {}
            if value.default_working_directory is None
            else {"defaultWorkingDirectory": value.default_working_directory}
        ),
        **(
            {}
            if value.inherit_environment is None
            else {"inheritEnvironment": value.inherit_environment}
        ),
        "environmentAllowlist": [item_0 for item_0 in value.environment_allowlist],
    }


def from_json_host_process_options(value: Json) -> HostProcessOptions:
    """Return one HostProcessOptions from one JSON value."""
    object_ = json_object(value)

    return HostProcessOptions(
        default_working_directory=json_optional(
            object_, "defaultWorkingDirectory", lambda value: json_string(value)
        ),
        inherit_environment=json_optional(
            object_, "inheritEnvironment", lambda value: json_bool(value)
        ),
        environment_allowlist=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "environmentAllowlist"))
        ],
    )


@dataclass(frozen=True, slots=True)
class HostAudioOptions:
    """Audio host defaults."""

    # optional preferred audio backend name
    backend: str | None
    # optional preferred output device identifier
    output_device: str | None
    # optional preferred input device identifier
    input_device: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_audio_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostAudioOptions:
        """Decode one HostAudioOptions."""
        return decode_host_audio_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_audio_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostAudioOptions:
        """Return one HostAudioOptions from one JSON value."""
        return from_json_host_audio_options(value)


def encode_host_audio_options(writer: BinaryWriter, value: HostAudioOptions) -> None:
    """Encode one HostAudioOptions."""
    if value.backend is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.backend)
    if value.output_device is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.output_device)
    if value.input_device is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.input_device)


def decode_host_audio_options(reader: BinaryReader) -> HostAudioOptions:
    """Decode one HostAudioOptions."""
    backend = reader.read_option(lambda: reader.read_string())
    output_device = reader.read_option(lambda: reader.read_string())
    input_device = reader.read_option(lambda: reader.read_string())

    return HostAudioOptions(
        backend=backend,
        output_device=output_device,
        input_device=input_device,
    )


def to_json_host_audio_options(value: HostAudioOptions) -> Json:
    """Return one JSON value for one HostAudioOptions."""
    return {
        **({} if value.backend is None else {"backend": value.backend}),
        **(
            {} if value.output_device is None else {"outputDevice": value.output_device}
        ),
        **({} if value.input_device is None else {"inputDevice": value.input_device}),
    }


def from_json_host_audio_options(value: Json) -> HostAudioOptions:
    """Return one HostAudioOptions from one JSON value."""
    object_ = json_object(value)

    return HostAudioOptions(
        backend=json_optional(object_, "backend", lambda value: json_string(value)),
        output_device=json_optional(
            object_, "outputDevice", lambda value: json_string(value)
        ),
        input_device=json_optional(
            object_, "inputDevice", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class HostDisplayOptions:
    """Display host defaults."""

    # optional preferred display backend name
    backend: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_display_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostDisplayOptions:
        """Decode one HostDisplayOptions."""
        return decode_host_display_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_display_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostDisplayOptions:
        """Return one HostDisplayOptions from one JSON value."""
        return from_json_host_display_options(value)


def encode_host_display_options(
    writer: BinaryWriter, value: HostDisplayOptions
) -> None:
    """Encode one HostDisplayOptions."""
    if value.backend is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.backend)


def decode_host_display_options(reader: BinaryReader) -> HostDisplayOptions:
    """Decode one HostDisplayOptions."""
    backend = reader.read_option(lambda: reader.read_string())

    return HostDisplayOptions(
        backend=backend,
    )


def to_json_host_display_options(value: HostDisplayOptions) -> Json:
    """Return one JSON value for one HostDisplayOptions."""
    return {
        **({} if value.backend is None else {"backend": value.backend}),
    }


def from_json_host_display_options(value: Json) -> HostDisplayOptions:
    """Return one HostDisplayOptions from one JSON value."""
    object_ = json_object(value)

    return HostDisplayOptions(
        backend=json_optional(object_, "backend", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class HostInputOptions:
    """Input host defaults."""

    # optional preferred input backend name
    backend: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_input_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostInputOptions:
        """Decode one HostInputOptions."""
        return decode_host_input_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_input_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostInputOptions:
        """Return one HostInputOptions from one JSON value."""
        return from_json_host_input_options(value)


def encode_host_input_options(writer: BinaryWriter, value: HostInputOptions) -> None:
    """Encode one HostInputOptions."""
    if value.backend is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.backend)


def decode_host_input_options(reader: BinaryReader) -> HostInputOptions:
    """Decode one HostInputOptions."""
    backend = reader.read_option(lambda: reader.read_string())

    return HostInputOptions(
        backend=backend,
    )


def to_json_host_input_options(value: HostInputOptions) -> Json:
    """Return one JSON value for one HostInputOptions."""
    return {
        **({} if value.backend is None else {"backend": value.backend}),
    }


def from_json_host_input_options(value: Json) -> HostInputOptions:
    """Return one HostInputOptions from one JSON value."""
    object_ = json_object(value)

    return HostInputOptions(
        backend=json_optional(object_, "backend", lambda value: json_string(value)),
    )


@dataclass(frozen=True, slots=True)
class HostGpuOptions:
    """GPU host defaults."""

    # optional preferred GPU backend name
    backend: str | None
    # optional preferred adapter name filter
    adapter_name: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_gpu_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostGpuOptions:
        """Decode one HostGpuOptions."""
        return decode_host_gpu_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_gpu_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostGpuOptions:
        """Return one HostGpuOptions from one JSON value."""
        return from_json_host_gpu_options(value)


def encode_host_gpu_options(writer: BinaryWriter, value: HostGpuOptions) -> None:
    """Encode one HostGpuOptions."""
    if value.backend is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.backend)
    if value.adapter_name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.adapter_name)


def decode_host_gpu_options(reader: BinaryReader) -> HostGpuOptions:
    """Decode one HostGpuOptions."""
    backend = reader.read_option(lambda: reader.read_string())
    adapter_name = reader.read_option(lambda: reader.read_string())

    return HostGpuOptions(
        backend=backend,
        adapter_name=adapter_name,
    )


def to_json_host_gpu_options(value: HostGpuOptions) -> Json:
    """Return one JSON value for one HostGpuOptions."""
    return {
        **({} if value.backend is None else {"backend": value.backend}),
        **({} if value.adapter_name is None else {"adapterName": value.adapter_name}),
    }


def from_json_host_gpu_options(value: Json) -> HostGpuOptions:
    """Return one HostGpuOptions from one JSON value."""
    object_ = json_object(value)

    return HostGpuOptions(
        backend=json_optional(object_, "backend", lambda value: json_string(value)),
        adapter_name=json_optional(
            object_, "adapterName", lambda value: json_string(value)
        ),
    )


@dataclass(frozen=True, slots=True)
class HostTlsOptions:
    """TLS host defaults."""

    # optional trust store path override
    trust_store_path: str | None
    # override list for Unix system certificate bundle files
    system_certificate_files: Sequence[str]
    # override list for Unix system certificate directories
    system_certificate_directories: Sequence[str]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_tls_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostTlsOptions:
        """Decode one HostTlsOptions."""
        return decode_host_tls_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_tls_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostTlsOptions:
        """Return one HostTlsOptions from one JSON value."""
        return from_json_host_tls_options(value)


def encode_host_tls_options(writer: BinaryWriter, value: HostTlsOptions) -> None:
    """Encode one HostTlsOptions."""
    if value.trust_store_path is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.trust_store_path)
    writer.write_unsigned(len(value.system_certificate_files))
    for item_value_system_certificate_files_0 in value.system_certificate_files:
        writer.write_string(item_value_system_certificate_files_0)
    writer.write_unsigned(len(value.system_certificate_directories))
    for (
        item_value_system_certificate_directories_0
    ) in value.system_certificate_directories:
        writer.write_string(item_value_system_certificate_directories_0)


def decode_host_tls_options(reader: BinaryReader) -> HostTlsOptions:
    """Decode one HostTlsOptions."""
    trust_store_path = reader.read_option(lambda: reader.read_string())
    system_certificate_files = [
        reader.read_string() for _ in range(reader.read_number())
    ]
    system_certificate_directories = [
        reader.read_string() for _ in range(reader.read_number())
    ]

    return HostTlsOptions(
        trust_store_path=trust_store_path,
        system_certificate_files=system_certificate_files,
        system_certificate_directories=system_certificate_directories,
    )


def to_json_host_tls_options(value: HostTlsOptions) -> Json:
    """Return one JSON value for one HostTlsOptions."""
    return {
        **(
            {}
            if value.trust_store_path is None
            else {"trustStorePath": value.trust_store_path}
        ),
        "systemCertificateFiles": [item_0 for item_0 in value.system_certificate_files],
        "systemCertificateDirectories": [
            item_0 for item_0 in value.system_certificate_directories
        ],
    }


def from_json_host_tls_options(value: Json) -> HostTlsOptions:
    """Return one HostTlsOptions from one JSON value."""
    object_ = json_object(value)

    return HostTlsOptions(
        trust_store_path=json_optional(
            object_, "trustStorePath", lambda value: json_string(value)
        ),
        system_certificate_files=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "systemCertificateFiles"))
        ],
        system_certificate_directories=[
            json_string(item_0)
            for item_0 in json_array(
                json_field(object_, "systemCertificateDirectories")
            )
        ],
    )


@dataclass(frozen=True, slots=True)
class HostOsOptions:
    """OS host defaults."""

    # optional default locale override
    default_locale: str | None
    # optional application data directory override
    data_directory: str | None
    # optional application state directory override
    state_directory: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_host_os_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> HostOsOptions:
        """Decode one HostOsOptions."""
        return decode_host_os_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_host_os_options(self)

    @classmethod
    def from_json(cls, value: Json) -> HostOsOptions:
        """Return one HostOsOptions from one JSON value."""
        return from_json_host_os_options(value)


def encode_host_os_options(writer: BinaryWriter, value: HostOsOptions) -> None:
    """Encode one HostOsOptions."""
    if value.default_locale is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.default_locale)
    if value.data_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.data_directory)
    if value.state_directory is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.state_directory)


def decode_host_os_options(reader: BinaryReader) -> HostOsOptions:
    """Decode one HostOsOptions."""
    default_locale = reader.read_option(lambda: reader.read_string())
    data_directory = reader.read_option(lambda: reader.read_string())
    state_directory = reader.read_option(lambda: reader.read_string())

    return HostOsOptions(
        default_locale=default_locale,
        data_directory=data_directory,
        state_directory=state_directory,
    )


def to_json_host_os_options(value: HostOsOptions) -> Json:
    """Return one JSON value for one HostOsOptions."""
    return {
        **(
            {}
            if value.default_locale is None
            else {"defaultLocale": value.default_locale}
        ),
        **(
            {}
            if value.data_directory is None
            else {"dataDirectory": value.data_directory}
        ),
        **(
            {}
            if value.state_directory is None
            else {"stateDirectory": value.state_directory}
        ),
    }


def from_json_host_os_options(value: Json) -> HostOsOptions:
    """Return one HostOsOptions from one JSON value."""
    object_ = json_object(value)

    return HostOsOptions(
        default_locale=json_optional(
            object_, "defaultLocale", lambda value: json_string(value)
        ),
        data_directory=json_optional(
            object_, "dataDirectory", lambda value: json_string(value)
        ),
        state_directory=json_optional(
            object_, "stateDirectory", lambda value: json_string(value)
        ),
    )


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
