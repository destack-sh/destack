import uuid
from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import structlog
from fastuuid import UUID
from opentelemetry import trace

from bench.language import (
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

from .core import DatabaseContext
from .map import BENCH_CUSTOM_FIELD_PREFIX
from .wiring import pack_column_flat, unpack_node_row

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def _compile_value(context: DatabaseContext, arguments_out: list[Any], value: Value) -> str:
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


def _compile_select(context: DatabaseContext, arguments_out: list[Any], select: Select) -> str:
    """Compile a Select into a SQL SELECT clause."""
    raise NotImplementedError(select)


def _compile_attribute(
    context: DatabaseContext, arguments_out: list[Any], attribute: AttributeReference
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
        field_name = f"{BENCH_CUSTOM_FIELD_PREFIX}{str(field.id).replace('-', '')}"
        if field.scalar_type == ScalarType.NODE_REFERENCE:
            return f"{field_name}_id"
        else:
            return field_name
    else:
        assert_never(attribute.type)


def _compile_condition(
    context: DatabaseContext, arguments_out: list[Any], condition: Condition
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


def _compile_sort(context: DatabaseContext, arguments_out: list[Any], sort: Sequence[Sort]) -> str:
    """Compile a Sort into a SQL ORDER BY clause."""
    from bench.language import SortType

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
    context: DatabaseContext, arguments_out: list[Any], function: Function
) -> str:
    """Compile a Function into a SQL expression."""
    from bench.language import FunctionType

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
    context: DatabaseContext, arguments_out: list[Any], aggregation: Aggregation
) -> str:
    """Compile an Aggregation into a SQL expression."""
    from bench.language import AggregationType

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
    context: DatabaseContext, arguments_out: list[Any], expr: Expression
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
    context: DatabaseContext,
    relation: RelationReference,
    roots: Sequence[NodeReference],
    direction: EdgeDirection,
    recursive: bool,
) -> list[NodeReference]:
    """Get the cascaded Nodes for a query."""
    raise NotImplementedError


@tracer.start_as_current_span("database.query_node")
async def _query_node(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    relation: RelationReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
) -> tuple[list[Value], list[NodeReference]]:
    """Execute a node Query."""

    # build statement
    table = context.get_relation(relation)
    arguments: list[Any] = []
    stmt_parts: list[str] = ["SELECT"]
    if select:
        stmt_parts.append(_compile_select(context, arguments, select))
    else:
        stmt_parts.append(", ".join(col.name for col in table.columns))
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
    node_rows: list[asyncpg.Record] = await conn.fetch(stmt, *arguments)
    node_values: list[Value] = []
    nodes_ptr: list[NodeReference] = []
    for row in node_rows:
        value, ptr = unpack_node_row(table, row)
        node_values.append(value)
        nodes_ptr.append(ptr)
    logger.debug("database.select", stmt=stmt, nodes=len(node_values), span="current")

    return node_values, nodes_ptr


@tracer.start_as_current_span("database.query_scalar")
async def _query_scalar(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    relation: RelationReference,
    aggregation: Aggregation,
    where: Condition | None,
) -> Value:
    """Execute a scalar Query."""

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
    logger.debug("database.scalar", stmt=stmt, scalar_row=scalar_row, span="current")
    if aggregation.type == AggregationType.EXISTS:
        return to_value(scalar_row is not None)
    elif scalar_row is None:
        return to_value(0 if aggregation.type == AggregationType.COUNT else 0.0)
    else:
        return to_value(scalar_row[0])


@tracer.start_as_current_span("database.query_grouped_node")
async def _query_grouped_node(
    conn: asyncpg.Connection,
    context: DatabaseContext,
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
    context: DatabaseContext,
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
    conn: asyncpg.Connection, context: DatabaseContext, query: Query, where: Condition | None
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


@tracer.start_as_current_span("database.query")
async def execute_query(
    conn: asyncpg.Connection, context: DatabaseContext, query: Query, where: Condition | None = None
) -> QueryResult:
    """Execute the Query (and any subqueries)."""

    # main query clause
    result, nodes_ptr = await _query_clause(conn=conn, context=context, query=query, where=where)
    nodes_id: list[UUID] = [n.id for n in nodes_ptr]
    subresults: list[QueryResult] = []

    # subqueries
    for subquery in query.subqueries:
        subresult: QueryResult
        assert subquery.join is not None, f"no join for subquery {subquery!r}"
        if subquery.join.type == JoinType.PARENT:
            raise NotImplementedError(subquery)
        elif subquery.join.type == JoinType.CHILD:
            subquery_where = subquery.relation.resolve_property_or_error("parent").in_(*nodes_id)
            subresult = await execute_query(
                conn=conn,
                context=context,
                query=subquery,
                where=subquery_where,
            )
            subresults.append(subresult)
        elif subquery.join.type == JoinType.LEFT:
            raise NotImplementedError(subquery)
        else:
            assert_never(subquery.join.type)

    result.subresults = subresults
    logger.debug(
        "database.query",
        query=query,
        result=result,
        subresults=len(subresults),
        span="current",
    )
    return result
