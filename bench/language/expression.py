# ruff: noqa: RUF012

import datetime
import functools
from collections.abc import Collection
from typing import TYPE_CHECKING, Any, Optional, Sequence, TypeVar, Union, cast
from uuid import UUID

import regex
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp

from bench.language.const import (
    IN_BENCH_NODE_TYPES,
    AggregationOp,
    ConditionalOp,
    EnumType,
    ExpressionKind,
    ExpressionOp,
    NodeType,
    PrimitiveType,
    SortMode,
    SortOp,
    StructType,
    TypeKind,
    enum_,
)
from bench.language.node import (
    Node,
    NodeReference,
    NodeReferenceBase,
    Property,
    PropertyReference,
    Struct,
    struct_,
)
from bench.language.property import p_regular, p_value_packed, p_value_runtime
from bench.language.value import unpack_proto_json
from bench.proto.wire import AnyNodeData, FileReferenceData, NodeReferenceData, SecretReferenceData
from bench.utils.func import IdEnum
from bench.utils.string import Casing, to_casing

if TYPE_CHECKING:
    from bench.language import Block, Code, Field, TypeInfoBase

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
    ConditionalOp.MATCHES_REGEX: "$re=",
    ConditionalOp.STARTS_WITH: "^=",
    ConditionalOp.ENDS_WITH: "$=",
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


@enum_(EnumType.SELECTION_KIND)
class SelectionType(IdEnum):
    LIST = 2


@struct_(StructType.SELECTION)
class Selection(Struct):
    """A selection of Nodes."""

    type: SelectionType = p_regular(30, require=True)

    nodes: list[Node] | None = p_regular(
        40, require=False, array=True, references=IN_BENCH_NODE_TYPES.tuple
    )
    fields: list["Field"] | None = p_regular(
        41, require=False, array=True, references=NodeType.FIELD
    )


property_ = property


@struct_(StructType.EXPRESSION)
class Expression(Struct):
    """
    An expression like a value, function, comparison or such.
    """

    op: ExpressionOp = p_regular(30, require=True)
    property: Optional[Property] = p_regular(
        31, require=False, default=None, array=False, struct=StructType.PROPERTY_REFERENCE
    )
    field: Optional["Field"] = p_regular(
        32, require=False, default=None, array=False, references=NodeType.FIELD
    )
    block: Optional["Block"] = p_regular(
        33,
        require=False,
        default=None,
        array=False,
        references=NodeType.FIELD,
        description="The Block the Field refers to (if ambiguous).",
    )
    if TYPE_CHECKING:
        property_ptr: Optional[PropertyReference] = None
        field_ptr: Optional[NodeReference] = None
        block_ptr: Optional[NodeReference] = None

    # content
    clauses: list["Expression"] | None = p_regular(40, array=True, struct=StructType.EXPRESSION)
    value_packed: Any = p_value_packed(46)
    value: Any = p_value_runtime(46, typ=lambda self: cast(Expression, self).value_type)
    sort_mode: Optional[SortMode] = p_regular(48, default=None)
    tolerance: Optional[float] = p_regular(49, default=None)

    def __content_str__(self):
        if self.op in ExpressionOps.COND_COMPOUND:
            inner = f" {_CONDITIONAL_OP_SIGN[self.op]} ".join(str(q) for q in self.clauses or ())
            return f"({inner})"
        elif (
            self.op in ExpressionOps.COND_EXACT
            or self.op in ExpressionOps.COND_RANGE
            or self.op in ExpressionOps.COND_STRING
        ):
            code_name = self.target.code_name if self.target is not None else "???"
            try:
                value_str = str(self.value)
            except Exception:  # don't crash if value is invalid
                value_str = "???"
            if len(value_str) > 60:
                value_str = f"{value_str[:48]}...{value_str[-12:]}"
            return f"{code_name}{_CONDITIONAL_OP_SIGN[self.op]}{value_str}"
        elif self.op in ExpressionOps.COND_EXISTENCE:
            code_name = self.target.code_name if self.target is not None else "???"
            return f"{code_name}{_CONDITIONAL_OP_SIGN[self.op]}"
        elif self.op in ExpressionOps.SORT:
            code_name = self.target.code_name if self.target is not None else "???"
            return f"{'-' if self.op == SortOp.DESCENDING else ''}{code_name}"
        return to_casing(self.op.name, Casing.CAMEL)

    @property_
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_OP[self.op]

    @property_
    def value_type(self) -> "TypeInfoBase | None":
        prop = self.property
        field = self.field
        if prop is not None:
            typ = prop.type_info
        elif field is not None:
            typ = field.type_info
        else:
            return None
        # wrap as list if needed (NOTE :Performance)
        if not typ.is_list and (self.op == ConditionalOp.IN or self.op == ConditionalOp.NOT_IN):
            typ = typ.clone()
            typ.is_list = True
        return typ

    def __bool__(self):
        # safe-guard to ensure expressions are not used directly in boolean context
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __invert__(self):
        if self.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"cannot invert {self!r}, expected Conditional, got {self.kind}")
        elif self.op == ConditionalOp.NOT:
            assert (
                self.clauses is not None and len(self.clauses) == 1
            ), f"expected 1 clause, got {self}"
            return self.clauses[0]
        elif self.op == ConditionalOp.EXISTS:
            return C(ConditionalOp.NOT_EXISTS, property=self.property)
        elif self.op == ConditionalOp.NOT_EXISTS:
            return C(ConditionalOp.EXISTS, property=self.property)
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

    @property_
    def target(self) -> Union["Field", Property, None]:
        prop = self.property
        if prop is not None:
            return prop
        field = self.field
        if field is not None:
            return field
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
    # conditionals
    COND_COMPOUND = {ConditionalOp.NOT, ConditionalOp.AND, ConditionalOp.OR}
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
    COND_STRING = {ConditionalOp.MATCHES_REGEX, ConditionalOp.STARTS_WITH, ConditionalOp.ENDS_WITH}
    COND_SCORED = {ConditionalOp.NEAR, *COND_STRING}
    # aggregations
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
    # sorts
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


@struct_(StructType.AGGREGATION)
class Aggregation(Struct):
    """The result of an aggregation expression."""

    op: AggregationOp = p_regular(30, require=True)
    exists: Optional[bool] = p_regular(31, default=None)
    count: Optional[int] = p_regular(32, default=None)
    scalar: Optional[float] = p_regular(33, default=None)


def coerce_conditional(
    *,
    node_cls: type[Node],
    block: Optional["Block"],
    expr: Optional[Expression] = None,
    kwargs: Optional[dict[str, Any]] = None,
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
        # parse out django str if present
        if "__" in arg:
            key, op_str = arg.split("__", 1)
            op = CONDITIONAL_OP_BY_DJANGO_STR.get(op_str)
            if op is None:
                raise TypeError(
                    f"unsupported conditional operator {op_str!r} (allowed: {list(CONDITIONAL_OP_BY_DJANGO_STR)})"
                )
        else:
            key, op = arg, ConditionalOp.EQUALS

        # map key into field/property
        target: Field | Property | None = None
        if prop := node_cls.__properties__.get(key):
            target = prop
        elif block is not None and (field := block.fields.get(key)):
            target = field
        if target is None:
            raise TypeError(f"{node_cls!r} has no attribute {key!r} in {block!r}")
        # coerce None to NOT_EXISTS/EXISTS
        if value is None:
            if op == ConditionalOp.EQUALS:
                op = ConditionalOp.NOT_EXISTS
            elif op == ConditionalOp.NOT_EQUALS:
                op = ConditionalOp.EXISTS
        _check_type_supports(target.type_info, op)
        clauses.append(
            Expression(
                op=op,
                property=target if type(target) is Property else None,
                field=target if not isinstance(target, Property) else None,
                value=value,
            )
        )
    if not clauses:
        return None
    else:
        return Expression.and_if_set(*clauses)


def coerce_sort(
    *,
    node_cls: type[Node],
    block: Optional["Block"],
    expr: "Sequence[Expression | str | Field | Property] | Expression | str | Field | Property | None",
    args: Sequence[str],
) -> Optional[list[Expression]]:
    """
    Coerce a sort expression from either the given expression or args.
    Strings are looked up as field names/identifiers.
    Like in Django, prefix with "-" for descending.
    """
    from bench.language import Field, Property

    # coerce into list[Expression | str]
    if expr is None:
        if args is None:
            return None
        expr = list(args)
    elif isinstance(expr, (str, Field, Property)) or (
        isinstance(expr, Expression) and expr.kind == ExpressionKind.SORT
    ):
        expr = [expr]
    if not isinstance(expr, (list, tuple)):
        raise TypeError(f"expected sort to be a list or tuple, got {expr}")
    if args:
        expr = cast("Sequence[Expression | str | Field | Property]", (*expr, *args))
    expr = cast("Sequence[Expression | str | Field | Property]", expr)

    # map into sorts
    coerced = []
    for item in expr:
        if isinstance(item, str):
            # -field or field
            if item.startswith("-"):
                op = SortOp.DESCENDING
                field_key = item[1:]
            else:
                op = SortOp.ASCENDING
                field_key = item

            # map key into field/property
            target = None
            if prop := node_cls.__properties__.get(field_key):
                target = prop
            elif block is not None and (field := block.fields.get(field_key)):
                target = field
            if target is None:
                raise AttributeError(f"{node_cls!r} has no attribute {item!r} in {block!r}")
            elif isinstance(target, Property):
                item = S(op, field=None, property=target)
            else:
                item = S(op, field=target, property=None)
            _check_type_supports(target.type_info, op)
        elif isinstance(item, Field):
            _check_type_supports(item, SortOp.ASCENDING)
            item = S(SortOp.ASCENDING, field=item)
        elif isinstance(item, Property):
            _check_type_supports(item.type_info, SortOp.ASCENDING)
            item = S(SortOp.ASCENDING, property=item)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


def _lower_expression_value(typ: "TypeInfoBase", value: Any) -> Any:
    """
    'Lowers' the given value to enable direct comparison.
    This is related to the lower_conditional pass we do in the sql engine backend,
     but we also down the value into its data format.
    """
    # auto lower collections
    if isinstance(value, Sequence) and type(value) is not str:
        return [_lower_expression_value(typ, v) for v in value]

    # identity
    if isinstance(value, Node):
        value = value.ck or value.id
    if isinstance(
        value, (NodeReferenceBase, NodeReferenceData, FileReferenceData, SecretReferenceData)
    ):
        value = value.ck or value.id
    if isinstance(value, UUID):
        value = str(value)

    # time
    if typ.primitive_type == PrimitiveType.DATETIME:
        if isinstance(value, Timestamp):
            value = value.ToNanoseconds()
        elif isinstance(value, datetime.datetime):
            value = value.timestamp() * 1e9
        else:
            raise ValueError(f"unexpected value type: {type(value).__name__}")
    elif typ.primitive_type == PrimitiveType.INTERVAL:
        if isinstance(value, Duration):
            value = value.seconds + value.nanos / 1e9
        elif isinstance(value, datetime.timedelta):
            value = value.total_seconds()
        else:
            raise ValueError(f"unexpected value type: {type(value).__name__}")

    return value


def _get_node_prop_expression_value(node: Node | AnyNodeData, prop: Property) -> Any:
    """Gets the value of a property from a Node / packed node data."""
    if isinstance(node, Node):
        value = getattr(node, prop.code_name)
    else:
        if prop.is_optional_scalar and not node.HasField(prop.name):
            value = None
        else:
            value = getattr(node, prop.name)
    return _lower_expression_value(prop.type_info, value)


def _get_node_field_expression_value(node: Node | AnyNodeData, field: "Field") -> Any:
    if isinstance(node, Node):
        value = getattr(node, field.name)
    else:
        value_packed = getattr(node, "value_packed")
        value_packed = unpack_proto_json(value_packed)
        value = value_packed.get(field.name) if type(value_packed) is dict else None
    return _lower_expression_value(field, value)


def evaluate_conditional(cond: Expression, node: Node | AnyNodeData) -> bool:
    """Evaluates the conditional expression on a Node / packed node data."""
    assert cond.kind == ExpressionKind.CONDITIONAL, f"expected Conditional, got {cond!r}"
    # logical
    if cond.op in ExpressionOps.COND_COMPOUND:
        if not cond.clauses:
            return True  # empty compound is True :EmptyCompoundConditional
        if cond.op == ConditionalOp.NOT:
            return not evaluate_conditional(cond.clauses[0], node)
        elif cond.op == ConditionalOp.AND:
            return all(evaluate_conditional(clause, node) for clause in cond.clauses)
        elif cond.op == ConditionalOp.OR:
            return any(evaluate_conditional(clause, node) for clause in cond.clauses)

    # some property-based comparison
    prop = cond.property
    assert prop is not None, f"expected Conditional with property, got {cond!r}"
    if prop.reference_wired_ptr is not None:
        prop = prop.reference_wired_ptr
    node_value = _get_node_prop_expression_value(node, prop)
    cond_value = cond.value
    cond_value = _lower_expression_value(prop.type_info, cond_value)
    # basic comparison
    if cond.op == ConditionalOp.EQUALS:
        return node_value == cond_value
    elif cond.op == ConditionalOp.NOT_EQUALS:
        return node_value != cond_value
    elif cond.op == ConditionalOp.GREATER_THAN:
        return node_value > cond_value
    elif cond.op == ConditionalOp.GREATER_THAN_OR_EQUALS:
        return node_value >= cond_value
    elif cond.op == ConditionalOp.LESS_THAN:
        return node_value < cond_value
    elif cond.op == ConditionalOp.LESS_THAN_OR_EQUALS:
        return node_value <= cond_value
    # string comparison
    elif cond.op == ConditionalOp.MATCHES_REGEX:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and regex.match(cond_value, node_value) is not None
    elif cond.op == ConditionalOp.STARTS_WITH:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and node_value.startswith(cond_value)
    elif cond.op == ConditionalOp.ENDS_WITH:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and node_value.endswith(cond_value)
    # containment
    elif cond.op == ConditionalOp.CONTAINS:
        return isinstance(node_value, Collection) and cond_value in node_value
    elif cond.op == ConditionalOp.NOT_CONTAINS:
        return isinstance(node_value, Collection) and cond_value not in node_value
    elif cond.op == ConditionalOp.IN:
        assert isinstance(cond_value, Collection), f"expected Collection value, got {cond_value!r}"
        return node_value in cond_value
    elif cond.op == ConditionalOp.NOT_IN:
        assert isinstance(cond_value, Collection), f"expected Collection value, got {cond_value!r}"
        return node_value not in cond_value
    # existence
    elif cond.op == ConditionalOp.EXISTS:
        return node_value is not None
    elif cond.op == ConditionalOp.NOT_EXISTS:
        return node_value is None
    # vector
    elif cond.op == ConditionalOp.NEAR:
        raise RuntimeError(f"cannot evaluate vector expression {cond!r}")
    # unknown
    else:
        raise RuntimeError(f"unsupported conditional {cond!r}")


def _compare_sort_key(
    sorts: Sequence[Expression], a: Node | AnyNodeData, b: Node | AnyNodeData
) -> int:
    """Compares the two values based on the given sort expressions."""
    for sort in sorts:
        prop = sort.property
        typ = None
        if prop is not None:
            if prop.reference_wired_ptr is not None:
                prop = prop.reference_wired_ptr
            typ = prop.type_info
            a_value = _get_node_prop_expression_value(a, prop)
            b_value = _get_node_prop_expression_value(b, prop)
        else:
            field = sort.field
            if field is None:
                continue  # invalid sort
            typ = field
            a_value = _get_node_field_expression_value(a, field)
            b_value = _get_node_field_expression_value(b, field)
        if a_value == b_value:
            continue
        if typ is not None and typ.primitive_type is not None:
            cmp = -1 if a_value < b_value else 1
            if sort.op == SortOp.DESCENDING:
                cmp = -cmp
            return cmp
        else:
            continue  # invalid sort
    return 0


def apply_sort[T: AnyNodeData | Node](sorts: Sequence[Expression], values: list[T]) -> list[T]:
    """Sorts the values in place for the given expression"""
    if not sorts:
        return values  # nothing to do
    assert all(sort.kind == ExpressionKind.SORT for sort in sorts), f"expected Sorts, got {sorts!r}"
    values.sort(key=functools.cmp_to_key(lambda a, b: _compare_sort_key(sorts, a, b)))
    return values


# single-letter convenience constructors
def E(  # noqa: N802
    op: ExpressionOp, *, _expect_kind: type[ExpressionKind] | None = None, **kwargs
) -> Expression:
    if _expect_kind is not None and op.kind != _expect_kind:
        raise TypeError(f"expected {_expect_kind}, got {op} ({op.kind})")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in Expression.__properties__}
    return Expression(op=op, **kwargs)


C = functools.partial(E, _expect_t=ExpressionKind.CONDITIONAL)
S = functools.partial(E, _expect_t=ExpressionKind.SORT)
A = functools.partial(E, _expect_t=ExpressionKind.AGGREGATION)


#
# Type query ops
# :ExpressionSupport
#


class UnsupportedExpressionError(ValueError):
    def __init__(self, type: "TypeInfoBase", thing: Any):
        super().__init__(f"{type!r} does not support {thing!r}")


def _check_type_supports(typ: "TypeInfoBase", op: ExpressionOp):
    """Asserts that the field supports the given expression operator."""
    if op in SortOp:
        if typ.primitive_type is not None and (
            typ.primitive_type.is_numeric
            or typ.primitive_type
            in (PrimitiveType.STRING, PrimitiveType.DATETIME, PrimitiveType.INTERVAL)
        ):
            return
    else:
        if (
            op in _ExprOps.COND_EXISTENCE
            or (
                typ.primitive_type is not None
                and op in SUPPORTED_PRIMITIVE_OPS.get(typ.primitive_type, _EMPTY_SET)
            )
            or (
                (typ.kind == TypeKind.NODE or typ.kind == TypeKind.BASED_NODE)
                and op in SUPPORTED_NODE_OPS
            )
        ):
            return
    raise UnsupportedExpressionError(typ, op)


# should probably also have expression support per query engine?
_ExprOps = ExpressionOps  # alias
SUPPORTED_PRIMITIVE_OPS: dict[PrimitiveType, set[ConditionalOp]] = {
    # cumulative supported query ops by type
    PrimitiveType.UUID: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INT16: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INT32: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INT64: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.DECIMAL: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.FLOAT32: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.FLOAT64: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.BOOLEAN: _ExprOps.COND_EXACT,
    PrimitiveType.DATETIME: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.INTERVAL: _ExprOps.COND_RANGE | _ExprOps.COND_EXACT,
    PrimitiveType.VECTOR: _ExprOps.COND_VECTOR,
    PrimitiveType.STRING: _ExprOps.COND_EXACT | _ExprOps.COND_RANGE | _ExprOps.COND_STRING,
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
            _check_type_supports(self.type_info, op)
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
    def type_info(self) -> "TypeInfoBase":
        raise NotImplementedError(f"{self!r} does not implement type")

    #
    # Conditional
    #

    # comparison

    @_require_expression_op(ConditionalOp.EQUALS)
    def is_equal(self: Any, value: Any) -> "Expression":
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
            return self.is_equal(other)

    def __ne__(self, other):  # type: ignore
        if isinstance(self, Node) and isinstance(other, Node):
            return Node.__ne__(self, other)
        else:
            return self.not_equal(other)

    __gt__ = greater_than
    __ge__ = greater_than_or_equals
    __lt__ = less_than
    __le__ = less_than_or_equals
    eq = is_equal
    neq = not_equal
    lt = less_than
    lte = less_than_or_equals
    gt = greater_than
    gte = greater_than_or_equals

    # string

    @_require_expression_op(ConditionalOp.MATCHES_REGEX)
    def matches_regex(self: Any, value: str | regex.Pattern) -> "Expression":
        if isinstance(value, regex.Pattern):
            value = value.pattern
        return _to_conditional(ConditionalOp.MATCHES_REGEX, self, value=value)

    @_require_expression_op(ConditionalOp.STARTS_WITH)
    def starts_with(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.STARTS_WITH, self, value=value)

    startswith = starts_with

    @_require_expression_op(ConditionalOp.ENDS_WITH)
    def ends_with(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalOp.ENDS_WITH, self, value=value)

    endswith = ends_with

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


@struct_(StructType.COMPUTED_VALUE)
class ComputedValue(Struct):
    """A value computed from an expression or code snippet."""

    expression: "Expression | None" = p_regular(
        35, require=False, array=False, struct=StructType.EXPRESSION
    )
    code: Optional["Code"] = p_regular(36, require=False, array=False, struct=StructType.CODE)
