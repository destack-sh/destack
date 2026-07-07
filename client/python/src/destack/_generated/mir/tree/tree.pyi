# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import BinaryReader, BinaryWriter, Json

import destack._generated.mir.source.token
import destack._generated.mir.tree.attribute
import destack._generated.mir.tree.block
import destack._generated.mir.tree.function
import destack._generated.mir.tree.global_
import destack._generated.mir.tree.immediate
import destack._generated.mir.tree.instruction
import destack._generated.mir.tree.lifetime
import destack._generated.mir.tree.local
import destack._generated.mir.tree.node
import destack._generated.mir.tree.origin
import destack._generated.mir.tree.terminator
import destack._generated.mir.tree.trivia
import destack._generated.mir.tree.type
import destack._generated.mir.tree.value
import destack._generated.source.file.model.span
import destack._generated.source.tree.index

@dataclass(frozen=True, slots=True)
class NodeIndexEntry:
    """Dense index entry for one MIR node id."""

    # the packed local id and node type
    packed: int

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeIndexEntry: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> NodeIndexEntry: ...

def encode_node_index_entry(writer: BinaryWriter, value: NodeIndexEntry) -> None: ...
def decode_node_index_entry(reader: BinaryReader) -> NodeIndexEntry: ...
def to_json_node_index_entry(value: NodeIndexEntry) -> Json: ...
def from_json_node_index_entry(value: Json) -> NodeIndexEntry: ...

@dataclass(frozen=True, slots=True)
class Tree:
    """MIR tree for a single unit."""

    # the first global node id stored in this tree
    first_global_id: int
    # the next global node id to allocate
    next_global_id: int
    # dense local id and node type by node id
    node_index_by_node_id: Sequence[NodeIndexEntry]
    # maps global node id → attached attributes
    attributes_by_node_id: Mapping[
        int, Sequence[destack._generated.mir.tree.attribute.Attribute]
    ]
    # DIR source id keyed by MIR node id
    source_id_by_node_id: Sequence[int | None]
    # how each pass-created node came to be
    origin_by_node_id: destack._generated.mir.tree.origin.OriginTable
    # source ranges and anchors for parsed MIR node ownership
    source_index: destack._generated.source.tree.index.SourceIndexData
    # the parsed MIR source text
    source_text: str | None
    # the full parsed token stream
    tokens: Sequence[destack._generated.mir.source.token.Token]
    # leading comment spans keyed by global node id
    leading_comment_spans_by_node_id: Sequence[
        destack._generated.source.file.model.span.Span | None
    ]
    # parsed attribute spans keyed by global node id
    attribute_spans_by_node_id: Mapping[
        int, Sequence[destack._generated.source.file.model.span.Span]
    ]
    # parsed declaration keyword spans keyed by global node id
    keyword_spans_by_node_id: Mapping[
        int, destack._generated.source.file.model.span.Span
    ]
    # parsed function parameter spans keyed by global node id
    function_parameter_spans_by_node_id: Mapping[
        int, Sequence[destack._generated.mir.tree.trivia.TypedValueSpan]
    ]
    # parsed function header spans keyed by global node id
    function_header_spans_by_node_id: Mapping[
        int, destack._generated.mir.tree.trivia.FunctionHeaderSpans
    ]
    # parsed type field spans keyed by global node id
    type_field_spans_by_node_id: Mapping[
        int, Sequence[destack._generated.mir.tree.trivia.FieldSpan]
    ]
    # parsed type declaration spans keyed by global node id
    type_declaration_spans_by_node_id: Mapping[
        int, destack._generated.mir.tree.trivia.TypeDeclarationSpans
    ]
    functions: Sequence[destack._generated.mir.tree.function.Function]
    blocks: Sequence[destack._generated.mir.tree.block.Block]
    instructions: Sequence[destack._generated.mir.tree.instruction.Instruction]
    terminators: Sequence[destack._generated.mir.tree.terminator.Terminator]
    locals: Sequence[destack._generated.mir.tree.local.Local]
    types: Sequence[destack._generated.mir.tree.type.Type]
    type_aliases: Sequence[destack._generated.mir.tree.type.TypeAlias]
    fields: Sequence[destack._generated.mir.tree.type.Field]
    globals: Sequence[destack._generated.mir.tree.global_.Global]
    # lifetime parameters keyed by type node
    lifetimes_by_type: Mapping[
        destack._generated.mir.tree.node.LocalNodeId,
        Sequence[destack._generated.mir.tree.lifetime.LifetimeParameter],
    ]
    # flat buffer of MIR values
    values: Sequence[destack._generated.mir.tree.value.Value]
    # flat buffer of instruction indices
    indices: Sequence[int]
    # flat buffer of instruction extents
    extents: Sequence[int]
    # flat buffer of instruction flags
    flags: builtins.bytes | bytearray | Sequence[int]
    # flat buffer of switch cases
    switch_cases: Sequence[destack._generated.mir.tree.terminator.SwitchCase]
    # structured tensor immediates
    tensor_immediates: Sequence[destack._generated.mir.tree.immediate.TensorImmediate]

    def encode(self, writer: BinaryWriter) -> None: ...
    @classmethod
    def decode(cls, reader: BinaryReader) -> Tree: ...
    def to_json(self) -> Json: ...
    @classmethod
    def from_json(cls, value: Json) -> Tree: ...

def encode_tree(writer: BinaryWriter, value: Tree) -> None: ...
def decode_tree(reader: BinaryReader) -> Tree: ...
def to_json_tree(value: Tree) -> Json: ...
def from_json_tree(value: Json) -> Tree: ...

__all__ = [
    "NodeIndexEntry",
    "encode_node_index_entry",
    "decode_node_index_entry",
    "to_json_node_index_entry",
    "from_json_node_index_entry",
    "Tree",
    "encode_tree",
    "decode_tree",
    "to_json_tree",
    "from_json_tree",
]
