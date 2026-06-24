# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

@dataclass(frozen=True, slots=True)
class RuntimeDiagnosticOptions:
    """Runtime diagnostics configuration."""

    # minimum diagnostic level recorded by the runtime
    level: RuntimeDiagnosticLevel
    # maximum number of diagnostic entries retained in the runtime ring buffer
    capacity: int | None

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> RuntimeDiagnosticOptions: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> RuntimeDiagnosticOptions: ...

def encode_runtime_diagnostic_options(
    writer: BinaryWriter, value: RuntimeDiagnosticOptions
) -> None: ...
def decode_runtime_diagnostic_options(
    reader: BinaryReader,
) -> RuntimeDiagnosticOptions: ...
def to_json_runtime_diagnostic_options(value: RuntimeDiagnosticOptions) -> Json: ...
def from_json_runtime_diagnostic_options(value: Json) -> RuntimeDiagnosticOptions: ...

"""Runtime diagnostic verbosity."""
RuntimeDiagnosticLevel: typing.TypeAlias = (
    typing.Literal["off"]
    | typing.Literal["error"]
    | typing.Literal["warn"]
    | typing.Literal["info"]
    | typing.Literal["debug"]
    | typing.Literal["trace"]
)

def encode_runtime_diagnostic_level(
    writer: BinaryWriter, value: RuntimeDiagnosticLevel
) -> None: ...
def decode_runtime_diagnostic_level(reader: BinaryReader) -> RuntimeDiagnosticLevel: ...
def to_json_runtime_diagnostic_level(value: RuntimeDiagnosticLevel) -> Json: ...
def from_json_runtime_diagnostic_level(value: Json) -> RuntimeDiagnosticLevel: ...

__all__ = [
    "RuntimeDiagnosticOptions",
    "encode_runtime_diagnostic_options",
    "decode_runtime_diagnostic_options",
    "to_json_runtime_diagnostic_options",
    "from_json_runtime_diagnostic_options",
    "RuntimeDiagnosticLevel",
    "encode_runtime_diagnostic_level",
    "decode_runtime_diagnostic_level",
    "to_json_runtime_diagnostic_level",
    "from_json_runtime_diagnostic_level",
]
