from __future__ import annotations

import functools
from typing import TYPE_CHECKING, Any, Callable, Optional, Union
from uuid import UUID

from bench.language.const import (
    _CONDITIONAL_OP_SIGN,
    AggregationOp,
    ConditionalOp,
    ExpressionKind,
    ExpressionOp,
    IssueType,
    QueryEngine,
    SortMode,
    SortOp,
    StructType,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
)
from bench.language.module import Struct, bproperty, struct

if TYPE_CHECKING:
    from bench.language import Field, HasFields, Node, ScopeNode
    from bench.language.validation import ValidationHandler


#
# Expression language. Primarily for module, search and storage (database).
#


class QueryEngineError(Exception):
    def __init__(self, engine: QueryEngine, expr: Expression | list[Expression], reason: str):
        super().__init__(f"query engine {engine.value} failed on {expr!r}: {reason}")


class QueryEngineIncapableError(QueryEngineError):
    pass


FieldReference = UUID | str  # str as an alias for fields that we don't have reflected yet


@struct(StructType.EXPRESSION)
class Expression(Struct):
    op: ExpressionOp = bproperty(is_required=True)
    field: FieldReference | Field | None = bproperty(default=None)
    clauses: list[Expression] | None = bproperty(default=None)
    value: Any = bproperty(default=None)
    mode: Optional[SortMode] = bproperty(default=None)

    @property
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_OP[self.op]

    def __bool__(self):
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __str__(self):
        if self.op in ExpressionOps.COND_STATIC:
            return self.op.name.lower()
        elif self.op in ExpressionOps.COND_LOGICAL:
            return f" {_CONDITIONAL_OP_SIGN[self.op]} ".join(str(q) for q in self.clauses)
        elif self.op in ExpressionOps.COND_EXACT:
            value_str = str(self.value)
            if len(value_str) > 32:
                value_str = f"{value_str[:24]}...{value_str[-12:]}"
            return f"{self._field_str}{_CONDITIONAL_OP_SIGN[self.op]}{value_str}"
        elif self.op in ExpressionOps.COND_EXISTENCE:
            return f"{self._field_str}{_CONDITIONAL_OP_SIGN[self.op]}"
        elif self.op in ExpressionOps.SORT:
            return f"{'-' if self.op == SortOp.DESCENDING else ''}{self._field_str}"
        return self.op.name

    def __repr__(self):
        return f"<{self.op.name} {self}>"

    def __invert__(self):
        if self.op == ConditionalOp.TRUE:
            return C(ConditionalOp.FALSE)
        elif self.op == ConditionalOp.FALSE:
            return C(ConditionalOp.TRUE)
        elif self.op == ConditionalOp.NOT:
            return self.clauses[0]
        elif self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, field=self.field)
        elif self.op == ConditionalOp.NOT_EXISTS:
            return C(ConditionalOp.EXISTS, field=self.field)
        else:
            return C(ConditionalOp.NOT, clauses=[self])

    def __and__(self, other: Expression):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.AND:
            if isinstance(other, Expression) and other.op == ConditionalOp.AND:
                return C(ConditionalOp.AND, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.AND, clauses=[*self.clauses, other])
        else:
            return C(ConditionalOp.AND, clauses=[self, other])

    def __or__(self, other: Expression):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.OR:
            if isinstance(other, Expression) and other.op == ConditionalOp.OR:
                return C(ConditionalOp.OR, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.OR, clauses=[*self.clauses, other])
        else:
            return C(ConditionalOp.OR, clauses=[self, other])

    def _walk_inner(self, on_member: Callable[[Union["Node", "Struct"]], None]):
        from bench.language import Node

        if isinstance(self.field, Node):
            on_member(self.field)
        if self.clauses:
            for clause in self.clauses:
                on_member(clause)

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        if not isinstance(self.field, (UUID, str)) and (
            scope is None or self.field.ck in scope._local_root_tree
        ):
            self._set_untracked("field", self.field.ck)

    def _interp_inner(self, scope: "ScopeNode", on_issue: "ValidationHandler"):
        if self.field:
            resolved = self.field
            if isinstance(self.field, UUID):
                resolved = scope.lookup(self.field)
            if resolved is None:
                on_issue(type=IssueType.MISSING_REFERENCE, subject=scope, path="<expression>")
            else:
                self._set_untracked("field", resolved)

    @property
    def _field_str(self) -> str:
        if not isinstance(self.field, (UUID, str)):
            return self.field.py_ident
        else:
            return str(self.field)

    @property
    def field_key(self) -> str:
        return self.field if isinstance(self.field, str) else self.field._source_key

    @property
    def is_scored(self) -> bool:
        return self.op in RANKED_CONDITIONAL_OPS

    @staticmethod
    def and_if_set(
        *clauses: Optional[Expression],
    ) -> Optional[Expression]:
        base = None
        for clause in clauses:
            if clause is not None:
                if base is None:
                    base = clause
                else:
                    base &= clause
        return base


class ExpressionOps:
    # Conditionals
    COND_STATIC = {ConditionalOp.TRUE, ConditionalOp.FALSE}
    COND_LOGICAL = {ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR}
    COND_EXACT = {
        ConditionalOp.EQUALS,
        ConditionalOp.NOT_EQUALS,
        ConditionalOp.IN,
        ConditionalOp.NOT_IN,
    }
    COND_STRUCT = {ConditionalOp.CONTAINS, ConditionalOp.NOT_CONTAINS}
    COND_RANGE = {
        ConditionalOp.GREATER_THAN,
        ConditionalOp.GREATER_THAN_OR_EQUALS,
        ConditionalOp.LESS_THAN,
        ConditionalOp.LESS_THAN_OR_EQUALS,
    }
    COND_EXISTENCE = {ConditionalOp.EXISTS, ConditionalOp.NOT_EXISTS}
    COND_VECTOR = {ConditionalOp.NEAR}
    COND_COMPARISON = {*COND_EXACT, *COND_RANGE}
    # Aggregations
    AGG_SINGLE = {
        AggregationOp.COUNT,
        AggregationOp.SUM,
        AggregationOp.AVERAGE,
        AggregationOp.MIN,
        AggregationOp.MAX,
        AggregationOp.MEDIAN,
    }
    AGG_BUCKET = {AggregationOp.HISTOGRAM}
    # Sorts
    SORT = {SortOp.ASCENDING, SortOp.DESCENDING}


RANKED_CONDITIONAL_OPS = {ConditionalOp.MATCHES, *ExpressionOps.COND_VECTOR}

EXPRESSION_OPS_BY_KIND: dict[ExpressionKind, set[ExpressionOp]] = {
    ExpressionKind.CONDITIONAL: {
        *ExpressionOps.COND_STATIC,
        *ExpressionOps.COND_LOGICAL,
        *ExpressionOps.COND_EXACT,
        *ExpressionOps.COND_STRUCT,
        *ExpressionOps.COND_RANGE,
        *ExpressionOps.COND_EXISTENCE,
        *ExpressionOps.COND_VECTOR,
    },
    ExpressionKind.AGGREGATION: {*ExpressionOps.AGG_SINGLE, *ExpressionOps.AGG_BUCKET},
    ExpressionKind.SORT: {*ExpressionOps.SORT},
}
EXPRESSION_KIND_BY_OP: dict[ExpressionOp, ExpressionKind] = {
    op: kind for kind, ops in EXPRESSION_OPS_BY_KIND.items() for op in ops
}

CONDITIONAL_OP_BY_DJANGO_STR: dict[str, ConditionalOp] = {
    "eq": ConditionalOp.EQUALS,
    "ne": ConditionalOp.NOT_EQUALS,
    "gt": ConditionalOp.GREATER_THAN,
    "gte": ConditionalOp.GREATER_THAN_OR_EQUALS,
    "lt": ConditionalOp.LESS_THAN,
    "lte": ConditionalOp.LESS_THAN_OR_EQUALS,
    "in": ConditionalOp.IN,
    "nin": ConditionalOp.NOT_IN,
}


def coerce_conditional(
    statement: "HasFields",
    expr: Optional[Expression],
    kwargs: Optional[dict[str, Any]] = None,
    return_none_if_empty: bool = False,
) -> Optional[Expression]:
    """
    Coerce a conditional expression from either the given expression or kwargs.
    Useful for basic Django-style querying (with optional __<op>, but no relation support yet).
    """
    if expr is not None and kwargs:
        raise TypeError(f"cannot specify both {expr} and {kwargs}")
    if expr is not None:
        if not isinstance(expr, Expression) or expr.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"expected Conditional, got {expr!r}")
        return expr

    clauses = []
    for arg, value in kwargs.items():
        if "__" in arg:
            field_key, op = arg.split("__", 1)
        else:
            field_key, op = arg, ConditionalOp.EQUALS
        field = statement.resolved_fields.get(field_key)
        if not field:  # try reflected property
            field = statement.__properties__.get(field_key)
            if field:
                field = field._as_field
        if not field:
            raise TypeError(f"{statement!r} has no field {field_key}")
        _check_field_supports(field, op)
        clauses.append(Expression(op=op, field=field, value=value))
    if not clauses:
        if return_none_if_empty:
            return None
        else:
            return C(ConditionalOp.TRUE)
    return Expression.and_if_set(*clauses)


def coerce_sort(
    statement: "HasFields",
    sort: list[Expression | str] | Expression | str | None,
    args: str | None = None,
) -> Optional[list[Expression]]:
    """
    Coerce a sort expression from either the given expression or args.
    Strings are looked up as field names/identifiers.
    Like in Django, prefix with "-" for descending.
    """
    if sort is None:
        if args is None:
            return None
        sort = args
    elif isinstance(sort, str):
        sort = [sort]
    elif isinstance(sort, Expression) and sort.kind == ExpressionKind.SORT:
        sort = [sort]
    if not isinstance(sort, (list, tuple)):
        raise TypeError(f"expected sort to be a list or tuple, got {sort}")
    if args:
        sort = (*sort, *args)
    coerced = []
    for item in sort:
        if isinstance(item, str):
            op = SortOp.DESCENDING if item.startswith("-") else SortOp.ASCENDING
            if op == SortOp.DESCENDING:
                item = item[1:]
            field = statement.resolved_fields.get(item)
            if not field:  # try reflected property
                field = statement.__properties__.get(item)
                if field:
                    field = field._as_field
            if not field:
                raise TypeError(f"{statement!r} has no field {item!r}")
            item = S(op, field=field)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


# single-letter convenience constructors
def E(op: ExpressionOp, *args, _expect_t: type[ExpressionKind] = None, **kwargs) -> Expression:
    if _expect_t is not None and EXPRESSION_KIND_BY_OP[op] != _expect_t:
        raise TypeError(f"expected {_expect_t}, got {EXPRESSION_KIND_BY_OP[op]}")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in Expression.__properties__}
    return Expression(op, *args, **kwargs)


C = functools.partial(E, _expect_t=ExpressionKind.CONDITIONAL)
S = functools.partial(E, _expect_t=ExpressionKind.SORT)
A = functools.partial(E, _expect_t=ExpressionKind.AGGREGATION)

SCORE_KEY = "_score"  # for ranking
TYPE_DISCRIMINATOR_KEY = "_type"


#
# Field query ops
#


class UnsupportedExpressionError(ValueError):
    def __init__(self, field: "Field", thing: Any):
        super().__init__(f"{repr(field)} does not support {thing}")


def _check_field_supports(field: "Field", op: ExpressionOp):
    """Asserts that the field supports the given expression operator."""
    if op in SortOp:
        return field._storage_format in (
            TypeStorageFormat.DATE,
            TypeStorageFormat.DOUBLE,
            TypeStorageFormat.LONG,
            TypeStorageFormat.KEYWORD,
        )
    else:
        if (
            op not in ExprOps.COND_EXISTENCE
            and op not in SUPPORTED_OPS_BY_TYPE.get(field._storage_format, _EMPTY_SET)
            and op not in SUPPORTED_OPS_BY_TYPE.get(field._effective_tag, _EMPTY_SET)
            and op not in SUPPORTED_OPS_BY_TYPE.get(field._effective_hint, _EMPTY_SET)
        ):
            raise UnsupportedExpressionError(field, op)


ExprOps = ExpressionOps  # alias
SUPPORTED_OPS_BY_TYPE: dict[TypeTag | TypeHint | TypeStorageFormat, set[ConditionalOp]] = {
    # cumulative supported query ops by type
    TypeStorageFormat.LONG: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.DOUBLE: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.BOOLEAN: ExprOps.COND_EXACT,
    TypeStorageFormat.DATE: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.KEYWORD: ExprOps.COND_EXACT,
    TypeStorageFormat.VECTOR: ExprOps.COND_VECTOR,
    TypeStorageFormat.RELATION: ExprOps.COND_EXACT,
    TypeTag.STRING: ExprOps.COND_EXACT | ExprOps.COND_RANGE | {ConditionalOp.MATCHES},
    TypeHint.NAME: {ConditionalOp.STARTS_WITH},
}
_EMPTY_SET = set()
