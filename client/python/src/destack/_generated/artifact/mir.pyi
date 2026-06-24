# generated client target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.core.bitset
import destack._generated.mir.analyses.link_graph
import destack._generated.mir.tree.patch
import destack._generated.mir.tree.symbol
import destack._generated.mir.tree.tree

@dataclass(frozen=True, slots=True)
class MirLowered:
    """Lowered MIR payload before optimization."""

    # the MIR tree
    tree: destack._generated.mir.tree.tree.Tree

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MirLowered: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MirLowered: ...

def encode_mir_lowered(writer: BinaryWriter, value: MirLowered) -> None: ...
def decode_mir_lowered(reader: BinaryReader) -> MirLowered: ...
def to_json_mir_lowered(value: MirLowered) -> Json: ...
def from_json_mir_lowered(value: Json) -> MirLowered: ...

@dataclass(frozen=True, slots=True)
class MirVerified:
    """Verified MIR patch after required semantic verification."""

    # required verification patch
    patch: destack._generated.mir.tree.patch.Patch

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MirVerified: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MirVerified: ...

def encode_mir_verified(writer: BinaryWriter, value: MirVerified) -> None: ...
def decode_mir_verified(reader: BinaryReader) -> MirVerified: ...
def to_json_mir_verified(value: MirVerified) -> Json: ...
def from_json_mir_verified(value: Json) -> MirVerified: ...

@dataclass(frozen=True, slots=True)
class MirOptimized:
    """Optimized MIR payload after pipeline transforms."""

    # ordered optimization patches
    patches: Sequence[destack._generated.mir.tree.patch.Patch]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MirOptimized: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MirOptimized: ...

def encode_mir_optimized(writer: BinaryWriter, value: MirOptimized) -> None: ...
def decode_mir_optimized(reader: BinaryReader) -> MirOptimized: ...
def to_json_mir_optimized(value: MirOptimized) -> Json: ...
def from_json_mir_optimized(value: Json) -> MirOptimized: ...

@dataclass(frozen=True, slots=True)
class MirAnalyzed:
    """Per-module link summary produced by program analysis."""

    # the module's symbol reference graph
    links: destack._generated.mir.analyses.link_graph.LinkGraph

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> MirAnalyzed: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> MirAnalyzed: ...

def encode_mir_analyzed(writer: BinaryWriter, value: MirAnalyzed) -> None: ...
def decode_mir_analyzed(reader: BinaryReader) -> MirAnalyzed: ...
def to_json_mir_analyzed(value: MirAnalyzed) -> Json: ...
def from_json_mir_analyzed(value: Json) -> MirAnalyzed: ...

@dataclass(frozen=True, slots=True)
class ProgramAnalysis:
    """Whole-program analysis columns shared across the optimization of every module."""

    # every defined symbol in the program
    symbols: Sequence[destack._generated.mir.tree.symbol.Symbol]
    # whether each symbol is reachable from a program root
    live: destack._generated.core.bitset.BitSet
    # program-wide incoming reference count of each symbol
    references: Sequence[int]
    # whether each symbol's address is taken anywhere in the program
    address_taken: destack._generated.core.bitset.BitSet
    # whether each symbol is internal to the program (not an external root)
    internal: destack._generated.core.bitset.BitSet
    # strongly connected components of the whole-program call graph
    components: destack._generated.mir.analyses.link_graph.CallComponentGraph

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramAnalysis: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> ProgramAnalysis: ...

def encode_program_analysis(writer: BinaryWriter, value: ProgramAnalysis) -> None: ...
def decode_program_analysis(reader: BinaryReader) -> ProgramAnalysis: ...
def to_json_program_analysis(value: ProgramAnalysis) -> Json: ...
def from_json_program_analysis(value: Json) -> ProgramAnalysis: ...

__all__ = [
    "MirLowered",
    "encode_mir_lowered",
    "decode_mir_lowered",
    "to_json_mir_lowered",
    "from_json_mir_lowered",
    "MirVerified",
    "encode_mir_verified",
    "decode_mir_verified",
    "to_json_mir_verified",
    "from_json_mir_verified",
    "MirOptimized",
    "encode_mir_optimized",
    "decode_mir_optimized",
    "to_json_mir_optimized",
    "from_json_mir_optimized",
    "MirAnalyzed",
    "encode_mir_analyzed",
    "decode_mir_analyzed",
    "to_json_mir_analyzed",
    "from_json_mir_analyzed",
    "ProgramAnalysis",
    "encode_program_analysis",
    "decode_program_analysis",
    "to_json_program_analysis",
    "from_json_program_analysis",
]
