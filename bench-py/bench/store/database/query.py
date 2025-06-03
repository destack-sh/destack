from collections.abc import Sequence
from typing import Any, assert_never

import asyncpg
import structlog
from opentelemetry import trace

from bench.language import (
    Aggregation,
    AttributeReference,
    AttributeType,
    Condition,
    ConditionalType,
    Expression,
    ExpressionType,
    Function,
    Query,
    QueryResult,
    QueryType,
    RelationReference,
    Select,
    Sort,
    Value,
)

from .core import DatabaseContext
from .wiring import pack_column, unpack_row_to_node_value

logger = structlog.get_logger(__name__)
tracer = trace.get_tracer(__name__)


def _compile_value(context: DatabaseContext, arguments_out: list[Any], value: Value) -> str:
    """Compile a Value into a SQL expression."""
    value_packed = pack_column(value.type, value.value)
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
        assert attribute.prop is not None, f"no property for {attribute!r}"
        return attribute.prop.name
    elif attribute.type == AttributeType.FIELD:
        raise NotImplementedError(attribute)
    elif attribute.type == AttributeType.QUERY:
        assert attribute.name is not None, f"no name for {attribute!r}"
        return attribute.name
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
        return f"{_compile_expression(context, arguments_out, condition.left)} IN ({_compile_expression(context, arguments_out, condition.right)})"
    elif condition.type == ConditionalType.NOT_IN:
        assert condition.right is not None, f"no right for {condition!r}"
        return f"{_compile_expression(context, arguments_out, condition.left)} NOT IN ({_compile_expression(context, arguments_out, condition.right)})"
    # existence
    elif condition.type == ConditionalType.EXISTS:
        return f"EXISTS ({_compile_expression(context, arguments_out, condition.left)})"
    elif condition.type == ConditionalType.NOT_EXISTS:
        return f"NOT EXISTS ({_compile_expression(context, arguments_out, condition.left)})"
    else:
        assert_never(condition.type)


def _compile_sort(context: DatabaseContext, arguments_out: list[Any], sort: Sequence[Sort]) -> str:
    """Compile a Sort into a SQL ORDER BY clause."""
    raise NotImplementedError(sort)


def _compile_function(
    context: DatabaseContext, arguments_out: list[Any], function: Function
) -> str:
    """Compile a Function into a SQL expression."""
    raise NotImplementedError(function)


def _compile_aggregation(
    context: DatabaseContext, arguments_out: list[Any], aggregation: Aggregation
) -> str:
    """Compile an Aggregation into a SQL expression."""
    raise NotImplementedError(aggregation)


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


async def _execute_select(
    conn: asyncpg.Connection,
    context: DatabaseContext,
    relation: RelationReference,
    select: Select | None,
    where: Condition | None,
    sort: Sequence[Sort] | None,
    limit: int | None,
    offset: int | None,
) -> list[Value]:
    """Execute a SELECT statement."""
    # build statement
    table = context.get_table(relation)
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
    print(stmt)
    print(arguments)

    # execute
    logger.debug("database.select", stmt=stmt)
    node_rows: list[asyncpg.Record] = await conn.fetch(stmt, *arguments)
    node_values = [unpack_row_to_node_value(table, row) for row in node_rows]

    return node_values


async def execute_query(
    conn: asyncpg.Connection, context: DatabaseContext, query: Query
) -> QueryResult:
    """Execute the Query."""

    if query.type == QueryType.NODE:
        assert query.join is None, f"root-level join: {query!r}"
        nodes = await _execute_select(
            conn=conn,
            context=context,
            relation=query.relation,
            select=query.select,
            where=query.where,
            sort=query.sort,
            limit=query.limit,
            offset=query.offset,
        )
        return QueryResult(query=query, nodes=nodes)
    elif query.type == QueryType.SCALAR:
        raise NotImplementedError
    else:
        assert_never(query.type)
