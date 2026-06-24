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
    json_field,
    json_int,
    json_object,
    json_optional,
    json_string,
    nested_bytes,
)

import destack._generated.dir.symbol.symbol
import destack._generated.js.tree.annotation
import destack._generated.js.tree.argument
import destack._generated.js.tree.block
import destack._generated.js.tree.declaration
import destack._generated.js.tree.declarator
import destack._generated.js.tree.dependency
import destack._generated.js.tree.expression
import destack._generated.js.tree.node
import destack._generated.js.tree.pattern
import destack._generated.js.tree.property
import destack._generated.js.tree.statement
import destack._generated.js.tree.type
import destack._generated.source.file.model.module


@dataclass(frozen=True, slots=True)
class Tree:
    """Mutable AST tree for a single source unit. NOT THREAD-SAFE."""

    # the next id to allocate
    next_global_id: int
    # dense local id and node type metadata by node id
    node_index_by_node_id: Sequence[NodeIndexEntry]
    # the annotations attached to nodes
    annotations_by_node_id: Mapping[
        int, Sequence[destack._generated.js.tree.node.LocalNodeId]
    ]
    # the origin node for each JS node
    origin_by_node_id: Sequence[NodeOrigin | None]
    # the alias node id by DIR node id
    alias_node_id_by_dir_id: Mapping[int, int]
    # the alias node id by JS AST node id
    alias_node_id_by_node_id: Mapping[int, int]
    # the symbol identity by JS AST node id
    symbol_id_by_node_id: Sequence[ScriptSymbolId | None]
    blocks: Sequence[destack._generated.js.tree.block.Block]
    catch_clauses: Sequence[destack._generated.js.tree.block.CatchClause]
    statements: Sequence[destack._generated.js.tree.statement.Statement]
    expressions: Sequence[destack._generated.js.tree.expression.Expression]
    array_elements: Sequence[destack._generated.js.tree.expression.ArrayElement]
    declarations: Sequence[destack._generated.js.tree.declaration.Declaration]
    declarators: Sequence[destack._generated.js.tree.declarator.Declarator]
    properties: Sequence[destack._generated.js.tree.property.Property]
    members: Sequence[destack._generated.js.tree.property.Member]
    type_expressions: Sequence[destack._generated.js.tree.type.TypeExpression]
    tuple_elements: Sequence[destack._generated.js.tree.type.TupleElement]
    type_members: Sequence[destack._generated.js.tree.type.TypeMember]
    enum_fields: Sequence[destack._generated.js.tree.declaration.EnumField]
    dependency_items: Sequence[destack._generated.js.tree.dependency.DependencyItem]
    switch_cases: Sequence[destack._generated.js.tree.block.SwitchCase]
    generic_parameters: Sequence[destack._generated.js.tree.argument.GenericParameter]
    parameters: Sequence[destack._generated.js.tree.argument.Parameter]
    arguments: Sequence[destack._generated.js.tree.argument.Argument]
    patterns: Sequence[destack._generated.js.tree.pattern.Pattern]
    pattern_fields: Sequence[destack._generated.js.tree.pattern.PatternField]
    assign_patterns: Sequence[destack._generated.js.tree.pattern.AssignPattern]
    assign_pattern_fields: Sequence[
        destack._generated.js.tree.pattern.AssignPatternField
    ]
    annotations: Sequence[destack._generated.js.tree.annotation.Annotation]

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
    writer.write_unsigned(value.next_global_id)
    writer.write_unsigned(len(value.node_index_by_node_id))
    for item_value_node_index_by_node_id_0 in value.node_index_by_node_id:
        encode_node_index_entry(writer, item_value_node_index_by_node_id_0)
    entries_value_annotations_by_node_id_0 = []
    for (
        key_value_annotations_by_node_id_0,
        item_value_annotations_by_node_id_0,
    ) in value.annotations_by_node_id.items():

        def write_key_value_annotations_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_annotations_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_annotations_by_node_id_0)
        entries_value_annotations_by_node_id_0.append(
            (
                key_value_annotations_by_node_id_0,
                item_value_annotations_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_annotations_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_annotations_by_node_id_0))
    for entry_value_annotations_by_node_id_0 in entries_value_annotations_by_node_id_0:
        writer.write_unsigned(entry_value_annotations_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_annotations_by_node_id_0[1]))
        for (
            item_entry_value_annotations_by_node_id_0_1_1
        ) in entry_value_annotations_by_node_id_0[1]:
            destack._generated.js.tree.node.encode_local_node_id(
                writer, item_entry_value_annotations_by_node_id_0_1_1
            )
    writer.write_unsigned(len(value.origin_by_node_id))
    for item_value_origin_by_node_id_0 in value.origin_by_node_id:
        if item_value_origin_by_node_id_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_node_origin(writer, item_value_origin_by_node_id_0)
    entries_value_alias_node_id_by_dir_id_0 = []
    for (
        key_value_alias_node_id_by_dir_id_0,
        item_value_alias_node_id_by_dir_id_0,
    ) in value.alias_node_id_by_dir_id.items():

        def write_key_value_alias_node_id_by_dir_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_alias_node_id_by_dir_id_0)

        key_bytes = nested_bytes(write_key_value_alias_node_id_by_dir_id_0)
        entries_value_alias_node_id_by_dir_id_0.append(
            (
                key_value_alias_node_id_by_dir_id_0,
                item_value_alias_node_id_by_dir_id_0,
                key_bytes,
            )
        )
    entries_value_alias_node_id_by_dir_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_alias_node_id_by_dir_id_0))
    for (
        entry_value_alias_node_id_by_dir_id_0
    ) in entries_value_alias_node_id_by_dir_id_0:
        writer.write_unsigned(entry_value_alias_node_id_by_dir_id_0[0])
        writer.write_unsigned(entry_value_alias_node_id_by_dir_id_0[1])
    entries_value_alias_node_id_by_node_id_0 = []
    for (
        key_value_alias_node_id_by_node_id_0,
        item_value_alias_node_id_by_node_id_0,
    ) in value.alias_node_id_by_node_id.items():

        def write_key_value_alias_node_id_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_alias_node_id_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_alias_node_id_by_node_id_0)
        entries_value_alias_node_id_by_node_id_0.append(
            (
                key_value_alias_node_id_by_node_id_0,
                item_value_alias_node_id_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_alias_node_id_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_alias_node_id_by_node_id_0))
    for (
        entry_value_alias_node_id_by_node_id_0
    ) in entries_value_alias_node_id_by_node_id_0:
        writer.write_unsigned(entry_value_alias_node_id_by_node_id_0[0])
        writer.write_unsigned(entry_value_alias_node_id_by_node_id_0[1])
    writer.write_unsigned(len(value.symbol_id_by_node_id))
    for item_value_symbol_id_by_node_id_0 in value.symbol_id_by_node_id:
        if item_value_symbol_id_by_node_id_0 is None:
            writer.write_byte(0)
        else:
            writer.write_byte(1)
            encode_script_symbol_id(writer, item_value_symbol_id_by_node_id_0)
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        destack._generated.js.tree.block.encode_block(writer, item_value_blocks_0)
    writer.write_unsigned(len(value.catch_clauses))
    for item_value_catch_clauses_0 in value.catch_clauses:
        destack._generated.js.tree.block.encode_catch_clause(
            writer, item_value_catch_clauses_0
        )
    writer.write_unsigned(len(value.statements))
    for item_value_statements_0 in value.statements:
        destack._generated.js.tree.statement.encode_statement(
            writer, item_value_statements_0
        )
    writer.write_unsigned(len(value.expressions))
    for item_value_expressions_0 in value.expressions:
        destack._generated.js.tree.expression.encode_expression(
            writer, item_value_expressions_0
        )
    writer.write_unsigned(len(value.array_elements))
    for item_value_array_elements_0 in value.array_elements:
        destack._generated.js.tree.expression.encode_array_element(
            writer, item_value_array_elements_0
        )
    writer.write_unsigned(len(value.declarations))
    for item_value_declarations_0 in value.declarations:
        destack._generated.js.tree.declaration.encode_declaration(
            writer, item_value_declarations_0
        )
    writer.write_unsigned(len(value.declarators))
    for item_value_declarators_0 in value.declarators:
        destack._generated.js.tree.declarator.encode_declarator(
            writer, item_value_declarators_0
        )
    writer.write_unsigned(len(value.properties))
    for item_value_properties_0 in value.properties:
        destack._generated.js.tree.property.encode_property(
            writer, item_value_properties_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.js.tree.property.encode_member(writer, item_value_members_0)
    writer.write_unsigned(len(value.type_expressions))
    for item_value_type_expressions_0 in value.type_expressions:
        destack._generated.js.tree.type.encode_type_expression(
            writer, item_value_type_expressions_0
        )
    writer.write_unsigned(len(value.tuple_elements))
    for item_value_tuple_elements_0 in value.tuple_elements:
        destack._generated.js.tree.type.encode_tuple_element(
            writer, item_value_tuple_elements_0
        )
    writer.write_unsigned(len(value.type_members))
    for item_value_type_members_0 in value.type_members:
        destack._generated.js.tree.type.encode_type_member(
            writer, item_value_type_members_0
        )
    writer.write_unsigned(len(value.enum_fields))
    for item_value_enum_fields_0 in value.enum_fields:
        destack._generated.js.tree.declaration.encode_enum_field(
            writer, item_value_enum_fields_0
        )
    writer.write_unsigned(len(value.dependency_items))
    for item_value_dependency_items_0 in value.dependency_items:
        destack._generated.js.tree.dependency.encode_dependency_item(
            writer, item_value_dependency_items_0
        )
    writer.write_unsigned(len(value.switch_cases))
    for item_value_switch_cases_0 in value.switch_cases:
        destack._generated.js.tree.block.encode_switch_case(
            writer, item_value_switch_cases_0
        )
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.js.tree.argument.encode_generic_parameter(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.js.tree.argument.encode_parameter(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.js.tree.argument.encode_argument(
            writer, item_value_arguments_0
        )
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        destack._generated.js.tree.pattern.encode_pattern(writer, item_value_patterns_0)
    writer.write_unsigned(len(value.pattern_fields))
    for item_value_pattern_fields_0 in value.pattern_fields:
        destack._generated.js.tree.pattern.encode_pattern_field(
            writer, item_value_pattern_fields_0
        )
    writer.write_unsigned(len(value.assign_patterns))
    for item_value_assign_patterns_0 in value.assign_patterns:
        destack._generated.js.tree.pattern.encode_assign_pattern(
            writer, item_value_assign_patterns_0
        )
    writer.write_unsigned(len(value.assign_pattern_fields))
    for item_value_assign_pattern_fields_0 in value.assign_pattern_fields:
        destack._generated.js.tree.pattern.encode_assign_pattern_field(
            writer, item_value_assign_pattern_fields_0
        )
    writer.write_unsigned(len(value.annotations))
    for item_value_annotations_0 in value.annotations:
        destack._generated.js.tree.annotation.encode_annotation(
            writer, item_value_annotations_0
        )


def decode_tree(reader: BinaryReader) -> Tree:
    """Decode one Tree."""
    next_global_id = reader.read_number()
    node_index_by_node_id = [
        decode_node_index_entry(reader) for _ in range(reader.read_number())
    ]
    annotations_by_node_id = {
        reader.read_number(): [
            destack._generated.js.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    origin_by_node_id = [
        reader.read_option(lambda: decode_node_origin(reader))
        for _ in range(reader.read_number())
    ]
    alias_node_id_by_dir_id = {
        reader.read_number(): reader.read_number() for _ in range(reader.read_number())
    }
    alias_node_id_by_node_id = {
        reader.read_number(): reader.read_number() for _ in range(reader.read_number())
    }
    symbol_id_by_node_id = [
        reader.read_option(lambda: decode_script_symbol_id(reader))
        for _ in range(reader.read_number())
    ]
    blocks = [
        destack._generated.js.tree.block.decode_block(reader)
        for _ in range(reader.read_number())
    ]
    catch_clauses = [
        destack._generated.js.tree.block.decode_catch_clause(reader)
        for _ in range(reader.read_number())
    ]
    statements = [
        destack._generated.js.tree.statement.decode_statement(reader)
        for _ in range(reader.read_number())
    ]
    expressions = [
        destack._generated.js.tree.expression.decode_expression(reader)
        for _ in range(reader.read_number())
    ]
    array_elements = [
        destack._generated.js.tree.expression.decode_array_element(reader)
        for _ in range(reader.read_number())
    ]
    declarations = [
        destack._generated.js.tree.declaration.decode_declaration(reader)
        for _ in range(reader.read_number())
    ]
    declarators = [
        destack._generated.js.tree.declarator.decode_declarator(reader)
        for _ in range(reader.read_number())
    ]
    properties = [
        destack._generated.js.tree.property.decode_property(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.js.tree.property.decode_member(reader)
        for _ in range(reader.read_number())
    ]
    type_expressions = [
        destack._generated.js.tree.type.decode_type_expression(reader)
        for _ in range(reader.read_number())
    ]
    tuple_elements = [
        destack._generated.js.tree.type.decode_tuple_element(reader)
        for _ in range(reader.read_number())
    ]
    type_members = [
        destack._generated.js.tree.type.decode_type_member(reader)
        for _ in range(reader.read_number())
    ]
    enum_fields = [
        destack._generated.js.tree.declaration.decode_enum_field(reader)
        for _ in range(reader.read_number())
    ]
    dependency_items = [
        destack._generated.js.tree.dependency.decode_dependency_item(reader)
        for _ in range(reader.read_number())
    ]
    switch_cases = [
        destack._generated.js.tree.block.decode_switch_case(reader)
        for _ in range(reader.read_number())
    ]
    generic_parameters = [
        destack._generated.js.tree.argument.decode_generic_parameter(reader)
        for _ in range(reader.read_number())
    ]
    parameters = [
        destack._generated.js.tree.argument.decode_parameter(reader)
        for _ in range(reader.read_number())
    ]
    arguments = [
        destack._generated.js.tree.argument.decode_argument(reader)
        for _ in range(reader.read_number())
    ]
    patterns = [
        destack._generated.js.tree.pattern.decode_pattern(reader)
        for _ in range(reader.read_number())
    ]
    pattern_fields = [
        destack._generated.js.tree.pattern.decode_pattern_field(reader)
        for _ in range(reader.read_number())
    ]
    assign_patterns = [
        destack._generated.js.tree.pattern.decode_assign_pattern(reader)
        for _ in range(reader.read_number())
    ]
    assign_pattern_fields = [
        destack._generated.js.tree.pattern.decode_assign_pattern_field(reader)
        for _ in range(reader.read_number())
    ]
    annotations = [
        destack._generated.js.tree.annotation.decode_annotation(reader)
        for _ in range(reader.read_number())
    ]

    return Tree(
        next_global_id=next_global_id,
        node_index_by_node_id=node_index_by_node_id,
        annotations_by_node_id=annotations_by_node_id,
        origin_by_node_id=origin_by_node_id,
        alias_node_id_by_dir_id=alias_node_id_by_dir_id,
        alias_node_id_by_node_id=alias_node_id_by_node_id,
        symbol_id_by_node_id=symbol_id_by_node_id,
        blocks=blocks,
        catch_clauses=catch_clauses,
        statements=statements,
        expressions=expressions,
        array_elements=array_elements,
        declarations=declarations,
        declarators=declarators,
        properties=properties,
        members=members,
        type_expressions=type_expressions,
        tuple_elements=tuple_elements,
        type_members=type_members,
        enum_fields=enum_fields,
        dependency_items=dependency_items,
        switch_cases=switch_cases,
        generic_parameters=generic_parameters,
        parameters=parameters,
        arguments=arguments,
        patterns=patterns,
        pattern_fields=pattern_fields,
        assign_patterns=assign_patterns,
        assign_pattern_fields=assign_pattern_fields,
        annotations=annotations,
    )


def to_json_tree(value: Tree) -> Json:
    """Return one JSON value for one Tree."""
    return {
        "nextGlobalId": value.next_global_id,
        "nodeIndexByNodeId": [
            to_json_node_index_entry(item_0) for item_0 in value.node_index_by_node_id
        ],
        "annotationsByNodeId": [
            [
                key_0,
                [
                    destack._generated.js.tree.node.to_json_local_node_id(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.annotations_by_node_id.items()
        ],
        "originByNodeId": [
            None if item_0 is None else to_json_node_origin(item_0)
            for item_0 in value.origin_by_node_id
        ],
        "aliasNodeIdByDirId": [
            [key_0, item_0] for key_0, item_0 in value.alias_node_id_by_dir_id.items()
        ],
        "aliasNodeIdByNodeId": [
            [key_0, item_0] for key_0, item_0 in value.alias_node_id_by_node_id.items()
        ],
        "symbolIdByNodeId": [
            None if item_0 is None else to_json_script_symbol_id(item_0)
            for item_0 in value.symbol_id_by_node_id
        ],
        "blocks": [
            destack._generated.js.tree.block.to_json_block(item_0)
            for item_0 in value.blocks
        ],
        "catchClauses": [
            destack._generated.js.tree.block.to_json_catch_clause(item_0)
            for item_0 in value.catch_clauses
        ],
        "statements": [
            destack._generated.js.tree.statement.to_json_statement(item_0)
            for item_0 in value.statements
        ],
        "expressions": [
            destack._generated.js.tree.expression.to_json_expression(item_0)
            for item_0 in value.expressions
        ],
        "arrayElements": [
            destack._generated.js.tree.expression.to_json_array_element(item_0)
            for item_0 in value.array_elements
        ],
        "declarations": [
            destack._generated.js.tree.declaration.to_json_declaration(item_0)
            for item_0 in value.declarations
        ],
        "declarators": [
            destack._generated.js.tree.declarator.to_json_declarator(item_0)
            for item_0 in value.declarators
        ],
        "properties": [
            destack._generated.js.tree.property.to_json_property(item_0)
            for item_0 in value.properties
        ],
        "members": [
            destack._generated.js.tree.property.to_json_member(item_0)
            for item_0 in value.members
        ],
        "typeExpressions": [
            destack._generated.js.tree.type.to_json_type_expression(item_0)
            for item_0 in value.type_expressions
        ],
        "tupleElements": [
            destack._generated.js.tree.type.to_json_tuple_element(item_0)
            for item_0 in value.tuple_elements
        ],
        "typeMembers": [
            destack._generated.js.tree.type.to_json_type_member(item_0)
            for item_0 in value.type_members
        ],
        "enumFields": [
            destack._generated.js.tree.declaration.to_json_enum_field(item_0)
            for item_0 in value.enum_fields
        ],
        "dependencyItems": [
            destack._generated.js.tree.dependency.to_json_dependency_item(item_0)
            for item_0 in value.dependency_items
        ],
        "switchCases": [
            destack._generated.js.tree.block.to_json_switch_case(item_0)
            for item_0 in value.switch_cases
        ],
        "genericParameters": [
            destack._generated.js.tree.argument.to_json_generic_parameter(item_0)
            for item_0 in value.generic_parameters
        ],
        "parameters": [
            destack._generated.js.tree.argument.to_json_parameter(item_0)
            for item_0 in value.parameters
        ],
        "arguments": [
            destack._generated.js.tree.argument.to_json_argument(item_0)
            for item_0 in value.arguments
        ],
        "patterns": [
            destack._generated.js.tree.pattern.to_json_pattern(item_0)
            for item_0 in value.patterns
        ],
        "patternFields": [
            destack._generated.js.tree.pattern.to_json_pattern_field(item_0)
            for item_0 in value.pattern_fields
        ],
        "assignPatterns": [
            destack._generated.js.tree.pattern.to_json_assign_pattern(item_0)
            for item_0 in value.assign_patterns
        ],
        "assignPatternFields": [
            destack._generated.js.tree.pattern.to_json_assign_pattern_field(item_0)
            for item_0 in value.assign_pattern_fields
        ],
        "annotations": [
            destack._generated.js.tree.annotation.to_json_annotation(item_0)
            for item_0 in value.annotations
        ],
    }


def from_json_tree(value: Json) -> Tree:
    """Return one Tree from one JSON value."""
    object_ = json_object(value)

    return Tree(
        next_global_id=json_int(json_field(object_, "nextGlobalId")),
        node_index_by_node_id=[
            from_json_node_index_entry(item_0)
            for item_0 in json_array(json_field(object_, "nodeIndexByNodeId"))
        ],
        annotations_by_node_id={
            json_int(key_0): [
                destack._generated.js.tree.node.from_json_local_node_id(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "annotationsByNodeId"))
        },
        origin_by_node_id=[
            None if item_0 is None else from_json_node_origin(item_0)
            for item_0 in json_array(json_field(object_, "originByNodeId"))
        ],
        alias_node_id_by_dir_id={
            json_int(key_0): json_int(item_0)
            for key_0, item_0 in json_array(json_field(object_, "aliasNodeIdByDirId"))
        },
        alias_node_id_by_node_id={
            json_int(key_0): json_int(item_0)
            for key_0, item_0 in json_array(json_field(object_, "aliasNodeIdByNodeId"))
        },
        symbol_id_by_node_id=[
            None if item_0 is None else from_json_script_symbol_id(item_0)
            for item_0 in json_array(json_field(object_, "symbolIdByNodeId"))
        ],
        blocks=[
            destack._generated.js.tree.block.from_json_block(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        catch_clauses=[
            destack._generated.js.tree.block.from_json_catch_clause(item_0)
            for item_0 in json_array(json_field(object_, "catchClauses"))
        ],
        statements=[
            destack._generated.js.tree.statement.from_json_statement(item_0)
            for item_0 in json_array(json_field(object_, "statements"))
        ],
        expressions=[
            destack._generated.js.tree.expression.from_json_expression(item_0)
            for item_0 in json_array(json_field(object_, "expressions"))
        ],
        array_elements=[
            destack._generated.js.tree.expression.from_json_array_element(item_0)
            for item_0 in json_array(json_field(object_, "arrayElements"))
        ],
        declarations=[
            destack._generated.js.tree.declaration.from_json_declaration(item_0)
            for item_0 in json_array(json_field(object_, "declarations"))
        ],
        declarators=[
            destack._generated.js.tree.declarator.from_json_declarator(item_0)
            for item_0 in json_array(json_field(object_, "declarators"))
        ],
        properties=[
            destack._generated.js.tree.property.from_json_property(item_0)
            for item_0 in json_array(json_field(object_, "properties"))
        ],
        members=[
            destack._generated.js.tree.property.from_json_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        type_expressions=[
            destack._generated.js.tree.type.from_json_type_expression(item_0)
            for item_0 in json_array(json_field(object_, "typeExpressions"))
        ],
        tuple_elements=[
            destack._generated.js.tree.type.from_json_tuple_element(item_0)
            for item_0 in json_array(json_field(object_, "tupleElements"))
        ],
        type_members=[
            destack._generated.js.tree.type.from_json_type_member(item_0)
            for item_0 in json_array(json_field(object_, "typeMembers"))
        ],
        enum_fields=[
            destack._generated.js.tree.declaration.from_json_enum_field(item_0)
            for item_0 in json_array(json_field(object_, "enumFields"))
        ],
        dependency_items=[
            destack._generated.js.tree.dependency.from_json_dependency_item(item_0)
            for item_0 in json_array(json_field(object_, "dependencyItems"))
        ],
        switch_cases=[
            destack._generated.js.tree.block.from_json_switch_case(item_0)
            for item_0 in json_array(json_field(object_, "switchCases"))
        ],
        generic_parameters=[
            destack._generated.js.tree.argument.from_json_generic_parameter(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        parameters=[
            destack._generated.js.tree.argument.from_json_parameter(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        arguments=[
            destack._generated.js.tree.argument.from_json_argument(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        patterns=[
            destack._generated.js.tree.pattern.from_json_pattern(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
        pattern_fields=[
            destack._generated.js.tree.pattern.from_json_pattern_field(item_0)
            for item_0 in json_array(json_field(object_, "patternFields"))
        ],
        assign_patterns=[
            destack._generated.js.tree.pattern.from_json_assign_pattern(item_0)
            for item_0 in json_array(json_field(object_, "assignPatterns"))
        ],
        assign_pattern_fields=[
            destack._generated.js.tree.pattern.from_json_assign_pattern_field(item_0)
            for item_0 in json_array(json_field(object_, "assignPatternFields"))
        ],
        annotations=[
            destack._generated.js.tree.annotation.from_json_annotation(item_0)
            for item_0 in json_array(json_field(object_, "annotations"))
        ],
    )


@dataclass(frozen=True, slots=True)
class NodeIndexEntry:
    """Dense metadata for one JS node id."""

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
class NodeOrigin:
    """One DIR node that produced a JS node."""

    # the origin module
    module_id: destack._generated.source.file.model.module.ModuleId
    # the origin DIR node id
    node_id: int

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_node_origin(writer, self)

    @classmethod
    def decode(cls, reader: BinaryReader) -> NodeOrigin:
        """Decode one NodeOrigin."""
        return decode_node_origin(reader)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_node_origin(self)

    @classmethod
    def from_json(cls, value: Json) -> NodeOrigin:
        """Return one NodeOrigin from one JSON value."""
        return from_json_node_origin(value)


def encode_node_origin(writer: BinaryWriter, value: NodeOrigin) -> None:
    """Encode one NodeOrigin."""
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.node_id)


def decode_node_origin(reader: BinaryReader) -> NodeOrigin:
    """Decode one NodeOrigin."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    node_id = reader.read_number()

    return NodeOrigin(
        module_id=module_id,
        node_id=node_id,
    )


def to_json_node_origin(value: NodeOrigin) -> Json:
    """Return one JSON value for one NodeOrigin."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "nodeId": value.node_id,
    }


def from_json_node_origin(value: Json) -> NodeOrigin:
    """Return one NodeOrigin from one JSON value."""
    object_ = json_object(value)

    return NodeOrigin(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        node_id=json_int(json_field(object_, "nodeId")),
    )


@dataclass(frozen=True, slots=True)
class ScriptSymbolIdSource:
    """One symbol lowered directly from source DIR."""

    source: destack._generated.dir.symbol.symbol.GlobalSymbolId
    kind: typing.Literal["source"] = "source"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_script_symbol_id(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_script_symbol_id(self)


@dataclass(frozen=True, slots=True)
class ScriptSymbolIdModuleDefault:
    """One generated default binding for one non-code script module."""

    module_default: destack._generated.source.file.model.module.ModuleId
    kind: typing.Literal["moduleDefault"] = "moduleDefault"

    def encode(self, writer: BinaryWriter) -> None:
        """Encode this value."""
        encode_script_symbol_id(writer, self)

    def to_json(self) -> Json:
        """Return this value as JSON."""
        return to_json_script_symbol_id(self)


"""One stable symbol identity in lowered script output."""
ScriptSymbolId: typing.TypeAlias = ScriptSymbolIdSource | ScriptSymbolIdModuleDefault


def encode_script_symbol_id(writer: BinaryWriter, value: ScriptSymbolId) -> None:
    """Encode one ScriptSymbolId."""
    if value.kind == "source":
        writer.write_unsigned(0)
        destack._generated.dir.symbol.symbol.encode_global_symbol_id(
            writer, value.source
        )
    elif value.kind == "moduleDefault":
        writer.write_unsigned(1)
        destack._generated.source.file.model.module.encode_module_id(
            writer, value.module_default
        )
    else:
        raise SerdeError("unknown enum variant")


def decode_script_symbol_id(reader: BinaryReader) -> ScriptSymbolId:
    """Decode one ScriptSymbolId."""
    variant = reader.read_number()

    if variant == 0:
        source = destack._generated.dir.symbol.symbol.decode_global_symbol_id(reader)

        return ScriptSymbolIdSource(source=source)
    elif variant == 1:
        module_default = destack._generated.source.file.model.module.decode_module_id(
            reader
        )

        return ScriptSymbolIdModuleDefault(module_default=module_default)
    else:
        raise SerdeError(f"unknown enum variant index: {variant}")


def to_json_script_symbol_id(value: ScriptSymbolId) -> Json:
    """Return one JSON value for one ScriptSymbolId."""
    if value.kind == "source":
        return {
            "kind": "source",
            "source": destack._generated.dir.symbol.symbol.to_json_global_symbol_id(
                value.source
            ),
        }
    elif value.kind == "moduleDefault":
        return {
            "kind": "moduleDefault",
            "module_default": destack._generated.source.file.model.module.to_json_module_id(
                value.module_default
            ),
        }
    else:
        raise SerdeError("unknown enum variant")


def from_json_script_symbol_id(value: Json) -> ScriptSymbolId:
    """Return one ScriptSymbolId from one JSON value."""
    object_ = json_object(value)
    kind = json_string(json_field(object_, "kind"))

    if kind == "source":
        return ScriptSymbolIdSource(
            source=destack._generated.dir.symbol.symbol.from_json_global_symbol_id(
                json_field(object_, "source")
            )
        )
    elif kind == "moduleDefault":
        return ScriptSymbolIdModuleDefault(
            module_default=destack._generated.source.file.model.module.from_json_module_id(
                json_field(object_, "module_default")
            )
        )
    else:
        raise SerdeError(f"unknown enum variant: {kind}")


__all__ = [
    "Tree",
    "encode_tree",
    "decode_tree",
    "to_json_tree",
    "from_json_tree",
    "NodeIndexEntry",
    "encode_node_index_entry",
    "decode_node_index_entry",
    "to_json_node_index_entry",
    "from_json_node_index_entry",
    "NodeOrigin",
    "encode_node_origin",
    "decode_node_origin",
    "to_json_node_origin",
    "from_json_node_origin",
    "ScriptSymbolId",
    "encode_script_symbol_id",
    "decode_script_symbol_id",
    "to_json_script_symbol_id",
    "from_json_script_symbol_id",
    "ScriptSymbolIdSource",
    "ScriptSymbolIdModuleDefault",
]
