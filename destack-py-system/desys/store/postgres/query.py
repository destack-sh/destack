import uuid
from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import structlog
from opentelemetry import trace

from destack.language import (
    Aggregation,
    AggregationType,
    Condition,
    ConditionalType,
    CustomProperty,
    EdgeDirection,
    Expression,
    ExpressionType,
    Function,
    JoinType,
    NodeDefinitionReference,
    NodeReference,
    NodeType,
    PropertyReference,
    PropertyReferenceType,
    Query,
    QueryResult,
    QueryResultGroup,
    QueryType,
    ScalarType,
    Select,
    Sort,
    Value,
    to_value,
)
from destack.utils.uuid import UUID

from .core import PostgresContext
from .map import DESTACK_CUSTOM_PROPERTY_PREFIX
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
    context: PostgresContext, arguments_out: list[Any], attribute: PropertyReference
) -> str:
    """Compile an Attribute into a SQL expression."""
    if attribute.type == PropertyReferenceType.BUILTIN:
        prop = attribute.resolve_or_error()
        if prop.scalar_type == ScalarType.NODE_REFERENCE:
            # unravel reference column into id
            return f"{prop.name}_id"
        else:
            return prop.name
    elif attribute.type == PropertyReferenceType.CUSTOM:
        field = attribute.resolve_or_error()
        assert isinstance(field, CustomProperty), f"no field for {attribute!r}"
        field_name = f"{DESTACK_CUSTOM_PROPERTY_PREFIX}{str(field.id).replace('-', '')}"
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


@tracer.start_as_current_span("postgres.walk_node")
async def _walk_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    roots_ptr: Sequence[NodeReference],
    roots_parents_ptr: Sequence[NodeReference],
    direction: EdgeDirection,
    depth: int,
    where: Condition | None,
) -> list[NodeReference]:
    """Get the cascaded Nodes for a query."""

    roots_ids: list[uuid.UUID] = [n.id for n in roots_ptr]
    roots_parents_ids: list[uuid.UUID] = [n.id for n in roots_parents_ptr]
    arguments: list[Any] = [roots_ids, roots_parents_ids, depth]
    where_sql = _compile_condition(context, arguments, where) if where is not None else "TRUE"

    # parent walk
    if direction == EdgeDirection.PARENT:
        if definition.is_multi:
            # fan out definition
            tables = [context.get(r) for r in context.resolve(definition)]
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
                node_ptr = NodeReference(node_type=node_type, id=UUID(str(row["id"])))
                result_nodes_ptr.append(node_ptr)
            return result_nodes_ptr
        else:
            table = context.get(definition)
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
                node_ptr = NodeReference(node_type=table.node_type, id=UUID(str(row["id"])))
                result_nodes_ptr.append(node_ptr)
            return result_nodes_ptr

    # child walk
    elif direction == EdgeDirection.CHILD:
        # fan out definition
        tables = [context.get(r) for r in context.resolve(definition)]

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
            node_ptr = NodeReference(node_type=node_type, id=UUID(str(row["id"])))
            result_nodes_ptr.append(node_ptr)
        return result_nodes_ptr

    # side walk
    elif direction == EdgeDirection.SIDE:
        raise NotImplementedError(f"cannot walk {definition!r} in direction: {direction!r}")

    else:
        assert_never(direction)


@tracer.start_as_current_span("postgres.query_node")
async def _query_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
    _ignore_multi: bool = False,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query."""
    if definition.is_multi and not _ignore_multi:
        # fan out multi definitions
        if limit is not None or offset is not None:
            raise NotImplementedError(f"cannot limit/offset for multi definition: {definition!r}")
        subdefinitions = context.resolve(definition)
        nodes_value: list[Value] = []
        nodes_ptr: list[NodeReference] = []
        for subdefinition in subdefinitions:
            subnodes_value, subnodes_ptr = await _query_node(
                conn=conn,
                context=context,
                definition=subdefinition,
                select=select,
                where=where,
                sort=sort,
                limit=limit,
                offset=offset,
                _ignore_multi=True,
            )
            nodes_value.extend(subnodes_value)
            nodes_ptr.extend(subnodes_ptr)
        return nodes_value, nodes_ptr

    else:
        # build statement
        table = context.get(definition)
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
        logger.trace("postgres.query_node", stmt=stmt, nodes=len(nodes_value), span="current")

        return nodes_value, nodes_ptr


@tracer.start_as_current_span("postgres.query_scalar")
async def _query_scalar(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query."""
    if definition.is_multi:
        raise NotImplementedError(f"cannot query scalar on multi definition: {definition!r}")

    # build statement
    table = context.get(definition)
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
    if aggregation.type == AggregationType.EXISTS:
        scalar_value = to_value(scalar_row is not None)
    elif scalar_row is None:
        scalar_value = to_value(0 if aggregation.type == AggregationType.COUNT else 0.0)
    else:
        scalar_value = to_value(scalar_row[0])
    logger.trace(
        "postgres.query_scalar",
        stmt=stmt,
        definition=definition,
        scalar_value=scalar_value,
        span="current",
    )
    return scalar_value


@tracer.start_as_current_span("postgres.query_grouped_node")
async def _query_grouped_node(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    select: Select | None,
    where: Condition | None,
    having: Condition | None,
    sort: Sequence[Sort] | None,
    group_by: Sequence[Expression],
    limit: int | None,
    offset: int | None,
) -> list[tuple[Value, list[Value], list[NodeReference]]]:
    """Execute a grouped node Query."""
    if definition.is_multi:
        raise NotImplementedError(f"cannot query grouped node on multi definition: {definition!r}")

    # build statement to get groups
    table = context.get(definition)
    group_arguments: list[Any] = []
    group_by_parts = [_compile_expression(context, group_arguments, expr) for expr in group_by]
    group_by_clause = ", ".join(group_by_parts)
    stmt_parts: list[str] = [
        "SELECT",
        f"{group_by_clause}, ARRAY_AGG(id ORDER BY id) as grouped_ids",
        f"FROM {table.name}",
    ]
    if where is not None:
        stmt_parts.append(f"WHERE {_compile_condition(context, group_arguments, where)}")
    stmt_parts.append(f"GROUP BY {group_by_clause}")
    if having is not None:
        stmt_parts.append(f"HAVING {_compile_condition(context, group_arguments, having)}")
    if sort:
        stmt_parts.append(f"ORDER BY {_compile_sort(context, group_arguments, sort)}")
    if limit is not None:
        stmt_parts.append(f"LIMIT {limit}")
    if offset is not None:
        stmt_parts.append(f"OFFSET {offset}")
    group_stmt = "\n".join(stmt_parts)

    # execute statement to get groups
    group_rows: list[asyncpg.Record] = await conn.fetch(group_stmt, *group_arguments)
    group_by_count = len(group_by)
    nodes_id: list[UUID] = []
    nodes_id_by_discriminator: dict[Any, list[UUID]] = {}
    for row in group_rows:
        group_values = row[:group_by_count]
        discriminator = group_values[0] if group_by_count == 1 else tuple(group_values)
        group_ids = [UUID(str(id)) for id in row[group_by_count]]  # ARRAY_AGG result
        nodes_id_by_discriminator[discriminator] = group_ids
        nodes_id.extend(group_ids)

    # build statement to get nodes
    node_arguments: list[Any] = []
    node_stmt_parts: list[str] = ["SELECT"]
    if select:
        node_stmt_parts.append(_compile_select(context, node_arguments, select))
    else:
        columns_clause = ", ".join(f'"{col.name}"' for col in table.columns)
        node_stmt_parts.append(columns_clause)
    node_stmt_parts.append(f"FROM {table.name}")  # noqa: FURB113
    node_stmt_parts.append(f"WHERE id = ANY(${len(node_arguments) + 1})")
    node_arguments.append(nodes_id)
    node_stmt = "\n".join(node_stmt_parts)

    # execute statement to get nodes
    node_rows: list[asyncpg.Record] = await conn.fetch(node_stmt, *node_arguments)
    nodes_by_id: dict[UUID, tuple[Value, NodeReference]] = {}
    for row in node_rows:
        value, ptr = unpack_node_row(table, row)
        nodes_by_id[ptr.id] = (value, ptr)

    # assemble results
    results: list[tuple[Value, list[Value], list[NodeReference]]] = []
    for discriminator, group_node_ids in nodes_id_by_discriminator.items():
        group_discriminator = to_value(discriminator)
        group_nodes_value: list[Value] = []
        group_nodes_ptr: list[NodeReference] = []
        for node_id in group_node_ids:
            assert node_id in nodes_by_id, f"node {node_id} not found in {nodes_by_id!r}"
            value, ptr = nodes_by_id[node_id]
            group_nodes_value.append(value)
            group_nodes_ptr.append(ptr)
        results.append((group_discriminator, group_nodes_value, group_nodes_ptr))

    logger.trace(
        "postgres.query_grouped_node",
        definition=definition,
        groups=len(results),
        total_nodes=len(node_rows),
        span="current",
    )
    return results


@tracer.start_as_current_span("postgres.query_grouped_scalar")
async def _query_grouped_scalar(
    conn: asyncpg.Connection,
    context: PostgresContext,
    definition: NodeDefinitionReference,
    aggregation: Aggregation,
    where: Condition | None,
    having: Condition | None,
    group_by: Sequence[Expression],
) -> list[tuple[Value, Value]]:
    """Execute a grouped scalar Query."""
    if definition.is_multi:
        raise NotImplementedError(
            f"cannot query grouped scalar on multi definition: {definition!r}"
        )

    # build statement
    table = context.get(definition)
    arguments: list[Any] = []
    # build GROUP BY clause
    group_by_parts = [_compile_expression(context, arguments, expr) for expr in group_by]
    group_by_clause = ", ".join(group_by_parts)
    stmt_parts: list[str] = [
        "SELECT",
        f"{group_by_clause}, {_compile_aggregation(context, arguments, aggregation)}",
        f"FROM {table.name}",
    ]
    if where is not None:
        stmt_parts.append(f"WHERE {_compile_condition(context, arguments, where)}")
    stmt_parts.append(f"GROUP BY {group_by_clause}")
    if having is not None:
        stmt_parts.append(f"HAVING {_compile_condition(context, arguments, having)}")
    stmt = "\n".join(stmt_parts)

    # execute
    rows: list[asyncpg.Record] = await conn.fetch(stmt, *arguments)
    logger.trace("postgres.query_grouped_scalar", stmt=stmt, rows=len(rows), span="current")
    results: list[tuple[Value, Value]] = []
    for row in rows:
        # first columns are group_by values, last column is aggregation result
        group_values = row[:-1]
        scalar_value = row[-1]
        # single vs multiple group by expressions
        if len(group_by) == 1:
            group_discriminator = to_value(group_values[0])
        else:
            # for multiple group by expressions, create a tuple
            group_discriminator = to_value(tuple(group_values))
        # aggregation result
        if aggregation.type == AggregationType.EXISTS:
            scalar_result = to_value(scalar_value is not None and scalar_value)
        elif scalar_value is None:
            scalar_result = to_value(0 if aggregation.type == AggregationType.COUNT else 0.0)
        else:
            scalar_result = to_value(scalar_value)
        results.append((group_discriminator, scalar_result))

    logger.trace(
        "postgres.query_grouped_scalar",
        definition=definition,
        groups=len(results),
        span="current",
    )
    return results


@tracer.start_as_current_span("postgres.query_clause")
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
            definition=query.definition,
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
            definition=query.definition,
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
            definition=query.definition,
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
            definition=query.definition,
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

    logger.trace(
        "postgres.query_clause",
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
            if (parent_ptr_value := node_value.value.get("3")) is not None:
                parent_id = UUID(parent_ptr_value["32"])
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
                definition=subquery.definition,
                roots_ptr=list(parents_ptr.values()),
                roots_parents_ptr=(),
                direction=EdgeDirection.PARENT,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )
            subquery_where = subquery.definition.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr),
            )
        else:
            subquery_where = subquery.definition.resolve_property_or_error("id").in_(
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
                definition=subquery.definition,
                roots_ptr=(),
                roots_parents_ptr=nodes_ptr,
                direction=EdgeDirection.CHILD,
                depth=subquery.join.depth or MAX_RECURSION_DEPTH,
                where=subquery.where,
            )
            subquery_where = subquery.definition.resolve_property_or_error("id").in_(
                *(n.id for n in expanded_nodes_ptr),
            )
        else:
            subquery_where = subquery.definition.resolve_property_or_error("parent").in_(
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


@tracer.start_as_current_span("postgres.query")
async def execute_query(
    conn: asyncpg.Connection, context: PostgresContext, query: Query, where: Condition | None = None
) -> QueryResult:
    """Execute the Query (and any subqueries)."""

    # main query clause
    result, nodes_ptr = await _query_clause(conn=conn, context=context, query=query, where=where)

    # subqueries (sequentially)
    # TODO :Performance: execute postgres statements in parallel
    subresults = []
    for subquery in query.subqueries:
        subresult = await _execute_subquery(
            conn=conn,
            context=context,
            result=result,
            nodes_ptr=nodes_ptr,
            subquery=subquery,
        )
        if subresult is not None:
            subresults.append(subresult)
    result.subresults = subresults

    logger.trace(
        "postgres.query",
        query=query,
        result=result,
        subresults=len(subresults),
        span="current",
    )
    return result
