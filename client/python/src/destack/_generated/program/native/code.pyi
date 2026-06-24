# generated client target, do not edit

from __future__ import annotations

from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.program.native.abi
import destack._generated.program.native.entry
import destack._generated.program.native.image
import destack._generated.program.native.import_
import destack._generated.program.native.map

@dataclass(frozen=True, slots=True)
class Code:
    """Durable native code produced for one program."""

    # the native ABI required by this code
    abi: destack._generated.program.native.abi.Abi
    # the native image kind
    image: destack._generated.program.native.image.Image
    # native imports required by this code
    import_: destack._generated.program.native.import_.ImportTable
    # native code map for safepoints and deoptimization
    map: destack._generated.program.native.map.CodeMap
    # native entries keyed by program ids
    entry: destack._generated.program.native.entry.EntryTable

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
