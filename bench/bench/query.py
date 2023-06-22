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
    # xy
    INTERSECTS = "intersects"
    DISJOINT = "disjoint"
    WITHIN = "within"
    CONTAINS = "contains"
    # knn
    NEAR = "near"


@dataclass
class Query:
    op: QueryOp


@dataclass
class CompoundQuery(Query):
    queries: list[Query]


@dataclass
class ComparisonQuery(Query):
    key: str
    value: Any


@dataclass
class KnnQuery(Query):
    key: str
    value: Any


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


@dataclass
class Sort:
    key: str
    order: SortOrder = SortOrder.ASC
    mode: Optional[SortMode] = None
