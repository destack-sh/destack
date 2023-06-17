import enum
from dataclasses import dataclass
from typing import Optional

#
# Dataset access ORM *and* wireable data representation.
# We abstract the database backend here to fit seamlessly with the language.
# (on the backend this is run against OpenSearch)
#

MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000


@dataclass(repr=False)
class Query:
    op: str


@dataclass(repr=False)
class Aggregation(Query):
    pass


class SortOrder(enum.StrEnum):
    ASC = "asc"
    DESC = "desc"


class SortMode(enum.StrEnum):
    MAX = "max"
    MIN = "min"
    AVG = "avg"
    SUM = "sum"


@dataclass(repr=False)
class Sort:
    key: str
    order: SortOrder = SortOrder.ASC
    mode: Optional[SortMode] = None
