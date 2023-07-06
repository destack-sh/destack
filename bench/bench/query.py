from __future__ import annotations

import enum
from dataclasses import dataclass
from functools import wraps
from typing import TYPE_CHECKING, Any, Optional

from bench.bench.const import TypeFlag, TypeHint, TypeStorageFormat, TypeTag

if TYPE_CHECKING:
    from bench.bench import Field


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
    # comparison (exact)
    EQUALS = "EQUALS"
    NOT_EQUALS = "NOT_EQUALS"
    # comparison (range)
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
    # vector
    NEAR = "NEAR"


class QueryOps:
    LOGICAL = {QueryOp.NOT, QueryOp.AND, QueryOp.OR}
    COMPARISON_EXACT = {QueryOp.EQUALS, QueryOp.NOT_EQUALS}
    COMPARISON_RANGE = {
        QueryOp.GREATER_THAN,
        QueryOp.GREATER_THAN_OR_EQUALS,
        QueryOp.LESS_THAN,
        QueryOp.LESS_THAN_OR_EQUALS,
    }
    EXISTENCE = {QueryOp.EXISTS, QueryOp.DOES_NOT_EXIST}
    XY = {QueryOp.INTERSECTS, QueryOp.DISJOINT, QueryOp.WITHIN}
    VECTOR = {QueryOp.NEAR}


SCORED_QUERY_OPS = {QueryOp.MATCHES, *QueryOps.XY, *QueryOps.VECTOR}

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
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        return Q(QueryOp.AND, queries=[self, other])

    def __or__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        return Q(QueryOp.OR, queries=[self, other])

    filter = __and__

    @property
    def is_scored(self) -> bool:
        return self.op in SCORED_QUERY_OPS

    @staticmethod
    def cls_from_attrs(d: dict[str, Any]) -> type[Query]:  # see :WireFormat
        op = QueryOp(d["op"])
        cls = _QUERIES[op]
        return cls


@query(QueryOp.NOT, QueryOp.AND, QueryOp.OR)
class CompoundQuery(Query):
    queries: list[Query]

    @property
    def is_scored(self) -> bool:
        return any(q.is_scored for q in self.queries)

    def __invert__(self):
        if self.op == QueryOp.NOT:
            return self.queries[0]
        else:
            return super().__invert__()

    def __and__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.op == QueryOp.AND:
            if isinstance(other, CompoundQuery) and other.op == QueryOp.AND:
                return Q(QueryOp.AND, queries=[*self.queries, *other.queries])
            else:
                return Q(QueryOp.AND, queries=[*self.queries, other])
        else:
            return super().__and__(other)

    def __or__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
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


def get_default_sort(query: "Query") -> list["Sort"]:
    if query.is_scored:
        return [Sort("_score", SortOrder.DESCENDING)]
    else:
        return [Sort("_id", SortOrder.ASCENDING)]


#
# Field query ops
#


class UnsupportedSearchError(Exception):
    def __init__(self, field: "Field", thing: Any):
        super().__init__(f"{repr(field)} does not support {thing}")


def _check_supports_query(field: "Field", op: QueryOp):
    if op not in field.supported_query_ops:
        raise UnsupportedSearchError(field, op)


def _check_supports_sort(field: "Field"):
    if not field.can_sort:
        raise UnsupportedSearchError(field, "sort")


def _check_supports_subfield(field: "Field", subfield: SubfieldType):
    if subfield not in field.supported_subfields:
        raise UnsupportedSearchError(field, subfield)


def _check_has_tag(field: "Field", tag: TypeTag):
    if field.effective_tag != tag:
        raise UnsupportedSearchError(field, tag)


def _check_support(op: QueryOp = None, sort: bool = False, subfield: SubfieldType = None):
    def decorator(func):
        @wraps(func)
        def wrapper(self, *args, **kwargs):
            if op is not None:
                _check_supports_query(self, op)
            if sort:
                _check_supports_sort(self)
            if subfield is not None:
                _check_supports_subfield(self, subfield)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


class FieldQueryOps:
    name: Optional[str]
    source_key: Optional[str]
    effective_tag: TypeTag
    hint: Optional[TypeHint]
    metadata: dict[str, Any]
    storage_format: TypeStorageFormat

    # basic support checks

    @property
    def can_sort(self) -> bool:
        return self.storage_format in (
            TypeStorageFormat.DATE,
            TypeStorageFormat.DOUBLE,
            TypeStorageFormat.LONG,
            TypeStorageFormat.KEYWORD,
        )

    @property
    def supported_subfields(self) -> set[SubfieldType]:
        hint_ops = _SUPPORTED_SUBFIELDS_BY_TYPE.get(self.hint, _EMPTY_SET)
        tag_ops = _SUPPORTED_SUBFIELDS_BY_TYPE.get(self.effective_tag, _EMPTY_SET)
        return hint_ops | tag_ops

    @property
    def supported_query_ops(self) -> set[QueryOp]:
        format_ops = _SUPPORTED_QUERY_OPS_BY_TYPE.get(self.storage_format, _EMPTY_SET)
        hint_ops = _SUPPORTED_QUERY_OPS_BY_TYPE.get(self.hint, _EMPTY_SET)
        tag_ops = _SUPPORTED_QUERY_OPS_BY_TYPE.get(self.effective_tag, _EMPTY_SET)
        return _BASE_QUERY_OPS | format_ops | hint_ops | tag_ops

    def _strip_value(self, value: Any) -> Any:
        from bench.bench.type import Field

        if isinstance(value, Field):
            if value.effective_tag == TypeTag.LITERAL:  # for enum members
                value = value.key
            else:
                # prevent confusion since this doesn't translate to a valid query
                # we could make it evaluate to actual comparison but that's even more confusing
                raise TypeError(f"cannot compare a field to a non-literal field: {self} == {value}")
        return value

    # comparison

    @_check_support(op=QueryOp.EQUALS)
    def equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        if value is None:
            return self.not_exists()
        return Q(QueryOp.EQUALS, self.source_key, value)

    def __eq__(self, other):
        if isinstance(other, FieldQueryOps):
            return self.id == other.id  # imitate Field equality
        return self.equals(other)

    @_check_support(op=QueryOp.NOT_EQUALS)
    def not_equal(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.NOT_EQUALS, self.source_key, value)

    def __ne__(self, other):
        return self.not_equal(other)

    @_check_support(op=QueryOp.EQUALS)
    def in_(self, *values: list[Any]) -> Query:
        values = [self._strip_value(value) for value in values]
        return Q(QueryOp.EQUALS, self.source_key, values)

    @_check_support(op=QueryOp.GREATER_THAN)
    def greater_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.GREATER_THAN, self.source_key, value)

    def __gt__(self, other):
        return self.greater_than(other)

    @_check_support(op=QueryOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.GREATER_THAN_OR_EQUALS, self.source_key, value)

    def __ge__(self, other):
        return self.greater_than_or_equals(other)

    @_check_support(op=QueryOp.LESS_THAN)
    def less_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.LESS_THAN, self.source_key, value)

    def __lt__(self, other):
        return self.less_than(other)

    @_check_support(op=QueryOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(QueryOp.LESS_THAN_OR_EQUALS, self.source_key, value)

    def __le__(self, other):
        return self.less_than_or_equals(other)

    # string comparison

    @_check_support(op=QueryOp.MATCHES)
    def matches(self, value: str) -> Query:
        return Q(QueryOp.MATCHES, self.source_key, value)

    contains = matches

    @_check_support(op=QueryOp.STARTS_WITH)
    def starts_with(self, value: str) -> Query:
        return Q(QueryOp.STARTS_WITH, self.source_key, value)

    # existence

    @_check_support(op=QueryOp.EXISTS)
    def exists(self) -> Query:
        return Q(QueryOp.EXISTS, self.source_key)

    @_check_support(op=QueryOp.DOES_NOT_EXIST)
    def not_exists(self) -> Query:
        return Q(QueryOp.DOES_NOT_EXIST, self.source_key)

    # xy

    # (not yet)

    # knn

    @_check_support(op=QueryOp.NEAR)
    def near(self, value: list[float], approximate: bool = True) -> Query:
        return Q(QueryOp.NEAR, self.source_key, value, approximate=approximate)

    # sort

    @_check_support(sort=True)
    def asc(self) -> Sort:
        return Sort(self.source_key, SortOrder.ASCENDING)

    @_check_support(sort=True)
    def desc(self) -> Sort:
        return Sort(self.source_key, SortOrder.DESCENDING)

    # subfields and properties
    # ... should probably put this elsewhere

    def _subfield(self, name: str, tag: TypeTag, hint: Optional[TypeHint] = None) -> Subfield:
        from bench.bench.type import get_storage_format

        storage_format = get_storage_format(tag, hint, TypeFlag.Zero)
        return Subfield(
            parent=self,
            name=name,
            effective_tag=tag,
            hint=hint,
            source_key=self.source_key + "." + name,
            storage_format=storage_format,
        )

    @property
    def raw(self):
        _check_has_tag(self, TypeTag.STRING)
        return self._subfield(SubfieldType.key.name, TypeTag.STRING, TypeHint.KEY)

    @property
    def file_name(self) -> Subfield:
        _check_has_tag(self, TypeTag.FILE)
        return self._subfield("name", TypeTag.STRING, TypeHint.NAME)

    @property
    def content_length(self) -> Subfield:
        _check_has_tag(self, TypeTag.FILE)
        return self._subfield("content_length", TypeTag.NUMBER, TypeHint.INTEGER)

    @property
    def content_type(self) -> Subfield:
        _check_has_tag(self, TypeTag.FILE)
        return self._subfield("content_type", TypeTag.STRING, TypeHint.KEY)

    @property
    def status(self) -> Subfield:
        _check_has_tag(self, TypeTag.FILE)
        return self._subfield("status", TypeTag.STRING, TypeHint.KEY)

    @property
    def token_count(self) -> Subfield:
        _check_supports_subfield(self, SubfieldType.token_count)
        return self._subfield("token_count", TypeTag.NUMBER, TypeHint.INTEGER)

    word_count = token_count  # for convenience

    @property
    def char_count(self) -> Subfield:
        _check_supports_subfield(self, SubfieldType.char_count)
        return self._subfield("char_count", TypeTag.NUMBER, TypeHint.INTEGER)

    length = char_count  # for convenience


@dataclass
class Subfield(FieldQueryOps):
    parent: Optional[FieldQueryOps]
    name: str
    source_key: str
    storage_format: TypeStorageFormat
    effective_tag: TypeTag
    hint: Optional[TypeHint]


# :QuerySubfields
_SUPPORTED_SUBFIELDS_BY_TYPE: dict[TypeHint | TypeTag, set[SubfieldType]] = {
    # cumulative supported subfields by type
    TypeTag.STRING: {SubfieldType.char_count, SubfieldType.token_count},
    TypeHint.EMAIL: {SubfieldType.key, SubfieldType.starts_with},
    TypeHint.NAME: {SubfieldType.key, SubfieldType.starts_with},
}

_BASE_QUERY_OPS = QueryOps.EXISTENCE
_SUPPORTED_QUERY_OPS_BY_TYPE: dict[TypeTag | TypeHint | TypeStorageFormat, set[QueryOp]] = {
    # cumulative supported query ops by type
    TypeStorageFormat.LONG: QueryOps.COMPARISON_RANGE | QueryOps.COMPARISON_EXACT,
    TypeStorageFormat.DOUBLE: QueryOps.COMPARISON_RANGE | QueryOps.COMPARISON_EXACT,
    TypeStorageFormat.BOOLEAN: QueryOps.COMPARISON_EXACT,
    TypeStorageFormat.DATE: QueryOps.COMPARISON_RANGE | QueryOps.COMPARISON_EXACT,
    TypeStorageFormat.KEYWORD: QueryOps.COMPARISON_EXACT,
    TypeStorageFormat.VECTOR: QueryOps.VECTOR,
    TypeTag.STRING: {QueryOp.MATCHES},
    TypeHint.NAME: {QueryOp.STARTS_WITH},
}
_EMPTY_SET = set()
