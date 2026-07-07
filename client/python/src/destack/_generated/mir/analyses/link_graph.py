# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass
import typing

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    SerdeError,
    json_array,
    json_bool,
    json_field,
    json_int,
    json_object,
    json_string,
    nested_bytes,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_call_component_graph(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> CallComponentGraph:
        """Decode one CallComponentGraph."""
        return decode_call_component_graph(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_call_component_graph(self)

    @classmethod
    def from_json(cls, value: Json) -> CallComponentGraph:
        """Return one CallComponentGraph from one JSON value."""
        return from_json_call_component_graph(value)


def encode_call_component_graph(
    writer: BinaryWriter, value: CallComponentGraph
) -> None:
    """Encode one CallComponentGraph."""
    writer.write_unsigned(len(value.component))
    for item_value_component_0 in value.component:
        writer.write_unsigned(item_value_component_0)
    destack._generated.core.bitset.encode_bit_set(writer, value.recursive)


def decode_call_component_graph(reader: BinaryReader) -> CallComponentGraph:
    """Decode one CallComponentGraph."""
    component = [reader.read_number() for _ in range(reader.read_number())]
    recursive = destack._generated.core.bitset.decode_bit_set(reader)

    return CallComponentGraph(
        component=component,
        recursive=recursive,
    )


def to_json_call_component_graph(value: CallComponentGraph) -> Json:
    """Return one JSON value for one CallComponentGraph."""
    return {
        "component": [item_0 for item_0 in value.component],
        "recursive": destack._generated.core.bitset.to_json_bit_set(value.recursive),
    }


def from_json_call_component_graph(value: Json) -> CallComponentGraph:
    """Return one CallComponentGraph from one JSON value."""
    object_ = json_object(value)

    return CallComponentGraph(
        component=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "component"))
        ],
        recursive=destack._generated.core.bitset.from_json_bit_set(
            json_field(object_, "recursive")
        ),
    )


@dataclass(frozen=True, slots=True)
class LinkGraph(LinkGraphImpl):
    """Symbol-scoped reference graph for a module's linkable surface."""

    # defined symbols, by identity
    nodes: Mapping[destack._generated.mir.tree.symbol.Symbol, LinkNode]
    # outgoing references, by source symbol
    edges: Mapping[destack._generated.mir.tree.symbol.Symbol, Sequence[LinkEdge]]

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_graph(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinkGraph:
        """Decode one LinkGraph."""
        return decode_link_graph(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_graph(self)

    @classmethod
    def from_json(cls, value: Json) -> LinkGraph:
        """Return one LinkGraph from one JSON value."""
        return from_json_link_graph(value)


def encode_link_graph(writer: BinaryWriter, value: LinkGraph) -> None:
    """Encode one LinkGraph."""
    entries_value_nodes_0 = []
    for key_value_nodes_0, item_value_nodes_0 in value.nodes.items():

        def write_key_value_nodes_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.symbol.encode_symbol(writer, key_value_nodes_0)

        key_bytes = nested_bytes(write_key_value_nodes_0)
        entries_value_nodes_0.append((key_value_nodes_0, item_value_nodes_0, key_bytes))
    entries_value_nodes_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_nodes_0))
    for entry_value_nodes_0 in entries_value_nodes_0:
        destack._generated.mir.tree.symbol.encode_symbol(writer, entry_value_nodes_0[0])
        encode_link_node(writer, entry_value_nodes_0[1])
    entries_value_edges_0 = []
    for key_value_edges_0, item_value_edges_0 in value.edges.items():

        def write_key_value_edges_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.symbol.encode_symbol(writer, key_value_edges_0)

        key_bytes = nested_bytes(write_key_value_edges_0)
        entries_value_edges_0.append((key_value_edges_0, item_value_edges_0, key_bytes))
    entries_value_edges_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_edges_0))
    for entry_value_edges_0 in entries_value_edges_0:
        destack._generated.mir.tree.symbol.encode_symbol(writer, entry_value_edges_0[0])
        writer.write_unsigned(len(entry_value_edges_0[1]))
        for item_entry_value_edges_0_1_1 in entry_value_edges_0[1]:
            encode_link_edge(writer, item_entry_value_edges_0_1_1)


def decode_link_graph(reader: BinaryReader) -> LinkGraph:
    """Decode one LinkGraph."""
    nodes = {
        destack._generated.mir.tree.symbol.decode_symbol(reader): decode_link_node(
            reader
        )
        for _ in range(reader.read_number())
    }
    edges = {
        destack._generated.mir.tree.symbol.decode_symbol(reader): [
            decode_link_edge(reader) for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }

    return LinkGraph(
        nodes=nodes,
        edges=edges,
    )


def to_json_link_graph(value: LinkGraph) -> Json:
    """Return one JSON value for one LinkGraph."""
    return {
        "nodes": [
            [
                destack._generated.mir.tree.symbol.to_json_symbol(key_0),
                to_json_link_node(item_0),
            ]
            for key_0, item_0 in value.nodes.items()
        ],
        "edges": [
            [
                destack._generated.mir.tree.symbol.to_json_symbol(key_0),
                [to_json_link_edge(item_1) for item_1 in item_0],
            ]
            for key_0, item_0 in value.edges.items()
        ],
    }


def from_json_link_graph(value: Json) -> LinkGraph:
    """Return one LinkGraph from one JSON value."""
    object_ = json_object(value)

    return LinkGraph(
        nodes={
            destack._generated.mir.tree.symbol.from_json_symbol(
                key_0
            ): from_json_link_node(item_0)
            for key_0, item_0 in json_array(json_field(object_, "nodes"))
        },
        edges={
            destack._generated.mir.tree.symbol.from_json_symbol(key_0): [
                from_json_link_edge(item_1) for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "edges"))
        },
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_node(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_node(self)


@dataclass(frozen=True, slots=True)
class LinkNodeGlobal:
    """A defined global."""

    # visibility and definition location
    linkage: destack._generated.mir.tree.global_.Linkage
    kind: typing.Literal["global"] = "global"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_node(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_node(self)


"""One defined symbol in the link graph."""
LinkNode: typing.TypeAlias = LinkNodeFunction | LinkNodeGlobal


def encode_link_node(writer: BinaryWriter, value: LinkNode) -> None:
    """Encode one LinkNode."""
    if value.kind == "function":
        writer.write_unsigned(0)
        destack._generated.mir.tree.global_.encode_linkage(writer, value.linkage)
        destack._generated.mir.table.effect.encode_memory_effect(writer, value.memory)
        destack._generated.mir.table.effect.encode_function_behavior(
            writer, value.behavior
        )
        writer.write_unsigned(value.inline_cost)
        writer.write_bool(value.indirect)
    elif value.kind == "global":
        writer.write_unsigned(1)
        destack._generated.mir.tree.global_.encode_linkage(writer, value.linkage)
    else:
        raise SerdeError("unknown enum variant")


def decode_link_node(reader: BinaryReader) -> LinkNode:
    """Decode one LinkNode."""
    variant = reader.read_number()

    if variant == 0:
        linkage = destack._generated.mir.tree.global_.decode_linkage(reader)
        memory = destack._generated.mir.table.effect.decode_memory_effect(reader)
        behavior = destack._generated.mir.table.effect.decode_function_behavior(reader)
        inline_cost = reader.read_number()
        indirect = reader.read_bool()

        return LinkNodeFunction(
            linkage=linkage,
            memory=memory,
            behavior=behavior,
            inline_cost=inline_cost,
            indirect=indirect,
        )
    elif variant == 1:
        linkage = destack._generated.mir.tree.global_.decode_linkage(reader)

        return LinkNodeGlobal(
            linkage=linkage,
        )
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_link_node(value: LinkNode) -> Json:
    """Return one JSON value for one LinkNode."""
    if value.kind == "function":
        return {
            "kind": "function",
            "linkage": destack._generated.mir.tree.global_.to_json_linkage(
                value.linkage
            ),
            "memory": destack._generated.mir.table.effect.to_json_memory_effect(
                value.memory
            ),
            "behavior": destack._generated.mir.table.effect.to_json_function_behavior(
                value.behavior
            ),
            "inlineCost": value.inline_cost,
            "indirect": value.indirect,
        }
    elif value.kind == "global":
        return {
            "kind": "global",
            "linkage": destack._generated.mir.tree.global_.to_json_linkage(
                value.linkage
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_link_node(value: Json) -> LinkNode:
    """Return one LinkNode from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "function":
        return LinkNodeFunction(
            linkage=destack._generated.mir.tree.global_.from_json_linkage(
                json_field(object_, "linkage")
            ),
            memory=destack._generated.mir.table.effect.from_json_memory_effect(
                json_field(object_, "memory")
            ),
            behavior=destack._generated.mir.table.effect.from_json_function_behavior(
                json_field(object_, "behavior")
            ),
            inline_cost=json_int(json_field(object_, "inlineCost")),
            indirect=json_bool(json_field(object_, "indirect")),
        )
    elif kind == "global":
        return LinkNodeGlobal(
            linkage=destack._generated.mir.tree.global_.from_json_linkage(
                json_field(object_, "linkage")
            ),
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


@dataclass(frozen=True, slots=True)
class LinkEdge:
    """One outgoing reference from a symbol to another symbol."""

    # the referenced symbol
    target: destack._generated.mir.tree.symbol.Symbol
    # how the target is referenced
    kind: LinkEdgeKind

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_link_edge(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> LinkEdge:
        """Decode one LinkEdge."""
        return decode_link_edge(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_link_edge(self)

    @classmethod
    def from_json(cls, value: Json) -> LinkEdge:
        """Return one LinkEdge from one JSON value."""
        return from_json_link_edge(value)


def encode_link_edge(writer: BinaryWriter, value: LinkEdge) -> None:
    """Encode one LinkEdge."""
    destack._generated.mir.tree.symbol.encode_symbol(writer, value.target)
    encode_link_edge_kind(writer, value.kind)


def decode_link_edge(reader: BinaryReader) -> LinkEdge:
    """Decode one LinkEdge."""
    target = destack._generated.mir.tree.symbol.decode_symbol(reader)
    kind = decode_link_edge_kind(reader)

    return LinkEdge(
        target=target,
        kind=kind,
    )


def to_json_link_edge(value: LinkEdge) -> Json:
    """Return one JSON value for one LinkEdge."""
    return {
        "target": destack._generated.mir.tree.symbol.to_json_symbol(value.target),
        "kind": to_json_link_edge_kind(value.kind),
    }


def from_json_link_edge(value: Json) -> LinkEdge:
    """Return one LinkEdge from one JSON value."""
    object_ = json_object(value)

    return LinkEdge(
        target=destack._generated.mir.tree.symbol.from_json_symbol(
            json_field(object_, "target")
        ),
        kind=from_json_link_edge_kind(json_field(object_, "kind")),
    )


"""How one symbol references another."""
LinkEdgeKind: typing.TypeAlias = typing.Literal["call"] | typing.Literal["address"]


def encode_link_edge_kind(writer: BinaryWriter, value: LinkEdgeKind) -> None:
    """Encode one LinkEdgeKind."""
    if value == "call":
        writer.write_unsigned(0)
    elif value == "address":
        writer.write_unsigned(1)
    else:
        raise SerdeError("unknown enum variant")


def decode_link_edge_kind(reader: BinaryReader) -> LinkEdgeKind:
    """Decode one LinkEdgeKind."""
    variant = reader.read_number()

    if variant == 0:
        return "call"
    elif variant == 1:
        return "address"
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_link_edge_kind(value: LinkEdgeKind) -> Json:
    """Return one JSON value for one LinkEdgeKind."""
    return value


def from_json_link_edge_kind(value: Json) -> LinkEdgeKind:
    """Return one LinkEdgeKind from one JSON value."""
    variant = json_string(value)

    if variant == "call":
        return "call"
    elif variant == "address":
        return "address"
    else:
        raise SerdeError(f"unknown enum variant: {variant}")


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
