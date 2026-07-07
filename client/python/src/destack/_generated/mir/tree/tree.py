# generated client target, do not edit

from __future__ import annotations

import builtins

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    bytes_from_json,
    bytes_to_json,
    json_array,
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_index_entry(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeIndexEntry:
        """Decode one NodeIndexEntry."""
        return decode_node_index_entry(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_index_entry(self)

    @classmethod
    def from_json(cls, value: Json) -> NodeIndexEntry:
        """Return one NodeIndexEntry from one JSON value."""
        return from_json_node_index_entry(value)


def encode_node_index_entry(writer: BinaryWriter, value: NodeIndexEntry) -> None:
    """Encode one NodeIndexEntry."""
    writer.write_unsigned(value.packed)


def decode_node_index_entry(reader: BinaryReader) -> NodeIndexEntry:
    """Decode one NodeIndexEntry."""
    packed = reader.read_number()

    return NodeIndexEntry(
        packed=packed,
    )


def to_json_node_index_entry(value: NodeIndexEntry) -> Json:
    """Return one JSON value for one NodeIndexEntry."""
    return {
        "packed": value.packed,
    }


def from_json_node_index_entry(value: Json) -> NodeIndexEntry:
    """Return one NodeIndexEntry from one JSON value."""
    object_ = json_object(value)

    return NodeIndexEntry(
        packed=json_int(json_field(object_, "packed")),
    )


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

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_tree(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> Tree:
        """Decode one Tree."""
        return decode_tree(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_tree(self)

    @classmethod
    def from_json(cls, value: Json) -> Tree:
        """Return one Tree from one JSON value."""
        return from_json_tree(value)


def encode_tree(writer: BinaryWriter, value: Tree) -> None:
    """Encode one Tree."""
    writer.write_unsigned(value.first_global_id)
    writer.write_unsigned(value.next_global_id)
    writer.write_unsigned(len(value.node_index_by_node_id))
    for item_value_node_index_by_node_id_0 in value.node_index_by_node_id:
        encode_node_index_entry(writer, item_value_node_index_by_node_id_0)
    entries_value_attributes_by_node_id_0 = []
    for (
        key_value_attributes_by_node_id_0,
        item_value_attributes_by_node_id_0,
    ) in value.attributes_by_node_id.items():

        def write_key_value_attributes_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_attributes_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_attributes_by_node_id_0)
        entries_value_attributes_by_node_id_0.append(
            (
                key_value_attributes_by_node_id_0,
                item_value_attributes_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_attributes_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_attributes_by_node_id_0))
    for entry_value_attributes_by_node_id_0 in entries_value_attributes_by_node_id_0:
        writer.write_unsigned(entry_value_attributes_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_attributes_by_node_id_0[1]))
        for (
            item_entry_value_attributes_by_node_id_0_1_1
        ) in entry_value_attributes_by_node_id_0[1]:
            destack._generated.mir.tree.attribute.encode_attribute(
                writer, item_entry_value_attributes_by_node_id_0_1_1
            )
    writer.write_unsigned(len(value.source_id_by_node_id))
    for item_value_source_id_by_node_id_0 in value.source_id_by_node_id:
        if item_value_source_id_by_node_id_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            writer.write_unsigned(item_value_source_id_by_node_id_0)
    destack._generated.mir.tree.origin.encode_origin_table(
        writer, value.origin_by_node_id
    )
    destack._generated.source.tree.index.encode_source_index_data(
        writer, value.source_index
    )
    if value.source_text is None:
        writer.write_byte(0)
    else:
        writer.write_byte(1)
        writer.write_string(value.source_text)
    writer.write_unsigned(len(value.tokens))
    for item_value_tokens_0 in value.tokens:
        destack._generated.mir.source.token.encode_token(writer, item_value_tokens_0)
    writer.write_unsigned(len(value.leading_comment_spans_by_node_id))
    for (
        item_value_leading_comment_spans_by_node_id_0
    ) in value.leading_comment_spans_by_node_id:
        if item_value_leading_comment_spans_by_node_id_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            destack._generated.source.file.model.span.encode_span(
                writer, item_value_leading_comment_spans_by_node_id_0
            )
    entries_value_attribute_spans_by_node_id_0 = []
    for (
        key_value_attribute_spans_by_node_id_0,
        item_value_attribute_spans_by_node_id_0,
    ) in value.attribute_spans_by_node_id.items():

        def write_key_value_attribute_spans_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_attribute_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_attribute_spans_by_node_id_0)
        entries_value_attribute_spans_by_node_id_0.append(
            (
                key_value_attribute_spans_by_node_id_0,
                item_value_attribute_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_attribute_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_attribute_spans_by_node_id_0))
    for (
        entry_value_attribute_spans_by_node_id_0
    ) in entries_value_attribute_spans_by_node_id_0:
        writer.write_unsigned(entry_value_attribute_spans_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_attribute_spans_by_node_id_0[1]))
        for (
            item_entry_value_attribute_spans_by_node_id_0_1_1
        ) in entry_value_attribute_spans_by_node_id_0[1]:
            destack._generated.source.file.model.span.encode_span(
                writer, item_entry_value_attribute_spans_by_node_id_0_1_1
            )
    entries_value_keyword_spans_by_node_id_0 = []
    for (
        key_value_keyword_spans_by_node_id_0,
        item_value_keyword_spans_by_node_id_0,
    ) in value.keyword_spans_by_node_id.items():

        def write_key_value_keyword_spans_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_keyword_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_keyword_spans_by_node_id_0)
        entries_value_keyword_spans_by_node_id_0.append(
            (
                key_value_keyword_spans_by_node_id_0,
                item_value_keyword_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_keyword_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_keyword_spans_by_node_id_0))
    for (
        entry_value_keyword_spans_by_node_id_0
    ) in entries_value_keyword_spans_by_node_id_0:
        writer.write_unsigned(entry_value_keyword_spans_by_node_id_0[0])
        destack._generated.source.file.model.span.encode_span(
            writer, entry_value_keyword_spans_by_node_id_0[1]
        )
    entries_value_function_parameter_spans_by_node_id_0 = []
    for (
        key_value_function_parameter_spans_by_node_id_0,
        item_value_function_parameter_spans_by_node_id_0,
    ) in value.function_parameter_spans_by_node_id.items():

        def write_key_value_function_parameter_spans_by_node_id_0(
            writer: BinaryWriter,
        ) -> None:
            writer.write_unsigned(key_value_function_parameter_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_function_parameter_spans_by_node_id_0)
        entries_value_function_parameter_spans_by_node_id_0.append(
            (
                key_value_function_parameter_spans_by_node_id_0,
                item_value_function_parameter_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_function_parameter_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_function_parameter_spans_by_node_id_0))
    for (
        entry_value_function_parameter_spans_by_node_id_0
    ) in entries_value_function_parameter_spans_by_node_id_0:
        writer.write_unsigned(entry_value_function_parameter_spans_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_function_parameter_spans_by_node_id_0[1]))
        for (
            item_entry_value_function_parameter_spans_by_node_id_0_1_1
        ) in entry_value_function_parameter_spans_by_node_id_0[1]:
            destack._generated.mir.tree.trivia.encode_typed_value_span(
                writer, item_entry_value_function_parameter_spans_by_node_id_0_1_1
            )
    entries_value_function_header_spans_by_node_id_0 = []
    for (
        key_value_function_header_spans_by_node_id_0,
        item_value_function_header_spans_by_node_id_0,
    ) in value.function_header_spans_by_node_id.items():

        def write_key_value_function_header_spans_by_node_id_0(
            writer: BinaryWriter,
        ) -> None:
            writer.write_unsigned(key_value_function_header_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_function_header_spans_by_node_id_0)
        entries_value_function_header_spans_by_node_id_0.append(
            (
                key_value_function_header_spans_by_node_id_0,
                item_value_function_header_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_function_header_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_function_header_spans_by_node_id_0))
    for (
        entry_value_function_header_spans_by_node_id_0
    ) in entries_value_function_header_spans_by_node_id_0:
        writer.write_unsigned(entry_value_function_header_spans_by_node_id_0[0])
        destack._generated.mir.tree.trivia.encode_function_header_spans(
            writer, entry_value_function_header_spans_by_node_id_0[1]
        )
    entries_value_type_field_spans_by_node_id_0 = []
    for (
        key_value_type_field_spans_by_node_id_0,
        item_value_type_field_spans_by_node_id_0,
    ) in value.type_field_spans_by_node_id.items():

        def write_key_value_type_field_spans_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_type_field_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_type_field_spans_by_node_id_0)
        entries_value_type_field_spans_by_node_id_0.append(
            (
                key_value_type_field_spans_by_node_id_0,
                item_value_type_field_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_type_field_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_type_field_spans_by_node_id_0))
    for (
        entry_value_type_field_spans_by_node_id_0
    ) in entries_value_type_field_spans_by_node_id_0:
        writer.write_unsigned(entry_value_type_field_spans_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_type_field_spans_by_node_id_0[1]))
        for (
            item_entry_value_type_field_spans_by_node_id_0_1_1
        ) in entry_value_type_field_spans_by_node_id_0[1]:
            destack._generated.mir.tree.trivia.encode_field_span(
                writer, item_entry_value_type_field_spans_by_node_id_0_1_1
            )
    entries_value_type_declaration_spans_by_node_id_0 = []
    for (
        key_value_type_declaration_spans_by_node_id_0,
        item_value_type_declaration_spans_by_node_id_0,
    ) in value.type_declaration_spans_by_node_id.items():

        def write_key_value_type_declaration_spans_by_node_id_0(
            writer: BinaryWriter,
        ) -> None:
            writer.write_unsigned(key_value_type_declaration_spans_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_type_declaration_spans_by_node_id_0)
        entries_value_type_declaration_spans_by_node_id_0.append(
            (
                key_value_type_declaration_spans_by_node_id_0,
                item_value_type_declaration_spans_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_type_declaration_spans_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_type_declaration_spans_by_node_id_0))
    for (
        entry_value_type_declaration_spans_by_node_id_0
    ) in entries_value_type_declaration_spans_by_node_id_0:
        writer.write_unsigned(entry_value_type_declaration_spans_by_node_id_0[0])
        destack._generated.mir.tree.trivia.encode_type_declaration_spans(
            writer, entry_value_type_declaration_spans_by_node_id_0[1]
        )
    writer.write_unsigned(len(value.functions))
    for item_value_functions_0 in value.functions:
        destack._generated.mir.tree.function.encode_function(
            writer, item_value_functions_0
        )
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        destack._generated.mir.tree.block.encode_block(writer, item_value_blocks_0)
    writer.write_unsigned(len(value.instructions))
    for item_value_instructions_0 in value.instructions:
        destack._generated.mir.tree.instruction.encode_instruction(
            writer, item_value_instructions_0
        )
    writer.write_unsigned(len(value.terminators))
    for item_value_terminators_0 in value.terminators:
        destack._generated.mir.tree.terminator.encode_terminator(
            writer, item_value_terminators_0
        )
    writer.write_unsigned(len(value.locals))
    for item_value_locals_0 in value.locals:
        destack._generated.mir.tree.local.encode_local(writer, item_value_locals_0)
    writer.write_unsigned(len(value.types))
    for item_value_types_0 in value.types:
        destack._generated.mir.tree.type.encode_type(writer, item_value_types_0)
    writer.write_unsigned(len(value.type_aliases))
    for item_value_type_aliases_0 in value.type_aliases:
        destack._generated.mir.tree.type.encode_type_alias(
            writer, item_value_type_aliases_0
        )
    writer.write_unsigned(len(value.fields))
    for item_value_fields_0 in value.fields:
        destack._generated.mir.tree.type.encode_field(writer, item_value_fields_0)
    writer.write_unsigned(len(value.globals))
    for item_value_globals_0 in value.globals:
        destack._generated.mir.tree.global_.encode_global(writer, item_value_globals_0)
    entries_value_lifetimes_by_type_0 = []
    for (
        key_value_lifetimes_by_type_0,
        item_value_lifetimes_by_type_0,
    ) in value.lifetimes_by_type.items():

        def write_key_value_lifetimes_by_type_0(writer: BinaryWriter) -> None:
            destack._generated.mir.tree.node.encode_local_node_id(
                writer, key_value_lifetimes_by_type_0
            )

        key_bytes = nested_bytes(write_key_value_lifetimes_by_type_0)
        entries_value_lifetimes_by_type_0.append(
            (key_value_lifetimes_by_type_0, item_value_lifetimes_by_type_0, key_bytes)
        )
    entries_value_lifetimes_by_type_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_lifetimes_by_type_0))
    for entry_value_lifetimes_by_type_0 in entries_value_lifetimes_by_type_0:
        destack._generated.mir.tree.node.encode_local_node_id(
            writer, entry_value_lifetimes_by_type_0[0]
        )
        writer.write_unsigned(len(entry_value_lifetimes_by_type_0[1]))
        for item_entry_value_lifetimes_by_type_0_1_1 in entry_value_lifetimes_by_type_0[
            1
        ]:
            destack._generated.mir.tree.lifetime.encode_lifetime_parameter(
                writer, item_entry_value_lifetimes_by_type_0_1_1
            )
    writer.write_unsigned(len(value.values))
    for item_value_values_0 in value.values:
        destack._generated.mir.tree.value.encode_value(writer, item_value_values_0)
    writer.write_unsigned(len(value.indices))
    for item_value_indices_0 in value.indices:
        writer.write_unsigned(item_value_indices_0)
    writer.write_unsigned(len(value.extents))
    for item_value_extents_0 in value.extents:
        writer.write_unsigned(item_value_extents_0)
    writer.write_byte_slice(value.flags)
    writer.write_unsigned(len(value.switch_cases))
    for item_value_switch_cases_0 in value.switch_cases:
        destack._generated.mir.tree.terminator.encode_switch_case(
            writer, item_value_switch_cases_0
        )
    writer.write_unsigned(len(value.tensor_immediates))
    for item_value_tensor_immediates_0 in value.tensor_immediates:
        destack._generated.mir.tree.immediate.encode_tensor_immediate(
            writer, item_value_tensor_immediates_0
        )


def decode_tree(reader: BinaryReader) -> Tree:
    """Decode one Tree."""
    first_global_id = reader.read_number()
    next_global_id = reader.read_number()
    node_index_by_node_id = [
        decode_node_index_entry(reader) for _ in range(reader.read_number())
    ]
    attributes_by_node_id = {
        reader.read_number(): [
            destack._generated.mir.tree.attribute.decode_attribute(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    source_id_by_node_id = [
        reader.read_option(lambda: reader.read_number())
        for _ in range(reader.read_number())
    ]
    origin_by_node_id = destack._generated.mir.tree.origin.decode_origin_table(reader)
    source_index = destack._generated.source.tree.index.decode_source_index_data(reader)
    source_text = reader.read_option(lambda: reader.read_string())
    tokens = [
        destack._generated.mir.source.token.decode_token(reader)
        for _ in range(reader.read_number())
    ]
    leading_comment_spans_by_node_id = [
        reader.read_option(
            lambda: destack._generated.source.file.model.span.decode_span(reader)
        )
        for _ in range(reader.read_number())
    ]
    attribute_spans_by_node_id = {
        reader.read_number(): [
            destack._generated.source.file.model.span.decode_span(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    keyword_spans_by_node_id = {
        reader.read_number(): destack._generated.source.file.model.span.decode_span(
            reader
        )
        for _ in range(reader.read_number())
    }
    function_parameter_spans_by_node_id = {
        reader.read_number(): [
            destack._generated.mir.tree.trivia.decode_typed_value_span(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    function_header_spans_by_node_id = {
        reader.read_number(): destack._generated.mir.tree.trivia.decode_function_header_spans(
            reader
        )
        for _ in range(reader.read_number())
    }
    type_field_spans_by_node_id = {
        reader.read_number(): [
            destack._generated.mir.tree.trivia.decode_field_span(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    type_declaration_spans_by_node_id = {
        reader.read_number(): destack._generated.mir.tree.trivia.decode_type_declaration_spans(
            reader
        )
        for _ in range(reader.read_number())
    }
    functions = [
        destack._generated.mir.tree.function.decode_function(reader)
        for _ in range(reader.read_number())
    ]
    blocks = [
        destack._generated.mir.tree.block.decode_block(reader)
        for _ in range(reader.read_number())
    ]
    instructions = [
        destack._generated.mir.tree.instruction.decode_instruction(reader)
        for _ in range(reader.read_number())
    ]
    terminators = [
        destack._generated.mir.tree.terminator.decode_terminator(reader)
        for _ in range(reader.read_number())
    ]
    locals = [
        destack._generated.mir.tree.local.decode_local(reader)
        for _ in range(reader.read_number())
    ]
    types = [
        destack._generated.mir.tree.type.decode_type(reader)
        for _ in range(reader.read_number())
    ]
    type_aliases = [
        destack._generated.mir.tree.type.decode_type_alias(reader)
        for _ in range(reader.read_number())
    ]
    fields = [
        destack._generated.mir.tree.type.decode_field(reader)
        for _ in range(reader.read_number())
    ]
    globals = [
        destack._generated.mir.tree.global_.decode_global(reader)
        for _ in range(reader.read_number())
    ]
    lifetimes_by_type = {
        destack._generated.mir.tree.node.decode_local_node_id(reader): [
            destack._generated.mir.tree.lifetime.decode_lifetime_parameter(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    values = [
        destack._generated.mir.tree.value.decode_value(reader)
        for _ in range(reader.read_number())
    ]
    indices = [reader.read_number() for _ in range(reader.read_number())]
    extents = [reader.read_number() for _ in range(reader.read_number())]
    flags = reader.read_byte_slice()
    switch_cases = [
        destack._generated.mir.tree.terminator.decode_switch_case(reader)
        for _ in range(reader.read_number())
    ]
    tensor_immediates = [
        destack._generated.mir.tree.immediate.decode_tensor_immediate(reader)
        for _ in range(reader.read_number())
    ]

    return Tree(
        first_global_id=first_global_id,
        next_global_id=next_global_id,
        node_index_by_node_id=node_index_by_node_id,
        attributes_by_node_id=attributes_by_node_id,
        source_id_by_node_id=source_id_by_node_id,
        origin_by_node_id=origin_by_node_id,
        source_index=source_index,
        source_text=source_text,
        tokens=tokens,
        leading_comment_spans_by_node_id=leading_comment_spans_by_node_id,
        attribute_spans_by_node_id=attribute_spans_by_node_id,
        keyword_spans_by_node_id=keyword_spans_by_node_id,
        function_parameter_spans_by_node_id=function_parameter_spans_by_node_id,
        function_header_spans_by_node_id=function_header_spans_by_node_id,
        type_field_spans_by_node_id=type_field_spans_by_node_id,
        type_declaration_spans_by_node_id=type_declaration_spans_by_node_id,
        functions=functions,
        blocks=blocks,
        instructions=instructions,
        terminators=terminators,
        locals=locals,
        types=types,
        type_aliases=type_aliases,
        fields=fields,
        globals=globals,
        lifetimes_by_type=lifetimes_by_type,
        values=values,
        indices=indices,
        extents=extents,
        flags=flags,
        switch_cases=switch_cases,
        tensor_immediates=tensor_immediates,
    )


def to_json_tree(value: Tree) -> Json:
    """Return one JSON value for one Tree."""
    return {
        "firstGlobalId": value.first_global_id,
        "nextGlobalId": value.next_global_id,
        "nodeIndexByNodeId": [
            to_json_node_index_entry(item_0) for item_0 in value.node_index_by_node_id
        ],
        "attributesByNodeId": [
            [
                key_0,
                [
                    destack._generated.mir.tree.attribute.to_json_attribute(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.attributes_by_node_id.items()
        ],
        "sourceIdByNodeId": [
            None if item_0 is None else item_0 for item_0 in value.source_id_by_node_id
        ],
        "originByNodeId": destack._generated.mir.tree.origin.to_json_origin_table(
            value.origin_by_node_id
        ),
        "sourceIndex": destack._generated.source.tree.index.to_json_source_index_data(
            value.source_index
        ),
        **({} if value.source_text is None else {"sourceText": value.source_text}),
        "tokens": [
            destack._generated.mir.source.token.to_json_token(item_0)
            for item_0 in value.tokens
        ],
        "leadingCommentSpansByNodeId": [
            None
            if item_0 is None
            else destack._generated.source.file.model.span.to_json_span(item_0)
            for item_0 in value.leading_comment_spans_by_node_id
        ],
        "attributeSpansByNodeId": [
            [
                key_0,
                [
                    destack._generated.source.file.model.span.to_json_span(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.attribute_spans_by_node_id.items()
        ],
        "keywordSpansByNodeId": [
            [key_0, destack._generated.source.file.model.span.to_json_span(item_0)]
            for key_0, item_0 in value.keyword_spans_by_node_id.items()
        ],
        "functionParameterSpansByNodeId": [
            [
                key_0,
                [
                    destack._generated.mir.tree.trivia.to_json_typed_value_span(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.function_parameter_spans_by_node_id.items()
        ],
        "functionHeaderSpansByNodeId": [
            [
                key_0,
                destack._generated.mir.tree.trivia.to_json_function_header_spans(
                    item_0
                ),
            ]
            for key_0, item_0 in value.function_header_spans_by_node_id.items()
        ],
        "typeFieldSpansByNodeId": [
            [
                key_0,
                [
                    destack._generated.mir.tree.trivia.to_json_field_span(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.type_field_spans_by_node_id.items()
        ],
        "typeDeclarationSpansByNodeId": [
            [
                key_0,
                destack._generated.mir.tree.trivia.to_json_type_declaration_spans(
                    item_0
                ),
            ]
            for key_0, item_0 in value.type_declaration_spans_by_node_id.items()
        ],
        "functions": [
            destack._generated.mir.tree.function.to_json_function(item_0)
            for item_0 in value.functions
        ],
        "blocks": [
            destack._generated.mir.tree.block.to_json_block(item_0)
            for item_0 in value.blocks
        ],
        "instructions": [
            destack._generated.mir.tree.instruction.to_json_instruction(item_0)
            for item_0 in value.instructions
        ],
        "terminators": [
            destack._generated.mir.tree.terminator.to_json_terminator(item_0)
            for item_0 in value.terminators
        ],
        "locals": [
            destack._generated.mir.tree.local.to_json_local(item_0)
            for item_0 in value.locals
        ],
        "types": [
            destack._generated.mir.tree.type.to_json_type(item_0)
            for item_0 in value.types
        ],
        "typeAliases": [
            destack._generated.mir.tree.type.to_json_type_alias(item_0)
            for item_0 in value.type_aliases
        ],
        "fields": [
            destack._generated.mir.tree.type.to_json_field(item_0)
            for item_0 in value.fields
        ],
        "globals": [
            destack._generated.mir.tree.global_.to_json_global(item_0)
            for item_0 in value.globals
        ],
        "lifetimesByType": [
            [
                destack._generated.mir.tree.node.to_json_local_node_id(key_0),
                [
                    destack._generated.mir.tree.lifetime.to_json_lifetime_parameter(
                        item_1
                    )
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.lifetimes_by_type.items()
        ],
        "values": [
            destack._generated.mir.tree.value.to_json_value(item_0)
            for item_0 in value.values
        ],
        "indices": [item_0 for item_0 in value.indices],
        "extents": [item_0 for item_0 in value.extents],
        "flags": bytes_to_json(value.flags),
        "switchCases": [
            destack._generated.mir.tree.terminator.to_json_switch_case(item_0)
            for item_0 in value.switch_cases
        ],
        "tensorImmediates": [
            destack._generated.mir.tree.immediate.to_json_tensor_immediate(item_0)
            for item_0 in value.tensor_immediates
        ],
    }


def from_json_tree(value: Json) -> Tree:
    """Return one Tree from one JSON value."""
    object_ = json_object(value)

    return Tree(
        first_global_id=json_int(json_field(object_, "firstGlobalId")),
        next_global_id=json_int(json_field(object_, "nextGlobalId")),
        node_index_by_node_id=[
            from_json_node_index_entry(item_0)
            for item_0 in json_array(json_field(object_, "nodeIndexByNodeId"))
        ],
        attributes_by_node_id={
            json_int(key_0): [
                destack._generated.mir.tree.attribute.from_json_attribute(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "attributesByNodeId"))
        },
        source_id_by_node_id=[
            None if item_0 is None else json_int(item_0)
            for item_0 in json_array(json_field(object_, "sourceIdByNodeId"))
        ],
        origin_by_node_id=destack._generated.mir.tree.origin.from_json_origin_table(
            json_field(object_, "originByNodeId")
        ),
        source_index=destack._generated.source.tree.index.from_json_source_index_data(
            json_field(object_, "sourceIndex")
        ),
        source_text=json_optional(
            object_, "sourceText", lambda value: json_string(value)
        ),
        tokens=[
            destack._generated.mir.source.token.from_json_token(item_0)
            for item_0 in json_array(json_field(object_, "tokens"))
        ],
        leading_comment_spans_by_node_id=[
            None
            if item_0 is None
            else destack._generated.source.file.model.span.from_json_span(item_0)
            for item_0 in json_array(json_field(object_, "leadingCommentSpansByNodeId"))
        ],
        attribute_spans_by_node_id={
            json_int(key_0): [
                destack._generated.source.file.model.span.from_json_span(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "attributeSpansByNodeId")
            )
        },
        keyword_spans_by_node_id={
            json_int(key_0): destack._generated.source.file.model.span.from_json_span(
                item_0
            )
            for key_0, item_0 in json_array(json_field(object_, "keywordSpansByNodeId"))
        },
        function_parameter_spans_by_node_id={
            json_int(key_0): [
                destack._generated.mir.tree.trivia.from_json_typed_value_span(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "functionParameterSpansByNodeId")
            )
        },
        function_header_spans_by_node_id={
            json_int(
                key_0
            ): destack._generated.mir.tree.trivia.from_json_function_header_spans(
                item_0
            )
            for key_0, item_0 in json_array(
                json_field(object_, "functionHeaderSpansByNodeId")
            )
        },
        type_field_spans_by_node_id={
            json_int(key_0): [
                destack._generated.mir.tree.trivia.from_json_field_span(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(
                json_field(object_, "typeFieldSpansByNodeId")
            )
        },
        type_declaration_spans_by_node_id={
            json_int(
                key_0
            ): destack._generated.mir.tree.trivia.from_json_type_declaration_spans(
                item_0
            )
            for key_0, item_0 in json_array(
                json_field(object_, "typeDeclarationSpansByNodeId")
            )
        },
        functions=[
            destack._generated.mir.tree.function.from_json_function(item_0)
            for item_0 in json_array(json_field(object_, "functions"))
        ],
        blocks=[
            destack._generated.mir.tree.block.from_json_block(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        instructions=[
            destack._generated.mir.tree.instruction.from_json_instruction(item_0)
            for item_0 in json_array(json_field(object_, "instructions"))
        ],
        terminators=[
            destack._generated.mir.tree.terminator.from_json_terminator(item_0)
            for item_0 in json_array(json_field(object_, "terminators"))
        ],
        locals=[
            destack._generated.mir.tree.local.from_json_local(item_0)
            for item_0 in json_array(json_field(object_, "locals"))
        ],
        types=[
            destack._generated.mir.tree.type.from_json_type(item_0)
            for item_0 in json_array(json_field(object_, "types"))
        ],
        type_aliases=[
            destack._generated.mir.tree.type.from_json_type_alias(item_0)
            for item_0 in json_array(json_field(object_, "typeAliases"))
        ],
        fields=[
            destack._generated.mir.tree.type.from_json_field(item_0)
            for item_0 in json_array(json_field(object_, "fields"))
        ],
        globals=[
            destack._generated.mir.tree.global_.from_json_global(item_0)
            for item_0 in json_array(json_field(object_, "globals"))
        ],
        lifetimes_by_type={
            destack._generated.mir.tree.node.from_json_local_node_id(key_0): [
                destack._generated.mir.tree.lifetime.from_json_lifetime_parameter(
                    item_1
                )
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "lifetimesByType"))
        },
        values=[
            destack._generated.mir.tree.value.from_json_value(item_0)
            for item_0 in json_array(json_field(object_, "values"))
        ],
        indices=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "indices"))
        ],
        extents=[
            json_int(item_0) for item_0 in json_array(json_field(object_, "extents"))
        ],
        flags=bytes_from_json(json_field(object_, "flags")),
        switch_cases=[
            destack._generated.mir.tree.terminator.from_json_switch_case(item_0)
            for item_0 in json_array(json_field(object_, "switchCases"))
        ],
        tensor_immediates=[
            destack._generated.mir.tree.immediate.from_json_tensor_immediate(item_0)
            for item_0 in json_array(json_field(object_, "tensorImmediates"))
        ],
    )


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
