# generated bridge target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_field,
    json_object,
    json_string,
)

"""Build distribution profile."""
BuildProfile: typing.TypeAlias = (
    typing.Literal["full"] | typing.Literal["minimal"] | typing.Literal["freestanding"]
)


def encode_build_profile(writer: BinaryWriter, value: BuildProfile) -> None:
    """Encode one BuildProfile."""
    if value == "full":
        writer.write_unsigned(0)
    elif value == "minimal":
        writer.write_unsigned(1)
    elif value == "freestanding":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_profile(reader: BinaryReader) -> BuildProfile:
    """Decode one BuildProfile."""
    variant = reader.read_number()

    if variant == 0:
        return "full"
    elif variant == 1:
        return "minimal"
    elif variant == 2:
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_build_profile(value: BuildProfile) -> Json:
    """Return one JSON value for one BuildProfile."""
    return value


def from_json_build_profile(value: Json) -> BuildProfile:
    """Return one BuildProfile from one JSON value."""
    variant = json_string(value)

    if variant == "full":
        return "full"
    elif variant == "minimal":
        return "minimal"
    elif variant == "freestanding":
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Build payload linkage."""
BuildLinkage: typing.TypeAlias = (
    typing.Literal["portable"] | typing.Literal["static"] | typing.Literal["dynamic"]
)


def encode_build_linkage(writer: BinaryWriter, value: BuildLinkage) -> None:
    """Encode one BuildLinkage."""
    if value == "portable":
        writer.write_unsigned(0)
    elif value == "static":
        writer.write_unsigned(1)
    elif value == "dynamic":
        writer.write_unsigned(2)
    else:
        raise SerdeError("unknown enum variant")


def decode_build_linkage(reader: BinaryReader) -> BuildLinkage:
    """Decode one BuildLinkage."""
    variant = reader.read_number()

    if variant == 0:
        return "portable"
    elif variant == 1:
        return "static"
    elif variant == 2:
        return "dynamic"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_build_linkage(value: BuildLinkage) -> Json:
    """Return one JSON value for one BuildLinkage."""
    return value


def from_json_build_linkage(value: Json) -> BuildLinkage:
    """Return one BuildLinkage from one JSON value."""
    variant = json_string(value)

    if variant == "portable":
        return "portable"
    elif variant == "static":
        return "static"
    elif variant == "dynamic":
        return "dynamic"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Semantic runtime contract for compiled code."""
Runtime: typing.TypeAlias = typing.Literal["destack"] | typing.Literal["js"]


def encode_runtime(writer: BinaryWriter, value: Runtime) -> None:
    """Encode one Runtime."""
    if value == "destack":
        writer.write_unsigned(0)
    elif value == "js":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_runtime(reader: BinaryReader) -> Runtime:
    """Decode one Runtime."""
    variant = reader.read_number()

    if variant == 0:
        return "destack"
    elif variant == 1:
        return "js"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_runtime(value: Runtime) -> Json:
    """Return one JSON value for one Runtime."""
    return value


def from_json_runtime(value: Json) -> Runtime:
    """Return one Runtime from one JSON value."""
    variant = json_string(value)

    if variant == "destack":
        return "destack"
    elif variant == "js":
        return "js"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


"""Host environment that provides target imports and ambient effects."""
Host: typing.TypeAlias = (
    typing.Literal["native"]
    | typing.Literal["browser"]
    | typing.Literal["wasi"]
    | typing.Literal["emscripten"]
    | typing.Literal["freestanding"]
)


def encode_host(writer: BinaryWriter, value: Host) -> None:
    """Encode one Host."""
    if value == "native":
        writer.write_unsigned(0)
    elif value == "browser":
        writer.write_unsigned(1)
    elif value == "wasi":
        writer.write_unsigned(2)
    elif value == "emscripten":
        writer.write_unsigned(3)
    elif value == "freestanding":
        writer.write_unsigned(4)
    else:
        raise SerdeError("unknown enum variant")


def decode_host(reader: BinaryReader) -> Host:
    """Decode one Host."""
    variant = reader.read_number()

    if variant == 0:
        return "native"
    elif variant == 1:
        return "browser"
    elif variant == 2:
        return "wasi"
    elif variant == 3:
        return "emscripten"
    elif variant == 4:
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_host(value: Host) -> Json:
    """Return one JSON value for one Host."""
    return value


def from_json_host(value: Json) -> Host:
    """Return one Host from one JSON value."""
    variant = json_string(value)

    if variant == "native":
        return "native"
    elif variant == "browser":
        return "browser"
    elif variant == "wasi":
        return "wasi"
    elif variant == "emscripten":
        return "emscripten"
    elif variant == "freestanding":
        return "freestanding"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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


def encode_platform(writer: BinaryWriter, value: Platform) -> None:
    """Encode one Platform."""
    if value == "unknown":
        writer.write_unsigned(0)
    elif value == "windows":
        writer.write_unsigned(1)
    elif value == "macOS":
        writer.write_unsigned(2)
    elif value == "linux":
        writer.write_unsigned(3)
    elif value == "freeBsd":
        writer.write_unsigned(4)
    elif value == "openBsd":
        writer.write_unsigned(5)
    elif value == "netBsd":
        writer.write_unsigned(6)
    elif value == "dragonFly":
        writer.write_unsigned(7)
    elif value == "solaris":
        writer.write_unsigned(8)
    elif value == "illumos":
        writer.write_unsigned(9)
    elif value == "haiku":
        writer.write_unsigned(10)
    elif value == "fuchsia":
        writer.write_unsigned(11)
    elif value == "redox":
        writer.write_unsigned(12)
    elif value == "hermit":
        writer.write_unsigned(13)
    elif value == "none":
        writer.write_unsigned(14)
    else:
        raise SerdeError("unknown enum variant")


def decode_platform(reader: BinaryReader) -> Platform:
    """Decode one Platform."""
    variant = reader.read_number()

    if variant == 0:
        return "unknown"
    elif variant == 1:
        return "windows"
    elif variant == 2:
        return "macOS"
    elif variant == 3:
        return "linux"
    elif variant == 4:
        return "freeBsd"
    elif variant == 5:
        return "openBsd"
    elif variant == 6:
        return "netBsd"
    elif variant == 7:
        return "dragonFly"
    elif variant == 8:
        return "solaris"
    elif variant == 9:
        return "illumos"
    elif variant == 10:
        return "haiku"
    elif variant == 11:
        return "fuchsia"
    elif variant == 12:
        return "redox"
    elif variant == 13:
        return "hermit"
    elif variant == 14:
        return "none"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_platform(value: Platform) -> Json:
    """Return one JSON value for one Platform."""
    return value


def from_json_platform(value: Json) -> Platform:
    """Return one Platform from one JSON value."""
    variant = json_string(value)

    if variant == "unknown":
        return "unknown"
    elif variant == "windows":
        return "windows"
    elif variant == "macOS":
        return "macOS"
    elif variant == "linux":
        return "linux"
    elif variant == "freeBsd":
        return "freeBsd"
    elif variant == "openBsd":
        return "openBsd"
    elif variant == "netBsd":
        return "netBsd"
    elif variant == "dragonFly":
        return "dragonFly"
    elif variant == "solaris":
        return "solaris"
    elif variant == "illumos":
        return "illumos"
    elif variant == "haiku":
        return "haiku"
    elif variant == "fuchsia":
        return "fuchsia"
    elif variant == "redox":
        return "redox"
    elif variant == "hermit":
        return "hermit"
    elif variant == "none":
        return "none"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class TargetArchX8664:
    """x86_64."""

    kind: typing.Literal["x8664"] = "x8664"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchX86:
    """x86."""

    kind: typing.Literal["x86"] = "x86"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchAarch64:
    """AArch64."""

    kind: typing.Literal["aarch64"] = "aarch64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchArmv7:
    """ARMv7."""

    kind: typing.Literal["armv7"] = "armv7"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchArmv6:
    """ARMv6."""

    kind: typing.Literal["armv6"] = "armv6"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchRiscv64:
    """RISC-V 64-bit."""

    kind: typing.Literal["riscv64"] = "riscv64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchRiscv32:
    """RISC-V 32-bit."""

    kind: typing.Literal["riscv32"] = "riscv32"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchPowerPc64:
    """PowerPC 64-bit."""

    kind: typing.Literal["powerPc64"] = "powerPc64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchPowerPc64le:
    """PowerPC 64-bit little-endian."""

    kind: typing.Literal["powerPc64le"] = "powerPc64le"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchS390x:
    """s390x."""

    kind: typing.Literal["s390x"] = "s390x"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchMips64:
    """MIPS64."""

    kind: typing.Literal["mips64"] = "mips64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchMips64el:
    """MIPS64 little-endian."""

    kind: typing.Literal["mips64el"] = "mips64el"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchLoongArch64:
    """LoongArch64."""

    kind: typing.Literal["loongArch64"] = "loongArch64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchWasm32:
    """WebAssembly 32-bit."""

    kind: typing.Literal["wasm32"] = "wasm32"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchWasm64:
    """WebAssembly 64-bit."""

    kind: typing.Literal["wasm64"] = "wasm64"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


@dataclass(frozen=True, slots=True)
class TargetArchOther:
    """Other architecture."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_arch(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_arch(self)


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


def encode_target_arch(writer: BinaryWriter, value: TargetArch) -> None:
    """Encode one TargetArch."""
    if value.kind == "x8664":
        writer.write_unsigned(0)
    elif value.kind == "x86":
        writer.write_unsigned(1)
    elif value.kind == "aarch64":
        writer.write_unsigned(2)
    elif value.kind == "armv7":
        writer.write_unsigned(3)
    elif value.kind == "armv6":
        writer.write_unsigned(4)
    elif value.kind == "riscv64":
        writer.write_unsigned(5)
    elif value.kind == "riscv32":
        writer.write_unsigned(6)
    elif value.kind == "powerPc64":
        writer.write_unsigned(7)
    elif value.kind == "powerPc64le":
        writer.write_unsigned(8)
    elif value.kind == "s390x":
        writer.write_unsigned(9)
    elif value.kind == "mips64":
        writer.write_unsigned(10)
    elif value.kind == "mips64el":
        writer.write_unsigned(11)
    elif value.kind == "loongArch64":
        writer.write_unsigned(12)
    elif value.kind == "wasm32":
        writer.write_unsigned(13)
    elif value.kind == "wasm64":
        writer.write_unsigned(14)
    elif value.kind == "other":
        writer.write_unsigned(15)
        writer.write_string(value.other)
    else:
        raise SerdeError("unknown enum variant")


def decode_target_arch(reader: BinaryReader) -> TargetArch:
    """Decode one TargetArch."""
    variant = reader.read_number()

    if variant == 0:
        return TargetArchX8664()
    elif variant == 1:
        return TargetArchX86()
    elif variant == 2:
        return TargetArchAarch64()
    elif variant == 3:
        return TargetArchArmv7()
    elif variant == 4:
        return TargetArchArmv6()
    elif variant == 5:
        return TargetArchRiscv64()
    elif variant == 6:
        return TargetArchRiscv32()
    elif variant == 7:
        return TargetArchPowerPc64()
    elif variant == 8:
        return TargetArchPowerPc64le()
    elif variant == 9:
        return TargetArchS390x()
    elif variant == 10:
        return TargetArchMips64()
    elif variant == 11:
        return TargetArchMips64el()
    elif variant == 12:
        return TargetArchLoongArch64()
    elif variant == 13:
        return TargetArchWasm32()
    elif variant == 14:
        return TargetArchWasm64()
    elif variant == 15:
        other = reader.read_string()

        return TargetArchOther(other=other)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_target_arch(value: TargetArch) -> Json:
    """Return one JSON value for one TargetArch."""
    if value.kind == "x8664":
        return {
            "kind": "x8664",
        }
    elif value.kind == "x86":
        return {
            "kind": "x86",
        }
    elif value.kind == "aarch64":
        return {
            "kind": "aarch64",
        }
    elif value.kind == "armv7":
        return {
            "kind": "armv7",
        }
    elif value.kind == "armv6":
        return {
            "kind": "armv6",
        }
    elif value.kind == "riscv64":
        return {
            "kind": "riscv64",
        }
    elif value.kind == "riscv32":
        return {
            "kind": "riscv32",
        }
    elif value.kind == "powerPc64":
        return {
            "kind": "powerPc64",
        }
    elif value.kind == "powerPc64le":
        return {
            "kind": "powerPc64le",
        }
    elif value.kind == "s390x":
        return {
            "kind": "s390x",
        }
    elif value.kind == "mips64":
        return {
            "kind": "mips64",
        }
    elif value.kind == "mips64el":
        return {
            "kind": "mips64el",
        }
    elif value.kind == "loongArch64":
        return {
            "kind": "loongArch64",
        }
    elif value.kind == "wasm32":
        return {
            "kind": "wasm32",
        }
    elif value.kind == "wasm64":
        return {
            "kind": "wasm64",
        }
    elif value.kind == "other":
        return {
            "kind": "other",
            "other": value.other,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_target_arch(value: Json) -> TargetArch:
    """Return one TargetArch from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "x8664":
        return TargetArchX8664()
    elif kind == "x86":
        return TargetArchX86()
    elif kind == "aarch64":
        return TargetArchAarch64()
    elif kind == "armv7":
        return TargetArchArmv7()
    elif kind == "armv6":
        return TargetArchArmv6()
    elif kind == "riscv64":
        return TargetArchRiscv64()
    elif kind == "riscv32":
        return TargetArchRiscv32()
    elif kind == "powerPc64":
        return TargetArchPowerPc64()
    elif kind == "powerPc64le":
        return TargetArchPowerPc64le()
    elif kind == "s390x":
        return TargetArchS390x()
    elif kind == "mips64":
        return TargetArchMips64()
    elif kind == "mips64el":
        return TargetArchMips64el()
    elif kind == "loongArch64":
        return TargetArchLoongArch64()
    elif kind == "wasm32":
        return TargetArchWasm32()
    elif kind == "wasm64":
        return TargetArchWasm64()
    elif kind == "other":
        return TargetArchOther(other=json_string(json_field(object_, "other")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TargetVendorUnknown:
    """Unknown vendor."""

    kind: typing.Literal["unknown"] = "unknown"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


@dataclass(frozen=True, slots=True)
class TargetVendorApple:
    """Apple."""

    kind: typing.Literal["apple"] = "apple"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


@dataclass(frozen=True, slots=True)
class TargetVendorPc:
    """PC."""

    kind: typing.Literal["pc"] = "pc"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


@dataclass(frozen=True, slots=True)
class TargetVendorIbm:
    """IBM."""

    kind: typing.Literal["ibm"] = "ibm"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


@dataclass(frozen=True, slots=True)
class TargetVendorNintendo:
    """Nintendo."""

    kind: typing.Literal["nintendo"] = "nintendo"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


@dataclass(frozen=True, slots=True)
class TargetVendorOther:
    """Other vendor."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_vendor(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_vendor(self)


"""Target vendor for native targets."""
TargetVendor: typing.TypeAlias = (
    TargetVendorUnknown
    | TargetVendorApple
    | TargetVendorPc
    | TargetVendorIbm
    | TargetVendorNintendo
    | TargetVendorOther
)


def encode_target_vendor(writer: BinaryWriter, value: TargetVendor) -> None:
    """Encode one TargetVendor."""
    if value.kind == "unknown":
        writer.write_unsigned(0)
    elif value.kind == "apple":
        writer.write_unsigned(1)
    elif value.kind == "pc":
        writer.write_unsigned(2)
    elif value.kind == "ibm":
        writer.write_unsigned(3)
    elif value.kind == "nintendo":
        writer.write_unsigned(4)
    elif value.kind == "other":
        writer.write_unsigned(5)
        writer.write_string(value.other)
    else:
        raise SerdeError("unknown enum variant")


def decode_target_vendor(reader: BinaryReader) -> TargetVendor:
    """Decode one TargetVendor."""
    variant = reader.read_number()

    if variant == 0:
        return TargetVendorUnknown()
    elif variant == 1:
        return TargetVendorApple()
    elif variant == 2:
        return TargetVendorPc()
    elif variant == 3:
        return TargetVendorIbm()
    elif variant == 4:
        return TargetVendorNintendo()
    elif variant == 5:
        other = reader.read_string()

        return TargetVendorOther(other=other)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_target_vendor(value: TargetVendor) -> Json:
    """Return one JSON value for one TargetVendor."""
    if value.kind == "unknown":
        return {
            "kind": "unknown",
        }
    elif value.kind == "apple":
        return {
            "kind": "apple",
        }
    elif value.kind == "pc":
        return {
            "kind": "pc",
        }
    elif value.kind == "ibm":
        return {
            "kind": "ibm",
        }
    elif value.kind == "nintendo":
        return {
            "kind": "nintendo",
        }
    elif value.kind == "other":
        return {
            "kind": "other",
            "other": value.other,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_target_vendor(value: Json) -> TargetVendor:
    """Return one TargetVendor from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "unknown":
        return TargetVendorUnknown()
    elif kind == "apple":
        return TargetVendorApple()
    elif kind == "pc":
        return TargetVendorPc()
    elif kind == "ibm":
        return TargetVendorIbm()
    elif kind == "nintendo":
        return TargetVendorNintendo()
    elif kind == "other":
        return TargetVendorOther(other=json_string(json_field(object_, "other")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class TargetAbiGnu:
    """GNU environment."""

    kind: typing.Literal["gnu"] = "gnu"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiMusl:
    """Musl environment."""

    kind: typing.Literal["musl"] = "musl"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiMsvc:
    """MSVC environment."""

    kind: typing.Literal["msvc"] = "msvc"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiGnuLlvm:
    """GNU + LLVM environment."""

    kind: typing.Literal["gnuLlvm"] = "gnuLlvm"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiEabi:
    """EABI."""

    kind: typing.Literal["eabi"] = "eabi"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiEabihf:
    """EABI with hard-float."""

    kind: typing.Literal["eabihf"] = "eabihf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiMuslEabi:
    """Musl + EABI."""

    kind: typing.Literal["muslEabi"] = "muslEabi"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiMuslEabihf:
    """Musl + EABI hard-float."""

    kind: typing.Literal["muslEabihf"] = "muslEabihf"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


@dataclass(frozen=True, slots=True)
class TargetAbiOther:
    """Other environment."""

    other: str
    kind: typing.Literal["other"] = "other"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_target_abi(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_target_abi(self)


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


def encode_target_abi(writer: BinaryWriter, value: TargetAbi) -> None:
    """Encode one TargetAbi."""
    if value.kind == "gnu":
        writer.write_unsigned(0)
    elif value.kind == "musl":
        writer.write_unsigned(1)
    elif value.kind == "msvc":
        writer.write_unsigned(2)
    elif value.kind == "gnuLlvm":
        writer.write_unsigned(3)
    elif value.kind == "eabi":
        writer.write_unsigned(4)
    elif value.kind == "eabihf":
        writer.write_unsigned(5)
    elif value.kind == "muslEabi":
        writer.write_unsigned(6)
    elif value.kind == "muslEabihf":
        writer.write_unsigned(7)
    elif value.kind == "other":
        writer.write_unsigned(8)
        writer.write_string(value.other)
    else:
        raise SerdeError("unknown enum variant")


def decode_target_abi(reader: BinaryReader) -> TargetAbi:
    """Decode one TargetAbi."""
    variant = reader.read_number()

    if variant == 0:
        return TargetAbiGnu()
    elif variant == 1:
        return TargetAbiMusl()
    elif variant == 2:
        return TargetAbiMsvc()
    elif variant == 3:
        return TargetAbiGnuLlvm()
    elif variant == 4:
        return TargetAbiEabi()
    elif variant == 5:
        return TargetAbiEabihf()
    elif variant == 6:
        return TargetAbiMuslEabi()
    elif variant == 7:
        return TargetAbiMuslEabihf()
    elif variant == 8:
        other = reader.read_string()

        return TargetAbiOther(other=other)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_target_abi(value: TargetAbi) -> Json:
    """Return one JSON value for one TargetAbi."""
    if value.kind == "gnu":
        return {
            "kind": "gnu",
        }
    elif value.kind == "musl":
        return {
            "kind": "musl",
        }
    elif value.kind == "msvc":
        return {
            "kind": "msvc",
        }
    elif value.kind == "gnuLlvm":
        return {
            "kind": "gnuLlvm",
        }
    elif value.kind == "eabi":
        return {
            "kind": "eabi",
        }
    elif value.kind == "eabihf":
        return {
            "kind": "eabihf",
        }
    elif value.kind == "muslEabi":
        return {
            "kind": "muslEabi",
        }
    elif value.kind == "muslEabihf":
        return {
            "kind": "muslEabihf",
        }
    elif value.kind == "other":
        return {
            "kind": "other",
            "other": value.other,
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_target_abi(value: Json) -> TargetAbi:
    """Return one TargetAbi from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "gnu":
        return TargetAbiGnu()
    elif kind == "musl":
        return TargetAbiMusl()
    elif kind == "msvc":
        return TargetAbiMsvc()
    elif kind == "gnuLlvm":
        return TargetAbiGnuLlvm()
    elif kind == "eabi":
        return TargetAbiEabi()
    elif kind == "eabihf":
        return TargetAbiEabihf()
    elif kind == "muslEabi":
        return TargetAbiMuslEabi()
    elif kind == "muslEabihf":
        return TargetAbiMuslEabihf()
    elif kind == "other":
        return TargetAbiOther(other=json_string(json_field(object_, "other")))
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


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
