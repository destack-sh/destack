# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

from destack._impl.mir.analyses.link_graph import (
    CallComponentGraphImpl,
)
from destack._impl.mir.analyses.link_graph import (
    LinkGraphImpl,
)

import destack._generated.core.bitset
import destack._generated.mir.table.effect
import destack._generated.mir.tree.global_
import destack._generated.mir.tree.symbol

@dataclass(frozen=True, slots=True)
class CallComponentGraph(CallComponentGraphImpl):
    """Strongly connected components of the whole-program call graph, by dense symbol id."""

    # the component id of each symbol, by dense id
    component: Sequence[int]
    # whether each component contains a cycle, by component id
    recursive: destack._generated.core.bitset.BitSet

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> CallComponentGraph: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> CallComponentGraph: ...

def encode_call_component_graph(
    writer: BinaryWriter, value: CallComponentGraph
) -> None: ...
def decode_call_component_graph(reader: BinaryReader) -> CallComponentGraph: ...
def to_json_call_component_graph(value: CallComponentGraph) -> Json: ...
def from_json_call_component_graph(value: Json) -> CallComponentGraph: ...

@dataclass(frozen=True, slots=True)
class LinkGraph(LinkGraphImpl):
    """Symbol-scoped reference graph for a module's linkable surface."""

    # defined symbols, by identity
    nodes: Mapping[destack._generated.mir.tree.symbol.Symbol, LinkNode]
    # outgoing references, by source symbol
    edges: Mapping[destack._generated.mir.tree.symbol.Symbol, Sequence[LinkEdge]]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinkGraph: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinkGraph: ...

def encode_link_graph(writer: BinaryWriter, value: LinkGraph) -> None: ...
def decode_link_graph(reader: BinaryReader) -> LinkGraph: ...
def to_json_link_graph(value: LinkGraph) -> Json: ...
def from_json_link_graph(value: Json) -> LinkGraph: ...

@dataclass(frozen=True, slots=True)
class LinkNodeFunction:
    """A defined function."""

    # visibility and definition location
    linkage: destack._generated.mir.tree.global_.Linkage
    # memory effect
    memory: destack._generated.mir.table.effect.MemoryEffect
    # behavioral effects (unwind, determinism, allocation, and so on)
    behavior: destack._generated.mir.table.effect.FunctionBehavior
    # estimated inline cost
    inline_cost: int
    # true when the function makes indirect or virtual calls
    indirect: bool
    kind: typing.Literal["function"] = "function"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

@dataclass(frozen=True, slots=True)
class LinkNodeGlobal:
    """A defined global."""

    # visibility and definition location
    linkage: destack._generated.mir.tree.global_.Linkage
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None: ...
    def to_json(self) -> Json: ...

"""One defined symbol in the link graph."""
LinkNode: typing.TypeAlias = LinkNodeFunction | LinkNodeGlobal

def encode_link_node(writer: BinaryWriter, value: LinkNode) -> None: ...
def decode_link_node(reader: BinaryReader) -> LinkNode: ...
def to_json_link_node(value: LinkNode) -> Json: ...
def from_json_link_node(value: Json) -> LinkNode: ...

@dataclass(frozen=True, slots=True)
class LinkEdge:
    """One outgoing reference from a symbol to another symbol."""

    # the referenced symbol
    target: destack._generated.mir.tree.symbol.Symbol
    # how the target is referenced
    kind: LinkEdgeKind

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> LinkEdge: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> LinkEdge: ...

def encode_link_edge(writer: BinaryWriter, value: LinkEdge) -> None: ...
def decode_link_edge(reader: BinaryReader) -> LinkEdge: ...
def to_json_link_edge(value: LinkEdge) -> Json: ...
def from_json_link_edge(value: Json) -> LinkEdge: ...

"""How one symbol references another."""
LinkEdgeKind: typing.TypeAlias = typing.Literal["call"] | typing.Literal["address"]

def encode_link_edge_kind(writer: BinaryWriter, value: LinkEdgeKind) -> None: ...
def decode_link_edge_kind(reader: BinaryReader) -> LinkEdgeKind: ...
def to_json_link_edge_kind(value: LinkEdgeKind) -> Json: ...
def from_json_link_edge_kind(value: Json) -> LinkEdgeKind: ...

__all__ = [
    "CallComponentGraph",
    "encode_call_component_graph",
    "decode_call_component_graph",
    "to_json_call_component_graph",
    "from_json_call_component_graph",
    "LinkGraph",
    "encode_link_graph",
    "decode_link_graph",
    "to_json_link_graph",
    "from_json_link_graph",
    "LinkNode",
    "encode_link_node",
    "decode_link_node",
    "to_json_link_node",
    "from_json_link_node",
    "LinkNodeFunction",
    "LinkNodeGlobal",
    "LinkEdge",
    "encode_link_edge",
    "decode_link_edge",
    "to_json_link_edge",
    "from_json_link_edge",
    "LinkEdgeKind",
    "encode_link_edge_kind",
    "decode_link_edge_kind",
    "to_json_link_edge_kind",
    "from_json_link_edge_kind",
]
