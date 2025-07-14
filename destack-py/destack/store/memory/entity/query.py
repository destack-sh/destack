from collections.abc import Sequence
from typing import assert_never

import structlog
from opentelemetry import trace

from destack.language import (
    EMPTY_DICT,
    Aggregation,
    AggregationType,
    Condition,
    ConditionalType,
    EdgeDirection,
    Entity,
    Expression,
    JoinType,
    NodeDefinitionReference,
    NodeReference,
    PropertyReference,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    Select,
    Sort,
    SortType,
    Value,
    to_value,
)
from destack.utils.uuid import UUID

from ..core import MemoryContext
from ..evaluate import (
    _extract_id_condition,
    evaluate_aggregation,
    evaluate_condition,
    evaluate_expression,
    evaluate_sort_key,
)
from .core import MemoryEntityRow, VersionedNodeKey
from .wiring import unpack_entity_row

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

MAX_RECURSION_DEPTH = 100

ENTITY_PARENT_KEY = str(Entity.property("parent").id)
NODE_ID_ID = Entity.property("id").id
NODE_ID_KEY = str(Entity.property("id").id)

NODE_REFERENCE_TYPE_KEY = str(NodeReference.property("type").id)
NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)
NODE_REFERENCE_SPACE_ID_KEY = str(NodeReference.property("space_id").id)
NODE_REFERENCE_DEFINITION_ID_KEY = str(NodeReference.property("definition_id").id)


def _filter_rows(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    where: Condition | None,
    snapshot_path: Sequence[UUID],
    ignore_multi: bool = False,
) -> list[MemoryEntityRow]:
    """Filter rows based on definition and where condition."""
    snapshot_id = snapshot_path[-1] if snapshot_path else None

    if definition.is_multi and not ignore_multi:
        definitions = context.resolve(definition)
        filtered_rows: list[MemoryEntityRow] = []
        for rel in definitions:
            table = context.get_entity_table(rel)
            for row in table.rows_by_snapshot.get(snapshot_id, EMPTY_DICT).values():
                if where is None or evaluate_condition(row.value, where):
                    filtered_rows.append(row)
    else:
        table = context.get_entity_table(definition)
        if where is not None:
            is_id_query, node_ids = _extract_id_condition(where)
            if is_id_query:
                filtered_rows = []
                for id_val in node_ids:
                    node_key = VersionedNodeKey(id=id_val, snapshot_id=snapshot_id)
                    if (row := table.rows.get(node_key)) is not None and evaluate_condition(
                        row.value, where
                    ):
                        filtered_rows.append(row)
            else:
                filtered_rows = []
                for row in table.rows_by_snapshot.get(snapshot_id, EMPTY_DICT).values():
                    if evaluate_condition(row.value, where):
                        filtered_rows.append(row)
        else:
            filtered_rows = list(table.rows_by_snapshot.get(snapshot_id, EMPTY_DICT).values())

    return filtered_rows


@tracer.start_as_current_span("memory.query_node")
def _query_node(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
    snapshot_path: Sequence[UUID],
    _ignore_multi: bool = False,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query."""
    # fan out multi definitions
    if definition.is_multi and not _ignore_multi:
        if limit is not None or offset is not None:
            raise NotImplementedError(f"cannot limit/offset for multi definition: {definition!r}")
        definitions = context.resolve(definition)
        all_values: list[Value] = []
        all_ptrs: list[NodeReference] = []
        for rel in definitions:
            values, ptrs = _query_node(
                context=context,
                definition=rel,
                select=select,
                where=where,
                sort=sort,
                limit=limit,
                offset=offset,
                snapshot_path=snapshot_path,
                _ignore_multi=True,
            )
            all_values.extend(values)
            all_ptrs.extend(ptrs)
        return all_values, all_ptrs

    # filter
    filtered_rows = _filter_rows(
        context=context,
        definition=definition,
        where=where,
        snapshot_path=snapshot_path,
        ignore_multi=True,
    )

    # sort
    if sort:

        def sort_key(row: MemoryEntityRow) -> tuple:
            return evaluate_sort_key(row.value, sort)

        reverse_flags = any(s.type == SortType.DESCENDING for s in sort)
        filtered_rows.sort(key=sort_key, reverse=reverse_flags)
    if offset:
        filtered_rows = filtered_rows[offset:]
    if limit:
        filtered_rows = filtered_rows[:limit]

    # convert to values
    values: list[Value] = []
    ptrs: list[NodeReference] = []
    for row in filtered_rows:
        values.append(unpack_entity_row(row))
        ptrs.append(row.ptr)

    return values, ptrs


@tracer.start_as_current_span("memory.query_scalar")
def _query_scalar(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
    snapshot_path: Sequence[UUID],
) -> Value:
    """Execute a scalar Query."""
    # filter
    filtered_rows = _filter_rows(
        context=context,
        definition=definition,
        where=where,
        snapshot_path=snapshot_path,
    )

    # execute
    scalar = evaluate_aggregation([row.value for row in filtered_rows], aggregation)
    scalar_value = to_value(scalar)
    logger.trace(
        "memory.query_scalar",
        definition=definition,
        scalar=scalar_value,
        span="current",
    )
    return scalar_value


@tracer.start_as_current_span("memory.query_grouped_node")
def _query_grouped_node(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    select: Select | None,
    where: Condition | None,
    having: Condition | None,
    sort: Sequence[Sort] | None,
    group_by: Sequence[Expression],
    limit: int | None,
    offset: int | None,
    snapshot_path: Sequence[UUID],
) -> list[tuple[Value, list[Value], list[NodeReference]]]:
    """Execute a grouped node Query."""
    if definition.is_multi:
        raise NotImplementedError("grouped node queries not supported for multi definitions")

    # filter
    filtered_rows = _filter_rows(
        context=context,
        definition=definition,
        where=where,
        snapshot_path=snapshot_path,
    )

    # group rows by group_by expressions
    groups: dict[tuple, list[MemoryEntityRow]] = {}
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

            def sort_key(row: MemoryEntityRow) -> tuple:
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
            values.append(unpack_entity_row(row))
            ptrs.append(row.ptr)
        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), values, ptrs))

    logger.trace(
        "memory.query_grouped_node",
        definition=definition,
        groups=len(results),
        total_nodes=len(filtered_rows),
        span="current",
    )
    return results


@tracer.start_as_current_span("memory.query_grouped_scalar")
def _query_grouped_scalar(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
    having: Condition | None,
    group_by: Sequence[Expression],
    snapshot_path: Sequence[UUID],
) -> list[tuple[Value, Value]]:
    """Execute a grouped scalar Query."""
    if definition.is_multi:
        raise NotImplementedError("grouped scalar queries not supported for multi definitions")

    # filter
    filtered_rows = _filter_rows(
        context=context,
        definition=definition,
        where=where,
        snapshot_path=snapshot_path,
    )

    # group rows by group_by expressions
    groups: dict[tuple, list[MemoryEntityRow]] = {}
    for row in filtered_rows:
        group_key = tuple(evaluate_expression(row.value, expr) for expr in group_by)
        if group_key not in groups:
            groups[group_key] = []
        groups[group_key].append(row)

    # apply having filter and aggregation to each group
    results: list[tuple[Value, Value]] = []
    for group_key, group_rows in groups.items():
        if having is not None and group_rows:
            if not evaluate_condition(group_rows[0].value, having):
                continue
        agg_result = evaluate_aggregation([row.value for row in group_rows], aggregation)
        discriminator = group_key[0] if group_key else None
        results.append((to_value(discriminator), to_value(agg_result)))

    logger.trace(
        "memory.query_grouped_scalar",
        definition=definition,
        groups=len(results),
        total_nodes=len(filtered_rows),
        span="current",
    )
    return results


@tracer.start_as_current_span("memory.walk_node")
def _walk_node(
    context: MemoryContext,
    definition: NodeDefinitionReference,
    nodes_ptr: Sequence[NodeReference],
    direction: EdgeDirection,
    depth: int,
    where: Condition | None,
    snapshot_path: Sequence[UUID],
) -> tuple[list[NodeReference], dict[UUID, UUID]]:
    """Get the cascaded Nodes for a query."""

    definitions = context.resolve(definition)
    snapshot_id = snapshot_path[-1] if snapshot_path else None
    nodes_by_id: dict[UUID, NodeReference] = {}
    source_id_by_node_id: dict[UUID, UUID] = {}

    # parent walk
    if direction == EdgeDirection.PARENT:
        # start with root nodes and walk up
        current_depth = 0
        current_node_ids: set[UUID] = {ptr.id for ptr in nodes_ptr}
        for ptr in nodes_ptr:
            nodes_by_id[ptr.id] = ptr
            source_id_by_node_id[ptr.id] = ptr.id
        while current_depth < depth:
            if not current_node_ids:
                break

            next_node_ids: set[UUID] = set()
            for rel in definitions:
                table = context.get_entity_table(rel)
                for node_id in current_node_ids:
                    node_key = VersionedNodeKey(id=node_id, snapshot_id=snapshot_id)
                    if (
                        (row := table.rows.get(node_key)) is not None
                        and (parent_ptr := row.parent_ptr) is not None
                        and (parent_id := parent_ptr.id) not in nodes_by_id
                        and (where is None or evaluate_condition(row.value, where))
                    ):
                        source_id_by_node_id[parent_id] = source_id_by_node_id[node_id]
                        nodes_by_id[parent_id] = parent_ptr
                        next_node_ids.add(parent_id)

            current_node_ids = next_node_ids
            current_depth += 1

    # child walk
    elif direction == EdgeDirection.CHILD:
        # start with root parents and walk down
        current_depth = 0
        current_parent_ids: set[UUID] = {ptr.id for ptr in nodes_ptr}
        for ptr in nodes_ptr:
            source_id_by_node_id[ptr.id] = ptr.id
        while current_depth < depth:
            if not current_parent_ids:
                break

            next_parent_ids: set[UUID] = set()
            for rel in definitions:
                table = context.get_entity_table(rel)
                for parent_id in current_parent_ids:
                    parent_key = VersionedNodeKey(id=parent_id, snapshot_id=snapshot_id)
                    if children := table.rows_by_parent.get(parent_key):
                        for row in children:
                            if (node_id := row.id) not in nodes_by_id and (
                                where is None or evaluate_condition(row.value, where)
                            ):
                                source_id_by_node_id[node_id] = source_id_by_node_id[parent_id]
                                nodes_by_id[node_id] = row.ptr
                                next_parent_ids.add(node_id)

            current_parent_ids = next_parent_ids
            current_depth += 1

    # side walk
    elif direction == EdgeDirection.SIDE:
        raise NotImplementedError(f"cannot walk {definition!r} in direction: {direction!r}")

    else:
        assert_never(direction)

    return list(nodes_by_id.values()), source_id_by_node_id


@tracer.start_as_current_span("memory.query_clause")
def _query_clause(
    context: MemoryContext,
    query: Query,
    where: Condition | None,
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
            definition=query.definition,
            select=query.select,
            where=combined_where,
            sort=query.sort,
            limit=query.limit,
            offset=query.offset,
            snapshot_path=query.snapshot_path,
        )
        result = QueryResult(id=query.id, type=query.type, nodes=nodes)

    # scalar
    elif query.type == QueryType.SCALAR:
        assert query.aggregation is not None, f"no aggregation for scalar query: {query!r}"
        scalar_result = _query_scalar(
            context=context,
            definition=query.definition,
            aggregation=query.aggregation,
            where=combined_where,
            snapshot_path=query.snapshot_path,
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
            definition=query.definition,
            select=query.select,
            where=combined_where,
            having=query.having,
            sort=query.sort,
            group_by=query.group_by,
            limit=query.limit,
            offset=query.offset,
            snapshot_path=query.snapshot_path,
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
            definition=query.definition,
            aggregation=query.aggregation,
            where=combined_where,
            having=query.having,
            group_by=query.group_by,
            snapshot_path=query.snapshot_path,
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

    logger.trace(
        "memory.query_clause",
        query=query,
        result=result,
        nodes=len(nodes_ptr),
        span="current",
    )
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
            if (
                node_value.value is not None
                and (parent_ptr_value := node_value.value.get(ENTITY_PARENT_KEY)) is not None
            ):
                parent_id = UUID(parent_ptr_value[NODE_REFERENCE_ID_KEY])
                if parent_id in parents_ptr:
                    continue
                parent_ptr = NodeReference.from_value(parent_ptr_value)
                parents_ptr[parent_ptr.id] = parent_ptr
        if not parents_ptr:
            return None  # nothing to query here
        if subquery.join.recursive:
            expanded_nodes_ptr, _ = _walk_node(
                context=context,
                definition=subquery.definition,
                nodes_ptr=list(parents_ptr.values()),
                direction=EdgeDirection.PARENT,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
                snapshot_path=subquery.snapshot_path,
            )
            subquery_where = subquery.definition.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr)
            )
        else:
            subquery_where = subquery.definition.resolve_property_or_error("id").in_(
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
            expanded_nodes_ptr, _ = _walk_node(
                context=context,
                definition=subquery.definition,
                nodes_ptr=nodes_ptr,
                direction=EdgeDirection.CHILD,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
                snapshot_path=subquery.snapshot_path,
            )
            subquery_where = Condition(
                type=ConditionalType.IN,
                left=Expression.of(
                    PropertyReference.of(subquery.definition.resolve_property_or_error("id"))
                ),
                right=Expression.of(to_value([n.id for n in expanded_nodes_ptr])),
            )
        else:
            subquery_where = subquery.definition.resolve_property_or_error("parent").in_(
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
    context: MemoryContext,
    query: Query,
    where: Condition | None = None,
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

    logger.trace(
        "memory.query",
        query=query,
        result=result,
        subresults=len(subresults),
        span="current",
    )
    return result
