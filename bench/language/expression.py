from __future__ import annotations

import enum
from dataclasses import dataclass
from functools import wraps
from typing import TYPE_CHECKING, Any, Optional, Union

from bench.language.const import TypeFlag, TypeHint, TypeStorageFormat, TypeTag
from bench.utils.utils import required_field

if TYPE_CHECKING:
    from bench.language import Field


#
# Expression language. Primarily for module, search and storage (database).
# Currently serves as both the in-memory representation and the wire format.
#


@dataclass
class Statement:
    pass


class ExpressionOp(enum.StrEnum):
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

    @property
    def sign(self) -> str | None:
        return _OP_SIGN.get(self)


ExprOp = ExpressionOp  # alias


@dataclass
class Expression:
    op: ExprOp

    def __repr__(self):
        return f"{self.__class__.__name__}({self})"


_OP_SIGN: dict[ExpressionOp, str] = {
    ExprOp.NOT: "~",
    ExprOp.AND: "&",
    ExprOp.OR: "|",
    ExprOp.EQUALS: "==",
    ExprOp.NOT_EQUALS: "!=",
    ExprOp.GREATER_THAN: ">",
    ExprOp.GREATER_THAN_OR_EQUALS: ">=",
    ExprOp.LESS_THAN: "<",
    ExprOp.LESS_THAN_OR_EQUALS: "<=",
    ExprOp.MATCHES: "~=",
    ExprOp.STARTS_WITH: "^=",
    ExprOp.EXISTS: "?",
    ExprOp.DOES_NOT_EXIST: "?!",
}


class ExpressionOps:
    LOGICAL = {ExprOp.NOT, ExprOp.AND, ExprOp.OR}
    COMPARISON_EXACT = {ExprOp.EQUALS, ExprOp.NOT_EQUALS}
    COMPARISON_RANGE = {
        ExprOp.GREATER_THAN,
        ExprOp.GREATER_THAN_OR_EQUALS,
        ExprOp.LESS_THAN,
        ExprOp.LESS_THAN_OR_EQUALS,
    }
    EXISTENCE = {ExprOp.EXISTS, ExprOp.DOES_NOT_EXIST}
    COMPARISON_XY = {ExprOp.INTERSECTS, ExprOp.DISJOINT, ExprOp.WITHIN}
    COMPARISON_VECTOR = {ExprOp.NEAR}


SCORED_QUERY_OPS = {ExprOp.MATCHES, *ExpressionOps.COMPARISON_XY, *ExpressionOps.COMPARISON_VECTOR}

_EXPRESSIONS: dict[ExprOp, type[Expression]] = {}


def expression(*ops: ExprOp):
    """Register a query class for the given ops."""

    def decorator(cls: type[Expression]):
        cls = dataclass(cls, repr=False)
        cls._PROPERTIES = {f.name: f for f in cls.__dataclass_fields__.values()}
        for op in ops:
            if op in _EXPRESSIONS:
                raise RuntimeError(f"expression for {op} already registered: {_EXPRESSIONS[op]}")
            _EXPRESSIONS[op] = cls
        return cls

    return decorator


@expression()
class Query(Expression):
    def __bool__(self):
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __invert__(self):
        return Q(ExprOp.NOT, queries=[self])

    def __and__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        return Q(ExprOp.AND, queries=[self, other])

    def __or__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        return Q(ExprOp.OR, queries=[self, other])

    filter = __and__

    @property
    def is_scored(self) -> bool:
        return self.op in SCORED_QUERY_OPS

    @staticmethod
    def cls_from_attrs(d: dict[str, Any]) -> type[Query]:  # see :WireFormat
        op = ExprOp(d["op"])
        cls = _EXPRESSIONS[op]
        return cls

    @staticmethod
    def and_if_set(base: Optional[Query], extra: Optional[Query]) -> Optional[Query]:
        if base is None:
            if extra is None:
                return None
            else:
                return extra
        else:
            if extra is None:
                return base
            else:
                return base & extra


@expression(ExprOp.NOT, ExprOp.AND, ExprOp.OR)
class CompoundQuery(Query):
    queries: list[Query]

    def __str__(self):
        return f" {self.op.sign} ".join(str(q) for q in self.queries)

    @property
    def is_scored(self) -> bool:
        return any(q.is_scored for q in self.queries)

    def __invert__(self):
        if self.op == ExprOp.NOT:
            return self.queries[0]
        else:
            return super().__invert__()

    def __and__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.op == ExprOp.AND:
            if isinstance(other, CompoundQuery) and other.op == ExprOp.AND:
                return Q(ExprOp.AND, queries=[*self.queries, *other.queries])
            else:
                return Q(ExprOp.AND, queries=[*self.queries, other])
        else:
            return super().__and__(other)

    def __or__(self, other):
        if not isinstance(other, Query):
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.op == ExprOp.OR:
            if isinstance(other, CompoundQuery) and other.op == ExprOp.OR:
                return Q(ExprOp.OR, queries=[*self.queries, *other.queries])
            else:
                return Q(ExprOp.OR, queries=[*self.queries, other])
        else:
            return super().__or__(other)


if TYPE_CHECKING:
    FieldOrStr = Union[Field, str]
else:
    FieldOrStr = str


@dataclass(repr=False)
class FieldQuery(Query):
    field: FieldOrStr
    subkey: str = None

    def encode_some_attrs(self):  # :WireFormat
        # always inline field key
        return {"field": self.field if isinstance(self.field, str) else self.field._source_key}

    @property
    def _field_str(self):
        return self.field if isinstance(self.field, str) else self.field.path

    @property
    def key(self):
        key = self.field if isinstance(self.field, str) else self.field._source_key
        return key if not self.subkey else f"{key}.{self.subkey}"


@expression(
    ExprOp.EQUALS,
    ExprOp.NOT_EQUALS,
    ExprOp.GREATER_THAN,
    ExprOp.GREATER_THAN_OR_EQUALS,
    ExprOp.LESS_THAN,
    ExprOp.LESS_THAN_OR_EQUALS,
    ExprOp.MATCHES,
    ExprOp.STARTS_WITH,
)
class ComparisonQuery(FieldQuery):
    value: Any = required_field()

    def __str__(self):
        return f"{self._field_str} {self.op.sign} {self.value!r}"


@expression(ExprOp.EXISTS, ExprOp.DOES_NOT_EXIST)
class ExistenceQuery(FieldQuery):
    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}"

    def __invert__(self):
        if self.op == ExprOp.EXISTS:
            return Q(ExprOp.DOES_NOT_EXIST, field=self.field, subkey=self.subkey)
        else:
            return Q(ExprOp.EXISTS, field=self.field, subkey=self.subkey)


@expression(ExprOp.NEAR)
class VectorQuery(FieldQuery):
    value: list[float] = required_field()
    approximate: bool = True

    def __str__(self):
        return f"{self._field_str}.{self.op.name.lower()}({self.value[:10]}...)"


def Q(op: ExprOp, *args, **kwargs) -> Query:
    cls = _EXPRESSIONS[op]
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in cls._PROPERTIES}
    return cls(op, *args, **kwargs)


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
class Sort(Expression):
    field: FieldOrStr
    order: SortOrder = SortOrder.ASCENDING
    mode: Optional[SortMode] = None
    subkey: str = None

    def encode_some_attrs(self):
        # always inline field key
        return {"field": self.field if isinstance(self.field, str) else self.field._source_key}

    @property
    def key(self) -> str:
        key = self.field if isinstance(self.field, str) else self.field._source_key
        return key if not self.subkey else f"{key}.{self.subkey}"


def get_default_sort(query: "Query") -> list["Sort"]:
    if query.is_scored:
        return [Sort("_score", SortOrder.DESCENDING)]
    else:
        return [Sort("_id", SortOrder.ASCENDING)]


TYPE_DISCRIMINATOR_KEY = "_type"


#
# Field query ops
#


class UnsupportedQueryError(Exception):
    def __init__(self, field: "Field", thing: Any):
        super().__init__(f"{repr(field)} does not support {thing}")


def _check_supports_query(field: "Field", op: ExprOp):
    if op not in field._supported_query_ops:
        raise UnsupportedQueryError(field, op)


def _check_supports_sort(field: "Field"):
    if not field.can_sort:
        raise UnsupportedQueryError(field, "sort")


def _check_supports_subfield(field: "Field", subfield: "SubfieldType"):
    if subfield not in field._supported_subfields:
        raise UnsupportedQueryError(field, subfield)


def _check_has_type_tag(field: "Field", tag: TypeTag):
    if field._effective_tag != tag:
        raise UnsupportedQueryError(field, tag)


def _check_support(op: ExprOp = None, sort: bool = False, subfield: "SubfieldType" = None):
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
    hint: Optional[TypeHint]
    _effective_tag: TypeTag
    _source_key: Optional[str]
    _subkey: Optional[str]
    _storage_format: TypeStorageFormat

    @property
    def _field(self) -> "Field":
        return self

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
    def _supported_subfields(self) -> set[SubfieldType]:
        hint_ops = _SUPPORTED_SUBFIELDS_BY_TYPE.get(self.hint, _EMPTY_SET)
        tag_ops = _SUPPORTED_SUBFIELDS_BY_TYPE.get(self._effective_tag, _EMPTY_SET)
        return hint_ops | tag_ops

    @property
    def _supported_query_ops(self) -> set[ExprOp]:
        format_ops = _SUPPORTED_EXPR_OPS_BY_TYPE.get(self._storage_format, _EMPTY_SET)
        hint_ops = _SUPPORTED_EXPR_OPS_BY_TYPE.get(self.hint, _EMPTY_SET)
        tag_ops = _SUPPORTED_EXPR_OPS_BY_TYPE.get(self._effective_tag, _EMPTY_SET)
        return _BASE_EXPR_OPS | format_ops | hint_ops | tag_ops

    def _strip_value(self, value: Any) -> Any:
        from bench.language.field import Field

        # coerce to field to get its key
        if self._effective_tag == TypeTag.ENUM and not isinstance(value, Field):
            value = self.resolved_fields.get(value)

        # coerce field to key
        if isinstance(value, Field):
            if value._effective_tag != TypeTag.LITERAL:
                # prevent confusion since this doesn't translate to a valid query
                raise TypeError(f"cannot compare a field to a non-literal field: {self} == {value}")
            value = value.key
        return value

    # comparison

    @_check_support(op=ExprOp.EQUALS)
    def equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        if value is None:
            return self.not_exists()
        return Q(ExprOp.EQUALS, self._field, self._subkey, value)

    def __eq__(self, other):
        from bench.language.module import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self, other)  # imitate Field equality
        return self.equals(other)

    @_check_support(op=ExprOp.NOT_EQUALS)
    def not_equal(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(ExprOp.NOT_EQUALS, self._field, self._subkey, value)

    def __ne__(self, other):
        from bench.language.module import Node

        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        return self.not_equal(other)

    @_check_support(op=ExprOp.EQUALS)
    def in_(self, *values: list[Any]) -> Query:
        values = [self._strip_value(value) for value in values]
        return Q(ExprOp.EQUALS, self._field, self._subkey, values)

    @_check_support(op=ExprOp.GREATER_THAN)
    def greater_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(ExprOp.GREATER_THAN, self._field, self._subkey, value)

    def __gt__(self, other):
        return self.greater_than(other)

    @_check_support(op=ExprOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(ExprOp.GREATER_THAN_OR_EQUALS, self._field, self._subkey, value)

    def __ge__(self, other):
        return self.greater_than_or_equals(other)

    @_check_support(op=ExprOp.LESS_THAN)
    def less_than(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(ExprOp.LESS_THAN, self._field, self._subkey, value)

    def __lt__(self, other):
        return self.less_than(other)

    @_check_support(op=ExprOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self, value: Any) -> Query:
        value = self._strip_value(value)
        return Q(ExprOp.LESS_THAN_OR_EQUALS, self._field, self._subkey, value)

    def __le__(self, other):
        return self.less_than_or_equals(other)

    # string comparison

    @_check_support(op=ExprOp.MATCHES)
    def matches(self, value: str) -> Query:
        return Q(ExprOp.MATCHES, self._field, self._subkey, value)

    contains = matches

    @_check_support(op=ExprOp.STARTS_WITH)
    def starts_with(self, value: str) -> Query:
        # :StartsWithHack
        return Q(ExprOp.STARTS_WITH, self._field, self._subkey, value.lower())

    @_check_support(op=ExprOp.STARTS_WITH)
    def like(self, value: str) -> Query:
        # combines matches and starts_with
        return Q(ExprOp.STARTS_WITH, self._field, self._subkey, value) | Q(
            ExprOp.MATCHES, self._field, self._subkey, value
        )

    # existence

    @_check_support(op=ExprOp.EXISTS)
    def exists(self) -> Query:
        return Q(ExprOp.EXISTS, self._source_key)

    @_check_support(op=ExprOp.DOES_NOT_EXIST)
    def not_exists(self) -> Query:
        return Q(ExprOp.DOES_NOT_EXIST, self._source_key)

    # xy

    # (not yet)

    # knn

    @_check_support(op=ExprOp.NEAR)
    def near(self, value: list[float], approximate: bool = True) -> Query:
        return Q(ExprOp.NEAR, self._field, self._subkey, value, approximate=approximate)

    # sort

    @_check_support(sort=True)
    def asc(self) -> Sort:
        return Sort(self._field, SortOrder.ASCENDING, subkey=self._subkey)

    ascending = asc

    @_check_support(sort=True)
    def desc(self) -> Sort:
        return Sort(self._field, SortOrder.DESCENDING, subkey=self._subkey)

    descending = desc

    # subfields and properties
    # TODO @Cleanup: wrap sub properties into accessor for disambiguation (like with FieldAccessor)

    def _subfield(self, name: str, tag: TypeTag, hint: Optional[TypeHint] = None) -> Subfield:
        from bench.language.field import get_storage_format

        storage_format = get_storage_format(tag, hint, TypeFlag.ZERO)
        return Subfield(
            parent=self,
            name=name,
            hint=hint,
            _effective_tag=tag,
            _source_key=self._source_key + "." + name,
            _subkey=name,
            _storage_format=storage_format,
        )

    @property
    def raw(self):
        _check_has_type_tag(self, TypeTag.STRING)
        return self._subfield(SubfieldType.key.name, TypeTag.STRING, TypeHint.KEY)

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


# TODO @Cleanup @Architecture: reconsider subfields in expressions (they're ugly)
class SubfieldType(enum.StrEnum):
    # :QuerySubfields
    key = "key"
    starts_with = "starts_with"
    token_count = "token_count"
    char_count = "char_count"


@dataclass(eq=False, frozen=True)
class Subfield(FieldQueryOps):
    parent: Optional[FieldQueryOps]
    name: str
    hint: Optional[TypeHint]
    _subkey: str
    _source_key: str
    _storage_format: TypeStorageFormat
    _effective_tag: TypeTag

    @property
    def _field(self):
        return self.parent


# :QuerySubfields
_SUPPORTED_SUBFIELDS_BY_TYPE: dict[TypeHint | TypeTag, set[SubfieldType]] = {
    # cumulative supported subfields by type
    TypeTag.STRING: {SubfieldType.char_count, SubfieldType.token_count},
    TypeHint.EMAIL: {SubfieldType.key, SubfieldType.starts_with},
    TypeHint.NAME: {SubfieldType.key, SubfieldType.starts_with},
}

ExprOps = ExpressionOps  # alias
_BASE_EXPR_OPS = ExprOps.EXISTENCE
_SUPPORTED_EXPR_OPS_BY_TYPE: dict[TypeTag | TypeHint | TypeStorageFormat, set[ExprOp]] = {
    # cumulative supported query ops by type
    TypeStorageFormat.LONG: ExprOps.COMPARISON_RANGE | ExprOps.COMPARISON_EXACT,
    TypeStorageFormat.DOUBLE: ExprOps.COMPARISON_RANGE | ExprOps.COMPARISON_EXACT,
    TypeStorageFormat.BOOLEAN: ExprOps.COMPARISON_EXACT,
    TypeStorageFormat.DATE: ExprOps.COMPARISON_RANGE | ExprOps.COMPARISON_EXACT,
    TypeStorageFormat.KEYWORD: ExprOps.COMPARISON_EXACT,
    TypeStorageFormat.VECTOR: ExprOps.COMPARISON_VECTOR,
    TypeTag.STRING: {ExprOp.MATCHES},
    TypeHint.NAME: {ExprOp.STARTS_WITH},
}
_EMPTY_SET = set()
