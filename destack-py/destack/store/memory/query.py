from collections.abc import Sequence
from typing import Any, assert_never

import structlog
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
    PrimitiveType,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    RelationReference,
    RelationType,
    ScalarType,
    Select,
    Sort,
    SortType,
    TypeCardinality,
    Value,
    attribute_ref,
    to_value,
)
from destack.language import expression as to_expression
from destack.utils.uuid import UUID

from .core import MemoryContext, MemoryRow
from .wiring import unpack_node_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

MAX_RECURSION_DEPTH = 100


def _evaluate_expression(context: MemoryContext, expression: Expression, row: MemoryRow) -> Any:
    """Evaluate an Expression against in-memory row data."""
    if expression.type == ExpressionType.LITERAL:
        assert expression.literal is not None, f"no literal for {expression!r}"
        if expression.literal.type.scalar_type == ScalarType.NODE_REFERENCE:
            return expression.literal.value["32"] if expression.literal.value is not None else None
        else:
            return expression.literal.value
    elif expression.type == ExpressionType.ATTRIBUTE:
        assert expression.attribute is not None, f"no attribute for {expression!r}"
        attr = expression.attribute
        if attr.type == AttributeType.PROPERTY:
            prop = attr.prop
            assert prop is not None, f"no property for {attr!r}"
            if prop.scalar_type == ScalarType.NODE_REFERENCE:
                node_ptr_packed = row.value.get(str(prop.id))
                return node_ptr_packed["32"] if node_ptr_packed is not None else None
            else:
                return row.value.get(str(prop.id))
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
        key_values: list[Any] = []
        for s in sort:
            val = _evaluate_expression(context, s.by, row)
            # Handle None values by putting them at the end
            val = (1, None) if val is None else (0, val)
            key_values.append(val)
        return tuple(key_values)

    reverse_flags = tuple(s.type == SortType.DESCENDING for s in sort)
    rows.sort(key=sort_key, reverse=all(reverse_flags))
    return rows


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
    if (
        (condition.type == ConditionalType.EQUALS or condition.type == ConditionalType.IN)
        and (left := condition.left) is not None
        and left.type == ExpressionType.ATTRIBUTE
        and (attribute := left.attribute) is not None
        and attribute.type == AttributeType.PROPERTY
        and (prop_ptr := attribute.prop_ptr) is not None
        and prop_ptr.id == 2
        and (right := condition.right) is not None
        and right.type == ExpressionType.LITERAL
    ):
        assert right.literal is not None, f"no literal for {right!r}"
        if right.literal.type.cardinality == TypeCardinality.SCALAR:
            if right.literal.type.primitive_type == PrimitiveType.UUID:
                ids = (right.literal.unpack(),)
            else:
                ids = (right.literal.unpack().id,)
        else:
            if right.literal.type.primitive_type == PrimitiveType.UUID:
                ids = right.literal.unpack()
            else:
                ids = [ptr.id for ptr in right.literal.unpack()]
        return True, ids

    return False, ()


@tracer.start_as_current_span("memory.walk_node")
def _walk_node(
    context: MemoryContext,
    relation: RelationReference,
    roots_ptr: Sequence[NodeReference],
    roots_parents_ptr: Sequence[NodeReference],
    direction: EdgeDirection,
    depth: int,
    where: Condition | None,
) -> list[NodeReference]:
    """Get the cascaded Nodes for a query."""

    nodes_by_id: dict[UUID, NodeReference] = {ptr.id: ptr for ptr in roots_ptr}
    relations = context.resolve_relation(relation)

    # parent walk
    if direction == EdgeDirection.PARENT:
        # start with root nodes and walk up
        current_depth = 0
        current_node_ids: set[UUID] = {ptr.id for ptr in roots_ptr}
        while current_depth < depth:
            if not current_node_ids:
                break

            next_node_ids: set[UUID] = set()
            for rel in relations:
                table = context.get_relation(rel)
                for node_id in current_node_ids:
                    if (
                        (row := table.rows.get(node_id)) is not None
                        and (parent_ptr := row.parent_ptr) is not None
                        and (parent_id := parent_ptr.id) not in nodes_by_id
                        and (where is None or _evaluate_condition(context, where, row))
                    ):
                        nodes_by_id[parent_id] = parent_ptr
                        next_node_ids.add(parent_id)

            current_node_ids = next_node_ids
            current_depth += 1

    # child walk
    elif direction == EdgeDirection.CHILD:
        # start with root parents and walk down
        current_depth = 0
        current_parent_ids: set[UUID] = {ptr.id for ptr in roots_parents_ptr}

        while current_depth < depth:
            if not current_parent_ids:
                break

            next_parent_ids: set[UUID] = set()
            for rel in relations:
                table = context.get_relation(rel)
                for parent_id in current_parent_ids:
                    if children := table.rows_by_parent_id.get(parent_id):
                        for row in children:
                            if (where is None or _evaluate_condition(context, where, row)) and (
                                node_id := row.id
                            ) not in nodes_by_id:
                                nodes_by_id[node_id] = row.ptr
                                next_parent_ids.add(node_id)

            current_parent_ids = next_parent_ids
            current_depth += 1

    # side walk
    elif direction == EdgeDirection.SIDE:
        raise NotImplementedError(f"cannot walk {relation!r} in direction: {direction!r}")

    else:
        assert_never(direction)

    return list(nodes_by_id.values())


@tracer.start_as_current_span("memory.query_scalar")
def _query_scalar(
    context: MemoryContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query."""
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
    """Execute a grouped scalar Query."""
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
    """Execute a grouped node Query."""
    if relation.type == RelationType.TRAIT:
        raise NotImplementedError("grouped node queries not supported for trait relations")

    table = context.get_relation(relation)

    # filter
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
            values.append(unpack_node_row(table, row))
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
    """Execute a node Query."""
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

    # filter
    if where is not None:
        is_id_query, id_values = _is_id_condition(where)
        if is_id_query:
            filtered_rows: list[MemoryRow] = []
            for id_val in id_values:
                if (row := table.rows.get(id_val)) is not None:
                    filtered_rows.append(row)
        else:
            filtered_rows = []
            for row in table.rows.values():
                if _evaluate_condition(context, where, row):
                    filtered_rows.append(row)
    else:
        filtered_rows = list(table.rows.values())

    # sort
    if sort:
        filtered_rows = _evaluate_sort(context, sort, filtered_rows)
    if offset:
        filtered_rows = filtered_rows[offset:]
    if limit:
        filtered_rows = filtered_rows[:limit]

    # convert to values
    values: list[Value] = []
    ptrs: list[NodeReference] = []
    for row in filtered_rows:
        values.append(unpack_node_row(table, row))
        ptrs.append(row.ptr)

    return values, ptrs


@tracer.start_as_current_span("memory.query_clause")
def _query_clause(
    context: MemoryContext, query: Query, where: Condition | None
) -> tuple[QueryResult, Sequence[NodeReference]]:
    """Execute the specific Query "clause" (ignoring subqueries)."""
    # combine wheres
    if where is not None:
        combined_where = where if query.where is None else query.where & where
    else:
        combined_where = query.where

    result: QueryResult
    nodes_ptr: list[NodeReference]

    # node
    if query.type == QueryType.NODE:
        nodes, nodes_ptr = _query_node(
            context=context,
            relation=query.relation,
            select=query.select,
            where=combined_where,
            sort=query.sort,
            limit=query.limit,
            offset=query.offset,
        )
        result = QueryResult(id=query.id, type=query.type, nodes=nodes)

    # scalar
    elif query.type == QueryType.SCALAR:
        assert query.aggregation is not None, f"no aggregation for scalar query: {query!r}"
        scalar_result = _query_scalar(
            context=context,
            relation=query.relation,
            aggregation=query.aggregation,
            where=combined_where,
        )
        nodes_ptr = []
        result = QueryResult(id=query.id, type=query.type, scalar=scalar_result)
        if query.aggregation.type == AggregationType.EXISTS:
            result.exists = scalar_result.unpack(bool)
        elif query.aggregation.type == AggregationType.COUNT:
            result.count = scalar_result.unpack(int)

    # grouped node
    elif query.type == QueryType.GROUPED_NODE:
        assert query.group_by, f"no group_by for grouped node query: {query!r}"
        groups_value = _query_grouped_node(
            context=context,
            relation=query.relation,
            select=query.select,
            where=combined_where,
            having=query.having,
            sort=query.sort,
            group_by=query.group_by,
            limit=query.limit,
            offset=query.offset,
        )
        nodes_ptr = []
        groups: list[QueryResultGroup] = []
        for group_discriminator, group_nodes, group_nodes_ptrs in groups_value:
            group = QueryResultGroup(
                type=QueryType.NODE,
                discriminator=group_discriminator,
                nodes=group_nodes,
            )
            groups.append(group)
            nodes_ptr.extend(group_nodes_ptrs)
        result = QueryResult(id=query.id, type=query.type, groups=groups)

    # grouped scalar
    elif query.type == QueryType.GROUPED_SCALAR:
        assert query.aggregation is not None, f"no aggregation for grouped scalar query: {query!r}"
        assert query.group_by, f"no group_by for grouped scalar query: {query!r}"
        groups_value = _query_grouped_scalar(
            context=context,
            relation=query.relation,
            aggregation=query.aggregation,
            where=combined_where,
            having=query.having,
            group_by=query.group_by,
        )
        nodes_ptr = []
        groups: list[QueryResultGroup] = []
        for group_discriminator, group_scalar_val in groups_value:
            group = QueryResultGroup(
                type=QueryType.SCALAR,
                discriminator=group_discriminator,
                scalar=group_scalar_val,
            )
            groups.append(group)
        result = QueryResult(id=query.id, type=query.type, groups=groups)

    else:
        assert_never(query.type)

    return result, nodes_ptr


def _execute_subquery(
    context: MemoryContext,
    result: QueryResult,
    nodes_ptr: Sequence[NodeReference],
    subquery: Query,
) -> QueryResult | None:
    """Execute a subquery to a main Query."""
    if not nodes_ptr:
        return None

    assert subquery.join is not None, f"no join for subquery {subquery!r}"

    # parent join
    if subquery.join.type == JoinType.PARENT:
        # collect/walk
        parents_ptr: dict[UUID, NodeReference] = {}
        for node_value in result.nodes:
            if (parent_ptr_value := node_value.value.get("3")) is not None:
                parent_id = UUID(parent_ptr_value["32"])
                if parent_id in parents_ptr:
                    continue
                parent_ptr = NodeReference.from_value(parent_ptr_value)
                parents_ptr[parent_ptr.id] = parent_ptr
        if not parents_ptr:
            return None  # nothing to query here
        if subquery.join.recursive:
            expanded_nodes_ptr = _walk_node(
                context=context,
                relation=subquery.relation,
                roots_ptr=list(parents_ptr.values()),
                roots_parents_ptr=(),
                direction=EdgeDirection.PARENT,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )
            subquery_where = subquery.relation.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr)
            )
        else:
            subquery_where = subquery.relation.resolve_property_or_error("id").in_(
                *parents_ptr.keys()
            )

        # execute subquery
        subresult = execute_query(context=context, query=subquery, where=subquery_where)
        return subresult

    # child join
    elif subquery.join.type == JoinType.CHILD:
        # collect/walk
        if not nodes_ptr:
            return None  # nothing to query here
        if subquery.join.recursive:
            expanded_nodes_ptr = _walk_node(
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
            subquery_where = subquery.relation.resolve_property_or_error("parent").in_(
                *(n.id for n in nodes_ptr),
            )

        # execute subquery
        subresult = execute_query(context=context, query=subquery, where=subquery_where)
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
    """Execute the Query (and any subqueries)."""

    # execute main query
    result, nodes_ptr = _query_clause(context, query, where)

    # execute subqueries
    subresults: list[QueryResult] = []
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
