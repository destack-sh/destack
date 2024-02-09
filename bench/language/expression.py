import functools
import re
from typing import TYPE_CHECKING, Any, Generic, NamedTuple, Optional, TypeVar, Union
from uuid import UUID

from asgiref.sync import async_to_sync

from bench.language.const import (
    AggregationOp,
    BenchError,
    BenchType,
    ConditionalOp,
    ExpressionKind,
    ExpressionOp,
    NodeType,
    QueryEngine,
    SortMode,
    SortOp,
    StructType,
    active_session,
)
from bench.language.node import (
    BENCH_CLASS_BY_TYPE,
    Node,
    Property,
    Struct,
    node,
    p_parent,
    p_regular,
    struct,
)
from bench.proto.wire import AnyNodeData, NodeReferenceData
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing
from bench.utils.func import _auto_async_to_sync

if TYPE_CHECKING:
    from bench.language import Block, Expression, Field, Node, Property, ReadOptions, TypeInfo
    from bench.language.field import HasFields

#
# Expression language. Primarily for package, search and storage (database).
#


_CONDITIONAL_OP_SIGN: dict[ConditionalOp, str] = {
    # logical
    ConditionalOp.NOT: "~",
    ConditionalOp.AND: "&",
    ConditionalOp.OR: "|",
    # comparison
    ConditionalOp.EQUALS: "==",
    ConditionalOp.NOT_EQUALS: "!=",
    ConditionalOp.GREATER_THAN: ">",
    ConditionalOp.GREATER_THAN_OR_EQUALS: ">=",
    ConditionalOp.LESS_THAN: "<",
    ConditionalOp.LESS_THAN_OR_EQUALS: "<=",
    # string comparison
    ConditionalOp.MATCHES: "~=",
    ConditionalOp.STARTS_WITH: "^=",
    ConditionalOp.REGEX: "$re=",
    # containment
    ConditionalOp.CONTAINS: "∋",
    ConditionalOp.NOT_CONTAINS: "!∋",
    ConditionalOp.IN: "∈",
    ConditionalOp.NOT_IN: "!∈",
    # existence
    ConditionalOp.EXISTS: "?",
    ConditionalOp.NOT_EXISTS: "!?",
    # vector
    ConditionalOp.NEAR: "~=",
}


@struct(StructType.NODE_REFERENCE)
class NodeReference(Struct):
    type: NodeType = p_regular(30, require=True)
    id: Optional[UUID] = p_regular(31, default=None)
    ck: Optional[UUID] = p_regular(32, default=None)
    base_ck: Optional[UUID] = p_regular(33, default=None)

    def __content_str__(self):
        selector_str_parts = []
        if self.id is not None:
            selector_str_parts.append(f"id={self.id}")
        if self.ck is not None:
            selector_str_parts.append(f"ck={self.ck}")
        if self.base_ck is not None:
            selector_str_parts.append(f"base_ck={self.base_ck}")
        selector_str = ", ".join(selector_str_parts)
        return f"{self.type.bench_name}:[{selector_str}]"

    @staticmethod
    def from_node(node: Optional[Node]) -> Optional["NodeReference"]:
        if node is None:
            return None
        if "ck" in node.__properties__:
            if node.metatype == NodeType.RECORD:
                return NodeReference(
                    type=node.metatype, id=node.id, ck=node.ck, base_ck=node.parent_ck
                )
            else:
                return NodeReference(type=node.metatype, id=node.id, ck=node.ck)
        else:
            assert node.id is not None, f"cannot reference node without id: {node!r}"
            return NodeReference(type=node.metatype, id=node.id)

    @staticmethod
    def from_node_data(node_data: Optional[AnyNodeData]) -> Optional["NodeReferenceData"]:
        from bench.proto import wire

        if node_data is None:
            return None
        node_cls = BENCH_CLASS_BY_TYPE[node_data.metatype]
        if "ck" in node_cls.__properties__:
            if node_data.metatype == NodeType.RECORD:
                return NodeReferenceData(
                    metatype=wire.StructType.NODE_REFERENCE,
                    type=node_data.metatype,
                    id=node_data.id,
                    ck=node_data.ck,
                    base_ck=node_data.parent_ptr.ck,
                )
            else:
                return NodeReferenceData(
                    metatype=wire.StructType.NODE_REFERENCE,
                    type=node_data.metatype,
                    id=node_data.id,
                    ck=node_data.ck,
                )
        else:
            assert node_data.id is not None, f"cannot reference node without id: {node_data!r}"
            return NodeReferenceData(
                metatype=wire.StructType.NODE_REFERENCE, type=node_data.metatype, id=node_data.id
            )


@struct(StructType.PROPERTY_REFERENCE)
class PropertyReference(Struct):
    type: BenchType = p_regular(30, require=True)
    id: int = p_regular(31)
    # to disambiguate contributed properties
    references_type: Optional[NodeType] = p_regular(32)

    def __content_str__(self):
        return f"{self.type.bench_name}.[id={self.id}]"

    def resolve(self) -> Property:
        bench_cls = BENCH_CLASS_BY_TYPE[self.type]
        return bench_cls._resolve_property(self)


@struct(StructType.PROPERTY_PATH)
class PropertyPath(Struct):
    """A path of Node/Struct properties."""

    properties: list[Property] = p_regular(
        30, require=False, default_factory=list, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    def __content_str__(self):
        return f"{'.'.join(property.name for property in self.properties)}"


@struct(StructType.FIELD_PATH)
class FieldPath(Struct):
    """A path of built-in Node/Struct properties and user-defined Fields."""

    segments: list["FieldPathSegment"] = p_regular(
        30, require=True, array=True, struct=StructType.FIELD_PATH_SEGMENT
    )

    def __content_str__(self):
        return ".".join(str(segment) for segment in self.segments)


@struct(StructType.FIELD_PATH_SEGMENT)
class FieldPathSegment(Struct):
    """A single segment of a FieldPath."""

    property: Optional[Property] = p_regular(
        30, require=False, default=None, array=False, struct=StructType.PROPERTY_REFERENCE
    )
    field: Optional["Field"] = p_regular(31, require=False, array=False, references=NodeType.FIELD)

    def __content_str__(self):
        return f"{self.property.name}{f'.{self.field}' if self.field else ''}"


@struct(StructType.VALUE_REFERENCE)
class ValueReference(Struct):
    """Reference a value at a path of a Node."""

    node: Node = p_regular(30, require=True, array=False, references=(NodeType.BLOCK,))
    path: FieldPath = p_regular(31, require=True, struct=StructType.FIELD_PATH)

    def __content_str__(self):
        if self.node is not None:
            return f"{self.node.absolute_path}.{self.path.__content_str__()}"
        else:
            return f"<detached>:{self.path.__content_str__()}"


@struct(StructType.VALUE_SELECTION)
class ValueSelection(Struct):
    """Select a range of values (or a single value) from a Node."""

    pass


class QueryEngineError(Exception):
    def __init__(
        self, engine: QueryEngine, expr: Union["Expression", list["Expression"]], reason: str
    ):
        super().__init__(f"query engine {engine.value} failed on {expr!r}: {reason}")


class QueryEngineIncapableError(QueryEngineError):
    pass


__property__ = property


@struct(StructType.EXPRESSION)
class Expression(Struct):
    """An expression (conditional, aggregation, sort, etc)."""

    op: ExpressionOp = p_regular(30, require=True)
    field: Optional["Field"] = p_regular(31, require=False, array=False, references=NodeType.FIELD)
    property: Optional[Property] = p_regular(
        32, require=False, default=None, array=False, struct=StructType.PROPERTY_REFERENCE
    )
    clauses: list["Expression"] | None = p_regular(35, default=None, struct=StructType.EXPRESSION)
    value: Any = p_regular(36, default=None, primitive_type=PrimitiveType.JSON)
    mode: Optional[SortMode] = p_regular(37, default=None)

    @__property__
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_OP[self.op]

    def __bool__(self):
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __content_str__(self):
        if self.op in ExpressionOps.COND_STATIC:
            return to_casing(self.op.name, Casing.CAMEL)
        elif self.op in ExpressionOps.COND_LOGICAL:
            return f" {_CONDITIONAL_OP_SIGN[self.op]} ".join(str(q) for q in self.clauses)
        elif (
            self.op in ExpressionOps.COND_EXACT
            or self.op in ExpressionOps.COND_RANGE
            or self.op in ExpressionOps.COND_STRING
        ):
            value_str = str(self.value)
            if len(value_str) > 32:
                value_str = f"{value_str[:24]}...{value_str[-12:]}"
            return f"{self.target.py_ident}{_CONDITIONAL_OP_SIGN[self.op]}{value_str}"
        elif self.op in ExpressionOps.COND_EXISTENCE:
            return f"{self.target.py_ident}{_CONDITIONAL_OP_SIGN[self.op]}"
        elif self.op in ExpressionOps.SORT:
            return f"{'-' if self.op == SortOp.DESCENDING else ''}{self.target.py_ident}"
        return to_casing(self.op.name, Casing.CAMEL)

    def __invert__(self):
        if self.op == ConditionalOp.TRUE:
            return C(ConditionalOp.FALSE)
        elif self.op == ConditionalOp.FALSE:
            return C(ConditionalOp.TRUE)
        elif self.op == ConditionalOp.NOT:
            return self.clauses[0]
        elif self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, field=self.field, property=self.property)
        elif self.op == ConditionalOp.NOT_EXISTS:
            return C(ConditionalOp.EXISTS, field=self.field, property=self.property)
        else:
            return C(ConditionalOp.NOT, clauses=[self])

    def __and__(self, other: "Expression"):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.AND:
            if isinstance(other, Expression) and other.op == ConditionalOp.AND:
                return C(ConditionalOp.AND, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.AND, clauses=[*self.clauses, other])
        else:
            return C(ConditionalOp.AND, clauses=[self, other])

    def __or__(self, other: "Expression"):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.OR:
            if isinstance(other, Expression) and other.op == ConditionalOp.OR:
                return C(ConditionalOp.OR, clauses=[*self.clauses, *other.clauses])
            else:
                return C(ConditionalOp.OR, clauses=[*self.clauses, other])
        else:
            return C(ConditionalOp.OR, clauses=[self, other])

    @__property__
    def target(self) -> Union["Field", Property, None]:
        if self.field is not None:
            return self.field
        elif self.property is not None:
            return self.property
        else:
            return None

    def _collect_ops(self) -> set[ExpressionOp]:
        """Collect all ops in this expression and its clauses (recursively)."""
        ops = {self.op}
        if self.clauses:
            for clause in self.clauses:
                ops |= clause._collect_ops()
        return ops

    @staticmethod
    def and_if_set(
        *clauses: Optional["Expression"],
    ) -> Optional["Expression"]:
        base = None
        for clause in clauses:
            if clause is not None:
                if base is None:
                    base = clause
                else:
                    base &= clause
        return base


class ExpressionOps:  # :ExpressionOps
    # Conditionals
    COND_STATIC = {ConditionalOp.TRUE, ConditionalOp.FALSE}
    COND_LOGICAL = {ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR}
    COND_EXACT = {
        ConditionalOp.EQUALS,
        ConditionalOp.NOT_EQUALS,
        ConditionalOp.IN,
        ConditionalOp.NOT_IN,
    }
    COND_RANGE = {
        ConditionalOp.GREATER_THAN,
        ConditionalOp.GREATER_THAN_OR_EQUALS,
        ConditionalOp.LESS_THAN,
        ConditionalOp.LESS_THAN_OR_EQUALS,
    }
    COND_COMPARISON = {*COND_EXACT, *COND_RANGE}
    COND_SET = {ConditionalOp.CONTAINS, ConditionalOp.NOT_CONTAINS}
    COND_EXISTENCE = {ConditionalOp.EXISTS, ConditionalOp.NOT_EXISTS}
    COND_VECTOR = {ConditionalOp.NEAR}
    COND_STRING = {ConditionalOp.STARTS_WITH, ConditionalOp.MATCHES}
    COND_SCORED = {ConditionalOp.NEAR, *COND_STRING}
    # Aggregations
    AGG_BOOLEAN = {AggregationOp.EXISTS}
    AGG_SCALAR = {
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


EXPRESSION_OPS_BY_KIND: dict[ExpressionKind, set[ExpressionOp]] = {
    ExpressionKind.CONDITIONAL: {*ConditionalOp},
    ExpressionKind.AGGREGATION: {*AggregationOp},
    ExpressionKind.SORT: {*SortOp},
}
EXPRESSION_KIND_BY_OP: dict[ExpressionOp, ExpressionKind] = {
    op: kind for kind, ops in EXPRESSION_OPS_BY_KIND.items() for op in ops
}

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


@struct(StructType.AGGREGATION)
class Aggregation(Struct):
    """The result of an aggregation expression."""

    op: AggregationOp = p_regular(30, require=True)
    exists: Optional[bool] = p_regular(31, default=None)
    count: Optional[int] = p_regular(32, default=None)
    scalar: Optional[float] = p_regular(33, default=None)
    buckets: list["AggregationBucket"] | None = p_regular(
        34, default=None, array=True, struct=StructType.AGGREGATION_BUCKET
    )


@struct(StructType.AGGREGATION_BUCKET)
class AggregationBucket(Struct):
    """One bucket of an aggregation histogram."""

    key: Any = p_regular(30, require=True, primitive_type=PrimitiveType.JSON)
    count: int = p_regular(31, require=True)


def coerce_conditional(
    node: Union[Node, type[Node], "HasFields"],
    expr: Optional[Expression] = None,
    kwargs: Optional[dict[str, Any]] = None,
    return_none_if_empty: bool = False,
) -> Optional[Expression]:
    """
    Coerce a conditional expression from either the given expression or kwargs.
    Useful for basic Django-style querying (with optional __<op>, but no relation support yet).
    """
    if expr is not None and kwargs:
        raise TypeError(f"cannot specify both {expr} and {kwargs}")
    if expr is not None:
        if not isinstance(expr, Expression) or expr.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"expected Conditional, got {expr!r}")
        return expr

    clauses = []
    for arg, value in kwargs.items():
        if "__" in arg:
            field_key, op = arg.split("__", 1)
            op = CONDITIONAL_OP_BY_DJANGO_STR[op]
        else:
            field_key, op = arg, ConditionalOp.EQUALS
        target = None
        if field_key in node.__properties__:
            target = node.__properties__[field_key]
        elif isinstance(node, Node) and HasFields in node._components:
            target = node.fields.get(field_key)
        if target is None:
            raise TypeError(f"{node!r} has no field {field_key}")
        elif isinstance(target, Property):
            field, property = None, target
        else:
            field, property = target, None
        _check_field_supports(target._as_type, op)
        if value is None:
            if op == ConditionalOp.EQUALS:
                op = ConditionalOp.NOT_EXISTS
            elif op == ConditionalOp.NOT_EQUALS:
                op = ConditionalOp.EXISTS
        clauses.append(Expression(op=op, field=field, property=property, value=value))
    if not clauses:
        if return_none_if_empty:
            return None
        else:
            return C(ConditionalOp.TRUE)
    return Expression.and_if_set(*clauses)


def coerce_sort(
    node: Union[Node, type[Node], "HasFields"],
    sort: list[Expression | str] | Expression | str | None,
    *args: str,
) -> Optional[list[Expression]]:
    """
    Coerce a sort expression from either the given expression or args.
    Strings are looked up as field names/identifiers.
    Like in Django, prefix with "-" for descending.
    """
    if sort is None:
        if args is None:
            return None
        sort = args
    elif isinstance(sort, str):
        sort = [sort]
    elif isinstance(sort, Expression) and sort.kind == ExpressionKind.SORT:
        sort = [sort]
    if not isinstance(sort, (list, tuple)):
        raise TypeError(f"expected sort to be a list or tuple, got {sort}")
    if args:
        sort = (*sort, *args)
    coerced = []
    for item in sort:
        if isinstance(item, str):
            if item.startswith("-"):
                op = SortOp.DESCENDING
                field_key = item[1:]
            else:
                op = SortOp.ASCENDING
                field_key = item
            target = None
            if field_key in node.__properties__:
                target = node.__properties__[field_key]
            elif isinstance(node, Node) and HasFields in node._components:
                target = node.fields.get(field_key)
            if target is None:
                raise TypeError(f"{node!r} has no field {item!r}")
            elif isinstance(target, Property):
                item = S(op, field=None, property=target)
            else:
                item = S(op, field=target, property=None)
            _check_field_supports(target._as_type, op)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


# single-letter convenience constructors
def E(op: ExpressionOp, *, _expect_kind: type[ExpressionKind] = None, **kwargs) -> Expression:
    if _expect_kind is not None and op.kind != _expect_kind:
        raise TypeError(f"expected {_expect_kind}, got {op} ({op.kind})")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in Expression.__properties__}
    return Expression(op=op, **kwargs)


C = functools.partial(E, _expect_t=ExpressionKind.CONDITIONAL)
S = functools.partial(E, _expect_t=ExpressionKind.SORT)
A = functools.partial(E, _expect_t=ExpressionKind.AGGREGATION)

CONDITIONAL_TRUE = C(ConditionalOp.TRUE)
SCORE_KEY = "_score"  # for ranking
TYPE_DISCRIMINATOR_KEY = "_type"


#
# Field query ops
# :ExpressionSupport
#


class UnsupportedExpressionError(ValueError):
    def __init__(self, field: "Field", thing: Any):
        super().__init__(f"{repr(field)} does not support {thing}")


def _check_field_supports(type: "TypeInfo", op: ExpressionOp):
    """Asserts that the field supports the given expression operator."""
    if op in SortOp:
        return type.primitive_type in (
            PrimitiveType.DATETIME,
            PrimitiveType.FLOAT32,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
        )
    else:
        if op not in _ExprOps.COND_EXISTENCE and op not in SUPPORTED_OPS_BY_TYPE.get(
            type.primitive_type, _EMPTY_SET
        ):
            raise UnsupportedExpressionError(type, op)


# should probably also have expression support per query engine?
_ExprOps = ExpressionOps  # alias
SUPPORTED_OPS_BY_TYPE: dict[PrimitiveType, set[ConditionalOp]] = {
    # cumulative supported query ops by type
    PrimitiveType.UUID: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INT32: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INT64: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.FLOAT32: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.FLOAT64: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.BOOLEAN: _ExprOps.COND_EXACT,
    PrimitiveType.DATETIME: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.VECTOR: _ExprOps.COND_VECTOR,
    PrimitiveType.STRING: _ExprOps.COND_EXACT
    | _ExprOps.COND_RANGE
    | {ConditionalOp.MATCHES, ConditionalOp.STARTS_WITH, ConditionalOp.REGEX},
}
_EMPTY_SET = set()

NodeT = TypeVar("NodeT", bound="Node")


def _require_expression_op(op: ExpressionOp):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(self: "_TypeExpressionBase", *args, **kwargs):
            _check_field_supports(self._as_type, op)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


def _to_conditional(op: ConditionalOp, target: Union["Field", "Property"], value: Any = None):
    if isinstance(target, Property):
        return C(op, field=None, property=target, value=value)
    else:
        return C(op, field=target, property=None, value=value)


def _to_sort(op: SortOp, target: Union["Field", "Property"]):
    if isinstance(target, Property):
        return S(op, field=None, property=target)
    else:
        return S(op, field=target, property=None)


class _TypeExpressionBase:
    """
    Base for field-like expressions on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def _as_type(self) -> "TypeInfo":
        raise NotImplementedError(f"{self!r} does not implement type")

    # comparison

    @_require_expression_op(ConditionalOp.EQUALS)
    def equals(self: Any, value: Any) -> "Expression":
        if value is None:
            return self.not_exists()
        return _to_conditional(ConditionalOp.EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.NOT_EQUALS)
    def not_equal(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.GREATER_THAN)
    def greater_than(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.GREATER_THAN, self, value=value)

    @_require_expression_op(ConditionalOp.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.GREATER_THAN_OR_EQUALS, self, value=value)

    @_require_expression_op(ConditionalOp.LESS_THAN)
    def less_than(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.LESS_THAN, self, value=value)

    @_require_expression_op(ConditionalOp.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.LESS_THAN_OR_EQUALS, self, value=value)

    def __eq__(self, other):
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self, other)  # imitate Field equality
        else:
            return self.equals(other)

    def __ne__(self, other):
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        else:
            return self.not_equal(other)

    __gt__ = greater_than
    __ge__ = greater_than_or_equals
    __lt__ = less_than
    __le__ = less_than_or_equals

    # string comparison

    @_require_expression_op(ConditionalOp.MATCHES)
    def matches(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.MATCHES, self, value=value)

    @_require_expression_op(ConditionalOp.STARTS_WITH)
    def starts_with(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.STARTS_WITH, self, value=value)

    @_require_expression_op(ConditionalOp.REGEX)
    def regex(self: Any, value: str | re.Pattern) -> "Expression":
        if isinstance(value, re.Pattern):
            value = value.pattern
        return _to_conditional(ConditionalOp.REGEX, self, value=value)

    # containment

    @_require_expression_op(ConditionalOp.IN)
    def in_(self: Any, *values: list[Any]) -> "Expression":
        return _to_conditional(ConditionalOp.IN, self, value=values)

    @_require_expression_op(ConditionalOp.NOT_IN)
    def not_in(self: Any, *values: list[Any]) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_IN, self, value=values)

    @_require_expression_op(ConditionalOp.CONTAINS)
    def contains(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.CONTAINS, self, value=value)

    @_require_expression_op(ConditionalOp.NOT_CONTAINS)
    def not_contains(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_CONTAINS, self, value=value)

    # existence

    @_require_expression_op(ConditionalOp.EXISTS)
    def exists(self: Any) -> "Expression":
        return _to_conditional(ConditionalOp.EXISTS, self)

    @_require_expression_op(ConditionalOp.NOT_EXISTS)
    def not_exists(self: Any) -> "Expression":
        return _to_conditional(ConditionalOp.NOT_EXISTS, self)

    # knn

    @_require_expression_op(ConditionalOp.NEAR)
    def near(self: Any, value: list[float]) -> "Expression":
        return _to_conditional(ConditionalOp.NEAR, self, value=value)

    # sort

    @_require_expression_op(SortOp.ASCENDING)
    def asc(self: Any) -> "Expression":
        return _to_sort(SortOp.ASCENDING, self)

    ascending = asc

    @_require_expression_op(SortOp.DESCENDING)
    def desc(self: Any) -> "Expression":
        return _to_sort(SortOp.DESCENDING, self)

    descending = desc


FieldOrProperty = Union["Field", "Property", Any]


class _NodeExpressionBase:
    @classmethod
    def query(cls: type["Node"]):
        return NodeQuery(node_type=cls.metatype)

    @classmethod
    async def tolist(cls: type["Node"]) -> list[NodeT]:
        return await NodeQuery(node_type=cls.metatype).tolist()

    @classmethod
    def get(cls: type["Node"], conditional: "Expression" = None, **kwargs) -> "NodeT":
        return NodeQuery(node_type=cls.metatype).get(conditional, **kwargs)

    @classmethod
    def filter(cls: type["Node"], filter: "Expression" = None, **kwargs) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).filter(filter, **kwargs)

    @classmethod
    def sort(cls: type["Node"], sort: "Expression" = None, *args: str) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).sort(sort, *args)

    @classmethod
    def include(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).include(*properties)

    @classmethod
    def exclude(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).exclude(*properties)

    @classmethod
    def related(cls: type["Node"], *properties: FieldOrProperty) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).related(*properties)

    @classmethod
    def first(cls: type["Node"], count: int) -> "NodeQuery":
        return NodeQuery(node_type=cls.metatype).first(count)

    @classmethod
    async def count(cls: type["Node"], filter: "Expression" = None, **kwargs) -> int:
        return NodeQuery(node_type=cls.metatype).count(filter, **kwargs)

    @classmethod
    async def exists(cls: type["Node"], filter: "Expression" = None, **kwargs) -> bool:
        return NodeQuery(node_type=cls.metatype).exists(filter, **kwargs)


#
# Queries
#


@node(NodeType.QUERY)
class Query(Node):
    """A persistent query."""

    parent: Union["Block"] = p_parent(4, NodeType.BLOCK)
    name: str | None = p_regular(30, default=None)
    order_key: str | None = p_regular(31, default=None)
    node_type: NodeType = p_regular(32)
    bases: list["Block"] | None = p_regular(
        33, array=True, require=False, default=None, references=NodeType.BLOCK
    )
    filter: Optional[Expression] = p_regular(34, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list[Expression]] = p_regular(35, default=None, struct=StructType.EXPRESSION)

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"


# NOTE: For some reason Node.id type checks as <annotation>', but it's a Property? dataclass transform broken?
_NodeFetchResult = NamedTuple(
    "_NodeFetchResult",
    [
        ("nodes", list[AnyNodeData]),
        ("cursors", list[str]),
        ("start_cursor", str | None),
        ("total", int),
        ("engine", QueryEngine),
    ],
)


class QueryError(BenchError, ValueError):
    def __init__(self, query: "NodeQuery", cause: Exception | None = None):
        super().__init__(repr(query))
        self.query = query
        self.cause = cause


class NodeNotFoundError(QueryError):
    pass


class MultipleNodesFoundError(QueryError):
    pass


class NodeQuery(Generic[NodeT]):
    def __init__(
        self,
        node_type: NodeType | None,
        filter: Optional["Expression"] = None,
        sort: list["Expression"] | None = None,
        first: int | None = None,
        skip: int | None = None,
        engine: Optional[QueryEngine] = None,
        options: Optional["ReadOptions"] = None,
        cache: bool = True,
    ):
        from bench.language.node import NODE_CLASS_BY_TYPE, Node

        self._node_type = node_type
        self._node_cls = NODE_CLASS_BY_TYPE[node_type] if node_type else Node
        self._filter = filter
        self._sort = sort
        self._first = first
        self._skip = skip
        self._engine = engine
        self._options = options
        self._cache = cache
        self._cached_nodes: list[NodeT] | None = None
        self._cached_cursors: list[str] | None = None

    def __str__(self):
        args_strs = []
        for k in ("filter", "sort", "first", "skip", "engine"):
            v = getattr(self, f"_{k}", None)
            if k == "query":
                v = f"({v})" if v is not None else None
            if v is not None:
                args_strs.append(f"{k}={v}")
        if args_strs:
            args_str = ", ".join(args_strs)
        else:
            args_str = "[*]"
        return f"{self._node_type.bench_name} {args_str}"

    def __repr__(self):
        return f"<{self.__class__.__name__} {self}>"

    def copy(self):
        """Clones the query (the properties are immutable)."""
        return NodeQuery(
            node_type=self._node_type,
            filter=self._filter,
            sort=self._sort,
            first=self._first,
            skip=self._skip,
            engine=self._engine,
            options=self._options,
            # cache is not copied on purpose as it shouldn't propagate
        )

    def _copy_options(self) -> "ReadOptions":
        from bench.language.access import ReadOptions

        if self._options is None:
            return ReadOptions()
        else:
            return self._options.copy()

    def _invalidate(self):
        self._cached_records = None
        self._cached_cursors = None

    def _get_target_engine(self, *with_ops: "ExpressionOp") -> QueryEngine:
        from bench.language.expression import ExpressionOps

        node_cls = self._node_cls
        if node_cls.__is_local__:
            ops = self._filter._collect_ops() if self._filter else ()
            if with_ops:
                ops |= set(with_ops)
            if node_cls.__is_indexed_in_search__ and (
                ops & ExpressionOps.AGG_SCALAR or ops & ExpressionOps.AGG_BUCKET
            ):
                return QueryEngine.LOCAL_OPENSEARCH
            else:
                return QueryEngine.LOCAL_POSTGRES
        else:
            return QueryEngine.GLOBAL_POSTGRES

    async def __aiter__(self):
        if self._cached_nodes is None:
            return iter(await self._fetch())
        return iter(self._cached_nodes)

    @_auto_async_to_sync
    async def tolist(self) -> list[NodeT]:
        if self._cached_nodes is None:
            return await self._fetch()
        return self._cached_nodes

    def __iter__(self):
        if self._cached_nodes is None:
            return iter(async_to_sync(self._fetch)())
        return iter(self._cached_nodes)

    def __len__(self):
        if self._cached_nodes is not None:
            return len(self._cached_nodes)
        return self.count()

    @_auto_async_to_sync
    async def get(self, filter: "Expression" = None, **kwargs) -> NodeT:
        """Returns the unique result matching the query (errors otherwise)."""
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        results = await self.filter(filter).tolist()
        if len(results) == 1:
            return results[0]
        else:
            combined_query = self.filter(filter)
            if len(results) == 0:
                raise NodeNotFoundError(combined_query)
            else:
                raise MultipleNodesFoundError(combined_query)

    def filter(self, filter: "Expression" = None, **kwargs) -> "NodeQuery[NodeT]":
        """Adds a filter clause to the query."""
        filter = coerce_conditional(self._node_cls, filter, kwargs)
        copy = self.copy()
        copy._filter = filter & self._filter if self._filter is not None else filter
        return copy

    def sort(
        self, sort: Union[list[Union["Expression", str]], str, "Expression"] = None, *args: str
    ) -> "NodeQuery[NodeT]":
        """Sorts the query results by the given sort criteria."""
        copy = self.copy()
        sort = coerce_sort(self._node_cls, sort, *args)
        copy._sort = sort
        return copy

    def first(self, count: int) -> "NodeQuery[NodeT]":
        """Returns the first N results."""
        copy = self.copy()
        copy._first = count
        return copy

    def skip(self, count: int) -> "NodeQuery[NodeT]":
        """Skips the first N results."""
        copy = self.copy()
        copy._skip = count
        return copy

    def include(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.include_properties.extend(properties)
        return copy

    def exclude(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.exclude_properties.extend(*properties)
        return copy

    def related(self, *properties: FieldOrProperty) -> "NodeQuery":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.related_properties.extend(*properties)
        return copy

    def ancestors(self, *node_types: NodeType) -> "NodeQuery":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.ancestor_types = node_types
        return copy

    def descendants(self, *node_types: NodeType) -> "NodeQuery":
        copy = self.copy()
        copy._options = self._copy_options()
        copy._options.descendant_types = node_types
        return copy

    def __getitem__(self, item: slice | int) -> Union["NodeQuery[NodeT]", NodeT]:
        if isinstance(item, slice):
            if item.stop is None:
                return self.skip(item.start or 0)
            elif item.start is not None:
                return self.skip(item.start).first(item.stop - item.start)
            else:
                return self.first(item.stop)
        elif isinstance(item, int):
            if self._cached_nodes is None:
                records = async_to_sync(self._fetch)()
            else:
                records = self._cached_nodes
            if item < 0:
                item += len(records)
            if item >= len(records):
                raise IndexError(f"index {item} out of range for {self!r} (got {len(self)})")
            return records[item]
        else:
            raise TypeError(f"expected slice or index into {self!r}, got {type(item)}: {item}")

    async def _fetch(self) -> list[NodeT] | tuple[NodeT, ...]:
        from bench.proto import wire, wiring
        from bench.sql.engine import ReadOptions, pg_search_nodes

        session = active_session()
        engine = self._get_target_engine()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            nodes, cursors, _ = await pg_search_nodes(
                session=session,
                node_type=self._node_type,
                options=self._options or ReadOptions(),
                filter=self._filter,
                sort=self._sort,
                first=self._first,
                skip=self._skip,
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request from host
            request = wire.SearchNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(self._filter),
                sort=[wiring.pack_struct(s) for s in self._sort] if self._sort else None,
                limit=self._first,
                options=wiring.pack_struct_maybe(self._options),
            )
            await session.host.search_nodes(request)
            # return [wiring.unwrap_some_node(n) for n in response.nodes]
            raise NotImplementedError("nocheckin: NodeQuery._do_fetch GLOBAL_POSTGRES")
        else:
            raise ValueError(f"unexpected query engine {engine}")

        if self._cache:
            self._cached_nodes = nodes
            self._cached_cursors = cursors

        return nodes

    @_auto_async_to_sync
    async def count(self, filter: "Expression" = None, **kwargs) -> int:
        """Returns the number of results. May refine the query."""
        from bench.proto import wire, wiring
        from bench.sql.engine import compile_pg_conditional, pg_count

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        engine = self._get_target_engine()
        session = active_session()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            cur = session._global_pg_cursor or session._local_pg_cursor
            return await pg_count(
                cur=cur,
                table=self._node_cls.__table__,
                where=compile_pg_conditional(self._node_cls, filter),
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request from host
            request = wire.AggregateNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(filter),
                limit=self._first,
                aggregation=wire.ExpressionData(op=wire.ExpressionOp.COUNT),
            )
            aggregation_data = await active_session()._host.aggregate_nodes(request)
            return int(aggregation_data.aggregation.scalar)
        else:
            raise ValueError(f"unexpected query engine {engine}")

    @_auto_async_to_sync
    async def exists(self, filter: "Expression" = None, **kwargs) -> bool:
        """Whether any results exist. May refine the query."""
        from bench.proto import wire, wiring
        from bench.sql.engine import compile_pg_conditional, pg_exists

        filter = coerce_conditional(self._node_cls, filter, kwargs, return_none_if_empty=True)
        engine = self._get_target_engine()
        session = active_session()
        if engine == QueryEngine.LOCAL_POSTGRES or (
            engine == QueryEngine.GLOBAL_POSTGRES and session._global_pg_cursor
        ):
            cur = session._global_pg_cursor or session._local_pg_cursor
            return await pg_exists(
                cur=cur,
                table=self._node_cls.__table__,
                where=compile_pg_conditional(self._node_cls, filter),
            )
        elif engine == QueryEngine.GLOBAL_POSTGRES:  # request remotely
            request = wire.AggregateNodesRequest(
                node_type=wiring.pack_enum(NodeType, self._node_type),
                filter=wiring.pack_struct_maybe(filter),
                aggregation=wire.ExpressionData(op=wire.ExpressionOp.EXISTS),
            )
            aggregation_data = await active_session()._host.aggregate_nodes(request)
            return aggregation_data.aggregation.exists
        else:
            raise ValueError(f"unexpected query engine {engine}")
