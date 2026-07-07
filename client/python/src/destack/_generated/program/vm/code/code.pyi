# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.vm.code.function
import destack._generated.program.vm.code.state
import destack._generated.program.vm.code.table

@dataclass(frozen=True, slots=True)
class Code:
    """Durable VM code body."""

    # lowered function bodies for the VM backend
    functions: destack._generated.program.vm.code.function.FunctionTable
    # side table referenced by compact side records
    side_table: destack._generated.program.vm.code.table.SideTable
    # resume states keyed by lowered VM program point
    resume: destack._generated.program.vm.code.state.ResumeTable

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Code: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Code: ...

def encode_code(writer: BinaryWriter, value: Code) -> None: ...
def decode_code(reader: BinaryReader) -> Code: ...
def to_json_code(value: Code) -> Json: ...
def from_json_code(value: Json) -> Code: ...

__all__ = [
    "Code",
    "encode_code",
    "decode_code",
    "to_json_code",
    "from_json_code",
]
