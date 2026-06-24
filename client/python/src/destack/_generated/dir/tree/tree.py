# generated client target, do not edit

from __future__ import annotations

from collections.abc import Mapping, Sequence
from dataclasses import dataclass

from destack.protocol.serde import (
    BinaryReader,
    BinaryWriter,
    Json,
    json_array,
    json_field,
    json_int,
    json_object,
    nested_bytes,
)

import destack._generated.dir.source.comment
import destack._generated.dir.source.parent
import destack._generated.dir.tree.argument
import destack._generated.dir.tree.block
import destack._generated.dir.tree.declaration
import destack._generated.dir.tree.declarator
import destack._generated.dir.tree.decorator
import destack._generated.dir.tree.dependency
import destack._generated.dir.tree.expression
import destack._generated.dir.tree.index
import destack._generated.dir.tree.match
import destack._generated.dir.tree.node
import destack._generated.dir.tree.pattern
import destack._generated.dir.tree.property
import destack._generated.dir.tree.sparse
import destack._generated.dir.tree.type
import destack._generated.source.file.model.module
import destack._generated.source.tree.index


@dataclass(frozen=True, slots=True)
class Tree:
    """Mutable DIR tree across a set of related source units."""

    # the module id of the tree
    module_id: destack._generated.source.file.model.module.ModuleId
    # the first global node id stored in this tree
    first_global_id: int
    # the next id to allocate
    next_global_id: int
    # dense local id and node type metadata by global node id
    node_index_by_node_id: Sequence[destack._generated.dir.tree.index.NodeIndexEntry]
    # source ranges and anchors keyed by parsed node id
    source_index: destack._generated.source.tree.index.SourceIndexData
    expressions: Sequence[destack._generated.dir.tree.expression.Expression]
    type_expressions: Sequence[destack._generated.dir.tree.type.TypeExpression]
    blocks: Sequence[destack._generated.dir.tree.block.Block]
    catches: Sequence[destack._generated.dir.tree.expression.Catch]
    declarations: Sequence[destack._generated.dir.tree.declaration.Declaration]
    declarators: Sequence[destack._generated.dir.tree.declarator.Declarator]
    properties: Sequence[destack._generated.dir.tree.property.Property]
    type_members: Sequence[destack._generated.dir.tree.type.TypeMember]
    type_mapped_parameters: Sequence[
        destack._generated.dir.tree.type.TypeMappedParameter
    ]
    members: Sequence[destack._generated.dir.tree.property.Member]
    enum_fields: Sequence[destack._generated.dir.tree.declaration.EnumField]
    where_clauses: Sequence[destack._generated.dir.tree.expression.WhereClause]
    dependency_items: Sequence[destack._generated.dir.tree.dependency.DependencyItem]
    generic_parameters: Sequence[destack._generated.dir.tree.argument.GenericParameter]
    parameters: Sequence[destack._generated.dir.tree.argument.Parameter]
    generic_arguments: Sequence[destack._generated.dir.tree.argument.GenericArgument]
    tuple_elements: Sequence[destack._generated.dir.tree.argument.TupleElement]
    arguments: Sequence[destack._generated.dir.tree.argument.Argument]
    match_cases: Sequence[destack._generated.dir.tree.match.MatchCase]
    patterns: Sequence[destack._generated.dir.tree.pattern.Pattern]
    pattern_fields: Sequence[destack._generated.dir.tree.pattern.PatternField]
    assign_patterns: Sequence[destack._generated.dir.tree.pattern.AssignPattern]
    assign_pattern_fields: Sequence[
        destack._generated.dir.tree.pattern.AssignPatternField
    ]
    comments: Sequence[destack._generated.dir.source.comment.Comment]
    decorators: Sequence[destack._generated.dir.tree.decorator.Decorator]
    # parent of every node, rebuilt when the tree is complete and updated in place
    parents: destack._generated.dir.source.parent.NodeParentIndex
    # how each derived node came to be
    origin_by_node_id: destack._generated.dir.tree.sparse.SparseNodeMap
    # the alias node id by replaced or derived node id
    alias_node_id_by_node_id: Mapping[int, int]
    # the decorators attached to nodes
    decorators_by_node_id: Mapping[
        int, Sequence[destack._generated.dir.tree.node.LocalNodeId]
    ]
    # the normalized documentation attached to nodes
    documentation_by_node_id: Mapping[
        int, destack._generated.dir.source.comment.Documentation
    ]
    # final source span overrides by node id
    source_span_by_node_id: destack._generated.dir.tree.sparse.SparseNodeMap
    # the detached node ids
    detached_node_ids: Sequence[int]

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
    destack._generated.source.file.model.module.encode_module_id(
        writer, value.module_id
    )
    writer.write_unsigned(value.first_global_id)
    writer.write_unsigned(value.next_global_id)
    writer.write_unsigned(len(value.node_index_by_node_id))
    for item_value_node_index_by_node_id_0 in value.node_index_by_node_id:
        destack._generated.dir.tree.index.encode_node_index_entry(
            writer, item_value_node_index_by_node_id_0
        )
    destack._generated.source.tree.index.encode_source_index_data(
        writer, value.source_index
    )
    writer.write_unsigned(len(value.expressions))
    for item_value_expressions_0 in value.expressions:
        destack._generated.dir.tree.expression.encode_expression(
            writer, item_value_expressions_0
        )
    writer.write_unsigned(len(value.type_expressions))
    for item_value_type_expressions_0 in value.type_expressions:
        destack._generated.dir.tree.type.encode_type_expression(
            writer, item_value_type_expressions_0
        )
    writer.write_unsigned(len(value.blocks))
    for item_value_blocks_0 in value.blocks:
        destack._generated.dir.tree.block.encode_block(writer, item_value_blocks_0)
    writer.write_unsigned(len(value.catches))
    for item_value_catches_0 in value.catches:
        destack._generated.dir.tree.expression.encode_catch(
            writer, item_value_catches_0
        )
    writer.write_unsigned(len(value.declarations))
    for item_value_declarations_0 in value.declarations:
        destack._generated.dir.tree.declaration.encode_declaration(
            writer, item_value_declarations_0
        )
    writer.write_unsigned(len(value.declarators))
    for item_value_declarators_0 in value.declarators:
        destack._generated.dir.tree.declarator.encode_declarator(
            writer, item_value_declarators_0
        )
    writer.write_unsigned(len(value.properties))
    for item_value_properties_0 in value.properties:
        destack._generated.dir.tree.property.encode_property(
            writer, item_value_properties_0
        )
    writer.write_unsigned(len(value.type_members))
    for item_value_type_members_0 in value.type_members:
        destack._generated.dir.tree.type.encode_type_member(
            writer, item_value_type_members_0
        )
    writer.write_unsigned(len(value.type_mapped_parameters))
    for item_value_type_mapped_parameters_0 in value.type_mapped_parameters:
        destack._generated.dir.tree.type.encode_type_mapped_parameter(
            writer, item_value_type_mapped_parameters_0
        )
    writer.write_unsigned(len(value.members))
    for item_value_members_0 in value.members:
        destack._generated.dir.tree.property.encode_member(writer, item_value_members_0)
    writer.write_unsigned(len(value.enum_fields))
    for item_value_enum_fields_0 in value.enum_fields:
        destack._generated.dir.tree.declaration.encode_enum_field(
            writer, item_value_enum_fields_0
        )
    writer.write_unsigned(len(value.where_clauses))
    for item_value_where_clauses_0 in value.where_clauses:
        destack._generated.dir.tree.expression.encode_where_clause(
            writer, item_value_where_clauses_0
        )
    writer.write_unsigned(len(value.dependency_items))
    for item_value_dependency_items_0 in value.dependency_items:
        destack._generated.dir.tree.dependency.encode_dependency_item(
            writer, item_value_dependency_items_0
        )
    writer.write_unsigned(len(value.generic_parameters))
    for item_value_generic_parameters_0 in value.generic_parameters:
        destack._generated.dir.tree.argument.encode_generic_parameter(
            writer, item_value_generic_parameters_0
        )
    writer.write_unsigned(len(value.parameters))
    for item_value_parameters_0 in value.parameters:
        destack._generated.dir.tree.argument.encode_parameter(
            writer, item_value_parameters_0
        )
    writer.write_unsigned(len(value.generic_arguments))
    for item_value_generic_arguments_0 in value.generic_arguments:
        destack._generated.dir.tree.argument.encode_generic_argument(
            writer, item_value_generic_arguments_0
        )
    writer.write_unsigned(len(value.tuple_elements))
    for item_value_tuple_elements_0 in value.tuple_elements:
        destack._generated.dir.tree.argument.encode_tuple_element(
            writer, item_value_tuple_elements_0
        )
    writer.write_unsigned(len(value.arguments))
    for item_value_arguments_0 in value.arguments:
        destack._generated.dir.tree.argument.encode_argument(
            writer, item_value_arguments_0
        )
    writer.write_unsigned(len(value.match_cases))
    for item_value_match_cases_0 in value.match_cases:
        destack._generated.dir.tree.match.encode_match_case(
            writer, item_value_match_cases_0
        )
    writer.write_unsigned(len(value.patterns))
    for item_value_patterns_0 in value.patterns:
        destack._generated.dir.tree.pattern.encode_pattern(
            writer, item_value_patterns_0
        )
    writer.write_unsigned(len(value.pattern_fields))
    for item_value_pattern_fields_0 in value.pattern_fields:
        destack._generated.dir.tree.pattern.encode_pattern_field(
            writer, item_value_pattern_fields_0
        )
    writer.write_unsigned(len(value.assign_patterns))
    for item_value_assign_patterns_0 in value.assign_patterns:
        destack._generated.dir.tree.pattern.encode_assign_pattern(
            writer, item_value_assign_patterns_0
        )
    writer.write_unsigned(len(value.assign_pattern_fields))
    for item_value_assign_pattern_fields_0 in value.assign_pattern_fields:
        destack._generated.dir.tree.pattern.encode_assign_pattern_field(
            writer, item_value_assign_pattern_fields_0
        )
    writer.write_unsigned(len(value.comments))
    for item_value_comments_0 in value.comments:
        destack._generated.dir.source.comment.encode_comment(
            writer, item_value_comments_0
        )
    writer.write_unsigned(len(value.decorators))
    for item_value_decorators_0 in value.decorators:
        destack._generated.dir.tree.decorator.encode_decorator(
            writer, item_value_decorators_0
        )
    destack._generated.dir.source.parent.encode_node_parent_index(writer, value.parents)
    destack._generated.dir.tree.sparse.encode_sparse_node_map(
        writer, value.origin_by_node_id
    )
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
    entries_value_decorators_by_node_id_0 = []
    for (
        key_value_decorators_by_node_id_0,
        item_value_decorators_by_node_id_0,
    ) in value.decorators_by_node_id.items():

        def write_key_value_decorators_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_decorators_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_decorators_by_node_id_0)
        entries_value_decorators_by_node_id_0.append(
            (
                key_value_decorators_by_node_id_0,
                item_value_decorators_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_decorators_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_decorators_by_node_id_0))
    for entry_value_decorators_by_node_id_0 in entries_value_decorators_by_node_id_0:
        writer.write_unsigned(entry_value_decorators_by_node_id_0[0])
        writer.write_unsigned(len(entry_value_decorators_by_node_id_0[1]))
        for (
            item_entry_value_decorators_by_node_id_0_1_1
        ) in entry_value_decorators_by_node_id_0[1]:
            destack._generated.dir.tree.node.encode_local_node_id(
                writer, item_entry_value_decorators_by_node_id_0_1_1
            )
    entries_value_documentation_by_node_id_0 = []
    for (
        key_value_documentation_by_node_id_0,
        item_value_documentation_by_node_id_0,
    ) in value.documentation_by_node_id.items():

        def write_key_value_documentation_by_node_id_0(writer: BinaryWriter) -> None:
            writer.write_unsigned(key_value_documentation_by_node_id_0)

        key_bytes = nested_bytes(write_key_value_documentation_by_node_id_0)
        entries_value_documentation_by_node_id_0.append(
            (
                key_value_documentation_by_node_id_0,
                item_value_documentation_by_node_id_0,
                key_bytes,
            )
        )
    entries_value_documentation_by_node_id_0.sort(key=lambda entry: entry[2])
    writer.write_unsigned(len(entries_value_documentation_by_node_id_0))
    for (
        entry_value_documentation_by_node_id_0
    ) in entries_value_documentation_by_node_id_0:
        writer.write_unsigned(entry_value_documentation_by_node_id_0[0])
        destack._generated.dir.source.comment.encode_documentation(
            writer, entry_value_documentation_by_node_id_0[1]
        )
    destack._generated.dir.tree.sparse.encode_sparse_node_map(
        writer, value.source_span_by_node_id
    )
    writer.write_unsigned(len(value.detached_node_ids))
    for item_value_detached_node_ids_0 in value.detached_node_ids:
        writer.write_unsigned(item_value_detached_node_ids_0)


def decode_tree(reader: BinaryReader) -> Tree:
    """Decode one Tree."""
    module_id = destack._generated.source.file.model.module.decode_module_id(reader)
    first_global_id = reader.read_number()
    next_global_id = reader.read_number()
    node_index_by_node_id = [
        destack._generated.dir.tree.index.decode_node_index_entry(reader)
        for _ in range(reader.read_number())
    ]
    source_index = destack._generated.source.tree.index.decode_source_index_data(reader)
    expressions = [
        destack._generated.dir.tree.expression.decode_expression(reader)
        for _ in range(reader.read_number())
    ]
    type_expressions = [
        destack._generated.dir.tree.type.decode_type_expression(reader)
        for _ in range(reader.read_number())
    ]
    blocks = [
        destack._generated.dir.tree.block.decode_block(reader)
        for _ in range(reader.read_number())
    ]
    catches = [
        destack._generated.dir.tree.expression.decode_catch(reader)
        for _ in range(reader.read_number())
    ]
    declarations = [
        destack._generated.dir.tree.declaration.decode_declaration(reader)
        for _ in range(reader.read_number())
    ]
    declarators = [
        destack._generated.dir.tree.declarator.decode_declarator(reader)
        for _ in range(reader.read_number())
    ]
    properties = [
        destack._generated.dir.tree.property.decode_property(reader)
        for _ in range(reader.read_number())
    ]
    type_members = [
        destack._generated.dir.tree.type.decode_type_member(reader)
        for _ in range(reader.read_number())
    ]
    type_mapped_parameters = [
        destack._generated.dir.tree.type.decode_type_mapped_parameter(reader)
        for _ in range(reader.read_number())
    ]
    members = [
        destack._generated.dir.tree.property.decode_member(reader)
        for _ in range(reader.read_number())
    ]
    enum_fields = [
        destack._generated.dir.tree.declaration.decode_enum_field(reader)
        for _ in range(reader.read_number())
    ]
    where_clauses = [
        destack._generated.dir.tree.expression.decode_where_clause(reader)
        for _ in range(reader.read_number())
    ]
    dependency_items = [
        destack._generated.dir.tree.dependency.decode_dependency_item(reader)
        for _ in range(reader.read_number())
    ]
    generic_parameters = [
        destack._generated.dir.tree.argument.decode_generic_parameter(reader)
        for _ in range(reader.read_number())
    ]
    parameters = [
        destack._generated.dir.tree.argument.decode_parameter(reader)
        for _ in range(reader.read_number())
    ]
    generic_arguments = [
        destack._generated.dir.tree.argument.decode_generic_argument(reader)
        for _ in range(reader.read_number())
    ]
    tuple_elements = [
        destack._generated.dir.tree.argument.decode_tuple_element(reader)
        for _ in range(reader.read_number())
    ]
    arguments = [
        destack._generated.dir.tree.argument.decode_argument(reader)
        for _ in range(reader.read_number())
    ]
    match_cases = [
        destack._generated.dir.tree.match.decode_match_case(reader)
        for _ in range(reader.read_number())
    ]
    patterns = [
        destack._generated.dir.tree.pattern.decode_pattern(reader)
        for _ in range(reader.read_number())
    ]
    pattern_fields = [
        destack._generated.dir.tree.pattern.decode_pattern_field(reader)
        for _ in range(reader.read_number())
    ]
    assign_patterns = [
        destack._generated.dir.tree.pattern.decode_assign_pattern(reader)
        for _ in range(reader.read_number())
    ]
    assign_pattern_fields = [
        destack._generated.dir.tree.pattern.decode_assign_pattern_field(reader)
        for _ in range(reader.read_number())
    ]
    comments = [
        destack._generated.dir.source.comment.decode_comment(reader)
        for _ in range(reader.read_number())
    ]
    decorators = [
        destack._generated.dir.tree.decorator.decode_decorator(reader)
        for _ in range(reader.read_number())
    ]
    parents = destack._generated.dir.source.parent.decode_node_parent_index(reader)
    origin_by_node_id = destack._generated.dir.tree.sparse.decode_sparse_node_map(
        reader
    )
    alias_node_id_by_node_id = {
        reader.read_number(): reader.read_number() for _ in range(reader.read_number())
    }
    decorators_by_node_id = {
        reader.read_number(): [
            destack._generated.dir.tree.node.decode_local_node_id(reader)
            for _ in range(reader.read_number())
        ]
        for _ in range(reader.read_number())
    }
    documentation_by_node_id = {
        reader.read_number(): destack._generated.dir.source.comment.decode_documentation(
            reader
        )
        for _ in range(reader.read_number())
    }
    source_span_by_node_id = destack._generated.dir.tree.sparse.decode_sparse_node_map(
        reader
    )
    detached_node_ids = [reader.read_number() for _ in range(reader.read_number())]

    return Tree(
        module_id=module_id,
        first_global_id=first_global_id,
        next_global_id=next_global_id,
        node_index_by_node_id=node_index_by_node_id,
        source_index=source_index,
        expressions=expressions,
        type_expressions=type_expressions,
        blocks=blocks,
        catches=catches,
        declarations=declarations,
        declarators=declarators,
        properties=properties,
        type_members=type_members,
        type_mapped_parameters=type_mapped_parameters,
        members=members,
        enum_fields=enum_fields,
        where_clauses=where_clauses,
        dependency_items=dependency_items,
        generic_parameters=generic_parameters,
        parameters=parameters,
        generic_arguments=generic_arguments,
        tuple_elements=tuple_elements,
        arguments=arguments,
        match_cases=match_cases,
        patterns=patterns,
        pattern_fields=pattern_fields,
        assign_patterns=assign_patterns,
        assign_pattern_fields=assign_pattern_fields,
        comments=comments,
        decorators=decorators,
        parents=parents,
        origin_by_node_id=origin_by_node_id,
        alias_node_id_by_node_id=alias_node_id_by_node_id,
        decorators_by_node_id=decorators_by_node_id,
        documentation_by_node_id=documentation_by_node_id,
        source_span_by_node_id=source_span_by_node_id,
        detached_node_ids=detached_node_ids,
    )


def to_json_tree(value: Tree) -> Json:
    """Return one JSON value for one Tree."""
    return {
        "moduleId": destack._generated.source.file.model.module.to_json_module_id(
            value.module_id
        ),
        "firstGlobalId": value.first_global_id,
        "nextGlobalId": value.next_global_id,
        "nodeIndexByNodeId": [
            destack._generated.dir.tree.index.to_json_node_index_entry(item_0)
            for item_0 in value.node_index_by_node_id
        ],
        "sourceIndex": destack._generated.source.tree.index.to_json_source_index_data(
            value.source_index
        ),
        "expressions": [
            destack._generated.dir.tree.expression.to_json_expression(item_0)
            for item_0 in value.expressions
        ],
        "typeExpressions": [
            destack._generated.dir.tree.type.to_json_type_expression(item_0)
            for item_0 in value.type_expressions
        ],
        "blocks": [
            destack._generated.dir.tree.block.to_json_block(item_0)
            for item_0 in value.blocks
        ],
        "catches": [
            destack._generated.dir.tree.expression.to_json_catch(item_0)
            for item_0 in value.catches
        ],
        "declarations": [
            destack._generated.dir.tree.declaration.to_json_declaration(item_0)
            for item_0 in value.declarations
        ],
        "declarators": [
            destack._generated.dir.tree.declarator.to_json_declarator(item_0)
            for item_0 in value.declarators
        ],
        "properties": [
            destack._generated.dir.tree.property.to_json_property(item_0)
            for item_0 in value.properties
        ],
        "typeMembers": [
            destack._generated.dir.tree.type.to_json_type_member(item_0)
            for item_0 in value.type_members
        ],
        "typeMappedParameters": [
            destack._generated.dir.tree.type.to_json_type_mapped_parameter(item_0)
            for item_0 in value.type_mapped_parameters
        ],
        "members": [
            destack._generated.dir.tree.property.to_json_member(item_0)
            for item_0 in value.members
        ],
        "enumFields": [
            destack._generated.dir.tree.declaration.to_json_enum_field(item_0)
            for item_0 in value.enum_fields
        ],
        "whereClauses": [
            destack._generated.dir.tree.expression.to_json_where_clause(item_0)
            for item_0 in value.where_clauses
        ],
        "dependencyItems": [
            destack._generated.dir.tree.dependency.to_json_dependency_item(item_0)
            for item_0 in value.dependency_items
        ],
        "genericParameters": [
            destack._generated.dir.tree.argument.to_json_generic_parameter(item_0)
            for item_0 in value.generic_parameters
        ],
        "parameters": [
            destack._generated.dir.tree.argument.to_json_parameter(item_0)
            for item_0 in value.parameters
        ],
        "genericArguments": [
            destack._generated.dir.tree.argument.to_json_generic_argument(item_0)
            for item_0 in value.generic_arguments
        ],
        "tupleElements": [
            destack._generated.dir.tree.argument.to_json_tuple_element(item_0)
            for item_0 in value.tuple_elements
        ],
        "arguments": [
            destack._generated.dir.tree.argument.to_json_argument(item_0)
            for item_0 in value.arguments
        ],
        "matchCases": [
            destack._generated.dir.tree.match.to_json_match_case(item_0)
            for item_0 in value.match_cases
        ],
        "patterns": [
            destack._generated.dir.tree.pattern.to_json_pattern(item_0)
            for item_0 in value.patterns
        ],
        "patternFields": [
            destack._generated.dir.tree.pattern.to_json_pattern_field(item_0)
            for item_0 in value.pattern_fields
        ],
        "assignPatterns": [
            destack._generated.dir.tree.pattern.to_json_assign_pattern(item_0)
            for item_0 in value.assign_patterns
        ],
        "assignPatternFields": [
            destack._generated.dir.tree.pattern.to_json_assign_pattern_field(item_0)
            for item_0 in value.assign_pattern_fields
        ],
        "comments": [
            destack._generated.dir.source.comment.to_json_comment(item_0)
            for item_0 in value.comments
        ],
        "decorators": [
            destack._generated.dir.tree.decorator.to_json_decorator(item_0)
            for item_0 in value.decorators
        ],
        "parents": destack._generated.dir.source.parent.to_json_node_parent_index(
            value.parents
        ),
        "originByNodeId": destack._generated.dir.tree.sparse.to_json_sparse_node_map(
            value.origin_by_node_id
        ),
        "aliasNodeIdByNodeId": [
            [key_0, item_0] for key_0, item_0 in value.alias_node_id_by_node_id.items()
        ],
        "decoratorsByNodeId": [
            [
                key_0,
                [
                    destack._generated.dir.tree.node.to_json_local_node_id(item_1)
                    for item_1 in item_0
                ],
            ]
            for key_0, item_0 in value.decorators_by_node_id.items()
        ],
        "documentationByNodeId": [
            [key_0, destack._generated.dir.source.comment.to_json_documentation(item_0)]
            for key_0, item_0 in value.documentation_by_node_id.items()
        ],
        "sourceSpanByNodeId": destack._generated.dir.tree.sparse.to_json_sparse_node_map(
            value.source_span_by_node_id
        ),
        "detachedNodeIds": [item_0 for item_0 in value.detached_node_ids],
    }


def from_json_tree(value: Json) -> Tree:
    """Return one Tree from one JSON value."""
    object_ = json_object(value)

    return Tree(
        module_id=destack._generated.source.file.model.module.from_json_module_id(
            json_field(object_, "moduleId")
        ),
        first_global_id=json_int(json_field(object_, "firstGlobalId")),
        next_global_id=json_int(json_field(object_, "nextGlobalId")),
        node_index_by_node_id=[
            destack._generated.dir.tree.index.from_json_node_index_entry(item_0)
            for item_0 in json_array(json_field(object_, "nodeIndexByNodeId"))
        ],
        source_index=destack._generated.source.tree.index.from_json_source_index_data(
            json_field(object_, "sourceIndex")
        ),
        expressions=[
            destack._generated.dir.tree.expression.from_json_expression(item_0)
            for item_0 in json_array(json_field(object_, "expressions"))
        ],
        type_expressions=[
            destack._generated.dir.tree.type.from_json_type_expression(item_0)
            for item_0 in json_array(json_field(object_, "typeExpressions"))
        ],
        blocks=[
            destack._generated.dir.tree.block.from_json_block(item_0)
            for item_0 in json_array(json_field(object_, "blocks"))
        ],
        catches=[
            destack._generated.dir.tree.expression.from_json_catch(item_0)
            for item_0 in json_array(json_field(object_, "catches"))
        ],
        declarations=[
            destack._generated.dir.tree.declaration.from_json_declaration(item_0)
            for item_0 in json_array(json_field(object_, "declarations"))
        ],
        declarators=[
            destack._generated.dir.tree.declarator.from_json_declarator(item_0)
            for item_0 in json_array(json_field(object_, "declarators"))
        ],
        properties=[
            destack._generated.dir.tree.property.from_json_property(item_0)
            for item_0 in json_array(json_field(object_, "properties"))
        ],
        type_members=[
            destack._generated.dir.tree.type.from_json_type_member(item_0)
            for item_0 in json_array(json_field(object_, "typeMembers"))
        ],
        type_mapped_parameters=[
            destack._generated.dir.tree.type.from_json_type_mapped_parameter(item_0)
            for item_0 in json_array(json_field(object_, "typeMappedParameters"))
        ],
        members=[
            destack._generated.dir.tree.property.from_json_member(item_0)
            for item_0 in json_array(json_field(object_, "members"))
        ],
        enum_fields=[
            destack._generated.dir.tree.declaration.from_json_enum_field(item_0)
            for item_0 in json_array(json_field(object_, "enumFields"))
        ],
        where_clauses=[
            destack._generated.dir.tree.expression.from_json_where_clause(item_0)
            for item_0 in json_array(json_field(object_, "whereClauses"))
        ],
        dependency_items=[
            destack._generated.dir.tree.dependency.from_json_dependency_item(item_0)
            for item_0 in json_array(json_field(object_, "dependencyItems"))
        ],
        generic_parameters=[
            destack._generated.dir.tree.argument.from_json_generic_parameter(item_0)
            for item_0 in json_array(json_field(object_, "genericParameters"))
        ],
        parameters=[
            destack._generated.dir.tree.argument.from_json_parameter(item_0)
            for item_0 in json_array(json_field(object_, "parameters"))
        ],
        generic_arguments=[
            destack._generated.dir.tree.argument.from_json_generic_argument(item_0)
            for item_0 in json_array(json_field(object_, "genericArguments"))
        ],
        tuple_elements=[
            destack._generated.dir.tree.argument.from_json_tuple_element(item_0)
            for item_0 in json_array(json_field(object_, "tupleElements"))
        ],
        arguments=[
            destack._generated.dir.tree.argument.from_json_argument(item_0)
            for item_0 in json_array(json_field(object_, "arguments"))
        ],
        match_cases=[
            destack._generated.dir.tree.match.from_json_match_case(item_0)
            for item_0 in json_array(json_field(object_, "matchCases"))
        ],
        patterns=[
            destack._generated.dir.tree.pattern.from_json_pattern(item_0)
            for item_0 in json_array(json_field(object_, "patterns"))
        ],
        pattern_fields=[
            destack._generated.dir.tree.pattern.from_json_pattern_field(item_0)
            for item_0 in json_array(json_field(object_, "patternFields"))
        ],
        assign_patterns=[
            destack._generated.dir.tree.pattern.from_json_assign_pattern(item_0)
            for item_0 in json_array(json_field(object_, "assignPatterns"))
        ],
        assign_pattern_fields=[
            destack._generated.dir.tree.pattern.from_json_assign_pattern_field(item_0)
            for item_0 in json_array(json_field(object_, "assignPatternFields"))
        ],
        comments=[
            destack._generated.dir.source.comment.from_json_comment(item_0)
            for item_0 in json_array(json_field(object_, "comments"))
        ],
        decorators=[
            destack._generated.dir.tree.decorator.from_json_decorator(item_0)
            for item_0 in json_array(json_field(object_, "decorators"))
        ],
        parents=destack._generated.dir.source.parent.from_json_node_parent_index(
            json_field(object_, "parents")
        ),
        origin_by_node_id=destack._generated.dir.tree.sparse.from_json_sparse_node_map(
            json_field(object_, "originByNodeId")
        ),
        alias_node_id_by_node_id={
            json_int(key_0): json_int(item_0)
            for key_0, item_0 in json_array(json_field(object_, "aliasNodeIdByNodeId"))
        },
        decorators_by_node_id={
            json_int(key_0): [
                destack._generated.dir.tree.node.from_json_local_node_id(item_1)
                for item_1 in json_array(item_0)
            ]
            for key_0, item_0 in json_array(json_field(object_, "decoratorsByNodeId"))
        },
        documentation_by_node_id={
            json_int(
                key_0
            ): destack._generated.dir.source.comment.from_json_documentation(item_0)
            for key_0, item_0 in json_array(
                json_field(object_, "documentationByNodeId")
            )
        },
        source_span_by_node_id=destack._generated.dir.tree.sparse.from_json_sparse_node_map(
            json_field(object_, "sourceSpanByNodeId")
        ),
        detached_node_ids=[
            json_int(item_0)
            for item_0 in json_array(json_field(object_, "detachedNodeIds"))
        ],
    )


__all__ = [
    "Tree",
    "encode_tree",
    "decode_tree",
    "to_json_tree",
    "from_json_tree",
]
