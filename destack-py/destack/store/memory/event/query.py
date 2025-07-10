from collections.abc import Sequence
from typing import Any, assert_never

import structlog
from opentelemetry import trace

from destack.language import (
    Aggregation,
    AggregationType,
    Condition,
    Event,
    Expression,
    Node,
    NodeDefinitionReference,
    NodeReference,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    Sort,
    SortType,
    Value,
    to_value,
)

from ..core import MemoryContext
from ..evaluate import (
    _extract_id_condition,
    evaluate_aggregation,
    evaluate_condition,
    evaluate_expression,
    evaluate_sort_key,
)
from .core import MemoryEventRow
from .wiring import unpack_event_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

NODE_ID_ID = Node.property("id").id
NODE_ID_KEY = str(Node.property("id").id)
NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)

EVENT_CREATED_AT_KEY = str(Event.property("created_at").id)


@tracer.start_as_current_span("memory.query_event_node")
def _query_event_node(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query for events."""
    table = context.get_event_table(definition)

    # filter rows
    if where is not None:
        is_id_query, node_ids = _extract_id_condition(where)
        if is_id_query:
            filtered_rows: list[MemoryEventRow] = []
            for id_val in node_ids:
                if (row := table.rows.get(id_val)) is not None and evaluate_condition(
                    row.value, where
                ):
                    filtered_rows.append(row)
        else:
            # filter from sorted list for better performance
            filtered_rows = []
            for row in table.rows_sorted:
                if evaluate_condition(row.value, where):
                    filtered_rows.append(row)
    else:
        # use pre-sorted list
        filtered_rows = list(table.rows_sorted)

    # sort if needed (override default created_at sort)
    if sort:

        def sort_key(row: MemoryEventRow) -> tuple:
            return evaluate_sort_key(row.value, sort)

        reverse_flags = any(s.type == SortType.DESCENDING for s in sort)
        filtered_rows.sort(key=sort_key, reverse=reverse_flags)

    # apply offset and limit
    if offset:
        filtered_rows = filtered_rows[offset:]
    if limit:
        filtered_rows = filtered_rows[:limit]

    # convert to values
    values: list[Value] = []
    ptrs: list[NodeReference] = []
    for row in filtered_rows:
        values.append(unpack_event_row(row))
        ptrs.append(row.ptr)

    return values, ptrs


@tracer.start_as_current_span("memory.query_event_scalar")
def _query_event_scalar(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query for events."""
    table = context.get_event_table(definition)

    # collect values for aggregation
    if where is not None:
        values_for_agg = []
        for row in table.rows_sorted:
            if evaluate_condition(row.value, where):
                values_for_agg.append(row.value)
    else:
        values_for_agg = [row.value for row in table.rows_sorted]

    # execute aggregation
    scalar = evaluate_aggregation(values_for_agg, aggregation)
    scalar_value = to_value(scalar)

    logger.trace(
        "memory.query_event_scalar",
        definition=definition,
        scalar=scalar_value,
        span="current",
    )
    return scalar_value


@tracer.start_as_current_span("memory.query_event_grouped_node")
def _query_event_grouped_node(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    where: Condition | None,
    having: Condition | None,
    sort: Sequence[Sort] | None,
    group_by: Sequence[Expression],
    limit: int | None,
    offset: int | None,
) -> list[tuple[Value, list[Value], list[NodeReference]]]:
    """Execute a grouped node Query for events."""
    table = context.get_event_table(definition)

    # filter
    if where is not None:
        filtered_rows = []
        for row in table.rows_sorted:
            if evaluate_condition(row.value, where):
                filtered_rows.append(row)
    else:
        filtered_rows = list(table.rows_sorted)

    # group rows by group_by expressions
    groups: dict[tuple, list[MemoryEventRow]] = {}
    for row in filtered_rows:
        group_key = tuple(evaluate_expression(row.value, expr) for expr in group_by)
        if group_key not in groups:
            groups[group_key] = []
        groups[group_key].append(row)

    # apply having filter and collect results
    results: list[tuple[Value, list[Value], list[NodeReference]]] = []
    for group_key, group_rows in groups.items():
        if having is not None and group_rows:
            if not evaluate_condition(group_rows[0].value, having):
                continue

        if sort:

            def sort_key(row: MemoryEventRow) -> tuple:
                return evaluate_sort_key(row.value, sort)

            reverse_flags = any(s.type == SortType.DESCENDING for s in sort)
            group_rows.sort(key=sort_key, reverse=reverse_flags)

        if offset:
            group_rows = group_rows[offset:]
        if limit:
            group_rows = group_rows[:limit]

        values: list[Value] = []
        ptrs: list[NodeReference] = []
        for row in group_rows:
            values.append(unpack_event_row(row))
            ptrs.append(row.ptr)

        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), values, ptrs))

    logger.trace(
        "memory.query_event_grouped_node",
        definition=definition,
        groups=len(results),
        total_nodes=len(filtered_rows),
        span="current",
    )
    return results


@tracer.start_as_current_span("memory.query_event_grouped_scalar")
def _query_event_grouped_scalar(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
    having: Condition | None,
    group_by: Sequence[Expression],
) -> list[tuple[Value, Value]]:
    """Execute a grouped scalar Query for events."""
    table = context.get_event_table(definition)

    # filter
    if where is not None:
        filtered_rows = []
        for row in table.rows_sorted:
            if evaluate_condition(row.value, where):
                filtered_rows.append(row)
    else:
        filtered_rows = list(table.rows_sorted)

    # group rows by group_by expressions
    groups: dict[tuple, list[dict[str, Any]]] = {}
    for row in filtered_rows:
        group_key = tuple(evaluate_expression(row.value, expr) for expr in group_by)
        if group_key not in groups:
            groups[group_key] = []
        groups[group_key].append(row.value)

    # apply having filter and aggregation to each group
    results: list[tuple[Value, Value]] = []
    for group_key, group_values in groups.items():
        if having is not None and group_values:
            if not evaluate_condition(group_values[0], having):
                continue
        agg_result = evaluate_aggregation(group_values, aggregation)
        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), to_value(agg_result)))

    logger.trace(
        "memory.query_event_grouped_scalar",
        definition=definition,
        groups=len(results),
        total_nodes=len(filtered_rows),
        span="current",
    )
    return results


@tracer.start_as_current_span("memory.query_event")
def execute_query(
    context: MemoryContext,
    query: Query,
) -> QueryResult:
    """Execute an event Query."""

    result: QueryResult
    nodes_ptr: list[NodeReference]

    # node
    if query.type == QueryType.NODE:
        nodes, nodes_ptr = _query_event_node(
            context=context,
            definition=query.definition,
            where=query.where,
            sort=query.sort,
            limit=query.limit,
            offset=query.offset,
        )
        result = QueryResult(id=query.id, type=query.type, nodes=nodes)

    # scalar
    elif query.type == QueryType.SCALAR:
        assert query.aggregation is not None, f"no aggregation for scalar query: {query!r}"
        scalar_result = _query_event_scalar(
            context=context,
            definition=query.definition,
            aggregation=query.aggregation,
            where=query.where,
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
        groups_value = _query_event_grouped_node(
            context=context,
            definition=query.definition,
            where=query.where,
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
        groups_value = _query_event_grouped_scalar(
            context=context,
            definition=query.definition,
            aggregation=query.aggregation,
            where=query.where,
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

    # note: subqueries not supported for events
    if query.subqueries:
        raise NotImplementedError("Subqueries not supported for event queries")

    logger.trace(
        "memory.query_event",
        query=query,
        result=result,
        span="current",
    )
    return result
