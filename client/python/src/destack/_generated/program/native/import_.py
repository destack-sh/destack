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
    json_field,
    json_object,
    json_string,
)


@dataclass(frozen=True, slots=True)
class ImportTable:
    """Native imports required by one native code payload."""

    # native imports in linker order
    import_: Sequence[Import]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import_table(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ImportTable:
        """Decode one ImportTable."""
        return decode_import_table(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import_table(self)

    @classmethod
    def from_json(cls, value: Json) -> ImportTable:
        """Return one ImportTable from one JSON value."""
        return from_json_import_table(value)


def encode_import_table(writer: BinaryWriter, value: ImportTable) -> None:
    """Encode one ImportTable."""
    writer.write_unsigned(len(value.import_))
    for item_value_import_0 in value.import_:
        encode_import(writer, item_value_import_0)


def decode_import_table(reader: BinaryReader) -> ImportTable:
    """Decode one ImportTable."""
    import_ = [decode_import(reader) for _ in range(reader.read_number())]

    return ImportTable(
        import_=import_,
    )


def to_json_import_table(value: ImportTable) -> Json:
    """Return one JSON value for one ImportTable."""
    return {
        "import": [to_json_import(item_0) for item_0 in value.import_],
    }


def from_json_import_table(value: Json) -> ImportTable:
    """Return one ImportTable from one JSON value."""
    object_ = json_object(value)

    return ImportTable(
        import_=[
            from_json_import(item_0)
            for item_0 in json_array(json_field(object_, "import"))
        ],
    )


@dataclass(frozen=True, slots=True)
class ImportRuntime:
    """Fixed Destack runtime binding."""

    runtime: RuntimeBinding
    kind: typing.Literal["runtime"] = "runtime"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import(self)


@dataclass(frozen=True, slots=True)
class ImportSymbol:
    """External linker-visible symbol."""

    symbol: SymbolImport
    kind: typing.Literal["symbol"] = "symbol"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_import(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_import(self)


"""One native import required by generated native code."""
Import: typing.TypeAlias = ImportRuntime | ImportSymbol


def encode_import(writer: BinaryWriter, value: Import) -> None:
    """Encode one Import."""
    if value.kind == "runtime":
        writer.write_unsigned(0)
        encode_runtime_binding(writer, value.runtime)
    elif value.kind == "symbol":
        writer.write_unsigned(1)
        encode_symbol_import(writer, value.symbol)
    else:
        raise SerdeError("unknown enum variant")


def decode_import(reader: BinaryReader) -> Import:
    """Decode one Import."""
    variant = reader.read_number()

    if variant == 0:
        runtime = decode_runtime_binding(reader)

        return ImportRuntime(runtime=runtime)
    elif variant == 1:
        symbol = decode_symbol_import(reader)

        return ImportSymbol(symbol=symbol)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_import(value: Import) -> Json:
    """Return one JSON value for one Import."""
    if value.kind == "runtime":
        return {
            "kind": "runtime",
            "runtime": to_json_runtime_binding(value.runtime),
        }
    elif value.kind == "symbol":
        return {
            "kind": "symbol",
            "symbol": to_json_symbol_import(value.symbol),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_import(value: Json) -> Import:
    """Return one Import from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "runtime":
        return ImportRuntime(
            runtime=from_json_runtime_binding(json_field(object_, "runtime"))
        )
    elif kind == "symbol":
        return ImportSymbol(
            symbol=from_json_symbol_import(json_field(object_, "symbol"))
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


"""Fixed Destack runtime ABI binding imported by generated native code."""
RuntimeBinding: typing.TypeAlias = (
    typing.Literal["new"]
    | typing.Literal["newSlice"]
    | typing.Literal["free"]
    | typing.Literal["pin"]
    | typing.Literal["unpin"]
    | typing.Literal["writeBarrier"]
    | typing.Literal["safepoint"]
    | typing.Literal["yield"]
    | typing.Literal["deopt"]
    | typing.Literal["trap"]
    | typing.Literal["panic"]
    | typing.Literal["unwindResume"]
    | typing.Literal["contextCurrent"]
    | typing.Literal["contextPush"]
    | typing.Literal["contextPop"]
    | typing.Literal["contextGet"]
    | typing.Literal["contextRequire"]
    | typing.Literal["contextFamily"]
    | typing.Literal["bindingCall"]
)


def encode_runtime_binding(writer: BinaryWriter, value: RuntimeBinding) -> None:
    """Encode one RuntimeBinding."""
    if value == "new":
        writer.write_unsigned(0)
    elif value == "newSlice":
        writer.write_unsigned(1)
    elif value == "free":
        writer.write_unsigned(2)
    elif value == "pin":
        writer.write_unsigned(3)
    elif value == "unpin":
        writer.write_unsigned(4)
    elif value == "writeBarrier":
        writer.write_unsigned(5)
    elif value == "safepoint":
        writer.write_unsigned(6)
    elif value == "yield":
        writer.write_unsigned(7)
    elif value == "deopt":
        writer.write_unsigned(8)
    elif value == "trap":
        writer.write_unsigned(9)
    elif value == "panic":
        writer.write_unsigned(10)
    elif value == "unwindResume":
        writer.write_unsigned(11)
    elif value == "contextCurrent":
        writer.write_unsigned(12)
    elif value == "contextPush":
        writer.write_unsigned(13)
    elif value == "contextPop":
        writer.write_unsigned(14)
    elif value == "contextGet":
        writer.write_unsigned(15)
    elif value == "contextRequire":
        writer.write_unsigned(16)
    elif value == "contextFamily":
        writer.write_unsigned(17)
    elif value == "bindingCall":
        writer.write_unsigned(18)
    else:
        raise SerdeError("unknown enum variant")


def decode_runtime_binding(reader: BinaryReader) -> RuntimeBinding:
    """Decode one RuntimeBinding."""
    variant = reader.read_number()

    if variant == 0:
        return "new"
    elif variant == 1:
        return "newSlice"
    elif variant == 2:
        return "free"
    elif variant == 3:
        return "pin"
    elif variant == 4:
        return "unpin"
    elif variant == 5:
        return "writeBarrier"
    elif variant == 6:
        return "safepoint"
    elif variant == 7:
        return "yield"
    elif variant == 8:
        return "deopt"
    elif variant == 9:
        return "trap"
    elif variant == 10:
        return "panic"
    elif variant == 11:
        return "unwindResume"
    elif variant == 12:
        return "contextCurrent"
    elif variant == 13:
        return "contextPush"
    elif variant == 14:
        return "contextPop"
    elif variant == 15:
        return "contextGet"
    elif variant == 16:
        return "contextRequire"
    elif variant == 17:
        return "contextFamily"
    elif variant == 18:
        return "bindingCall"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_runtime_binding(value: RuntimeBinding) -> Json:
    """Return one JSON value for one RuntimeBinding."""
    return value


def from_json_runtime_binding(value: Json) -> RuntimeBinding:
    """Return one RuntimeBinding from one JSON value."""
    variant = json_string(value)

    if variant == "new":
        return "new"
    elif variant == "newSlice":
        return "newSlice"
    elif variant == "free":
        return "free"
    elif variant == "pin":
        return "pin"
    elif variant == "unpin":
        return "unpin"
    elif variant == "writeBarrier":
        return "writeBarrier"
    elif variant == "safepoint":
        return "safepoint"
    elif variant == "yield":
        return "yield"
    elif variant == "deopt":
        return "deopt"
    elif variant == "trap":
        return "trap"
    elif variant == "panic":
        return "panic"
    elif variant == "unwindResume":
        return "unwindResume"
    elif variant == "contextCurrent":
        return "contextCurrent"
    elif variant == "contextPush":
        return "contextPush"
    elif variant == "contextPop":
        return "contextPop"
    elif variant == "contextGet":
        return "contextGet"
    elif variant == "contextRequire":
        return "contextRequire"
    elif variant == "contextFamily":
        return "contextFamily"
    elif variant == "bindingCall":
        return "bindingCall"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


@dataclass(frozen=True, slots=True)
class SymbolImport:
    """External linker-visible symbol import."""

    # the imported native symbol
    symbol: str

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_symbol_import(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> SymbolImport:
        """Decode one SymbolImport."""
        return decode_symbol_import(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_symbol_import(self)

    @classmethod
    def from_json(cls, value: Json) -> SymbolImport:
        """Return one SymbolImport from one JSON value."""
        return from_json_symbol_import(value)


def encode_symbol_import(writer: BinaryWriter, value: SymbolImport) -> None:
    """Encode one SymbolImport."""
    writer.write_string(value.symbol)


def decode_symbol_import(reader: BinaryReader) -> SymbolImport:
    """Decode one SymbolImport."""
    symbol = reader.read_string()

    return SymbolImport(
        symbol=symbol,
    )


def to_json_symbol_import(value: SymbolImport) -> Json:
    """Return one JSON value for one SymbolImport."""
    return {
        "symbol": value.symbol,
    }


def from_json_symbol_import(value: Json) -> SymbolImport:
    """Return one SymbolImport from one JSON value."""
    object_ = json_object(value)

    return SymbolImport(
        symbol=json_string(json_field(object_, "symbol")),
    )


__all__ = [
    "ImportTable",
    "encode_import_table",
    "decode_import_table",
    "to_json_import_table",
    "from_json_import_table",
    "Import",
    "encode_import",
    "decode_import",
    "to_json_import",
    "from_json_import",
    "ImportRuntime",
    "ImportSymbol",
    "RuntimeBinding",
    "encode_runtime_binding",
    "decode_runtime_binding",
    "to_json_runtime_binding",
    "from_json_runtime_binding",
    "SymbolImport",
    "encode_symbol_import",
    "decode_symbol_import",
    "to_json_symbol_import",
    "from_json_symbol_import",
]
