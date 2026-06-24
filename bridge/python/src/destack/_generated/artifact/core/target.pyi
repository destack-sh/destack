# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

"""Build distribution profile."""
BuildProfile: typing.TypeAlias = (
    typing.Literal["full"] | typing.Literal["minimal"] | typing.Literal["freestanding"]
)

def encode_build_profile(writer: BinaryWriter, value: BuildProfile) -> None: ...
def decode_build_profile(reader: BinaryReader) -> BuildProfile: ...
def to_json_build_profile(value: BuildProfile) -> Json: ...
def from_json_build_profile(value: Json) -> BuildProfile: ...

"""Build payload linkage."""
BuildLinkage: typing.TypeAlias = (
    typing.Literal["portable"] | typing.Literal["static"] | typing.Literal["dynamic"]
)

def encode_build_linkage(writer: BinaryWriter, value: BuildLinkage) -> None: ...
def decode_build_linkage(reader: BinaryReader) -> BuildLinkage: ...
def to_json_build_linkage(value: BuildLinkage) -> Json: ...
def from_json_build_linkage(value: Json) -> BuildLinkage: ...

"""Semantic runtime contract for compiled code."""
Runtime: typing.TypeAlias = typing.Literal["destack"] | typing.Literal["js"]

def encode_runtime(writer: BinaryWriter, value: Runtime) -> None: ...
def decode_runtime(reader: BinaryReader) -> Runtime: ...
def to_json_runtime(value: Runtime) -> Json: ...
def from_json_runtime(value: Json) -> Runtime: ...

"""Host environment that provides target imports and ambient effects."""
Host: typing.TypeAlias = (
    typing.Literal["native"]
    | typing.Literal["browser"]
    | typing.Literal["wasi"]
    | typing.Literal["emscripten"]
    | typing.Literal["freestanding"]
)

def encode_host(writer: BinaryWriter, value: Host) -> None: ...
def decode_host(reader: BinaryReader) -> Host: ...
def to_json_host(value: Host) -> Json: ...
def from_json_host(value: Json) -> Host: ...

"""Operating system component of a target."""
Platform: typing.TypeAlias = (
    typing.Literal["unknown"]
    | typing.Literal["windows"]
    | typing.Literal["macOS"]
    | typing.Literal["linux"]
    | typing.Literal["freeBsd"]
    | typing.Literal["openBsd"]
    | typing.Literal["netBsd"]
    | typing.Literal["dragonFly"]
    | typing.Literal["solaris"]
    | typing.Literal["illumos"]
    | typing.Literal["haiku"]
    | typing.Literal["fuchsia"]
    | typing.Literal["redox"]
    | typing.Literal["hermit"]
    | typing.Literal["none"]
)

def encode_platform(writer: BinaryWriter, value: Platform) -> None: ...
def decode_platform(reader: BinaryReader) -> Platform: ...
def to_json_platform(value: Platform) -> Json: ...
def from_json_platform(value: Json) -> Platform: ...

@dataclass(frozen=True, slots=True)
class TargetArchX8664:
    """x86_64."""

    kind: typing.Literal["x8664"] = "x8664"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchX86:
    """x86."""

    kind: typing.Literal["x86"] = "x86"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchAarch64:
    """AArch64."""

    kind: typing.Literal["aarch64"] = "aarch64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchArmv7:
    """ARMv7."""

    kind: typing.Literal["armv7"] = "armv7"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchArmv6:
    """ARMv6."""

    kind: typing.Literal["armv6"] = "armv6"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchRiscv64:
    """RISC-V 64-bit."""

    kind: typing.Literal["riscv64"] = "riscv64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchRiscv32:
    """RISC-V 32-bit."""

    kind: typing.Literal["riscv32"] = "riscv32"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchPowerPc64:
    """PowerPC 64-bit."""

    kind: typing.Literal["powerPc64"] = "powerPc64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchPowerPc64le:
    """PowerPC 64-bit little-endian."""

    kind: typing.Literal["powerPc64le"] = "powerPc64le"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchS390x:
    """s390x."""

    kind: typing.Literal["s390x"] = "s390x"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchMips64:
    """MIPS64."""

    kind: typing.Literal["mips64"] = "mips64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchMips64el:
    """MIPS64 little-endian."""

    kind: typing.Literal["mips64el"] = "mips64el"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchLoongArch64:
    """LoongArch64."""

    kind: typing.Literal["loongArch64"] = "loongArch64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchWasm32:
    """WebAssembly 32-bit."""

    kind: typing.Literal["wasm32"] = "wasm32"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchWasm64:
    """WebAssembly 64-bit."""

    kind: typing.Literal["wasm64"] = "wasm64"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetArchOther:
    """Other architecture."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""CPU architecture for native targets."""
TargetArch: typing.TypeAlias = (
    TargetArchX8664
    | TargetArchX86
    | TargetArchAarch64
    | TargetArchArmv7
    | TargetArchArmv6
    | TargetArchRiscv64
    | TargetArchRiscv32
    | TargetArchPowerPc64
    | TargetArchPowerPc64le
    | TargetArchS390x
    | TargetArchMips64
    | TargetArchMips64el
    | TargetArchLoongArch64
    | TargetArchWasm32
    | TargetArchWasm64
    | TargetArchOther
)

def encode_target_arch(writer: BinaryWriter, value: TargetArch) -> None: ...
def decode_target_arch(reader: BinaryReader) -> TargetArch: ...
def to_json_target_arch(value: TargetArch) -> Json: ...
def from_json_target_arch(value: Json) -> TargetArch: ...

@dataclass(frozen=True, slots=True)
class TargetVendorUnknown:
    """Unknown vendor."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetVendorApple:
    """Apple."""

    kind: typing.Literal["apple"] = "apple"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetVendorPc:
    """PC."""

    kind: typing.Literal["pc"] = "pc"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetVendorIbm:
    """IBM."""

    kind: typing.Literal["ibm"] = "ibm"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetVendorNintendo:
    """Nintendo."""

    kind: typing.Literal["nintendo"] = "nintendo"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetVendorOther:
    """Other vendor."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target vendor for native targets."""
TargetVendor: typing.TypeAlias = (
    TargetVendorUnknown
    | TargetVendorApple
    | TargetVendorPc
    | TargetVendorIbm
    | TargetVendorNintendo
    | TargetVendorOther
)

def encode_target_vendor(writer: BinaryWriter, value: TargetVendor) -> None: ...
def decode_target_vendor(reader: BinaryReader) -> TargetVendor: ...
def to_json_target_vendor(value: TargetVendor) -> Json: ...
def from_json_target_vendor(value: Json) -> TargetVendor: ...

@dataclass(frozen=True, slots=True)
class TargetAbiGnu:
    """GNU environment."""

    kind: typing.Literal["gnu"] = "gnu"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiMusl:
    """Musl environment."""

    kind: typing.Literal["musl"] = "musl"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiMsvc:
    """MSVC environment."""

    kind: typing.Literal["msvc"] = "msvc"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiGnuLlvm:
    """GNU + LLVM environment."""

    kind: typing.Literal["gnuLlvm"] = "gnuLlvm"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiEabi:
    """EABI."""

    kind: typing.Literal["eabi"] = "eabi"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiEabihf:
    """EABI with hard-float."""

    kind: typing.Literal["eabihf"] = "eabihf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiMuslEabi:
    """Musl + EABI."""

    kind: typing.Literal["muslEabi"] = "muslEabi"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiMuslEabihf:
    """Musl + EABI hard-float."""

    kind: typing.Literal["muslEabihf"] = "muslEabihf"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class TargetAbiOther:
    """Other environment."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""Target ABI flavor."""
TargetAbi: typing.TypeAlias = (
    TargetAbiGnu
    | TargetAbiMusl
    | TargetAbiMsvc
    | TargetAbiGnuLlvm
    | TargetAbiEabi
    | TargetAbiEabihf
    | TargetAbiMuslEabi
    | TargetAbiMuslEabihf
    | TargetAbiOther
)

def encode_target_abi(writer: BinaryWriter, value: TargetAbi) -> None: ...
def decode_target_abi(reader: BinaryReader) -> TargetAbi: ...
def to_json_target_abi(value: TargetAbi) -> Json: ...
def from_json_target_abi(value: Json) -> TargetAbi: ...

__all__ = [
    "BuildProfile",
    "encode_build_profile",
    "decode_build_profile",
    "to_json_build_profile",
    "from_json_build_profile",
    "BuildLinkage",
    "encode_build_linkage",
    "decode_build_linkage",
    "to_json_build_linkage",
    "from_json_build_linkage",
    "Runtime",
    "encode_runtime",
    "decode_runtime",
    "to_json_runtime",
    "from_json_runtime",
    "Host",
    "encode_host",
    "decode_host",
    "to_json_host",
    "from_json_host",
    "Platform",
    "encode_platform",
    "decode_platform",
    "to_json_platform",
    "from_json_platform",
    "TargetArch",
    "encode_target_arch",
    "decode_target_arch",
    "to_json_target_arch",
    "from_json_target_arch",
    "TargetArchX8664",
    "TargetArchX86",
    "TargetArchAarch64",
    "TargetArchArmv7",
    "TargetArchArmv6",
    "TargetArchRiscv64",
    "TargetArchRiscv32",
    "TargetArchPowerPc64",
    "TargetArchPowerPc64le",
    "TargetArchS390x",
    "TargetArchMips64",
    "TargetArchMips64el",
    "TargetArchLoongArch64",
    "TargetArchWasm32",
    "TargetArchWasm64",
    "TargetArchOther",
    "TargetVendor",
    "encode_target_vendor",
    "decode_target_vendor",
    "to_json_target_vendor",
    "from_json_target_vendor",
    "TargetVendorUnknown",
    "TargetVendorApple",
    "TargetVendorPc",
    "TargetVendorIbm",
    "TargetVendorNintendo",
    "TargetVendorOther",
    "TargetAbi",
    "encode_target_abi",
    "decode_target_abi",
    "to_json_target_abi",
    "from_json_target_abi",
    "TargetAbiGnu",
    "TargetAbiMusl",
    "TargetAbiMsvc",
    "TargetAbiGnuLlvm",
    "TargetAbiEabi",
    "TargetAbiEabihf",
    "TargetAbiMuslEabi",
    "TargetAbiMuslEabihf",
    "TargetAbiOther",
]
