from collections.abc import Sequence
from typing import Any, assert_never

import structlog
from fastuuid import UUID
from opentelemetry import trace

from destack.language import (
    Aggregation,
    AggregationType,
    AttributeType,
    Condition,
    ConditionalType,
    EdgeDirection,
    Expression,
    ExpressionType,
    Function,
    FunctionType,
    JoinType,
    NodeReference,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    RelationReference,
    RelationType,
    Select,
    Sort,
    SortType,
    Value,
    attribute_ref,
    to_value,
)
from destack.language import expression as to_expression
from destack.language.registry import NODE_CLASS_BY_TYPE

from .core import MemoryContext, MemoryRow, MemoryTable

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

MAX_RECURSION_DEPTH = 100


def _unpack_node_row_value(table: MemoryTable, row: MemoryRow) -> Value:
    """unpack a MemoryRow to a Value since unpack_node_row is not implemented."""
    # return the raw value object with proper type info
    from destack.language import ScalarType, Type, TypeCardinality

    assert row.metatype is not None, f"no metatype for {row!r}"
    # create type info for the node
    type_info = Type(
        cardinality=TypeCardinality.SCALAR,
        scalar_type=ScalarType.NODE_VALUE,
        node_type=row.metatype,
    )
    return Value(type=type_info, value=row.value)


def _evaluate_expression(context: MemoryContext, expression: Expression, row: MemoryRow) -> Any:
    """Evaluate an Expression against in-memory row data."""
    if expression.type == ExpressionType.LITERAL:
        assert expression.literal is not None, f"no literal for {expression!r}"
        return expression.literal.value
    elif expression.type == ExpressionType.ATTRIBUTE:
        assert expression.attribute is not None, f"no attribute for {expression!r}"
        attr = expression.attribute
        if attr.type == AttributeType.PROPERTY:
            assert attr.prop_ptr is not None, f"no property for {attr!r}"
            prop_key = str(attr.prop_ptr.id)
            return row.value.get(prop_key)
        else:
            raise NotImplementedError(f"unsupported attribute type: {attr.type}")
    elif expression.type == ExpressionType.CONDITION:
        assert expression.condition is not None, f"no condition for {expression!r}"
        return _evaluate_condition(context, expression.condition, row)
    elif expression.type == ExpressionType.FUNCTION:
        assert expression.function is not None, f"no function for {expression!r}"
        return _evaluate_function(context, expression.function, row)
    elif expression.type == ExpressionType.AGGREGATION:
        # Aggregations need to be handled at a higher level with multiple rows
        raise RuntimeError(f"aggregation cannot be evaluated on single row: {expression!r}")
    else:
        assert_never(expression.type)


def _evaluate_condition(context: MemoryContext, condition: Condition, row: MemoryRow) -> bool:
    """Evaluate a Condition against in-memory row data."""
    # logical
    if condition.type == ConditionalType.NOT:
        left_val = _evaluate_expression(context, condition.left, row)
        return not bool(left_val)
    elif condition.type == ConditionalType.AND:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        if not left_val:
            return False
        right_val = _evaluate_expression(context, condition.right, row)
        return bool(right_val)
    elif condition.type == ConditionalType.OR:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        if left_val:
            return True
        right_val = _evaluate_expression(context, condition.right, row)
        return bool(right_val)
    # comparison
    elif condition.type == ConditionalType.EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val == right_val
    elif condition.type == ConditionalType.NOT_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val != right_val
    elif condition.type == ConditionalType.GREATER_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val > right_val
    elif condition.type == ConditionalType.GREATER_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val >= right_val
    elif condition.type == ConditionalType.LESS_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val < right_val
    elif condition.type == ConditionalType.LESS_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        return left_val <= right_val
    # string
    elif condition.type == ConditionalType.MATCHES:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        if left_val is None or right_val is None:
            return False
        # Simple pattern matching (could be enhanced with regex)
        return str(right_val) in str(left_val)
    elif condition.type == ConditionalType.STARTS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        if left_val is None or right_val is None:
            return False
        return str(left_val).startswith(str(right_val))
    elif condition.type == ConditionalType.ENDS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        if left_val is None or right_val is None:
            return False
        return str(left_val).endswith(str(right_val))
    # collections
    elif condition.type == ConditionalType.IN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        if right_val is None:
            return False
        if isinstance(right_val, (list, tuple)):
            return left_val in right_val
        return left_val == right_val
    elif condition.type == ConditionalType.NOT_IN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = _evaluate_expression(context, condition.left, row)
        right_val = _evaluate_expression(context, condition.right, row)
        if right_val is None:
            return True
        if isinstance(right_val, (list, tuple)):
            return left_val not in right_val
        return left_val != right_val
    # existence
    elif condition.type == ConditionalType.EXISTS:
        left_val = _evaluate_expression(context, condition.left, row)
        return left_val is not None
    elif condition.type == ConditionalType.NOT_EXISTS:
        left_val = _evaluate_expression(context, condition.left, row)
        return left_val is None
    else:
        assert_never(condition.type)


def _evaluate_sort(
    context: MemoryContext, sort: Sequence[Sort], rows: list[MemoryRow]
) -> list[MemoryRow]:
    """Sort in-memory rows based on Sort criteria."""
    if not sort:
        return rows

    def sort_key(row: MemoryRow) -> tuple:
        key_values = []
        for s in sort:
            val = _evaluate_expression(context, s.by, row)
            # Handle None values by putting them at the end
            val = (1, None) if val is None else (0, val)
            key_values.append(val)
        return tuple(key_values)

    # Sort with reverse=True for descending sorts
    reverse_flags = [s.type == SortType.DESCENDING for s in sort]

    # For multiple sort criteria, we need to handle each level
    sorted_rows = sorted(rows, key=sort_key, reverse=all(reverse_flags))
    return sorted_rows


def _evaluate_function(context: MemoryContext, function: Function, row: MemoryRow) -> Any:
    """Evaluate a Function against in-memory row data."""
    left_val = _evaluate_expression(context, function.left, row)
    if function.type == FunctionType.ADD:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val + right_val
    elif function.type == FunctionType.SUBTRACT:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val - right_val
    elif function.type == FunctionType.MULTIPLY:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val * right_val
    elif function.type == FunctionType.DIVIDE:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val / right_val
    elif function.type == FunctionType.MODULO:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val % right_val
    elif function.type == FunctionType.POWER:
        assert function.right is not None, f"no right for {function!r}"
        right_val = _evaluate_expression(context, function.right, row)
        return left_val**right_val
    else:
        assert_never(function.type)


def _evaluate_aggregation(
    context: MemoryContext, aggregation: Aggregation, rows: list[MemoryRow]
) -> Any:
    """Evaluate an Aggregation against in-memory rows."""
    if aggregation.type == AggregationType.EXISTS:
        return len(rows) > 0
    elif aggregation.type == AggregationType.COUNT:
        return len(rows)
    elif aggregation.type == AggregationType.SUM:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        total = 0
        for row in rows:
            val = _evaluate_expression(context, aggregation.expression, row)
            if val is not None:
                total += val
        return total
    elif aggregation.type == AggregationType.MIN:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        min_val = None
        for row in rows:
            val = _evaluate_expression(context, aggregation.expression, row)
            if val is not None and (min_val is None or val < min_val):
                min_val = val
        return min_val
    elif aggregation.type == AggregationType.MAX:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        max_val = None
        for row in rows:
            val = _evaluate_expression(context, aggregation.expression, row)
            if val is not None and (max_val is None or val > max_val):
                max_val = val
        return max_val
    elif aggregation.type == AggregationType.AVERAGE:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        total = 0
        count = 0
        for row in rows:
            val = _evaluate_expression(context, aggregation.expression, row)
            if val is not None:
                total += val
                count += 1
        return total / count if count > 0 else None
    else:
        assert_never(aggregation.type)


def _is_id_condition(condition: Condition) -> tuple[bool, Sequence[UUID]]:
    """Check if condition is id = value or id IN values and return the value(s)."""
    if condition.type == ConditionalType.EQUALS or condition.type == ConditionalType.IN:
        if (
            (left := condition.left) is not None
            and left.type == ExpressionType.ATTRIBUTE
            and (attribute := left.attribute) is not None
            and attribute.type == AttributeType.PROPERTY
            and (prop_ptr := attribute.prop_ptr) is not None
            and prop_ptr.id == 2
            and (right := condition.right) is not None
            and right.type == ExpressionType.LITERAL
        ):
            assert right.literal is not None, f"no literal for {right!r}"
            ids = right.literal.value.unpack()
            if isinstance(ids, UUID):
                ids = (ids,)
            return True, ids
    return False, ()


@tracer.start_as_current_span("memory.walk")
def _walk(
    context: MemoryContext,
    relation: RelationReference,
    roots_ptr: Sequence[NodeReference],
    roots_parents_ptr: Sequence[NodeReference],
    direction: EdgeDirection,
    depth: int,
    where: Condition | None,
) -> list[NodeReference]:
    """Walk the in-memory graph from root nodes."""
    if depth <= 0 or depth > MAX_RECURSION_DEPTH:
        return []

    # handle multi-relations
    if relation.type == RelationType.TRAIT:
        # expand trait relations to all implementing node types
        relations = context.resolve_relation(relation)
    else:
        relations = [relation]

    # track visited nodes to avoid cycles
    visited: set[str] = set()
    result_ptrs: list[NodeReference] = []

    # collect all node ids we're starting from
    root_ids: set[str] = {str(ptr.id) for ptr in roots_ptr}
    root_parent_ids: set[str] = {str(ptr.id) for ptr in roots_parents_ptr}

    current_level_ptrs: list[NodeReference] = list(roots_ptr)
    current_depth = 0

    while current_depth < depth and current_level_ptrs:
        next_level_ptrs: list[NodeReference] = []

        for rel in relations:
            table = context.get_relation(rel)

            if direction == EdgeDirection.CHILD:
                # Use rows_by_parent_id for efficient child lookup
                for ptr in current_level_ptrs:
                    if ptr.id in table.rows_by_parent_id:
                        for row in table.rows_by_parent_id[ptr.id]:
                            row_id_str = str(row.ptr.id)
                            if row_id_str not in visited and (
                                where is None or _evaluate_condition(context, where, row)
                            ):
                                visited.add(row_id_str)
                                next_level_ptrs.append(row.ptr)
                                result_ptrs.append(row.ptr)

            elif direction == EdgeDirection.PARENT:
                # Use direct table.rows lookup for efficient parent access
                for ptr in current_level_ptrs:
                    if ptr.id in table.rows:
                        row = table.rows[ptr.id]
                        if row.parent_ptr:
                            parent_id_str = str(row.parent_ptr.id)
                            if parent_id_str not in visited:
                                # Find parent row in appropriate table
                                parent_found = False
                                for parent_rel in relations:
                                    parent_table = context.get_relation(parent_rel)
                                    if row.parent_ptr.id in parent_table.rows:
                                        parent_row = parent_table.rows[row.parent_ptr.id]
                                        if where is None or _evaluate_condition(
                                            context, where, parent_row
                                        ):
                                            visited.add(parent_id_str)
                                            next_level_ptrs.append(parent_row.ptr)
                                            result_ptrs.append(parent_row.ptr)
                                            parent_found = True
                                            break

                                if not parent_found and row.parent_ptr.node_type:
                                    # parent might be in a different table not covered by current relations
                                    parent_ref = NodeReference(
                                        node_type=row.parent_ptr.node_type,
                                        id=row.parent_ptr.id,
                                    )
                                    parent_table = context.get_relation(parent_ref)
                                    if row.parent_ptr.id in parent_table.rows:
                                        parent_row = parent_table.rows[row.parent_ptr.id]
                                        if where is None or _evaluate_condition(
                                            context, where, parent_row
                                        ):
                                            visited.add(parent_id_str)
                                            next_level_ptrs.append(parent_row.ptr)
                                            result_ptrs.append(parent_row.ptr)

            elif direction == EdgeDirection.SIDE:
                raise NotImplementedError(f"side edges not implemented: {direction!r}")

            else:
                assert_never(direction)

        current_level_ptrs = next_level_ptrs
        current_depth += 1

    return result_ptrs


@tracer.start_as_current_span("memory.query_scalar")
def _query_scalar(
    context: MemoryContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query against in-memory data."""
    # handle multi-relations
    if relation.type == RelationType.TRAIT:
        relations = context.resolve_relation(relation)
        filtered_rows: list[MemoryRow] = []
        for rel in relations:
            table = context.get_relation(rel)
            for row in table.rows.values():
                if where is None or _evaluate_condition(context, where, row):
                    filtered_rows.append(row)
    else:
        table = context.get_relation(relation)
        filtered_rows = []
        for row in table.rows.values():
            if where is None or _evaluate_condition(context, where, row):
                filtered_rows.append(row)

    # apply aggregation
    result = _evaluate_aggregation(context, aggregation, filtered_rows)
    return to_value(result)


@tracer.start_as_current_span("memory.query_grouped_scalar")
def _query_grouped_scalar(
    context: MemoryContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
    having: Condition | None,
    group_by: Sequence[Expression],
) -> list[tuple[Value, Value]]:
    """Execute a grouped scalar Query against in-memory data."""
    if relation.type == RelationType.TRAIT:
        raise NotImplementedError("grouped scalar queries not supported for trait relations")

    table = context.get_relation(relation)

    # filter rows based on where condition
    filtered_rows: list[MemoryRow] = []
    for row in table.rows.values():
        if where is None or _evaluate_condition(context, where, row):
            filtered_rows.append(row)

    # group rows by group_by expressions
    groups: dict[tuple, list[MemoryRow]] = {}
    for row in filtered_rows:
        group_key = tuple(_evaluate_expression(context, expr, row) for expr in group_by)
        if group_key not in groups:
            groups[group_key] = []
        groups[group_key].append(row)

    # apply having filter and aggregation to each group
    results: list[tuple[Value, Value]] = []
    for group_key, group_rows in groups.items():
        if having is not None and group_rows:
            if not _evaluate_condition(context, having, group_rows[0]):
                continue

        agg_result = _evaluate_aggregation(context, aggregation, group_rows)
        # Use first element of group key as discriminator (simplified)
        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), to_value(agg_result)))

    return results


@tracer.start_as_current_span("memory.query_grouped_node")
def _query_grouped_node(
    context: MemoryContext,
    relation: RelationReference,
    select: Select | None,
    where: Condition | None,
    having: Condition | None,
    sort: Sequence[Sort] | None,
    group_by: Sequence[Expression],
    limit: int | None,
    offset: int | None,
) -> list[tuple[Value, list[Value], list[NodeReference]]]:
    """Execute a grouped node Query against in-memory data."""
    if relation.type == RelationType.TRAIT:
        raise NotImplementedError("grouped node queries not supported for trait relations")

    table = context.get_relation(relation)

    # Short-circuit for id-based queries
    if where is not None:
        is_id_query, id_values = _is_id_condition(where)
        if is_id_query:
            filtered_rows: list[MemoryRow] = []
            if isinstance(id_values, (list, tuple)):
                for id_val in id_values:
                    if id_val in table.rows:
                        filtered_rows.append(table.rows[id_val])
            else:
                if id_values in table.rows:
                    filtered_rows.append(table.rows[id_values])
        else:
            # filter rows based on where condition
            filtered_rows = []
            for row in table.rows.values():
                if _evaluate_condition(context, where, row):
                    filtered_rows.append(row)
    else:
        filtered_rows = list(table.rows.values())

    # group rows by group_by expressions
    groups: dict[tuple, list[MemoryRow]] = {}
    for row in filtered_rows:
        group_key = tuple(_evaluate_expression(context, expr, row) for expr in group_by)
        if group_key not in groups:
            groups[group_key] = []
        groups[group_key].append(row)

    # apply having filter and collect results
    results: list[tuple[Value, list[Value], list[NodeReference]]] = []
    for group_key, group_rows in groups.items():
        if having is not None and group_rows:
            if not _evaluate_condition(context, having, group_rows[0]):
                continue
        if sort:
            group_rows = _evaluate_sort(context, sort, group_rows)
        if offset:
            group_rows = group_rows[offset:]
        if limit:
            group_rows = group_rows[:limit]

        values: list[Value] = []
        ptrs: list[NodeReference] = []
        for row in group_rows:
            values.append(_unpack_node_row_value(table, row))
            ptrs.append(row.ptr)
        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), values, ptrs))

    return results


@tracer.start_as_current_span("memory.query_node")
def _query_node(
    context: MemoryContext,
    relation: RelationReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query against in-memory data."""
    # handle multi-relations
    if relation.type == RelationType.TRAIT:
        # fan out trait relations
        if limit is not None or offset is not None:
            raise NotImplementedError(f"cannot limit/offset for multi relation: {relation!r}")
        relations = context.resolve_relation(relation)
        all_values: list[Value] = []
        all_ptrs: list[NodeReference] = []
        for rel in relations:
            values, ptrs = _query_node(
                context=context,
                relation=rel,
                select=select,
                where=where,
                sort=sort,
                limit=limit,
                offset=offset,
            )
            all_values.extend(values)
            all_ptrs.extend(ptrs)
        return all_values, all_ptrs

    table = context.get_relation(relation)

    # Short-circuit for id-based queries
    if where is not None:
        is_id_query, id_values = _is_id_condition(where)
        if is_id_query:
            filtered_rows: list[MemoryRow] = []
            if isinstance(id_values, (list, tuple)):
                for id_val in id_values:
                    if id_val in table.rows:
                        filtered_rows.append(table.rows[id_val])
            else:
                if id_values in table.rows:
                    filtered_rows.append(table.rows[id_values])
        else:
            # Filter rows based on where condition
            filtered_rows = []
            for row in table.rows.values():
                if _evaluate_condition(context, where, row):
                    filtered_rows.append(row)
    else:
        filtered_rows = list(table.rows.values())

    # Sort if needed
    if sort:
        filtered_rows = _evaluate_sort(context, sort, filtered_rows)

    # Apply limit/offset
    if offset:
        filtered_rows = filtered_rows[offset:]
    if limit:
        filtered_rows = filtered_rows[:limit]

    # Convert to Values
    values: list[Value] = []
    ptrs: list[NodeReference] = []
    for row in filtered_rows:
        values.append(_unpack_node_row_value(table, row))
        ptrs.append(row.ptr)

    return values, ptrs


@tracer.start_as_current_span("memory.query_clause")
def _query_clause(
    context: MemoryContext, query: Query, where: Condition | None
) -> tuple[QueryResult, Sequence[NodeReference]]:
    """Execute the specific Query "clause" (ignoring subqueries) against in-memory data."""
    result = QueryResult(id=query.id, type=query.type)
    nodes_ptr: list[NodeReference] = []

    # Combine query where with additional where
    combined_where = query.where
    if where is not None:
        if combined_where is not None:
            from destack.language import Condition, ConditionalType, expression

            combined_where = Condition(
                type=ConditionalType.AND, left=expression(combined_where), right=expression(where)
            )
        else:
            combined_where = where

    if query.type == QueryType.SCALAR:
        assert query.aggregation is not None, f"no aggregation for scalar query: {query!r}"
        scalar_result = _query_scalar(context, query.relation, query.aggregation, combined_where)
        result.scalar = scalar_result
        result.exists = scalar_result.value is not None

    elif query.type == QueryType.GROUPED_SCALAR:
        assert query.aggregation is not None, f"no aggregation for grouped scalar query: {query!r}"
        assert query.group_by, f"no group_by for grouped scalar query: {query!r}"
        grouped_results = _query_grouped_scalar(
            context, query.relation, query.aggregation, combined_where, query.having, query.group_by
        )

        result.groups = []
        for discriminator, scalar_val in grouped_results:
            group = QueryResultGroup(
                type=QueryType.SCALAR,
                discriminator=discriminator,
                scalar=scalar_val,
                exists=scalar_val.value is not None,
            )
            result.groups.append(group)

    elif query.type == QueryType.GROUPED_NODE:
        assert query.group_by, f"no group_by for grouped node query: {query!r}"
        grouped_results = _query_grouped_node(
            context,
            query.relation,
            query.select,
            combined_where,
            query.having,
            query.sort,
            query.group_by,
            query.limit,
            query.offset,
        )

        result.groups = []
        for discriminator, values, ptrs in grouped_results:
            group = QueryResultGroup(
                type=QueryType.NODE,
                discriminator=discriminator,
                nodes=values,
                count=len(values),
                exists=len(values) > 0,
            )
            result.groups.append(group)
            nodes_ptr.extend(ptrs)

    elif query.type == QueryType.NODE:
        values, ptrs = _query_node(
            context,
            query.relation,
            query.select,
            combined_where,
            query.sort,
            query.limit,
            query.offset,
        )
        result.nodes = values
        result.count = len(values)
        result.exists = len(values) > 0
        nodes_ptr = list(ptrs)

    else:
        assert_never(query.type)

    return result, nodes_ptr


@tracer.start_as_current_span("memory.execute_subquery")
def _execute_subquery(
    context: MemoryContext,
    result: QueryResult,
    nodes_ptr: Sequence[NodeReference],
    subquery: Query,
) -> QueryResult | None:
    """Execute a subquery to a main Query against in-memory data."""
    if not nodes_ptr:
        return None

    assert subquery.join is not None, f"no join for subquery {subquery!r}"

    # parent join
    if subquery.join.type == JoinType.PARENT:
        # collect parent pointers from result nodes
        parents_ptr: dict[Any, NodeReference] = {}
        for node_value in result.nodes:
            if (parent_ptr_value := node_value.value.get("3")) is not None:
                parent_id = parent_ptr_value["32"]
                if parent_id in parents_ptr:
                    continue
                parent_ptr = NodeReference.from_value(parent_ptr_value)
                parents_ptr[parent_ptr.id] = parent_ptr
        if not parents_ptr:
            return None  # nothing to query here

        if subquery.join.recursive:
            expanded_nodes_ptr = _walk(
                context=context,
                relation=subquery.relation,
                roots_ptr=list(parents_ptr.values()),
                roots_parents_ptr=[],
                direction=EdgeDirection.PARENT,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )

            subquery_where = Condition(
                type=ConditionalType.IN,
                left=to_expression(
                    attribute_ref(subquery.relation.resolve_property_or_error("id"))
                ),
                right=to_expression(to_value([n.id for n in expanded_nodes_ptr])),
            )
        else:
            subquery_where = Condition(
                type=ConditionalType.IN,
                left=to_expression(
                    attribute_ref(subquery.relation.resolve_property_or_error("id"))
                ),
                right=to_expression(to_value(list(parents_ptr.keys()))),
            )

        # execute subquery
        subresult, _ = _query_clause(context, subquery, subquery_where)
        return subresult

    # child join
    elif subquery.join.type == JoinType.CHILD:
        if subquery.join.recursive:
            expanded_nodes_ptr = _walk(
                context=context,
                relation=subquery.relation,
                roots_ptr=[],
                roots_parents_ptr=nodes_ptr,
                direction=EdgeDirection.CHILD,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )

            subquery_where = Condition(
                type=ConditionalType.IN,
                left=to_expression(
                    attribute_ref(subquery.relation.resolve_property_or_error("id"))
                ),
                right=to_expression(to_value([n.id for n in expanded_nodes_ptr])),
            )
        else:
            # Filter subquery to children of the main query nodes
            # Get parent property for the subquery relation
            assert subquery.relation.node_type is not None, (
                f"no node type for {subquery.relation!r}"
            )
            subquery_node_cls = NODE_CLASS_BY_TYPE[subquery.relation.node_type]
            parent_prop = subquery_node_cls.__parent_property__

            parent_condition = Condition(
                type=ConditionalType.IN,
                left=to_expression(attribute_ref(parent_prop)),
                right=to_expression(to_value([ptr.id for ptr in nodes_ptr])),
            )

            if subquery.where is not None:
                subquery_where = Condition(
                    type=ConditionalType.AND,
                    left=to_expression(subquery.where),
                    right=to_expression(parent_condition),
                )
            else:
                subquery_where = parent_condition

        # Execute the subquery
        subresult, _ = _query_clause(context, subquery, subquery_where)
        return subresult

    # left join
    elif subquery.join.type == JoinType.LEFT:
        raise NotImplementedError(f"left join not implemented for subquery: {subquery!r}")

    else:
        assert_never(subquery.join.type)


@tracer.start_as_current_span("memory.query")
def execute_query(
    context: MemoryContext, query: Query, where: Condition | None = None
) -> QueryResult:
    """Execute the Query (and any subqueries) against in-memory data."""
    # Execute main query
    result, nodes_ptr = _query_clause(context, query, where)

    # Execute subqueries
    subresults = []
    for subquery in query.subqueries:
        subresult = _execute_subquery(context, result, nodes_ptr, subquery)
        if subresult is not None:
            subresults.append(subresult)
    result.subresults = subresults

    logger.debug(
        "memory.query",
        query=query,
        result=result,
        subresults=len(subresults),
        span="current",
    )
    return result
