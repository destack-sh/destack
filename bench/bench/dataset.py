from __future__ import annotations

import enum
from dataclasses import dataclass
from typing import Optional

#
# Dataset access ORM *and* wireable data representation.
# We abstract the database backend here to fit seamlessly with the language.
#

MAX_VERSIONED_RECORDS_TOTAL = 64_000
MAX_VERSIONED_RECORDS_PER_DATASET = 8_000


@dataclass
class Query:
    op: str


@dataclass
class Aggregation:
    op: str


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


Q = Query
A = Aggregation
S = Sort
