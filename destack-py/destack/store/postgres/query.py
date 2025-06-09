import asyncio
import uuid
from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import fastuuid
import structlog
from fastuuid import UUID
from opentelemetry import trace

from destack.language import (
    Aggregation,
    AggregationType,
    AttributeReference,
    AttributeType,
    Condition,
    ConditionalType,
    EdgeDirection,
    Expression,
    ExpressionType,
    Function,
    JoinType,
    NodeReference,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    RelationReference,
    ScalarType,
    Select,
    Sort,
    Value,
    to_value,
)
from destack.language.core.builtin.const import NodeType

from .core import PostgresContext
from .map import DESTACK_CUSTOM_FIELD_PREFIX
from .wiring import pack_column_flat, unpack_node_row

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)

MAX_RECURSION_DEPTH = 1_000


def _compile_value(context: PostgresContext, arguments_out: list[Any], value: Value) -> str:
    """Compile a Value into a SQL expression."""
    if value.type.scalar_type == ScalarType.NODE_REFERENCE:
        # unravel reference column into id
        value_id = uuid.UUID(value.value["32"])
        arguments_out.append(value_id)
        return f"${len(arguments_out)}"
    else:
        value_packed = pack_column_flat(value.type, value.value)
        arguments_out.append(value_packed)
        return f"${len(arguments_out)}"


def _compile_select(context: PostgresContext, arguments_out: list[Any], select: Select) -> str:
    """Compile a Select into a SQL SELECT clause."""
    raise NotImplementedError(select)


def _compile_attribute(
    context: PostgresContext, arguments_out: list[Any], attribute: AttributeReference
) -> str:
    """Compile an Attribute into a SQL expression."""
    if attribute.type == AttributeType.PROPERTY:
        prop = attribute.prop
        assert prop is not None, f"no property for {attribute!r}"
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # unravel reference column into id
            return f"{prop.name}_id"
        else:
            return prop.name
    elif attribute.type == AttributeType.FIELD:
        field = attribute.field
        assert field is not None, f"no field for {attribute!r}"
        field_name = f"{DESTACK_CUSTOM_FIELD_PREFIX}{str(field.id).replace('-', '')}"
        if field.scalar_type == ScalarType.NODE_REFERENCE:
            return f"{field_name}_id"
        else:
            return field_name
    else:
        assert_never(attribute.type)


def _compile_condition(
    context: PostgresContext, arguments_out: list[Any], condition: Condition
) -> str:
    """Compile a Condition into a SQL WHERE clause."""
    # logical
    if condition.type == ConditionalType.NOT:
        return f"NOT ({_compile_expression(context, arguments_out, condition.left)})"
    elif condition.type == ConditionalType.AND:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"({_compile_expression(context, arguments_out, condition.left)} AND {_compile_expression(context, arguments_out, condition.right)})"
    elif condition.type == ConditionalType.OR:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"({_compile_expression(context, arguments_out, condition.left)} OR {_compile_expression(context, arguments_out, condition.right)})"
    # comparison
    elif condition.type == ConditionalType.EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} = {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.NOT_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} != {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.GREATER_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} > {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.GREATER_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} >= {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.LESS_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} < {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.LESS_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} <= {_compile_expression(context, arguments_out, condition.right)}"
    # string
    elif condition.type == ConditionalType.MATCHES:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} LIKE {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.STARTS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} LIKE '%' + {_compile_expression(context, arguments_out, condition.right)}"
    elif condition.type == ConditionalType.ENDS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} LIKE {_compile_expression(context, arguments_out, condition.right)} + '%'"
    # collections
    elif condition.type == ConditionalType.IN:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} = ANY({_compile_expression(context, arguments_out, condition.right)})"
    elif condition.type == ConditionalType.NOT_IN:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} != ANY({_compile_expression(context, arguments_out, condition.right)})"
    # existence
    elif condition.type == ConditionalType.EXISTS:
        return f"EXISTS ({_compile_expression(context, arguments_out, condition.left)})"
    elif condition.type == ConditionalType.NOT_EXISTS:
        return f"NOT EXISTS ({_compile_expression(context, arguments_out, condition.left)})"
    else:
        assert_never(condition.type)


def _compile_sort(context: PostgresContext, arguments_out: list[Any], sort: Sequence[Sort]) -> str:
    """Compile a Sort into a SQL ORDER BY clause."""
    from destack.language import SortType

    sort_parts = []
    for s in sort:
        expr_sql = _compile_expression(context, arguments_out, s.by)
        if s.type == SortType.ASCENDING:
            sort_parts.append(f"{expr_sql} ASC")
        elif s.type == SortType.DESCENDING:
            sort_parts.append(f"{expr_sql} DESC")
        else:
            assert_never(s.type)

    return ", ".join(sort_parts)


def _compile_function(
    context: PostgresContext, arguments_out: list[Any], function: Function
) -> str:
    """Compile a Function into a SQL expression."""
    from destack.language import FunctionType

    left_sql = _compile_expression(context, arguments_out, function.left)

    if function.type == FunctionType.ADD:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"({left_sql} + {right_sql})"
    elif function.type == FunctionType.SUBTRACT:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"({left_sql} - {right_sql})"
    elif function.type == FunctionType.MULTIPLY:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"({left_sql} * {right_sql})"
    elif function.type == FunctionType.DIVIDE:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"({left_sql} / {right_sql})"
    elif function.type == FunctionType.MODULO:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"({left_sql} % {right_sql})"
    elif function.type == FunctionType.POWER:
        assert function.right is not None, f"no right operand for {function!r}"
        right_sql = _compile_expression(context, arguments_out, function.right)
        return f"POWER({left_sql}, {right_sql})"
    else:
        assert_never(function.type)


def _compile_aggregation(
    context: PostgresContext, arguments_out: list[Any], aggregation: Aggregation
) -> str:
    """Compile an Aggregation into a SQL expression."""
    from destack.language import AggregationType

    if aggregation.type == AggregationType.EXISTS:
        return "1"
    elif aggregation.type == AggregationType.COUNT:
        if aggregation.expression is not None:
            expr_sql = _compile_expression(context, arguments_out, aggregation.expression)
            return f"COUNT({expr_sql})"
        else:
            return "COUNT(*)"
    elif aggregation.type == AggregationType.SUM:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        expr_sql = _compile_expression(context, arguments_out, aggregation.expression)
        return f"SUM({expr_sql})"
    elif aggregation.type == AggregationType.MIN:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        expr_sql = _compile_expression(context, arguments_out, aggregation.expression)
        return f"MIN({expr_sql})"
    elif aggregation.type == AggregationType.MAX:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        expr_sql = _compile_expression(context, arguments_out, aggregation.expression)
        return f"MAX({expr_sql})"
    elif aggregation.type == AggregationType.AVERAGE:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        expr_sql = _compile_expression(context, arguments_out, aggregation.expression)
        return f"AVG({expr_sql})"
    else:
        assert_never(aggregation.type)


def _compile_expression(
    context: PostgresContext, arguments_out: list[Any], expr: Expression
) -> str:
    """Compile an Expression into a SQL expression."""
    if expr.type == ExpressionType.LITERAL:
        assert expr.literal is not None, f"no literal for {expr!r}"
        return _compile_value(context, arguments_out, expr.literal)
    elif expr.type == ExpressionType.ATTRIBUTE:
        assert expr.attribute is not None, f"no attribute for {expr!r}"
        return _compile_attribute(context, arguments_out, expr.attribute)
    elif expr.type == ExpressionType.CONDITION:
        assert expr.condition is not None, f"no condition for {expr!r}"
        return _compile_condition(context, arguments_out, expr.condition)
    elif expr.type == ExpressionType.FUNCTION:
        assert expr.function is not None, f"no function for {expr!r}"
        return _compile_function(context, arguments_out, expr.function)
    elif expr.type == ExpressionType.AGGREGATION:
        assert expr.aggregation is not None, f"no aggregation for {expr!r}"
        return _compile_aggregation(context, arguments_out, expr.aggregation)
    else:
        assert_never(expr.type)


@tracer.start_as_current_span("database.walk_node")
async def _walk_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
    relation: RelationReference,
    roots_ptr: Sequence[NodeReference],
    roots_parents_ptr: Sequence[NodeReference],
    direction: EdgeDirection,
    depth: int,
    where: Condition | None,
) -> list[NodeReference]:
    """Get the cascaded Nodes for a query."""

    roots_ids: list[UUID] = [n.id for n in roots_ptr]
    roots_parents_ids: list[UUID] = [n.id for n in roots_parents_ptr]
    arguments: list[Any] = [roots_ids, roots_parents_ids, depth]
    where_sql = _compile_condition(context, arguments, where) if where is not None else "TRUE"

    # parent walk
    if direction == EdgeDirection.PARENT:
        if relation.is_multi:
            # fan out relation
            tables = [context.get_relation(r) for r in context.resolve_relation(relation)]

            # build a single UNION of all node tables
            union_parts = [
                f"SELECT id, parent_id, {tbl.node_type.value}::int AS node_type FROM {tbl.name}"
                for tbl in tables
            ]
            union_subquery = " UNION ALL ".join(union_parts)
            # anchor term
            base_sql = f"""
SELECT id,
    parent_id,
    1 AS depth,
    node_type
FROM (
    {union_subquery}
) roots
WHERE (id = ANY($1) OR parent_id = ANY($2))
AND {where_sql}
            """
            # recursive term
            recursive_sql = f"""
SELECT p.id,
    p.parent_id,
    t.depth + 1 AS depth,
    p.node_type
FROM (
    {union_subquery}
) p
JOIN tree t ON p.id = t.parent_id
WHERE t.depth < $3
AND {where_sql}
            """
            # final statement
            stmt = f"""
WITH RECURSIVE tree AS (
    {base_sql}
    UNION ALL
    {recursive_sql}
)
SELECT id, parent_id, depth, node_type
FROM tree;
            """

            result_rows = await conn.fetch(stmt, *arguments)
            result_nodes_ptr: list[NodeReference] = []
            for row in result_rows:
                node_type = NodeType(int(row["node_type"]))
                node_ptr = NodeReference(node_type=node_type, id=fastuuid.UUID(str(row["id"])))
                result_nodes_ptr.append(node_ptr)
            return result_nodes_ptr
        else:
            table = context.get_relation(relation)
            stmt = f"""
WITH RECURSIVE tree AS (
    SELECT  id,
            parent_id,
            1 AS depth
    FROM    {table.name}
    WHERE   (id = ANY($1) OR parent_id = ANY($2)) AND {where_sql}

    UNION ALL

    SELECT  p.id,
            p.parent_id,
            t.depth + 1
    FROM    {table.name}  AS p
    JOIN    tree      AS t ON p.id = t.parent_id
    WHERE   t.depth < $3 AND {where_sql}
)
SELECT  id,
        parent_id,
        depth
FROM    tree;
"""
            result_rows = await conn.fetch(stmt, *arguments)
            result_nodes_ptr: list[NodeReference] = []
            for row in result_rows:
                node_ptr = NodeReference(
                    node_type=table.node_type, id=fastuuid.UUID(str(row["id"]))
                )
                result_nodes_ptr.append(node_ptr)
            return result_nodes_ptr

    # child walk
    elif direction == EdgeDirection.CHILD:
        # fan out relation
        tables = [context.get_relation(r) for r in context.resolve_relation(relation)]

        # build a single UNION of all node tables
        union_parts = [
            f"SELECT id, parent_id, {tbl.node_type.value}::int AS node_type FROM {tbl.name}"
            for tbl in tables
        ]
        union_subquery = " UNION ALL ".join(union_parts)
        # anchor term
        base_sql = f"""
SELECT id,
    parent_id,
    1 AS depth,
    node_type
FROM (
    {union_subquery}
) roots
WHERE (id = ANY($1) OR parent_id = ANY($2))
AND {where_sql}
        """
        # recursive term
        recursive_sql = f"""
SELECT c.id,
    c.parent_id,
    t.depth + 1 AS depth,
    c.node_type
FROM (
    {union_subquery}
) c
JOIN tree t ON c.parent_id = t.id
WHERE t.depth < $3
AND {where_sql}
        """
        # final statement
        stmt = f"""
WITH RECURSIVE tree AS (
    {base_sql}
    UNION ALL
    {recursive_sql}
)
SELECT id, parent_id, depth, node_type
FROM tree;
        """

        result_rows = await conn.fetch(stmt, *arguments)
        result_nodes_ptr: list[NodeReference] = []
        for row in result_rows:
            node_type = NodeType(int(row["node_type"]))
            node_ptr = NodeReference(node_type=node_type, id=fastuuid.UUID(str(row["id"])))
            result_nodes_ptr.append(node_ptr)
        return result_nodes_ptr

    # side walk
    elif direction == EdgeDirection.SIDE:
        raise NotImplementedError(f"cannot walk {relation!r} in direction: {direction!r}")

    else:
        assert_never(direction)


@tracer.start_as_current_span("database.query_node")
async def _query_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
    relation: RelationReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query."""
    if relation.is_multi:
        # fan out multi relations
        if limit is not None or offset is not None:
            raise NotImplementedError(f"cannot limit/offset for multi relation: {relation!r}")
        subrelations = context.resolve_relation(relation)
        nodes_value: list[Value] = []
        nodes_ptr: list[NodeReference] = []
        for subrelation in subrelations:
            subnodes_value, subnodes_ptr = await _query_node(
                conn=conn,
                context=context,
                relation=subrelation,
                select=select,
                where=where,
                sort=sort,
                limit=limit,
                offset=offset,
            )
            nodes_value.extend(subnodes_value)
            nodes_ptr.extend(subnodes_ptr)
        return nodes_value, nodes_ptr

    else:
        # build statement
        table = context.get_relation(relation)
        arguments: list[Any] = []
        stmt_parts: list[str] = ["SELECT"]
        if select:
            stmt_parts.append(_compile_select(context, arguments, select))
        else:
            stmt_parts.append(", ".join(f'"{col.name}"' for col in table.columns))
        stmt_parts.append(f"FROM {table.name}")
        if where is not None:
            stmt_parts.append(f"WHERE {_compile_condition(context, arguments, where)}")
        if sort:
            stmt_parts.append(f"ORDER BY {_compile_sort(context, arguments, sort)}")
        if limit is not None:
            stmt_parts.append(f"LIMIT {limit}")
        if offset is not None:
            stmt_parts.append(f"OFFSET {offset}")
        stmt = "\n".join(stmt_parts)

        # execute
        nodes_row: list[asyncpg.Record] = await conn.fetch(stmt, *arguments)
        nodes_value: list[Value] = []
        nodes_ptr: list[NodeReference] = []
        for row in nodes_row:
            value, ptr = unpack_node_row(table, row)
            nodes_value.append(value)
            nodes_ptr.append(ptr)
        logger.debug("database.query_node", stmt=stmt, nodes=len(nodes_value), span="current")

        return nodes_value, nodes_ptr


@tracer.start_as_current_span("database.query_scalar")
async def _query_scalar(
    conn: asyncpg.Connection,
    context: PostgresContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query."""
    if relation.is_multi:
        raise NotImplementedError(f"cannot query scalar on multi relation: {relation!r}")

    # build statement
    table = context.get_relation(relation)
    arguments: list[Any] = []
    stmt_parts: list[str] = [
        "SELECT",
        _compile_aggregation(context, arguments, aggregation),
        f"FROM {table.name}",
    ]
    if where is not None:
        stmt_parts.append(f"WHERE {_compile_condition(context, arguments, where)}")
    if aggregation.type == AggregationType.EXISTS:
        stmt_parts.append("LIMIT 1")
    stmt = "\n".join(stmt_parts)

    # execute
    scalar_row: asyncpg.Record | None = await conn.fetchrow(stmt, *arguments)
    logger.debug("database.query_scalar", stmt=stmt, scalar_row=scalar_row, span="current")
    if aggregation.type == AggregationType.EXISTS:
        return to_value(scalar_row is not None)
    elif scalar_row is None:
        return to_value(0 if aggregation.type == AggregationType.COUNT else 0.0)
    else:
        return to_value(scalar_row[0])


@tracer.start_as_current_span("database.query_grouped_node")
async def _query_grouped_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
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
    raise NotImplementedError


@tracer.start_as_current_span("database.query_grouped_scalar")
async def _query_grouped_scalar(
    conn: asyncpg.Connection,
    context: PostgresContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
    having: Condition | None,
    group_by: Sequence[Expression],
) -> list[tuple[Value, Value]]:
    """Execute a grouped scalar Query."""
    raise NotImplementedError


@tracer.start_as_current_span("database.query_clause")
async def _query_clause(
    conn: asyncpg.Connection, context: PostgresContext, query: Query, where: Condition | None
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
        nodes, nodes_ptr = await _query_node(
            conn=conn,
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
        assert query.aggregation is not None, f"no aggregation for {query!r}"
        scalar = await _query_scalar(
            conn=conn,
            context=context,
            relation=query.relation,
            aggregation=query.aggregation,
            where=combined_where,
        )
        nodes_ptr = []
        result = QueryResult(id=query.id, type=query.type, scalar=scalar)
        if query.aggregation.type == AggregationType.EXISTS:
            result.exists = scalar.unpack(bool)
        elif query.aggregation.type == AggregationType.COUNT:
            result.count = scalar.unpack(int)

    # grouped node
    elif query.type == QueryType.GROUPED_NODE:
        assert query.group_by is not None, f"no group_by for {query!r}"
        groups_value = await _query_grouped_node(
            conn=conn,
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
        for group_discriminator, group_nodes, group_nodes_ptr in groups_value:
            group = QueryResultGroup(
                type=query.type,
                discriminator=group_discriminator,
                nodes=group_nodes,
            )
            groups.append(group)
            nodes_ptr.extend(group_nodes_ptr)
        result = QueryResult(id=query.id, type=query.type, groups=groups)

    # grouped scalar
    elif query.type == QueryType.GROUPED_SCALAR:
        assert query.group_by is not None, f"no group_by for {query!r}"
        assert query.aggregation is not None, f"no aggregation for {query!r}"
        groups_value = await _query_grouped_scalar(
            conn=conn,
            context=context,
            relation=query.relation,
            aggregation=query.aggregation,
            where=combined_where,
            having=query.having,
            group_by=query.group_by,
        )
        nodes_ptr = []
        groups: list[QueryResultGroup] = []
        for group_discriminator, group_scalar in groups_value:
            group = QueryResultGroup(
                type=query.type,
                discriminator=group_discriminator,
                scalar=group_scalar,
            )
            groups.append(group)
        result = QueryResult(id=query.id, type=query.type, groups=groups)

    else:
        assert_never(query.type)

    logger.debug(
        "database.query_clause",
        query=query,
        result=result,
        nodes=len(nodes_ptr),
        span="current",
    )
    return result, nodes_ptr


async def _execute_subquery(
    conn: asyncpg.Connection,
    context: PostgresContext,
    result: QueryResult,
    nodes_ptr: Sequence[NodeReference],
    subquery: Query,
) -> QueryResult | None:
    """Execute a subquery to a main Query."""

    assert subquery.join is not None, f"no join for subquery {subquery!r}"

    # parent join
    if subquery.join.type == JoinType.PARENT:
        # collect/walk
        parents_ptr: dict[UUID, NodeReference] = {}
        for node_value in result.nodes:
            if (parent_ptr_value := node_value.value.get("4")) is not None:
                parent_id = fastuuid.UUID(parent_ptr_value["32"])
                if parent_id in parents_ptr:
                    continue
                parent_ptr = NodeReference.from_value(parent_ptr_value)
                parents_ptr[parent_ptr.id] = parent_ptr
        if not parents_ptr:
            return None  # nothing to query here
        if subquery.join.recursive:
            expanded_nodes_ptr = await _walk_node(
                conn=conn,
                context=context,
                relation=subquery.relation,
                roots_ptr=list(parents_ptr.values()),
                roots_parents_ptr=(),
                direction=EdgeDirection.PARENT,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )
            subquery_where = subquery.relation.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr),
            )
        else:
            subquery_where = subquery.relation.resolve_property_or_error("id").in_(
                *parents_ptr,
            )
        # subquery
        subresult = await execute_query(
            conn=conn,
            context=context,
            query=subquery,
            where=subquery_where,
        )
        return subresult

    # child join
    elif subquery.join.type == JoinType.CHILD:
        # collect/walk
        if not nodes_ptr:
            return None  # nothing to query here
        if subquery.join.recursive:
            expanded_nodes_ptr = await _walk_node(
                conn=conn,
                context=context,
                relation=subquery.relation,
                roots_ptr=(),
                roots_parents_ptr=nodes_ptr,
                direction=EdgeDirection.CHILD,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )
            subquery_where = subquery.relation.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr),
            )
        else:
            subquery_where = subquery.relation.resolve_property_or_error("parent").in_(
                *(n.id for n in nodes_ptr),
            )
        # subquery
        subresult = await execute_query(
            conn=conn,
            context=context,
            query=subquery,
            where=subquery_where,
        )
        return subresult

    # left join
    elif subquery.join.type == JoinType.LEFT:
        raise NotImplementedError(subquery)

    else:
        assert_never(subquery.join.type)


@tracer.start_as_current_span("database.query")
async def execute_query(
    conn: asyncpg.Connection, context: PostgresContext, query: Query, where: Condition | None = None
) -> QueryResult:
    """Execute the Query (and any subqueries)."""

    # main query clause
    result, nodes_ptr = await _query_clause(conn=conn, context=context, query=query, where=where)

    # subqueries (in parallel)
    subqueries = tuple(
        _execute_subquery(
            conn=conn,
            context=context,
            result=result,
            nodes_ptr=nodes_ptr,
            subquery=subquery,
        )
        for subquery in query.subqueries
    )
    subresults = await asyncio.gather(*subqueries)
    result.subresults = [subresult for subresult in subresults if subresult is not None]

    logger.debug(
        "database.query",
        query=query,
        result=result,
        subresults=len(subresults),
        span="current",
    )
    return result
