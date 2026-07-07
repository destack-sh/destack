# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.string
import destack._generated.mir.table.target
import destack._generated.program.native.code.entry
import destack._generated.program.native.code.image
import destack._generated.program.native.code.import_
import destack._generated.program.native.code.map

@dataclass(frozen=True, slots=True)
class Code:
    """Durable native code produced for one program."""

    # destack native ABI version required by this code
    abi_version: int
    # target triple or equivalent target identity
    target: destack._generated.core.string.StringId
    # target ABI layout expected by this code
    target_layout: destack._generated.mir.table.target.TargetLayout
    # the native image
    image: destack._generated.program.native.code.image.Image
    # native imports required by this code
    imports: destack._generated.program.native.code.import_.ImportTable
    # native code map for safepoints and deoptimization
    map: destack._generated.program.native.code.map.CodeMap
    # native entries keyed by program ids
    entries: destack._generated.program.native.code.entry.EntryTable

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
