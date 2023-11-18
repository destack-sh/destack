from __future__ import annotations

import enum
import functools
from dataclasses import dataclass
from functools import wraps
from typing import TYPE_CHECKING, Any, Optional, cast, dataclass_transform
from uuid import UUID

from bench.language.const import IssueType, TypeHint, TypeStorageFormat, TypeTag
from bench.utils.func import get_subclasses
from bench.utils.utils import required_field

if TYPE_CHECKING:
    from bench.language import Field, HasFields, ScopeNode
    from bench.language.validation import ValidationHandler


@dataclass
class Struct:
    """
    A non-node data structure, usually inside a node.
    Will activate, track, etc. when we start using these in nodes.
    """

    def _clear(self, scope: Optional["ScopeNode"] = None):
        pass

    def _interp(self, scope: "ScopeNode", on_issue: "ValidationHandler"):
        pass

    def _set_untracked(self, key: str, value: Any):
        self.__dict__[key] = value


@dataclass_transform()
def struct(cls: type[Struct] = None):
    """Register a struct class."""

    def decorator(cls: type[Struct]):
        if not issubclass(cls, Struct):
            raise TypeError(f"struct {cls} must be a subclass of {Struct}")
        cls = dataclass(cls)
        return cls

    if cls is not None:
        return decorator(cls)
    return decorator


#
# Expression language. Primarily for module, search and storage (database).
#


class QueryEngine(enum.StrEnum):
    LOCAL = "LOCAL"
    HOST = "HOST"
    OPENSEARCH = "OS"
    POSTGRES = "PG"


class QueryEngineIncapableError(Exception):
    def __init__(self, engine: QueryEngine, expr: Expression, reason: str):
        super().__init__(f"query engine {engine.value} is incapable of {expr}: {reason}")


class ExpressionKind(enum.StrEnum):
    CONDITIONAL = "CONDITIONAL"
    SORT = "SORT"
    AGGREGATION = "AGGREGATION"


class ConditionalOp(enum.StrEnum):
    # logical
    TRUE = "TRUE"
    FALSE = "FALSE"
    NOT = "NOT"
    AND = "AND"
    OR = "OR"
    # comparison
    EQUALS = "EQUALS"
    NOT_EQUALS = "NOT_EQUALS"
    GREATER_THAN = "GREATER_THAN"
    GREATER_THAN_OR_EQUALS = "GREATER_THAN_OR_EQUALS"
    LESS_THAN = "LESS_THAN"
    LESS_THAN_OR_EQUALS = "LESS_THAN_OR_EQUALS"
    # string comparison
    MATCHES = "MATCHES"
    STARTS_WITH = "STARTS_WITH"
    # containment
    CONTAINS = "CONTAINS"
    NOT_CONTAINS = "NOT_CONTAINS"
    IN = "IN"
    NOT_IN = "NOT_IN"
    # existence
    EXISTS = "EXISTS"
    NOT_EXISTS = "DOES_NOT_EXIST"
    # vector
    NEAR = "NEAR"

    @property
    def sign(self) -> str | None:
        return _OP_SIGN.get(self)


class AggregationOp(enum.StrEnum):
    # Single value
    COUNT = "COUNT"
    SUM = "SUM"
    AVERAGE = "AVERAGE"
    MIN = "MIN"
    MAX = "MAX"
    MEDIAN = "MEDIAN"
    # Bucket value
    HISTOGRAM = "HISTOGRAM"


class SortOp(enum.StrEnum):
    ASCENDING = "ASCENDING"
    DESCENDING = "DESCENDING"


class SortMode(enum.StrEnum):
    MAX = "MAX"
    MIN = "MIN"
    AVERAGE = "AVERAGE"
    SUM = "SUM"
    MEDIAN = "MEDIAN"


if TYPE_CHECKING:
    ExpressionOp = ConditionalOp | AggregationOp | SortOp
else:
    ExpressionOp = enum.StrEnum(
        "ExpressionOp",
        {**ConditionalOp.__members__, **AggregationOp.__members__, **SortOp.__members__},
    )
FieldReference = UUID | str  # str as an alias for fields that we don't have reflected yet


@struct
class Expression(Struct):
    op: ExpressionOp

    def __str__(self):
        return self.__class__.__name__

    def __repr__(self):
        return f"{self.kind}({self})"

    @property
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_CLASS[type(self)]


@struct
class FieldExpression(Expression):
    field: Field | FieldReference = required_field()

    @property
    def _field_str(self) -> str:
        if not isinstance(self.field, (UUID, str)):
            return self.field.py_ident
        else:
            return str(self.field)

    def _clear(self, scope: Optional["ScopeNode"] = None):
        if not isinstance(self.field, (UUID, str)) and (
            scope is None or self.field.ck in scope._local_root_tree
        ):
            self._set_untracked("field", self.field.ck)

    def _interp(self, scope: "ScopeNode", on_issue: "ValidationHandler"):
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


_OP_SIGN: dict[ConditionalOp, str] = {
    ConditionalOp.NOT: "~",
    ConditionalOp.AND: "&",
    ConditionalOp.OR: "|",
    ConditionalOp.EQUALS: "==",
    ConditionalOp.NOT_EQUALS: "!=",
    ConditionalOp.GREATER_THAN: ">",
    ConditionalOp.GREATER_THAN_OR_EQUALS: ">=",
    ConditionalOp.LESS_THAN: "<",
    ConditionalOp.LESS_THAN_OR_EQUALS: "<=",
    ConditionalOp.MATCHES: "~=",
    ConditionalOp.STARTS_WITH: "^=",
    ConditionalOp.EXISTS: "?",
    ConditionalOp.NOT_EXISTS: "?!",
}


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
        cls = dataclass(cls, repr=False)
        cls._PROPERTIES = {f.name: f for f in cls.__dataclass_fields__.values()}
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
    op: ConditionalOp

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
    clauses: list[Conditional] = required_field()

    def __str__(self):
        return f" {self.op.sign} ".join(str(q) for q in self.clauses)

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
)
class ComparisonConditional(FieldExpression, Conditional):
    value: Any = required_field()

    def __str__(self):
        return f"{self._field_str} {self.op.sign} {self.value!r}"


@expression(ConditionalOp.EXISTS, ConditionalOp.NOT_EXISTS)
class ExistenceConditional(FieldExpression, Conditional):
    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}"

    def __invert__(self):
        if self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, field=self.field)
        else:
            return C(ConditionalOp.EXISTS, field=self.field)


@expression(ConditionalOp.NEAR)
class VectorConditional(FieldExpression, Conditional):
    value: list[float] = required_field()
    approximate: bool = True

    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}({self.value[:10]}...)"


@expression(*SortOp)
class Sort(FieldExpression):
    op: SortOp = SortOp.ASCENDING
    mode: Optional[SortMode] = None


@expression(*AggregationOp)
class Aggregation(FieldExpression):
    op: AggregationOp = required_field()


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
    statement: "HasFields", expr: Optional[Conditional], kwargs: Optional[dict[str, Any]] = None
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
        if not field:
            raise TypeError(f"unknown field {field_key}")
        if op not in field._supported_query_ops:
            raise TypeError(f"unsupported comparison operand {op} for field {field!r}")
        clauses.append(ComparisonConditional(op=op, field=field, value=value))
    if not clauses:
        return C(ConditionalOp.TRUE)
    return Conditional.and_if_set(*clauses)


# single-letter convenience constructors
def E(op: ExpressionOp, *args, _expect_t: type[Expression] = None, **kwargs) -> Sort:
    cls = EXPRESSION_CLASS_BY_OP[op]
    if _expect_t is not None and not issubclass(cls, _expect_t):
        raise TypeError(f"expected {_expect_t}, got {cls}")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cast(Sort, cls(op, *args, **kwargs))


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


def _check_supports_conditional(field: "Field", op: ConditionalOp):
    if op not in field._supported_query_ops:
        raise UnsupportedExpressionError(field, op)


def _check_supports_sort(field: "Field"):
    if not field.can_sort:
        raise UnsupportedExpressionError(field, "sort")


def _check_support(op: ConditionalOp = None, sort: bool = False):
    def decorator(func):
        @wraps(func)
        def wrapper(self, *args, **kwargs):
            if op is not None:
                _check_supports_conditional(self, op)
            if sort:
                _check_supports_sort(self)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


class FieldQueryOps:
    # for typing, assumes Field superclass
    name: Optional[str]
    hint: Optional[TypeHint]
    _effective_tag: TypeTag
    _source_key: Optional[str]
    _storage_format: TypeStorageFormat

    # basic support checks

    @property
    def can_sort(self) -> bool:
        return self._storage_format in (
            TypeStorageFormat.DATE,
            TypeStorageFormat.DOUBLE,
            TypeStorageFormat.LONG,
            TypeStorageFormat.KEYWORD,
        )

    @property
    def _supported_query_ops(self) -> set[ConditionalOp]:
        format_ops = SUPPORTED_OPS_BY_TYPE.get(self._storage_format, _EMPTY_SET)
        hint_ops = SUPPORTED_OPS_BY_TYPE.get(self.hint, _EMPTY_SET)
        tag_ops = SUPPORTED_OPS_BY_TYPE.get(self._effective_tag, _EMPTY_SET)
        return ExprOps.COND_EXISTENCE | format_ops | hint_ops | tag_ops

    def _strip_value(self: "Field", value: Any) -> Any:
        from bench.language.field import Field

        # coerce to field to get its key
        if self._effective_tag == TypeTag.ENUM and not isinstance(value, Field):
            value = self.resolved_fields.get(value)
        return value

    # comparison

    @_check_support(op=ConditionalOp.EQUALS)
    def equals(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        if value is None:
            return self.not_exists()
        return C(ConditionalOp.EQUALS, self, value)

    def __eq__(self, other):
        from bench.language.module import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self, other)  # imitate Field equality
        return self.equals(other)

    @_check_support(op=ConditionalOp.NOT_EQUALS)
    def not_equal(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        return C(ConditionalOp.NOT_EQUALS, self, value)

    def __ne__(self, other):
        from bench.language.module import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        return self.not_equal(other)

    @_check_support(op=ConditionalOp.IN)
    def in_(self, *values: list[Any]) -> Conditional:
        values = [self._strip_value(value) for value in values]
        return C(ConditionalOp.IN, self, values)

    @_check_support(op=ConditionalOp.NOT_IN)
    def not_in(self, *values: list[Any]) -> Conditional:
        values = [self._strip_value(value) for value in values]
        return C(ConditionalOp.NOT_IN, self, values)

    @_check_support(op=ConditionalOp.GREATER_THAN)
    def greater_than(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        return C(ConditionalOp.GREATER_THAN, self, value)

    def __gt__(self, other):
        return self.greater_than(other)

    @_check_support(op=ConditionalOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        return C(ConditionalOp.GREATER_THAN_OR_EQUALS, self, value)

    def __ge__(self, other):
        return self.greater_than_or_equals(other)

    @_check_support(op=ConditionalOp.LESS_THAN)
    def less_than(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        return C(ConditionalOp.LESS_THAN, self, value)

    def __lt__(self, other):
        return self.less_than(other)

    @_check_support(op=ConditionalOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self, value: Any) -> Conditional:
        value = self._strip_value(value)
        return C(ConditionalOp.LESS_THAN_OR_EQUALS, self, value)

    def __le__(self, other):
        return self.less_than_or_equals(other)

    # string comparison

    @_check_support(op=ConditionalOp.MATCHES)
    def matches(self, value: str) -> Conditional:
        return C(ConditionalOp.MATCHES, self, value)

    contains = matches

    @_check_support(op=ConditionalOp.STARTS_WITH)
    def starts_with(self, value: str) -> Conditional:
        # :StartsWithHack
        return C(ConditionalOp.STARTS_WITH, self, value.lower())

    # existence

    @_check_support(op=ConditionalOp.EXISTS)
    def exists(self) -> Conditional:
        return C(ConditionalOp.EXISTS, self._source_key)

    @_check_support(op=ConditionalOp.NOT_EXISTS)
    def not_exists(self) -> Conditional:
        return C(ConditionalOp.NOT_EXISTS, self._source_key)

    # xy

    # (not yet)

    # knn

    @_check_support(op=ConditionalOp.NEAR)
    def near(self, value: list[float], approximate: bool = True) -> Conditional:
        return C(ConditionalOp.NEAR, self, value, approximate=approximate)

    # sort

    @_check_support(sort=True)
    def asc(self) -> Sort:
        return Sort(self, SortOp.ASCENDING)

    ascending = asc

    @_check_support(sort=True)
    def desc(self) -> Sort:
        return Sort(self, SortOp.DESCENDING)

    descending = desc


ExprOps = ExpressionOps  # alias
SUPPORTED_OPS_BY_TYPE: dict[TypeTag | TypeHint | TypeStorageFormat, set[ConditionalOp]] = {
    # cumulative supported query ops by type
    TypeStorageFormat.LONG: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.DOUBLE: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.BOOLEAN: ExprOps.COND_EXACT,
    TypeStorageFormat.DATE: ExprOps.COND_RANGE | ExprOps.COND_EXACT,
    TypeStorageFormat.KEYWORD: ExprOps.COND_EXACT,
    TypeStorageFormat.VECTOR: ExprOps.COND_VECTOR,
    TypeTag.STRING: ExprOps.COND_EXACT | {ConditionalOp.MATCHES},
    TypeHint.NAME: {ConditionalOp.STARTS_WITH},
}
_EMPTY_SET = set()
