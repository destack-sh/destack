from __future__ import annotations

import enum
from dataclasses import dataclass
from functools import wraps
from typing import TYPE_CHECKING, Any, ClassVar, Optional, cast

from bench.language.const import TypeHint, TypeStorageFormat, TypeTag

if TYPE_CHECKING:
    from bench.language import Field


#
# Expression language. Primarily for module, search and storage (database).
# nocheckin: separate expression language and wire format
#


class QueryEngine(enum.StrEnum):
    LOCAL = "LOCAL"
    RUNTIME = "RUNTIME"
    OPENSEARCH = "OS"
    POSTGRES = "PG"


class QueryEngineIncapableError(Exception):
    pass


class ExpressionKind(enum.StrEnum):
    CONDITIONAL = "CONDITIONAL"
    SORT = "SORT"
    AGGREGATION = "AGGREGATION"


class ConditionalOp(enum.StrEnum):
    # logical
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
    # containment
    CONTAINS = "CONTAINS"
    NOT_CONTAINS = "NOT_CONTAINS"
    IN = "IN"
    NOT_IN = "NOT_IN"
    # string comparison
    MATCHES = "MATCHES"
    STARTS_WITH = "STARTS_WITH"
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


ExpressionOp = ConditionalOp | AggregationOp | SortOp


@dataclass
class Expression:
    kind: ClassVar[ExpressionKind]

    def __str__(self):
        return self.__class__.__name__

    def __repr__(self):
        return f"{self.__class__.__name__}({self})"


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
    COND_LOGICAL = {ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR}
    COND_EXACT = {ConditionalOp.EQUALS, ConditionalOp.NOT_EQUALS}
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

_EXPRESSIONS: dict[ExpressionOp, type[Expression]] = {}


def expression(*ops: ExpressionOp):
    """Register a query class for the given ops."""

    def decorator(cls: type[Expression]):
        if not issubclass(cls, Expression):
            raise TypeError(f"expression {cls} must be a subclass of {Expression}")
        cls = dataclass(cls, repr=False)
        cls._PROPERTIES = {f.name: f for f in cls.__dataclass_fields__.values()}
        for op in ops:
            if op in _EXPRESSIONS:
                raise RuntimeError(f"expression for {op} already registered: {_EXPRESSIONS[op]}")
            _EXPRESSIONS[op] = cls
        return cls

    return decorator


@expression()
class Conditional(Expression):
    kind: ClassVar[ExpressionKind] = ExpressionKind.CONDITIONAL
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
    def cls_from_attrs(d: dict[str, Any]) -> type[Conditional]:  # see :WireFormat
        op = ConditionalOp(d["op"])
        cls = _EXPRESSIONS[op]
        return cls

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


@expression(ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR)
class CompoundConditional(Conditional):
    clauses: list[Conditional]

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
class ComparisonConditional(Conditional):
    field: Field
    value: Any | Field

    def __str__(self):
        return f"{self._field_str} {self.op.sign} {self.value!r}"


@expression(ConditionalOp.EXISTS, ConditionalOp.NOT_EXISTS)
class ExistenceConditional(Conditional):
    field: Field

    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}"

    def __invert__(self):
        if self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, field=self.field)
        else:
            return C(ConditionalOp.EXISTS, field=self.field)


@expression(ConditionalOp.NEAR)
class VectorConditional(Conditional):
    field: Field
    value: list[float]
    approximate: bool = True

    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}({self.value[:10]}...)"


@expression(*SortOp)
class Sort(Expression):
    kind: ClassVar[ExpressionKind] = ExpressionKind.SORT
    field: Field
    op: SortOp = SortOp.ASCENDING
    mode: Optional[SortMode] = None

    def encode_some_attrs(self):
        # always inline field key
        return {"field": self.field if isinstance(self.field, str) else self.field._source_key}


@expression(*AggregationOp)
class Aggregation(Expression):
    kind: ClassVar[ExpressionKind] = ExpressionKind.AGGREGATION
    op: AggregationOp
    field: Field


def get_default_sort(query: "Conditional") -> list["Sort"]:
    if query.is_scored:
        return [Sort("_score", SortOp.DESCENDING)]
    else:
        return [Sort("_id", SortOp.ASCENDING)]


def C(op: ConditionalOp, *args, **kwargs) -> Conditional:
    cls = _EXPRESSIONS[op]
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cast(Conditional, cls(op, *args, **kwargs))


def S(op: SortOp, *args, **kwargs) -> Sort:
    cls = _EXPRESSIONS[op]
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cast(Sort, cls(op, *args, **kwargs))


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
        # coerce field to key
        if isinstance(value, Field):
            if value._effective_tag != TypeTag.LITERAL:
                # prevent confusion since this doesn't translate to a valid query
                raise TypeError(f"cannot compare a field to a non-literal field: {self} == {value}")
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
    TypeTag.STRING: {ConditionalOp.MATCHES},
    TypeHint.NAME: {ConditionalOp.STARTS_WITH},
}
_EMPTY_SET = set()
