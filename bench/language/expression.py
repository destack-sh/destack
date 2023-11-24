from __future__ import annotations

import functools
from typing import TYPE_CHECKING, Any, Optional
from uuid import UUID

from bench.language.const import (
    _CONDITIONAL_OP_SIGN,
    AggregationOp,
    ConditionalOp,
    ExpressionKind,
    ExpressionOp,
    IssueType,
    QueryEngine,
    SortOp,
    StructType,
    TypeHint,
    TypeStorageFormat,
    TypeTag,
)
from bench.language.module import Struct, bproperty, struct
from bench.utils.func import get_subclasses

if TYPE_CHECKING:
    from bench.language import Field, HasFields, ScopeNode, SortMode
    from bench.language.validation import ValidationHandler


#
# Expression language. Primarily for module, search and storage (database).
#


class QueryEngineIncapableError(Exception):
    def __init__(self, engine: QueryEngine, expr: Expression, reason: str):
        super().__init__(f"query engine {engine.value} is incapable of {expr}: {reason}")


FieldReference = UUID | str  # str as an alias for fields that we don't have reflected yet


@struct(StructType.EXPRESSION)
class Expression(Struct):
    op: ExpressionOp = bproperty(is_required=True)

    def __str__(self):
        return self.__class__.__name__

    def __repr__(self):
        return f"{self.kind}({self})"

    @property
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_CLASS[type(self)]


@struct(StructType.EXPRESSION)
class FieldExpression(Expression):
    field: Field | FieldReference = bproperty(is_required=True)

    @property
    def _field_str(self) -> str:
        if not isinstance(self.field, (UUID, str)):
            return self.field.py_ident
        else:
            return str(self.field)

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        if not isinstance(self.field, (UUID, str)) and (
            scope is None or self.field.ck in scope._local_root_tree
        ):
            self._set_untracked("field", self.field.ck)

    def _interp_inner(self, scope: "ScopeNode", on_issue: "ValidationHandler"):
        resolved = self.field
        if isinstance(self.field, UUID):
            resolved = scope.lookup(self.field)
        if resolved is None:
            on_issue(type=IssueType.MISSING_REFERENCE, subject=self, path="<expression>")
        else:
            self._set_untracked("field", resolved)

    @property
    def field_key(self) -> str:
        return self.field if isinstance(self.field, str) else self.field._source_key


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

EXPRESSION_CLASS_BY_OP: dict[ExpressionOp, type[Expression]] = {}


def expression(*ops: ExpressionOp):
    """Register a query class for the given ops."""

    def decorator(cls: type[Expression]):
        if not issubclass(cls, Expression):
            raise TypeError(f"expression {cls} must be a subclass of {Expression}")
        cls = struct(StructType.EXPRESSION)(cls)
        for op in ops:
            if op in EXPRESSION_CLASS_BY_OP:
                raise RuntimeError(
                    f"expression for {op} already registered: {EXPRESSION_CLASS_BY_OP[op]}"
                )
            EXPRESSION_CLASS_BY_OP[op] = cls
        return cls

    return decorator


@expression()
class Conditional(Expression):
    def __bool__(self):
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __invert__(self):
        return C(ConditionalOp.NOT, clauses=[self])

    def __and__(self, other):
        if not isinstance(other, Conditional):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        return C(ConditionalOp.AND, clauses=[self, other])

    def __or__(self, other):
        if not isinstance(other, Conditional):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        return C(ConditionalOp.OR, clauses=[self, other])

    @property
    def is_scored(self) -> bool:
        return self.op in RANKED_CONDITIONAL_OPS

    @staticmethod
    def and_if_set(
        *clauses: Optional[Conditional],
    ) -> Optional[Conditional]:
        base = None
        for clause in clauses:
            if clause is not None:
                if base is None:
                    base = clause
                else:
                    base &= clause
        return base


@expression(ConditionalOp.TRUE, ConditionalOp.FALSE)
class StaticConditional(Conditional):
    def __invert__(self):
        if self.op == ConditionalOp.TRUE:
            return C(ConditionalOp.FALSE)
        else:
            return C(ConditionalOp.TRUE)

    def __str__(self):
        return self.op.name.lower()


@expression(ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR)
class CompoundConditional(Conditional):
    clauses: list[Conditional] = bproperty(is_required=True)

    def __str__(self):
        return f" {_CONDITIONAL_OP_SIGN[self.op]} ".join(str(q) for q in self.clauses)

    def __invert__(self):
        if self.op == ConditionalOp.NOT:
            return self.clauses[0]
        else:
            return super().__invert__()

    def __and__(self, other):
        if not isinstance(other, Conditional):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.AND:
            if isinstance(other, CompoundConditional) and other.op == ConditionalOp.AND:
                return C(ConditionalOp.AND, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.AND, clauses=[*self.clauses, other])
        else:
            return super().__and__(other)

    def __or__(self, other):
        if not isinstance(other, Conditional):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.OR:
            if isinstance(other, CompoundConditional) and other.op == ConditionalOp.OR:
                return C(ConditionalOp.OR, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.OR, clauses=[*self.clauses, other])
        else:
            return super().__or__(other)

    @property
    def is_scored(self) -> bool:
        return any(q.is_scored for q in self.clauses)


@expression(
    ConditionalOp.EQUALS,
    ConditionalOp.NOT_EQUALS,
    ConditionalOp.GREATER_THAN,
    ConditionalOp.GREATER_THAN_OR_EQUALS,
    ConditionalOp.LESS_THAN,
    ConditionalOp.LESS_THAN_OR_EQUALS,
    ConditionalOp.MATCHES,
    ConditionalOp.STARTS_WITH,
    ConditionalOp.NEAR,
)
class ComparisonConditional(FieldExpression, Conditional):
    value: Any = bproperty(is_required=True)

    def __str__(self):
        value_str = str(self.value)
        return f"{self._field_str}{_CONDITIONAL_OP_SIGN[self.op]}{value_str}"


@expression(ConditionalOp.EXISTS, ConditionalOp.NOT_EXISTS)
class ExistenceConditional(FieldExpression, Conditional):
    def __str__(self):
        return f"{self._field_str}{_CONDITIONAL_OP_SIGN[self.op]}"

    def __invert__(self):
        if self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, field=self.field)
        else:
            return C(ConditionalOp.EXISTS, field=self.field)


@expression(*SortOp)
class Sort(FieldExpression):
    mode: Optional[SortMode] = bproperty(default=None)


@expression(*AggregationOp)
class Aggregation(FieldExpression):
    pass


EXPRESSION_KIND_BY_CLASS: dict[type[Expression], ExpressionKind] = {
    Conditional: ExpressionKind.CONDITIONAL,
    Sort: ExpressionKind.SORT,
    Aggregation: ExpressionKind.AGGREGATION,
}
# expand into subclasses
for super_t, kind in list(EXPRESSION_KIND_BY_CLASS.items()):
    for sub_t in get_subclasses(super_t):
        EXPRESSION_KIND_BY_CLASS[sub_t] = kind

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
    expr: Optional[Conditional],
    kwargs: Optional[dict[str, Any]] = None,
    return_none_if_empty: bool = False,
) -> Optional[Conditional]:
    """
    Coerce a conditional expression from either the given expression or kwargs.
    Useful for basic Django-style querying (with optional __<op>, but no relation support yet).
    """
    if expr is not None and kwargs:
        raise TypeError(f"cannot specify both {expr} and {kwargs}")
    if expr is not None:
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
        clauses.append(ComparisonConditional(op=op, field=field, value=value))
    if not clauses:
        if return_none_if_empty:
            return None
        else:
            return C(ConditionalOp.TRUE)
    return Conditional.and_if_set(*clauses)


# single-letter convenience constructors
def E(op: ExpressionOp, *args, _expect_t: type[Expression] = None, **kwargs) -> Expression:
    cls = EXPRESSION_CLASS_BY_OP[op]
    if _expect_t is not None and not issubclass(cls, _expect_t):
        raise TypeError(f"expected {_expect_t}, got {cls}")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls.__properties__}
    return cls(op, *args, **kwargs)


C = functools.partial(E, _expect_t=Conditional)
S = functools.partial(E, _expect_t=Sort)
A = functools.partial(E, _expect_t=Aggregation)

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
    TypeTag.STRING: ExprOps.COND_EXACT | ExprOps.COND_RANGE | {ConditionalOp.MATCHES},
    TypeHint.NAME: {ConditionalOp.STARTS_WITH},
}
_EMPTY_SET = set()
