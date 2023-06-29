from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Any, Optional

#
# Dataset access ORM *and* wireable data representation.
# We abstract the database backend here to fit seamlessly with the language.
#


class QueryOp(enum.StrEnum):
    # logical
    NOT = "not"
    AND = "and"
    OR = "or"
    # comparison
    EQUALS = "eq"
    NOT_EQUALS = "neq"
    GREATER_THAN = "gt"
    GREATER_THAN_OR_EQUALS = "gte"
    LESS_THAN = "lt"
    LESS_THAN_OR_EQUALS = "lte"
    # string comparison
    MATCHES = "matches"
    STARTS_WITH = "starts_with"
    # existence
    EXISTS = "exists"
    DOES_NOT_EXIST = "does_not_exist"
    # xy
    INTERSECTS = "intersects"
    DISJOINT = "disjoint"
    WITHIN = "within"
    # knn
    NEAR = "near"


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


@query(QueryOp.NEAR)
class KnnQuery(Query):
    key: str
    value: list[float]
    approximate: bool = True


def Q(op: QueryOp, **kwargs) -> Query:
    cls = _QUERIES[op]
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cls(op, **kwargs)


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
    ASC = "asc"
    DESC = "desc"


class SortMode(enum.StrEnum):
    MAX = "max"
    MIN = "min"
    AVG = "avg"
    SUM = "sum"
    MEDIAN = "median"


@dataclass
class Sort:
    key: str
    order: SortOrder = SortOrder.ASC
    mode: Optional[SortMode] = None
