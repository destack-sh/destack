# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetNativeOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetNativeOptions: ...

def encode_target_native_options(
    writer: BinaryWriter, value: TargetNativeOptions
) -> None: ...
def decode_target_native_options(reader: BinaryReader) -> TargetNativeOptions: ...
def to_json_target_native_options(value: TargetNativeOptions) -> Json: ...
def from_json_target_native_options(value: Json) -> TargetNativeOptions: ...

"""Native output kind for one target."""
NativeOutputKind: typing.TypeAlias = (
    typing.Literal["executable"]
    | typing.Literal["staticLibrary"]
    | typing.Literal["sharedLibrary"]
)

def encode_native_output_kind(
    writer: BinaryWriter, value: NativeOutputKind
) -> None: ...
def decode_native_output_kind(reader: BinaryReader) -> NativeOutputKind: ...
def to_json_native_output_kind(value: NativeOutputKind) -> Json: ...
def from_json_native_output_kind(value: Json) -> NativeOutputKind: ...

"""C runtime linkage policy for one native target."""
CrtLinkage: typing.TypeAlias = (
    typing.Literal["default"] | typing.Literal["dynamic"] | typing.Literal["static"]
)

def encode_crt_linkage(writer: BinaryWriter, value: CrtLinkage) -> None: ...
def decode_crt_linkage(reader: BinaryReader) -> CrtLinkage: ...
def to_json_crt_linkage(value: CrtLinkage) -> Json: ...
def from_json_crt_linkage(value: Json) -> CrtLinkage: ...

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

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> TargetLinkOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> TargetLinkOptions: ...

def encode_target_link_options(
    writer: BinaryWriter, value: TargetLinkOptions
) -> None: ...
def decode_target_link_options(reader: BinaryReader) -> TargetLinkOptions: ...
def to_json_target_link_options(value: TargetLinkOptions) -> Json: ...
def from_json_target_link_options(value: Json) -> TargetLinkOptions: ...

"""Symbol visibility policy for one native target."""
SymbolVisibility: typing.TypeAlias = (
    typing.Literal["default"] | typing.Literal["hidden"] | typing.Literal["protected"]
)

def encode_symbol_visibility(writer: BinaryWriter, value: SymbolVisibility) -> None: ...
def decode_symbol_visibility(reader: BinaryReader) -> SymbolVisibility: ...
def to_json_symbol_visibility(value: SymbolVisibility) -> Json: ...
def from_json_symbol_visibility(value: Json) -> SymbolVisibility: ...

"""Position independent code policy for one native target."""
PositionIndependentMode: typing.TypeAlias = (
    typing.Literal["default"]
    | typing.Literal["disabled"]
    | typing.Literal["pie"]
    | typing.Literal["staticPie"]
)

def encode_position_independent_mode(
    writer: BinaryWriter, value: PositionIndependentMode
) -> None: ...
def decode_position_independent_mode(
    reader: BinaryReader,
) -> PositionIndependentMode: ...
def to_json_position_independent_mode(value: PositionIndependentMode) -> Json: ...
def from_json_position_independent_mode(value: Json) -> PositionIndependentMode: ...

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
