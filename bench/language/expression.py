import functools
import re
from typing import TYPE_CHECKING, Any, Collection, Optional, TypeVar, Union, cast
from uuid import UUID

from bench.language.const import (
    BASED_NODE_TYPES,
    IN_BENCH_NODE_TYPES,
    SUB_BENCH_NODE_TYPES,
    AggregationOp,
    ConditionalOp,
    EnumType,
    ExpressionKind,
    ExpressionOp,
    NodeType,
    ObjectType,
    SortMode,
    SortOp,
    StructType,
    enum_,
)
from bench.language.node import BasedNode, Node, Property, Struct, struct
from bench.language.property import p_regular, p_value_packed, p_value_runtime
from bench.language.setup import BENCH_CLASS_BY_TYPE
from bench.language.validation import ValidationHandler
from bench.language.value import HasValues
from bench.proto.wire import AnyNodeData, NodeReferenceData
from bench.sql.core import PrimitiveType
from bench.utils.casing import Casing, to_casing
from bench.utils.func import IdEnum

if TYPE_CHECKING:
    from bench.language import Block, Field, Path, TypeInfo

# pyright: reportIncompatibleVariableOverride=false

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
    ConditionalOp.EXISTS: "!",
    ConditionalOp.NOT_EXISTS: "!!",
    # vector
    ConditionalOp.NEAR: "~=",
}


@struct(StructType.NODE_REFERENCE, inline=True)
class NodeReference(Struct):
    """
    A reference to a Node.
    If the reference is to a node in a Bench, we include the Bench ID and 'ck' (where available).
    If the second half of a ck is zero, it matches the closest node with the 'ck' prefix.
    Base tracks which node the node is 'based' on (like Record.parent->Block, Signal.type->Block).
    """

    type: NodeType = p_regular(30, require=True)
    id: Optional[UUID] = p_regular(31, default=None)
    ck: Optional[UUID] = p_regular(32, default=None)
    bench_id: Optional[UUID] = p_regular(33, default=None)
    base_ck: Optional[UUID] = p_regular(34, default=None)
    # base could be in a different Bench (e.g. a Signal in Bench A with a type from Bench B)
    base_bench_id: Optional[UUID] = p_regular(35, default=None)

    def __content_str__(self):
        selector_str_parts = []
        if self.id is not None:
            selector_str_parts.append(f"id={self.id}")
        if self.ck is not None:
            selector_str_parts.append(f"ck={self.ck}")
        if self.bench_id is not None:
            selector_str_parts.append(f"bench_id={self.bench_id}")
        if self.base_ck is not None:
            selector_str_parts.append(f"base_ck={self.base_ck}")
        if self.base_bench_id is not None:
            selector_str_parts.append(f"base_bench_id={self.base_bench_id}")
        selector_str = ", ".join(selector_str_parts)
        return f"{self.type.bench_name}:[{selector_str}]"

    def _validate_component(
        self, properties: Collection[Property], invalid: "ValidationHandler"
    ) -> None:
        if self.id is None:
            invalid(self, "id is required", (NodeReference.id,))
        if self.type in SUB_BENCH_NODE_TYPES and self.bench_id is None:
            invalid(self, "bench_id is required", (NodeReference.bench_id,))
        if self.type in BASED_NODE_TYPES and self.base_ck is None:
            invalid(self, "base_ck is required", (NodeReference.base_ck,))

    @staticmethod
    def from_node(node: Node) -> "NodeReference":  # type: ignore
        assert isinstance(node, Node), f"expected Node, got {node!r}"
        reference = NodeReference(type=node.metatype, id=node.id)

        # bench_id
        if node.metatype == NodeType.BENCH:
            reference.bench_id = node.id
        elif "bench" in node.__properties__:
            reference.bench_id = node.bench_id
        # ck
        if "ck" in node.__properties__:
            reference.ck = node.ck
        # base
        if node.metatype in BASED_NODE_TYPES:
            base = cast(BasedNode, node).base
            if base is not None:
                reference.base_ck = base.ck
                reference.base_bench_id = base.bench_id

        return reference

    @staticmethod
    def from_node_data(node_data: AnyNodeData) -> "NodeReferenceData":
        from bench.proto import wire

        node_cls = BENCH_CLASS_BY_TYPE[cast(ObjectType, node_data.metatype)]
        reference = NodeReferenceData(
            metatype=wire.ObjectType.NODE_REFERENCE,
            type=cast(wire.NodeType, node_data.metatype),
            id=node_data.id,
        )

        # bench_id
        if node_data.metatype == NodeType.BENCH:
            reference.bench_id = node_data.id
        elif "bench" in node_cls.__properties__ and node_data.parent_ptr is not None:
            reference.bench_id = node_data.parent_ptr.bench_id
        # ck
        if "ck" in node_cls.__properties__:
            reference.ck = getattr(node_data, "ck")
        # base
        if NodeType(node_data.metatype) in BASED_NODE_TYPES:
            base = cast(BasedNode, node_cls).get_base_from_data(node_data)
            if base is not None:
                reference.base_ck = base.ck
                reference.base_bench_id = base.bench_id

        return reference


@struct(StructType.PROPERTY_REFERENCE, inline=True)
class PropertyReference(Struct):
    type: Optional[ObjectType] = p_regular(30, require=False)
    id: int = p_regular(31)
    # to disambiguate contributed properties
    references_type: Optional[NodeType] = p_regular(32, default=None)

    def __content_str__(self):
        if self.type is not None:
            return f"{self.type.bench_name}.[id={self.id}]"
        else:
            return f"???.[id={self.id}]"

    def resolve(self) -> Property:
        if self.type is not None:
            bench_cls = BENCH_CLASS_BY_TYPE[self.type]
            return bench_cls._resolve_property(self)
        else:
            return Node._resolve_property(self)


@struct(StructType.VALUE_REFERENCE)
class ValueReference(Struct):
    """Reference a value at a path of a Node."""

    path: "Path" = p_regular(31, require=True, struct=StructType.PATH)

    def __content_str__(self):
        return f"@{self.path}" if self.path is not None else "@???"


@enum_(EnumType.SELECTION_KIND)
class SelectionKind(IdEnum):
    RANGE = 1
    LIST = 2


@struct(StructType.SELECTION, inline=True)
class Selection(Struct):
    """A selection of nodes/values."""

    kind: SelectionKind = p_regular(30, require=True)
    nodes: list[Node] | None = p_regular(
        31, require=False, array=True, references=IN_BENCH_NODE_TYPES.tuple
    )
    from_node: Optional[Node] = p_regular(
        32, require=False, array=False, references=IN_BENCH_NODE_TYPES.tuple
    )
    to_node: Optional[Node] = p_regular(
        33, require=False, array=False, references=IN_BENCH_NODE_TYPES.tuple
    )


__property__ = property


@struct(StructType.EXPRESSION)
class Expression(HasValues):
    """An expression (conditional, aggregation, sort, etc)."""

    op: ExpressionOp = p_regular(30, require=True)
    field: Optional["Field"] = p_regular(31, require=False, array=False, references=NodeType.FIELD)
    property: Optional[Property] = p_regular(
        32, require=False, default=None, array=False, struct=StructType.PROPERTY_REFERENCE
    )
    clauses: list["Expression"] | None = p_regular(35, array=True, struct=StructType.EXPRESSION)
    value_packed: Any = p_value_packed(36)
    value: Any = p_value_runtime(36, type=lambda self: cast(Expression, self)._value_type)
    sort_mode: Optional[SortMode] = p_regular(38, default=None)

    @__property__
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_OP[self.op]

    @__property__
    def _value_type(self) -> "TypeInfo":
        if self.field is not None:
            typ = self.field.as_type_info
        elif self.property is not None:
            typ = self.property.as_type_info
        else:
            raise ValueError(f"no target for {self!r}")
        # wrap as list if needed
        if not typ.is_list and (self.op == ConditionalOp.IN or self.op == ConditionalOp.NOT_IN):
            typ = typ._copy(is_list=True)
        return typ

    def __bool__(self):
        # safe-guard to ensure expressions are not used directly in boolean context
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __content_str__(self):
        if self.op in ExpressionOps.COND_STATIC:
            return to_casing(self.op.name, Casing.CAMEL)
        elif self.op in ExpressionOps.COND_LOGICAL:
            return f" {_CONDITIONAL_OP_SIGN[self.op]} ".join(str(q) for q in self.clauses or ())
        elif (
            self.op in ExpressionOps.COND_EXACT
            or self.op in ExpressionOps.COND_RANGE
            or self.op in ExpressionOps.COND_STRING
        ):
            py_ident = self.target.py_ident if self.target is not None else "???"
            value_str = str(self.value)
            if len(value_str) > 32:
                value_str = f"{value_str[:24]}...{value_str[-12:]}"
            return f"{py_ident}{_CONDITIONAL_OP_SIGN[self.op]}{value_str}"
        elif self.op in ExpressionOps.COND_EXISTENCE:
            py_ident = self.target.py_ident if self.target is not None else "???"
            return f"{py_ident}{_CONDITIONAL_OP_SIGN[self.op]}"
        elif self.op in ExpressionOps.SORT:
            py_ident = self.target.py_ident if self.target is not None else "???"
            return f"{'-' if self.op == SortOp.DESCENDING else ''}{py_ident}"
        return to_casing(self.op.name, Casing.CAMEL)

    def __invert__(self):
        if self.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"cannot invert {self!r} (expected Conditional, got {self.kind})")
        if self.op == ConditionalOp.TRUE:
            return C(ConditionalOp.FALSE)
        elif self.op == ConditionalOp.FALSE:
            return C(ConditionalOp.TRUE)
        elif self.op == ConditionalOp.NOT:
            assert (
                self.clauses is not None and len(self.clauses) == 1
            ), f"expected 1 clause, got {self}"
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
                return C(ConditionalOp.AND, clauses=[*(self.clauses or ()), *(other.clauses or ())])
            else:
                return C(ConditionalOp.AND, clauses=[*(self.clauses or ()), other])
        else:
            return C(ConditionalOp.AND, clauses=[self, other])

    def __or__(self, other: "Expression"):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.op == ConditionalOp.OR:
            if isinstance(other, Expression) and other.op == ConditionalOp.OR:
                return C(ConditionalOp.OR, clauses=[*(self.clauses or ()), *(other.clauses or ())])
            else:
                return C(ConditionalOp.OR, clauses=[*(self.clauses or ()), other])
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
        34, array=True, struct=StructType.AGGREGATION_BUCKET
    )


@struct(StructType.AGGREGATION_BUCKET, inline=True)
class AggregationBucket(Struct):
    """One bucket of an aggregation histogram."""

    key: Any = p_regular(30, require=True, primitive_type=PrimitiveType.JSON)
    count: int = p_regular(31, require=True)


def coerce_conditional(
    node: Union[type[Node], "Block"],
    expr: Optional[Expression] = None,
    kwargs: Optional[dict[str, Any]] = None,
    return_none_if_empty: bool = False,
) -> Expression | None:
    """
    Coerce a conditional expression from either the given expression or kwargs.
    Useful for basic Django-style querying (with optional __<op>, but no relation support yet).
    """
    if expr is not None and kwargs:
        raise TypeError(f"cannot specify both {expr} and {kwargs}")
    if expr is not None:
        if not isinstance(expr, Expression) or expr.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"expected Conditional, got {expr!r} ({type(expr)})")
        return expr

    clauses = []
    for arg, value in (kwargs or {}).items():
        if "__" in arg:
            field_key, op = arg.split("__", 1)
            op = CONDITIONAL_OP_BY_DJANGO_STR[op]
        else:
            field_key, op = arg, ConditionalOp.EQUALS
        target = None
        if field_key in node.__properties__:
            target = node.__properties__[field_key]
        elif isinstance(node, Node) and "fields" in node.__node_list_properties__:
            target = node.fields.get(field_key)
        if target is None:
            raise TypeError(f"{node!r} has no field {field_key}")
        elif isinstance(target, Property):
            field, property = None, target
        else:
            field, property = target, None
        _check_type_supports(target.as_type_info, op)
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
    node: Union[Node, type[Node], "Block"],
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
        sort = list(args)
    elif isinstance(sort, str):
        sort = [sort]
    elif isinstance(sort, Expression) and sort.kind == ExpressionKind.SORT:
        sort = [sort]
    if not isinstance(sort, (list, tuple)):
        raise TypeError(f"expected sort to be a list or tuple, got {sort}")
    if args:
        sort = cast(list[Expression | str], (*sort, *args))
    sort = cast(list[Expression | str], sort)
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
            elif isinstance(node, Node) and "fields" in node.__node_list_properties__:
                target = getattr(node, "fields").get(field_key)
            if target is None:
                raise TypeError(f"{node!r} has no field {item!r}")
            elif isinstance(target, Property):
                item = S(op, field=None, property=target)
            else:
                item = S(op, field=target, property=None)
            _check_type_supports(target.as_type_info, op)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


# single-letter convenience constructors
def E(
    op: ExpressionOp, *, _expect_kind: type[ExpressionKind] | None = None, **kwargs
) -> Expression:
    if _expect_kind is not None and op.kind != _expect_kind:
        raise TypeError(f"expected {_expect_kind}, got {op} ({op.kind})")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in Expression.__properties__}
    return Expression(op=op, **kwargs)


C = functools.partial(E, _expect_t=ExpressionKind.CONDITIONAL)
S = functools.partial(E, _expect_t=ExpressionKind.SORT)
A = functools.partial(E, _expect_t=ExpressionKind.AGGREGATION)

SCORE_KEY = "_score"  # for ranking
METATYPE_KEY = "_type"


#
# Field query ops
# :ExpressionSupport
#


class UnsupportedExpressionError(ValueError):
    def __init__(self, type: "TypeInfo", thing: Any):
        super().__init__(f"{type!r} does not support {thing!r}")


def _check_type_supports(type: "TypeInfo", op: ExpressionOp):
    """Asserts that the field supports the given expression operator."""
    if op in SortOp:
        if type.primitive_type in (
            PrimitiveType.DATETIME,
            PrimitiveType.FLOAT32,
            PrimitiveType.INT32,
            PrimitiveType.INT64,
        ):
            return
    else:
        if op in _ExprOps.COND_EXISTENCE:
            return True
        elif type.primitive_type is not None and op in SUPPORTED_PRIMITIVE_OPS.get(
            type.primitive_type, _EMPTY_SET
        ):
            return True
        elif type.bench_type is not None and op in SUPPORTED_NODE_OPS:
            return True
    raise UnsupportedExpressionError(type, op)


# should probably also have expression support per query engine?
_ExprOps = ExpressionOps  # alias
SUPPORTED_PRIMITIVE_OPS: dict[PrimitiveType, set[ConditionalOp]] = {
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
SUPPORTED_NODE_OPS = _ExprOps.COND_RANGE | _ExprOps.COND_EXACT
_EMPTY_SET = set()

NodeT = TypeVar("NodeT", bound="Node")
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union["Field", "Property", Any]
NodeTypeOrClass = Union[NodeType, type[Node]]


def _require_expression_op(op: ExpressionOp):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(self: "_TypeQueryBuilder", *args, **kwargs):
            _check_type_supports(self.as_type_info, op)
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


class _TypeQueryBuilder:
    """
    Base for field-like  on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def as_type_info(self) -> "TypeInfo":
        raise NotImplementedError(f"{self!r} does not implement type")

    #
    # Conditional
    #

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

    def __eq__(self, other):  # type: ignore
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__eq__(self, other)  # imitate Field equality
        else:
            return self.equals(other)

    def __ne__(self, other):  # type: ignore
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        else:
            return self.not_equal(other)

    __gt__ = greater_than
    __ge__ = greater_than_or_equals
    __lt__ = less_than
    __le__ = less_than_or_equals

    # string

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

    #
    # Sort
    #

    @_require_expression_op(SortOp.ASCENDING)
    def asc(self: Any) -> "Expression":
        return _to_sort(SortOp.ASCENDING, self)

    ascending = asc

    @_require_expression_op(SortOp.DESCENDING)
    def desc(self: Any) -> "Expression":
        return _to_sort(SortOp.DESCENDING, self)

    descending = desc

    #
    # Aggregation
    #

    ...
