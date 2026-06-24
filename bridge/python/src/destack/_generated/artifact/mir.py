# generated bridge target, do not edit

from __future__ import annotations

from collections.abc import Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mir_lowered(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MirLowered:
        """Decode one MirLowered."""
        return decode_mir_lowered(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mir_lowered(self)

    @classmethod
    def from_json(cls, value: Json) -> MirLowered:
        """Return one MirLowered from one JSON value."""
        return from_json_mir_lowered(value)


def encode_mir_lowered(writer: BinaryWriter, value: MirLowered) -> None:
    """Encode one MirLowered."""
    destack._generated.mir.tree.tree.encode_tree(writer, value.tree)


def decode_mir_lowered(reader: BinaryReader) -> MirLowered:
    """Decode one MirLowered."""
    tree = destack._generated.mir.tree.tree.decode_tree(reader)

    return MirLowered(
        tree=tree,
    )


def to_json_mir_lowered(value: MirLowered) -> Json:
    """Return one JSON value for one MirLowered."""
    return {
        "tree": destack._generated.mir.tree.tree.to_json_tree(value.tree),
    }


def from_json_mir_lowered(value: Json) -> MirLowered:
    """Return one MirLowered from one JSON value."""
    object_ = json_object(value)

    return MirLowered(
        tree=destack._generated.mir.tree.tree.from_json_tree(
            json_field(object_, "tree")
        ),
    )


@dataclass(frozen=True, slots=True)
class MirVerified:
    """Verified MIR patch after required semantic verification."""

    # required verification patch
    patch: destack._generated.mir.tree.patch.Patch

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mir_verified(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MirVerified:
        """Decode one MirVerified."""
        return decode_mir_verified(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mir_verified(self)

    @classmethod
    def from_json(cls, value: Json) -> MirVerified:
        """Return one MirVerified from one JSON value."""
        return from_json_mir_verified(value)


def encode_mir_verified(writer: BinaryWriter, value: MirVerified) -> None:
    """Encode one MirVerified."""
    destack._generated.mir.tree.patch.encode_patch(writer, value.patch)


def decode_mir_verified(reader: BinaryReader) -> MirVerified:
    """Decode one MirVerified."""
    patch = destack._generated.mir.tree.patch.decode_patch(reader)

    return MirVerified(
        patch=patch,
    )


def to_json_mir_verified(value: MirVerified) -> Json:
    """Return one JSON value for one MirVerified."""
    return {
        "patch": destack._generated.mir.tree.patch.to_json_patch(value.patch),
    }


def from_json_mir_verified(value: Json) -> MirVerified:
    """Return one MirVerified from one JSON value."""
    object_ = json_object(value)

    return MirVerified(
        patch=destack._generated.mir.tree.patch.from_json_patch(
            json_field(object_, "patch")
        ),
    )


@dataclass(frozen=True, slots=True)
class MirOptimized:
    """Optimized MIR payload after pipeline transforms."""

    # ordered optimization patches
    patches: Sequence[destack._generated.mir.tree.patch.Patch]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mir_optimized(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MirOptimized:
        """Decode one MirOptimized."""
        return decode_mir_optimized(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mir_optimized(self)

    @classmethod
    def from_json(cls, value: Json) -> MirOptimized:
        """Return one MirOptimized from one JSON value."""
        return from_json_mir_optimized(value)


def encode_mir_optimized(writer: BinaryWriter, value: MirOptimized) -> None:
    """Encode one MirOptimized."""
    writer.write_unsigned(len(value.patches))
    for item_value_patches_0 in value.patches:
        destack._generated.mir.tree.patch.encode_patch(writer, item_value_patches_0)


def decode_mir_optimized(reader: BinaryReader) -> MirOptimized:
    """Decode one MirOptimized."""
    patches = [
        destack._generated.mir.tree.patch.decode_patch(reader)
        for _ in range(reader.read_number())
    ]

    return MirOptimized(
        patches=patches,
    )


def to_json_mir_optimized(value: MirOptimized) -> Json:
    """Return one JSON value for one MirOptimized."""
    return {
        "patches": [
            destack._generated.mir.tree.patch.to_json_patch(item_0)
            for item_0 in value.patches
        ],
    }


def from_json_mir_optimized(value: Json) -> MirOptimized:
    """Return one MirOptimized from one JSON value."""
    object_ = json_object(value)

    return MirOptimized(
        patches=[
            destack._generated.mir.tree.patch.from_json_patch(item_0)
            for item_0 in json_array(json_field(object_, "patches"))
        ],
    )


@dataclass(frozen=True, slots=True)
class MirAnalyzed:
    """Per-module link summary produced by program analysis."""

    # the module's symbol reference graph
    links: destack._generated.mir.analyses.link_graph.LinkGraph

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_mir_analyzed(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> MirAnalyzed:
        """Decode one MirAnalyzed."""
        return decode_mir_analyzed(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_mir_analyzed(self)

    @classmethod
    def from_json(cls, value: Json) -> MirAnalyzed:
        """Return one MirAnalyzed from one JSON value."""
        return from_json_mir_analyzed(value)


def encode_mir_analyzed(writer: BinaryWriter, value: MirAnalyzed) -> None:
    """Encode one MirAnalyzed."""
    destack._generated.mir.analyses.link_graph.encode_link_graph(writer, value.links)


def decode_mir_analyzed(reader: BinaryReader) -> MirAnalyzed:
    """Decode one MirAnalyzed."""
    links = destack._generated.mir.analyses.link_graph.decode_link_graph(reader)

    return MirAnalyzed(
        links=links,
    )


def to_json_mir_analyzed(value: MirAnalyzed) -> Json:
    """Return one JSON value for one MirAnalyzed."""
    return {
        "links": destack._generated.mir.analyses.link_graph.to_json_link_graph(
            value.links
        ),
    }


def from_json_mir_analyzed(value: Json) -> MirAnalyzed:
    """Return one MirAnalyzed from one JSON value."""
    object_ = json_object(value)

    return MirAnalyzed(
        links=destack._generated.mir.analyses.link_graph.from_json_link_graph(
            json_field(object_, "links")
        ),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_program_analysis(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> ProgramAnalysis:
        """Decode one ProgramAnalysis."""
        return decode_program_analysis(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_program_analysis(self)

    @classmethod
    def from_json(cls, value: Json) -> ProgramAnalysis:
        """Return one ProgramAnalysis from one JSON value."""
        return from_json_program_analysis(value)


def encode_program_analysis(writer: BinaryWriter, value: ProgramAnalysis) -> None:
    """Encode one ProgramAnalysis."""
    writer.write_unsigned(len(value.symbols))
    for item_value_symbols_0 in value.symbols:
        destack._generated.mir.tree.symbol.encode_symbol(writer, item_value_symbols_0)
    destack._generated.core.bitset.encode_bit_set(writer, value.live)
    writer.write_unsigned(len(value.references))
    for item_value_references_0 in value.references:
        writer.write_unsigned(item_value_references_0)
    destack._generated.core.bitset.encode_bit_set(writer, value.address_taken)
    destack._generated.core.bitset.encode_bit_set(writer, value.internal)
    destack._generated.mir.analyses.link_graph.encode_call_component_graph(
        writer, value.components
    )


def decode_program_analysis(reader: BinaryReader) -> ProgramAnalysis:
    """Decode one ProgramAnalysis."""
    symbols = [
        destack._generated.mir.tree.symbol.decode_symbol(reader)
        for _ in range(reader.read_number())
    ]
    live = destack._generated.core.bitset.decode_bit_set(reader)
    references = [reader.read_number() for _ in range(reader.read_number())]
    address_taken = destack._generated.core.bitset.decode_bit_set(reader)
    internal = destack._generated.core.bitset.decode_bit_set(reader)
    components = destack._generated.mir.analyses.link_graph.decode_call_component_graph(
        reader
    )

    return ProgramAnalysis(
        symbols=symbols,
        live=live,
        references=references,
        address_taken=address_taken,
        internal=internal,
        components=components,
    )


def to_json_program_analysis(value: ProgramAnalysis) -> Json:
    """Return one JSON value for one ProgramAnalysis."""
    return {
        "symbols": [
            destack._generated.mir.tree.symbol.to_json_symbol(item_0)
            for item_0 in value.symbols
        ],
        "live": destack._generated.core.bitset.to_json_bit_set(value.live),
        "references": [item_0 for item_0 in value.references],
        "addressTaken": destack._generated.core.bitset.to_json_bit_set(
            value.address_taken
        ),
        "internal": destack._generated.core.bitset.to_json_bit_set(value.internal),
        "components": destack._generated.mir.analyses.link_graph.to_json_call_component_graph(
            value.components
        ),
    }


def from_json_program_analysis(value: Json) -> ProgramAnalysis:
    """Return one ProgramAnalysis from one JSON value."""
    object_ = json_object(value)

    return ProgramAnalysis(
        symbols=[
            destack._generated.mir.tree.symbol.from_json_symbol(item_0)
            for item_0 in json_array(json_field(object_, "symbols"))
        ],
        live=destack._generated.core.bitset.from_json_bit_set(
            json_field(object_, "live")
        ),
        references=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "references"))
        ],
        address_taken=destack._generated.core.bitset.from_json_bit_set(
            json_field(object_, "addressTaken")
        ),
        internal=destack._generated.core.bitset.from_json_bit_set(
            json_field(object_, "internal")
        ),
        components=destack._generated.mir.analyses.link_graph.from_json_call_component_graph(
            json_field(object_, "components")
        ),
    )


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
