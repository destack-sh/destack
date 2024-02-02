import functools
from typing import TYPE_CHECKING, Any, Optional, Union
from uuid import UUID

from bench.language.const import (
    _CONDITIONAL_OP_SIGN,
    AggregationOp,
    BenchType,
    ConditionalOp,
    ExpressionKind,
    ExpressionOp,
    NodeType,
    QueryEngine,
    SortMode,
    SortOp,
    StructType,
)
from bench.language.node import (
    BENCH_CLASS_BY_TYPE,
    Node,
    Property,
    Struct,
    node,
    node_parent,
    struct,
    struct_property,
    struct_runtime,
)
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing

if TYPE_CHECKING:
    from bench.language import Block, Field, TypeInfo, ScopeNode
    from bench.language.field import HasFields
    from bench.language.notice import NoticeHandler


#
# Expression language. Primarily for package, search and storage (database).
#


@struct(StructType.NODE_REFERENCE)
class NodeReference(Struct):
    type: NodeType = struct_property(30, require=True)
    id: Optional[UUID] = struct_property(31, default=None)
    ck: Optional[UUID] = struct_property(32, default=None)
    base_ck: Optional[UUID] = struct_property(33, default=None)

    def __content_str__(self):
        return f"{self.type.bench_name}:[id={self.id}, ck={self.ck}]"

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


@struct(StructType.PROPERTY_REFERENCE)
class PropertyReference(Struct):
    type: BenchType = struct_property(30, require=True)
    id: int = struct_property(31)
    # to disambiguate contributed properties
    references_type: Optional[NodeType] = struct_property(32)
    _resolved_property: Optional[Property] = struct_runtime(default=None)

    def __content_str__(self):
        return f"{self.type.bench_name}.[id={self.id}]"

    def _clear_inner(self, scope: Optional["ScopeNode"] = None):
        self._resolved_property = None

    def _interp_inner(self, scope: "ScopeNode", on_notice: "NoticeHandler"):
        bench_cls = BENCH_CLASS_BY_TYPE[self.type]
        self._resolved_property = bench_cls._resolve_property(self)

    @property
    def resolved_name(self) -> str:
        assert self._resolved_property is not None, f"{self!r} is not resolved"
        return self._resolved_property.name

    @property
    def resolved_property(self) -> Property:
        assert self._resolved_property is not None, f"{self!r} is not resolved"
        return self._resolved_property


@struct(StructType.PROPERTY_PATH)
class PropertyPath(Struct):
    """A path of Node/Struct properties."""

    properties: list[PropertyReference] = struct_property(
        30, require=True, array=True, struct=StructType.PROPERTY_REFERENCE
    )

    def __content_str__(self):
        return f"{'.'.join(str(property) for property in self.properties)}"


@struct(StructType.FIELD_PATH)
class FieldPath(Struct):
    """A path of built-in Node/Struct properties and user-defined Fields."""

    segments: list["FieldPathSegment"] = struct_property(
        30, require=True, array=True, struct=StructType.FIELD_PATH_SEGMENT
    )

    def __content_str__(self):
        return ".".join(str(segment) for segment in self.segments)


@struct(StructType.FIELD_PATH_SEGMENT)
class FieldPathSegment(Struct):
    """A single segment of a FieldPath."""

    property: PropertyReference = struct_property(
        30, require=True, struct=StructType.PROPERTY_REFERENCE
    )
    field: Optional["Field"] = struct_property(
        31, require=False, array=False, references=NodeType.FIELD
    )

    def __content_str__(self):
        return f"{self.property}{f'.{self.field}' if self.field else ''}"


@struct(StructType.VALUE_REFERENCE)
class ValueReference(Struct):
    """Reference a value at a path of a Node."""

    node: Node = struct_property(30, require=True, array=False, references=(NodeType.BLOCK,))
    path: FieldPath = struct_property(31, require=True, struct=StructType.FIELD_PATH)

    def __content_str__(self):
        if self.node is not None:
            return f"{self.node.path}.{self.path.__content_str__()}"
        else:
            return f"<detached>:{self.path.__content_str__()}"


class QueryEngineError(Exception):
    def __init__(
        self, engine: QueryEngine, expr: Union["Expression", list["Expression"]], reason: str
    ):
        super().__init__(f"query engine {engine.value} failed on {expr!r}: {reason}")


class QueryEngineIncapableError(QueryEngineError):
    pass


@struct(StructType.EXPRESSION)
class Expression(Struct):
    """An expression (conditional, aggregation, sort, etc)."""

    op: ExpressionOp = struct_property(30, require=True)
    field: Optional["Field"] = struct_property(
        31, require=False, array=False, references=NodeType.FIELD
    )
    property_ptr: Optional[PropertyReference] = struct_property(
        32, default=None, struct=StructType.PROPERTY_REFERENCE
    )
    clauses: list["Expression"] | None = struct_property(
        35, default=None, struct=StructType.EXPRESSION
    )
    value: Any = struct_property(36, default=None, primitive_type=PrimitiveType.JSON)
    mode: Optional[SortMode] = struct_property(37, default=None)

    @property
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
            return C(ConditionalOp.NOT_EXISTS, field=self.field, property_ptr=self.property_ptr)
        elif self.op == ConditionalOp.NOT_EXISTS:
            return C(ConditionalOp.EXISTS, field=self.field, property_ptr=self.property_ptr)
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

    @property
    def target(self) -> Union["Field", Property, None]:
        if self.field is not None:
            return self.field
        elif self.property_ptr is not None:
            return self.property_ptr.resolved_property
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
assert len(EXPRESSION_KIND_BY_OP) == len(ExpressionOp), "missing expression op/kind mapping"

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

    op: AggregationOp = struct_property(30, require=True)
    exists: Optional[bool] = struct_property(31, default=False)
    scalar: Optional[float] = struct_property(32, default=None)
    buckets: list["AggregationBucket"] | None = struct_property(
        33, default=None, array=True, struct=StructType.AGGREGATION_BUCKET
    )


@struct(StructType.AGGREGATION_BUCKET)
class AggregationBucket(Struct):
    """One bucket of an aggregation histogram."""

    key: Any = struct_property(30, require=True, primitive_type=PrimitiveType.JSON)
    count: int = struct_property(31, require=True)


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
        _check_field_supports(target._as_type, op)
        if isinstance(target, Property):
            field, property = None, target.to_ref
        else:
            field, property = target, None
        if value is None:
            if op == ConditionalOp.EQUALS:
                op = ConditionalOp.NOT_EXISTS
            elif op == ConditionalOp.NOT_EQUALS:
                op = ConditionalOp.EXISTS
        clauses.append(Expression(op=op, field=field, property_ptr=property, value=value))
    if not clauses:
        if return_none_if_empty:
            return None
        else:
            return C(ConditionalOp.TRUE)
    return Expression.and_if_set(*clauses)


def coerce_sort(
    node: Union[Node, type[Node], "HasFields"],
    sort: list[Expression | str] | Expression | str | None,
    args: str | None = None,
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
            if isinstance(target, Property):
                field, property = None, target.to_ref
            else:
                field, property = target, None
            item = S(op, field=field, property_ptr=property)
            _check_field_supports(target._as_type, op)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


# single-letter convenience constructors
def E(op: ExpressionOp, *, _expect_t: type[ExpressionKind] = None, **kwargs) -> Expression:
    if _expect_t is not None and EXPRESSION_KIND_BY_OP[op] != _expect_t:
        raise TypeError(f"expected {_expect_t}, got {EXPRESSION_KIND_BY_OP[op]}")
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


#
# Queries
#


@node(NodeType.QUERY)
class Query(Node):
    """A persistent query."""

    parent: Union["Block"] = node_parent(4, NodeType.BLOCK)
    name: str | None = struct_property(30, default=None)
    order_key: str | None = struct_property(31, default=None)
    node_type: NodeType = struct_property(32)
    bases: list["Block"] | None = struct_property(
        33, array=True, require=False, default=None, references=NodeType.BLOCK
    )
    filter: Optional[Expression] = struct_property(34, default=None, struct=StructType.EXPRESSION)
    sort: Optional[list[Expression]] = struct_property(
        35, default=None, struct=StructType.EXPRESSION
    )

    def __content_str__(self):
        return f"{self.node_type}[{self.filter}, {self.sort or '<default sort>'}]"
