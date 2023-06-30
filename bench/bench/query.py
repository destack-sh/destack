from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Any, Optional

from bench.bench.const import TypeHint, TypeTag

#
# Dataset access ORM *and* wireable data representation.
# We abstract the database backend here to fit seamlessly with the language.
#


class SubfieldType(enum.StrEnum):
    # :QuerySubfields
    key = "key"
    starts_with = "starts_with"
    token_count = "token_count"
    char_count = "char_count"


class QueryOp(enum.StrEnum):
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
    # string comparison
    MATCHES = "MATCHES"
    STARTS_WITH = "STARTS_WITH"
    # existence
    EXISTS = "EXISTS"
    DOES_NOT_EXIST = "DOES_NOT_EXIST"
    # xy
    INTERSECTS = "INTERSECTS"
    DISJOINT = "DISJOINT"
    WITHIN = "WITHIN"
    # knn
    NEAR = "NEAR"


_QUERIES: dict[QueryOp, type[Query]] = {}


def query(*ops: QueryOp):
    """Register a query class for the given ops."""

    def decorator(cls: type[Query]):
        cls = dataclass(cls)
        cls._PROPERTIES = {f.name: f for f in cls.__dataclass_fields__.values()}
        for op in ops:
            if op in _QUERIES:
                raise RuntimeError(f"query for {op} already registered: {_QUERIES[op]}")
            _QUERIES[op] = cls
        return cls

    return decorator


@query()
class Query:
    op: QueryOp

    def __invert__(self):
        return Q(QueryOp.NOT, queries=[self])

    def __and__(self, other):
        return Q(QueryOp.AND, queries=[self, other])

    def __or__(self, other):
        return Q(QueryOp.OR, queries=[self, other])

    def filter(self, other):
        return Q(QueryOp.AND, queries=[self, other])

    @staticmethod
    def cls_from_attrs(d: dict[str, Any]) -> type[Query]:  # see :WireFormat
        op = QueryOp(d["op"])
        cls = _QUERIES[op]
        return cls


@query(QueryOp.NOT, QueryOp.AND, QueryOp.OR)
class CompoundQuery(Query):
    queries: list[Query]

    def __invert__(self):
        if self.op == QueryOp.NOT:
            return self.queries[0]
        else:
            return super().__invert__()

    def __and__(self, other):
        if self.op == QueryOp.AND:
            if isinstance(other, CompoundQuery) and other.op == QueryOp.AND:
                return Q(QueryOp.AND, queries=[*self.queries, *other.queries])
            else:
                return Q(QueryOp.AND, queries=[*self.queries, other])
        else:
            return super().__and__(other)

    def __or__(self, other):
        if self.op == QueryOp.OR:
            if isinstance(other, CompoundQuery) and other.op == QueryOp.OR:
                return Q(QueryOp.OR, queries=[*self.queries, *other.queries])
            else:
                return Q(QueryOp.OR, queries=[*self.queries, other])
        else:
            return super().__or__(other)


@query(
    QueryOp.EQUALS,
    QueryOp.NOT_EQUALS,
    QueryOp.GREATER_THAN,
    QueryOp.GREATER_THAN_OR_EQUALS,
    QueryOp.LESS_THAN,
    QueryOp.MATCHES,
    QueryOp.STARTS_WITH,
)
class ComparisonQuery(Query):
    key: str
    value: Any


@query(QueryOp.EXISTS, QueryOp.DOES_NOT_EXIST)
class ExistenceQuery(Query):
    key: str

    def __invert__(self):
        if self.op == QueryOp.EXISTS:
            return Q(QueryOp.DOES_NOT_EXIST, key=self.key)
        else:
            return Q(QueryOp.EXISTS, key=self.key)


@query()
class VectorQuery(Query):
    key: str
    value: list[float]
    approximate: bool = True


def Q(op: QueryOp, *args, **kwargs) -> Query:
    cls = _QUERIES[op]
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cls(op, *args, **kwargs)


class AggregationOp(enum.StrEnum):
    COUNT = "count"
    SUM = "sum"
    AVG = "avg"
    MIN = "min"
    MAX = "max"
    STATS = "stats"
    PERCENTILES = "percentiles"
    HISTOGRAM = "histogram"


@dataclass
class Aggregation:
    op: AggregationOp
    name: Optional[str]


@dataclass
class MetricAggregation(Aggregation):
    key: str


class SortOrder(enum.StrEnum):
    ASCENDING = "ASCENDING"
    DESCENDING = "DESCENDING"


class SortMode(enum.StrEnum):
    MAX = "MAX"
    MIN = "MIN"
    AVERAGE = "AVERAGE"
    SUM = "SUM"
    MEDIAN = "MEDIAN"


@dataclass
class Sort:
    key: str
    order: SortOrder = SortOrder.ASCENDING
    mode: Optional[SortMode] = None


# TODO @Robustness @UX: check which query ops are actually available for the field
class FieldQueryOps:
    name: Optional[str]
    source_key: Optional[str]
    tag: TypeTag
    hint: Optional[TypeHint]
    metadata: dict[str, Any]

    def _strip_value(self, value: Any) -> Any:
        from bench.bench.type import Field

        if isinstance(value, Field):
            if value.tag == TypeTag.LITERAL:  # for enum members
                value = value.key
            else:
                # prevent confusion since this doesn't translate to a valid query
                # we could make it evaluate to actual comparison but that's even more confusing
                raise TypeError(f"cannot compare a field to a non-literal field: {self} == {value}")
        return value

    # comparison

    def equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        if value is None:
            return self.not_exists()
        return Q(QueryOp.EQUALS, self.source_key, value)

    def __eq__(self, other):
        return self.equals(other)

    def not_equal(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.NOT_EQUALS, self.source_key, value)

    def __ne__(self, other):
        return self.not_equal(other)

    def in_(self, *values: list[Any]) -> Query:
        values = [self._strip_value(value) for value in values]
        return Q(QueryOp.EQUALS, self.source_key, values)

    def greater_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.GREATER_THAN, self.source_key, value)

    def __gt__(self, other):
        return self.greater_than(other)

    def greater_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.GREATER_THAN_OR_EQUALS, self.source_key, value)

    def __ge__(self, other):
        return self.greater_than_or_equals(other)

    def less_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.LESS_THAN, self.source_key, value)

    def __lt__(self, other):
        return self.less_than(other)

    def less_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.LESS_THAN_OR_EQUALS, self.source_key, value)

    def __le__(self, other):
        return self.less_than_or_equals(other)

    # string comparison

    def matches(self, value: str) -> Query:
        return Q(QueryOp.MATCHES, self.source_key, value)

    def contains(self, value: str) -> Query:
        return Q(QueryOp.MATCHES, self.source_key, value)

    def starts_with(self, value: str) -> Query:
        return Q(QueryOp.STARTS_WITH, self.source_key, value)

    # existence

    def exists(self) -> Query:
        return Q(QueryOp.EXISTS, self.source_key)

    def not_exists(self) -> Query:
        return Q(QueryOp.DOES_NOT_EXIST, self.source_key)

    # xy

    # (not yet)

    # knn

    def near(self, value: list[float], approximate: bool = True) -> Query:
        return Q(QueryOp.NEAR, self.source_key, value, approximate=approximate)

    # sort

    def asc(self) -> Sort:
        return Sort(self.source_key, SortOrder.ASCENDING)

    def desc(self) -> Sort:
        return Sort(self.source_key, SortOrder.DESCENDING)

    # subfields and properties
    # we could probably auto generate these.... maybe when we get to checking available ops

    def _subfield(self, name: str, tag: TypeTag, hint: Optional[TypeHint] = None) -> Subfield:
        return Subfield(
            parent=self,
            name=name,
            tag=tag,
            hint=hint,
            source_key=self.source_key + "." + name,
        )

    @property
    def name(self) -> Subfield:
        return self._subfield("name", TypeTag.STRING, TypeHint.NAME)

    @property
    def content_length(self) -> Subfield:
        return self._subfield("content_length", TypeTag.NUMBER, TypeHint.INTEGER)

    @property
    def content_type(self) -> Subfield:
        return self._subfield("content_type", TypeTag.STRING, TypeHint.KEY)

    @property
    def status(self) -> Subfield:
        return self._subfield("status", TypeTag.STRING, TypeHint.KEY)

    @property
    def token_count(self) -> Subfield:
        return self._subfield("token_count", TypeTag.NUMBER, TypeHint.INTEGER)

    @property
    def char_count(self) -> Subfield:
        return self._subfield("char_count", TypeTag.NUMBER, TypeHint.INTEGER)


@dataclass
class Subfield(FieldQueryOps):
    parent: FieldQueryOps
    name: str
    source_key: str
    tag: TypeTag
    hint: Optional[TypeHint]
    metadata: dict[str, Any] = None
