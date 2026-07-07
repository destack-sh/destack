# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.dir.index.call
import destack._generated.dir.index.decorator
import destack._generated.dir.index.export
import destack._generated.dir.index.extension
import destack._generated.dir.index.heritage
import destack._generated.dir.index.member
import destack._generated.dir.index.reference
import destack._generated.dir.index.specifier
import destack._generated.dir.index.symbol
import destack._generated.source.file.model.module

"""One observable projection of a module index artifact."""
ModuleIndexProjection: typing.TypeAlias = (
    typing.Literal["symbols"]
    | typing.Literal["exports"]
    | typing.Literal["members"]
    | typing.Literal["references"]
    | typing.Literal["calls"]
    | typing.Literal["heritage"]
    | typing.Literal["extensions"]
    | typing.Literal["specifiers"]
    | typing.Literal["decorators"]
)

def encode_module_index_projection(
    writer: BinaryWriter, value: ModuleIndexProjection
) -> None: ...
def decode_module_index_projection(reader: BinaryReader) -> ModuleIndexProjection: ...
def to_json_module_index_projection(value: ModuleIndexProjection) -> Json: ...
def from_json_module_index_projection(value: Json) -> ModuleIndexProjection: ...

@dataclass(frozen=True, slots=True)
class ModuleIndex:
    """Indexed checked DIR facts for one module profile."""

    # indexed declared symbols
    symbols: destack._generated.dir.index.symbol.SymbolIndex
    # indexed exports
    exports: destack._generated.dir.index.export.ExportIndex
    # indexed checked members
    members: destack._generated.dir.index.member.MemberIndex
    # indexed reference memberships
    references: destack._generated.dir.index.reference.ReferenceIndex
    # indexed call edges
    calls: destack._generated.dir.index.call.CallIndex
    # indexed nominal heritage edges
    heritage: destack._generated.dir.index.heritage.HeritageIndex
    # indexed checked extensions
    extensions: destack._generated.dir.index.extension.ExtensionIndex
    # indexed module specifiers
    specifiers: destack._generated.dir.index.specifier.SpecifierIndex
    # indexed decorators
    decorators: destack._generated.dir.index.decorator.DecoratorIndex

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ModuleIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ModuleIndex: ...

def encode_module_index(writer: BinaryWriter, value: ModuleIndex) -> None: ...
def decode_module_index(reader: BinaryReader) -> ModuleIndex: ...
def to_json_module_index(value: ModuleIndex) -> Json: ...
def from_json_module_index(value: Json) -> ModuleIndex: ...

@dataclass(frozen=True, slots=True)
class ProgramIndex:
    """Indexed checked DIR module set for one program profile."""

    # the indexed modules in stable ordinal order
    modules: Sequence[destack._generated.source.file.model.module.ModuleId]
    # symbol postings
    symbols: destack._generated.dir.index.symbol.SymbolPostings
    # export postings
    exports: destack._generated.dir.index.export.ExportPostings
    # member postings
    members: destack._generated.dir.index.member.MemberPostings
    # reference postings
    references: destack._generated.dir.index.reference.ReferencePostings
    # call postings
    calls: destack._generated.dir.index.call.CallPostings
    # heritage postings
    heritage: destack._generated.dir.index.heritage.HeritagePostings
    # extension postings
    extensions: destack._generated.dir.index.extension.ExtensionPostings
    # module specifier postings
    specifiers: destack._generated.dir.index.specifier.SpecifierPostings
    # decorator postings
    decorators: destack._generated.dir.index.decorator.DecoratorPostings

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramIndex: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramIndex: ...

def encode_program_index(writer: BinaryWriter, value: ProgramIndex) -> None: ...
def decode_program_index(reader: BinaryReader) -> ProgramIndex: ...
def to_json_program_index(value: ProgramIndex) -> Json: ...
def from_json_program_index(value: Json) -> ProgramIndex: ...

__all__ = [
    "ModuleIndexProjection",
    "encode_module_index_projection",
    "decode_module_index_projection",
    "to_json_module_index_projection",
    "from_json_module_index_projection",
    "ModuleIndex",
    "encode_module_index",
    "decode_module_index",
    "to_json_module_index",
    "from_json_module_index",
    "ProgramIndex",
    "encode_program_index",
    "decode_program_index",
    "to_json_program_index",
    "from_json_program_index",
]
