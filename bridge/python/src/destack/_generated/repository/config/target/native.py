# generated bridge target, do not edit

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
    json_field,
    json_object,
    json_optional,
    json_string,
)

import destack._generated.artifact.core.target


@dataclass(frozen=True, slots=True)
class TargetNativeOptions:
    """Native target configuration."""

    # native output shape
    output: NativeOutputKind
    # target architecture for native codegen
    arch: destack._generated.artifact.core.target.TargetArch | None
    # target vendor for native codegen
    vendor: destack._generated.artifact.core.target.TargetVendor | None
    # target ABI for native codegen
    abi: destack._generated.artifact.core.target.TargetAbi | None
    # CPU name for native codegen
    cpu: str | None
    # CPU feature flags for native codegen
    cpu_features: Sequence[str]
    # sysroot path for native toolchains
    sysroot: str | None
    # c runtime linkage policy
    crt: CrtLinkage
    # native linker configuration
    link: TargetLinkOptions

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_native_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetNativeOptions:
        """Decode one TargetNativeOptions."""
        return decode_target_native_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_native_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetNativeOptions:
        """Return one TargetNativeOptions from one JSON value."""
        return from_json_target_native_options(value)


def encode_target_native_options(
    writer: BinaryWriter, value: TargetNativeOptions
) -> None:
    """Encode one TargetNativeOptions."""
    encode_native_output_kind(writer, value.output)
    if value.arch is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_target_arch(writer, value.arch)
    if value.vendor is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_target_vendor(
            writer, value.vendor
        )
    if value.abi is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        destack._generated.artifact.core.target.encode_target_abi(writer, value.abi)
    if value.cpu is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.cpu)
    writer.write_unsigned(len(value.cpu_features))
    for item_value_cpu_features_0 in value.cpu_features:
        writer.write_string(item_value_cpu_features_0)
    if value.sysroot is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.sysroot)
    encode_crt_linkage(writer, value.crt)
    encode_target_link_options(writer, value.link)


def decode_target_native_options(reader: BinaryReader) -> TargetNativeOptions:
    """Decode one TargetNativeOptions."""
    output = decode_native_output_kind(reader)
    arch = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_target_arch(reader)
    )
    vendor = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_target_vendor(reader)
    )
    abi = reader.read_option(
        lambda: destack._generated.artifact.core.target.decode_target_abi(reader)
    )
    cpu = reader.read_option(lambda: reader.read_string())
    cpu_features = [reader.read_string() for _ in range(reader.read_number())]
    sysroot = reader.read_option(lambda: reader.read_string())
    crt = decode_crt_linkage(reader)
    link = decode_target_link_options(reader)

    return TargetNativeOptions(
        output=output,
        arch=arch,
        vendor=vendor,
        abi=abi,
        cpu=cpu,
        cpu_features=cpu_features,
        sysroot=sysroot,
        crt=crt,
        link=link,
    )


def to_json_target_native_options(value: TargetNativeOptions) -> Json:
    """Return one JSON value for one TargetNativeOptions."""
    return {
        "output": to_json_native_output_kind(value.output),
        **(
            {}
            if value.arch is None
            else {
                "arch": destack._generated.artifact.core.target.to_json_target_arch(
                    value.arch
                )
            }
        ),
        **(
            {}
            if value.vendor is None
            else {
                "vendor": destack._generated.artifact.core.target.to_json_target_vendor(
                    value.vendor
                )
            }
        ),
        **(
            {}
            if value.abi is None
            else {
                "abi": destack._generated.artifact.core.target.to_json_target_abi(
                    value.abi
                )
            }
        ),
        **({} if value.cpu is None else {"cpu": value.cpu}),
        "cpuFeatures": [item_0 for item_0 in value.cpu_features],
        **({} if value.sysroot is None else {"sysroot": value.sysroot}),
        "crt": to_json_crt_linkage(value.crt),
        "link": to_json_target_link_options(value.link),
    }


def from_json_target_native_options(value: Json) -> TargetNativeOptions:
    """Return one TargetNativeOptions from one JSON value."""
    object_ = json_object(value)

    return TargetNativeOptions(
        output=from_json_native_output_kind(json_field(object_, "output")),
        arch=json_optional(
            object_,
            "arch",
            lambda value: destack._generated.artifact.core.target.from_json_target_arch(
                value
            ),
        ),
        vendor=json_optional(
            object_,
            "vendor",
            lambda value: (
                destack._generated.artifact.core.target.from_json_target_vendor(value)
            ),
        ),
        abi=json_optional(
            object_,
            "abi",
            lambda value: destack._generated.artifact.core.target.from_json_target_abi(
                value
            ),
        ),
        cpu=json_optional(object_, "cpu", lambda value: json_string(value)),
        cpu_features=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "cpuFeatures"))
        ],
        sysroot=json_optional(object_, "sysroot", lambda value: json_string(value)),
        crt=from_json_crt_linkage(json_field(object_, "crt")),
        link=from_json_target_link_options(json_field(object_, "link")),
    )


"""Native output kind for one target."""
NativeOutputKind: typing.TypeAlias = (
    typing.Literal["executable"]
    | typing.Literal["staticLibrary"]
    | typing.Literal["sharedLibrary"]
)


def encode_native_output_kind(writer: BinaryWriter, value: NativeOutputKind) -> None:
    """Encode one NativeOutputKind."""
    if value == "executable":
        writer.write_unsigned(0)
    elif value == "staticLibrary":
        writer.write_unsigned(1)
    elif value == "sharedLibrary":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_native_output_kind(reader: BinaryReader) -> NativeOutputKind:
    """Decode one NativeOutputKind."""
    variant = reader.read_number()

    if variant == 0:
        return "executable"
    elif variant == 1:
        return "staticLibrary"
    elif variant == 2:
        return "sharedLibrary"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_native_output_kind(value: NativeOutputKind) -> Json:
    """Return one JSON value for one NativeOutputKind."""
    return value


def from_json_native_output_kind(value: Json) -> NativeOutputKind:
    """Return one NativeOutputKind from one JSON value."""
    variant = json_string(value)

    if variant == "executable":
        return "executable"
    elif variant == "staticLibrary":
        return "staticLibrary"
    elif variant == "sharedLibrary":
        return "sharedLibrary"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""C runtime linkage policy for one native target."""
CrtLinkage: typing.TypeAlias = (
    typing.Literal["default"] | typing.Literal["dynamic"] | typing.Literal["static"]
)


def encode_crt_linkage(writer: BinaryWriter, value: CrtLinkage) -> None:
    """Encode one CrtLinkage."""
    if value == "default":
        writer.write_unsigned(0)
    elif value == "dynamic":
        writer.write_unsigned(1)
    elif value == "static":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_crt_linkage(reader: BinaryReader) -> CrtLinkage:
    """Decode one CrtLinkage."""
    variant = reader.read_number()

    if variant == 0:
        return "default"
    elif variant == 1:
        return "dynamic"
    elif variant == 2:
        return "static"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_crt_linkage(value: CrtLinkage) -> Json:
    """Return one JSON value for one CrtLinkage."""
    return value


def from_json_crt_linkage(value: Json) -> CrtLinkage:
    """Return one CrtLinkage from one JSON value."""
    variant = json_string(value)

    if variant == "default":
        return "default"
    elif variant == "dynamic":
        return "dynamic"
    elif variant == "static":
        return "static"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TargetLinkOptions:
    """Target native linker configuration."""

    # explicit linker executable
    linker: str | None
    # extra linker arguments
    args: Sequence[str]
    # additional library search paths
    library_paths: Sequence[str]
    # additional libraries to link
    libraries: Sequence[str]
    # additional framework search paths
    framework_paths: Sequence[str]
    # additional frameworks to link
    frameworks: Sequence[str]
    # runtime dynamic library search paths
    runtime_library_paths: Sequence[str]
    # symbol visibility policy
    symbol_visibility: SymbolVisibility
    # version script for exported symbols
    version_script: str | None
    # linker script for the final link
    linker_script: str | None
    # position independent code policy
    position_independent: PositionIndependentMode
    # shared object soname
    soname: str | None
    # darwin install name
    install_name: str | None

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_link_options(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetLinkOptions:
        """Decode one TargetLinkOptions."""
        return decode_target_link_options(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_link_options(self)

    @classmethod
    def from_json(cls, value: Json) -> TargetLinkOptions:
        """Return one TargetLinkOptions from one JSON value."""
        return from_json_target_link_options(value)


def encode_target_link_options(writer: BinaryWriter, value: TargetLinkOptions) -> None:
    """Encode one TargetLinkOptions."""
    if value.linker is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.linker)
    writer.write_unsigned(len(value.args))
    for item_value_args_0 in value.args:
        writer.write_string(item_value_args_0)
    writer.write_unsigned(len(value.library_paths))
    for item_value_library_paths_0 in value.library_paths:
        writer.write_string(item_value_library_paths_0)
    writer.write_unsigned(len(value.libraries))
    for item_value_libraries_0 in value.libraries:
        writer.write_string(item_value_libraries_0)
    writer.write_unsigned(len(value.framework_paths))
    for item_value_framework_paths_0 in value.framework_paths:
        writer.write_string(item_value_framework_paths_0)
    writer.write_unsigned(len(value.frameworks))
    for item_value_frameworks_0 in value.frameworks:
        writer.write_string(item_value_frameworks_0)
    writer.write_unsigned(len(value.runtime_library_paths))
    for item_value_runtime_library_paths_0 in value.runtime_library_paths:
        writer.write_string(item_value_runtime_library_paths_0)
    encode_symbol_visibility(writer, value.symbol_visibility)
    if value.version_script is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.version_script)
    if value.linker_script is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.linker_script)
    encode_position_independent_mode(writer, value.position_independent)
    if value.soname is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.soname)
    if value.install_name is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.install_name)


def decode_target_link_options(reader: BinaryReader) -> TargetLinkOptions:
    """Decode one TargetLinkOptions."""
    linker = reader.read_option(lambda: reader.read_string())
    args = [reader.read_string() for _ in range(reader.read_number())]
    library_paths = [reader.read_string() for _ in range(reader.read_number())]
    libraries = [reader.read_string() for _ in range(reader.read_number())]
    framework_paths = [reader.read_string() for _ in range(reader.read_number())]
    frameworks = [reader.read_string() for _ in range(reader.read_number())]
    runtime_library_paths = [reader.read_string() for _ in range(reader.read_number())]
    symbol_visibility = decode_symbol_visibility(reader)
    version_script = reader.read_option(lambda: reader.read_string())
    linker_script = reader.read_option(lambda: reader.read_string())
    position_independent = decode_position_independent_mode(reader)
    soname = reader.read_option(lambda: reader.read_string())
    install_name = reader.read_option(lambda: reader.read_string())

    return TargetLinkOptions(
        linker=linker,
        args=args,
        library_paths=library_paths,
        libraries=libraries,
        framework_paths=framework_paths,
        frameworks=frameworks,
        runtime_library_paths=runtime_library_paths,
        symbol_visibility=symbol_visibility,
        version_script=version_script,
        linker_script=linker_script,
        position_independent=position_independent,
        soname=soname,
        install_name=install_name,
    )


def to_json_target_link_options(value: TargetLinkOptions) -> Json:
    """Return one JSON value for one TargetLinkOptions."""
    return {
        **({} if value.linker is None else {"linker": value.linker}),
        "args": [item_0 for item_0 in value.args],
        "libraryPaths": [item_0 for item_0 in value.library_paths],
        "libraries": [item_0 for item_0 in value.libraries],
        "frameworkPaths": [item_0 for item_0 in value.framework_paths],
        "frameworks": [item_0 for item_0 in value.frameworks],
        "runtimeLibraryPaths": [item_0 for item_0 in value.runtime_library_paths],
        "symbolVisibility": to_json_symbol_visibility(value.symbol_visibility),
        **(
            {}
            if value.version_script is None
            else {"versionScript": value.version_script}
        ),
        **(
            {} if value.linker_script is None else {"linkerScript": value.linker_script}
        ),
        "positionIndependent": to_json_position_independent_mode(
            value.position_independent
        ),
        **({} if value.soname is None else {"soname": value.soname}),
        **({} if value.install_name is None else {"installName": value.install_name}),
    }


def from_json_target_link_options(value: Json) -> TargetLinkOptions:
    """Return one TargetLinkOptions from one JSON value."""
    object_ = json_object(value)

    return TargetLinkOptions(
        linker=json_optional(object_, "linker", lambda value: json_string(value)),
        args=[
            json_string(item_0) for item_0 in json_array(json_field(object_, "args"))
        ],
        library_paths=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "libraryPaths"))
        ],
        libraries=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "libraries"))
        ],
        framework_paths=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "frameworkPaths"))
        ],
        frameworks=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "frameworks"))
        ],
        runtime_library_paths=[
            json_string(item_0)
            for item_0 in json_array(json_field(object_, "runtimeLibraryPaths"))
        ],
        symbol_visibility=from_json_symbol_visibility(
            json_field(object_, "symbolVisibility")
        ),
        version_script=json_optional(
            object_, "versionScript", lambda value: json_string(value)
        ),
        linker_script=json_optional(
            object_, "linkerScript", lambda value: json_string(value)
        ),
        position_independent=from_json_position_independent_mode(
            json_field(object_, "positionIndependent")
        ),
        soname=json_optional(object_, "soname", lambda value: json_string(value)),
        install_name=json_optional(
            object_, "installName", lambda value: json_string(value)
        ),
    )


"""Symbol visibility policy for one native target."""
SymbolVisibility: typing.TypeAlias = (
    typing.Literal["default"] | typing.Literal["hidden"] | typing.Literal["protected"]
)


def encode_symbol_visibility(writer: BinaryWriter, value: SymbolVisibility) -> None:
    """Encode one SymbolVisibility."""
    if value == "default":
        writer.write_unsigned(0)
    elif value == "hidden":
        writer.write_unsigned(1)
    elif value == "protected":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_symbol_visibility(reader: BinaryReader) -> SymbolVisibility:
    """Decode one SymbolVisibility."""
    variant = reader.read_number()

    if variant == 0:
        return "default"
    elif variant == 1:
        return "hidden"
    elif variant == 2:
        return "protected"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_symbol_visibility(value: SymbolVisibility) -> Json:
    """Return one JSON value for one SymbolVisibility."""
    return value


def from_json_symbol_visibility(value: Json) -> SymbolVisibility:
    """Return one SymbolVisibility from one JSON value."""
    variant = json_string(value)

    if variant == "default":
        return "default"
    elif variant == "hidden":
        return "hidden"
    elif variant == "protected":
        return "protected"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Position independent code policy for one native target."""
PositionIndependentMode: typing.TypeAlias = (
    typing.Literal["default"]
    | typing.Literal["disabled"]
    | typing.Literal["pie"]
    | typing.Literal["staticPie"]
)


def encode_position_independent_mode(
    writer: BinaryWriter, value: PositionIndependentMode
) -> None:
    """Encode one PositionIndependentMode."""
    if value == "default":
        writer.write_unsigned(0)
    elif value == "disabled":
        writer.write_unsigned(1)
    elif value == "pie":
        writer.write_unsigned(2)
    elif value == "staticPie":
        writer.write_unsigned(3)
    else:
        raise SerdeError("unknown enum variant")


def decode_position_independent_mode(reader: BinaryReader) -> PositionIndependentMode:
    """Decode one PositionIndependentMode."""
    variant = reader.read_number()

    if variant == 0:
        return "default"
    elif variant == 1:
        return "disabled"
    elif variant == 2:
        return "pie"
    elif variant == 3:
        return "staticPie"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_position_independent_mode(value: PositionIndependentMode) -> Json:
    """Return one JSON value for one PositionIndependentMode."""
    return value


def from_json_position_independent_mode(value: Json) -> PositionIndependentMode:
    """Return one PositionIndependentMode from one JSON value."""
    variant = json_string(value)

    if variant == "default":
        return "default"
    elif variant == "disabled":
        return "disabled"
    elif variant == "pie":
        return "pie"
    elif variant == "staticPie":
        return "staticPie"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


__all__ = [
    "TargetNativeOptions",
    "encode_target_native_options",
    "decode_target_native_options",
    "to_json_target_native_options",
    "from_json_target_native_options",
    "NativeOutputKind",
    "encode_native_output_kind",
    "decode_native_output_kind",
    "to_json_native_output_kind",
    "from_json_native_output_kind",
    "CrtLinkage",
    "encode_crt_linkage",
    "decode_crt_linkage",
    "to_json_crt_linkage",
    "from_json_crt_linkage",
    "TargetLinkOptions",
    "encode_target_link_options",
    "decode_target_link_options",
    "to_json_target_link_options",
    "from_json_target_link_options",
    "SymbolVisibility",
    "encode_symbol_visibility",
    "decode_symbol_visibility",
    "to_json_symbol_visibility",
    "from_json_symbol_visibility",
    "PositionIndependentMode",
    "encode_position_independent_mode",
    "decode_position_independent_mode",
    "to_json_position_independent_mode",
    "from_json_position_independent_mode",
]
