# ruff: noqa: RUF012

import datetime
import functools
from collections.abc import Collection
from typing import TYPE_CHECKING, Any, Optional, Sequence, TypeVar, Union, cast
from uuid import UUID

import regex
from google.protobuf.duration_pb2 import Duration
from google.protobuf.timestamp_pb2 import Timestamp

from bench.pb2 import AnyNodeData, NodeReferenceData
from bench.utils.func import IdEnum
from bench.utils.string import Casing, to_casing

from .const import (
    AggregationType,
    ConditionalType,
    EnumType,
    ExpressionKind,
    ExpressionType,
    FieldType,
    NodeType,
    ObjectKind,
    PrimitiveType,
    SortMode,
    SortType,
    StructType,
    enum_,
)
from .node import (
    Node,
    NodeReference,
    Property,
    PropertyReference,
    Struct,
    struct_,
)
from .path import PathIn, to_path
from .property import p_regular, p_value_packed, p_value_runtime
from .validation import NAME_CONSTRAINT
from .value import unpack_proto_json

if TYPE_CHECKING:
    from bench.language import Block, Code, Field, Path, Text, TypeBase, TypeInfo

# pyright: reportIncompatibleVariableOverride=false

#
# Expression language. Primarily for package, search and storage (database).
#


_CONDITIONAL_OP_SIGN: dict[ConditionalType, str] = {
    # logical
    ConditionalType.NOT: "~",
    ConditionalType.AND: "&",
    ConditionalType.OR: "|",
    # comparison
    ConditionalType.EQUALS: "==",
    ConditionalType.NOT_EQUALS: "!=",
    ConditionalType.GREATER_THAN: ">",
    ConditionalType.GREATER_THAN_OR_EQUALS: ">=",
    ConditionalType.LESS_THAN: "<",
    ConditionalType.LESS_THAN_OR_EQUALS: "<=",
    # string comparison
    ConditionalType.MATCHES: "~=",
    ConditionalType.STARTS_WITH: "^=",
    ConditionalType.ENDS_WITH: "=$",
    ConditionalType.MATCHES_REGEX: "$re=",
    # containment
    ConditionalType.CONTAINS: "∋",
    ConditionalType.NOT_CONTAINS: "!∋",
    ConditionalType.IN: "∈",
    ConditionalType.NOT_IN: "!∈",
    # existence
    ConditionalType.EXISTS: "!",
    ConditionalType.NOT_EXISTS: "!!",
    # vector
    ConditionalType.NEAR: "~=",
}


@struct_(StructType.SELECTION)
class Selection(Struct):
    """A selection of Nodes."""

    nodes: list[Node] = p_regular(40, require=False, array=True, references="any")
    fields: list["Field"] = p_regular(41, require=False, array=True, references=NodeType.FIELD)
    properties: list[Property] = p_regular(
        42, require=False, array=True, struct=StructType.PROPERTY_REFERENCE
    )


property_ = property


@struct_(StructType.EXPRESSION)
class Expression(Struct):
    """
    An Expression like a value, function, comparison or such.
    """

    type: ExpressionType = p_regular(30, require=True)
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
    value: Any = p_value_runtime(
        46, kind=ObjectKind.MEMBER, typ=lambda self: cast(Expression, self).value_type
    )
    sort_mode: Optional[SortMode] = p_regular(48, default=None)
    tolerance: Optional[float] = p_regular(49, default=None)

    def __content_str__(self):
        if self.type in ExpressionTypes.COMPOUND:
            inner = f" {_CONDITIONAL_OP_SIGN[self.type]} ".join(str(q) for q in self.clauses or ())
            return f"({inner})"
        elif (
            self.type in ExpressionTypes.EXACT
            or self.type in ExpressionTypes.RANGE
            or self.type in ExpressionTypes.STRING
        ):
            code_name = self.target.code_name if self.target is not None else "???"
            try:
                value_str = str(self.value)
            except Exception:  # don't crash if value is invalid
                value_str = "???"
            if len(value_str) > 60:
                value_str = f"{value_str[:48]}...{value_str[-12:]}"
            return f"{code_name}{_CONDITIONAL_OP_SIGN[self.type]}{value_str}"
        elif self.type in ExpressionTypes.EXISTENCE:
            code_name = self.target.code_name if self.target is not None else "???"
            return f"{code_name}{_CONDITIONAL_OP_SIGN[self.type]}"
        elif self.type in ExpressionTypes.SORT:
            code_name = self.target.code_name if self.target is not None else "???"
            return f"{'-' if self.type == SortType.DESCENDING else ''}{code_name}"
        return to_casing(self.type.name, Casing.CAMEL)

    @property_
    def kind(self) -> ExpressionKind:
        return EXPRESSION_KIND_BY_OP[self.type]

    @property_
    def value_type(self) -> "TypeBase | None":
        prop = self.property
        field = self.field
        if prop is not None:
            typ = prop.type_info
        elif field is not None:
            typ = field.type_info
        else:
            return None
        # wrap as list if needed (NOTE :Performance)
        if not typ.is_list and (
            self.type == ConditionalType.IN or self.type == ConditionalType.NOT_IN
        ):
            typ = typ.clone()
            typ.is_list = True
        return typ

    def __bool__(self):
        # safe-guard to ensure expressions are not used directly in boolean context
        raise TypeError(f"cannot evaluate {self!r} directly (did you mean to compare a property?)")

    def __invert__(self):
        if self.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"cannot invert {self!r}, expected Conditional, got {self.kind}")
        elif self.type == ConditionalType.NOT:
            assert (
                self.clauses is not None and len(self.clauses) == 1
            ), f"expected 1 clause, got {self}"
            return self.clauses[0]
        elif self.type == ConditionalType.EXISTS:
            return C(ConditionalType.NOT_EXISTS, property=self.property)
        elif self.type == ConditionalType.NOT_EXISTS:
            return C(ConditionalType.EXISTS, property=self.property)
        else:
            return C(ConditionalType.NOT, clauses=[self])

    def __and__(self, other: "Expression"):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for &: {type(self)} and {type(other)}")
        if self.type == ConditionalType.AND:
            if isinstance(other, Expression) and other.type == ConditionalType.AND:
                return C(
                    ConditionalType.AND, clauses=[*(self.clauses or ()), *(other.clauses or ())]
                )
            else:
                return C(ConditionalType.AND, clauses=[*(self.clauses or ()), other])
        else:
            return C(ConditionalType.AND, clauses=[self, other])

    def __or__(self, other: "Expression"):
        if not isinstance(other, Expression) or other.kind != ExpressionKind.CONDITIONAL:
            raise TypeError(f"unsupported operand type(s) for |: {type(self)} and {type(other)}")
        if self.type == ConditionalType.OR:
            if isinstance(other, Expression) and other.type == ConditionalType.OR:
                return C(
                    ConditionalType.OR, clauses=[*(self.clauses or ()), *(other.clauses or ())]
                )
            else:
                return C(ConditionalType.OR, clauses=[*(self.clauses or ()), other])
        else:
            return C(ConditionalType.OR, clauses=[self, other])

    @property_
    def target(self) -> Union["Field", Property, None]:
        prop = self.property
        if prop is not None:
            return prop
        field = self.field
        if field is not None:
            return field
        return None

    def _collect_ops(self) -> set[ExpressionType]:
        """Collect all ops in this expression and its clauses (recursively)."""
        ops = {self.type}
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


class ExpressionTypes:  # :ExpressionOps
    # conditionals
    COMPOUND = {ConditionalType.NOT, ConditionalType.AND, ConditionalType.OR}
    EXACT = {
        ConditionalType.EQUALS,
        ConditionalType.NOT_EQUALS,
        ConditionalType.IN,
        ConditionalType.NOT_IN,
    }
    RANGE = {
        ConditionalType.GREATER_THAN,
        ConditionalType.GREATER_THAN_OR_EQUALS,
        ConditionalType.LESS_THAN,
        ConditionalType.LESS_THAN_OR_EQUALS,
    }
    SET = {ConditionalType.CONTAINS, ConditionalType.NOT_CONTAINS}
    EXISTENCE = {ConditionalType.EXISTS, ConditionalType.NOT_EXISTS}
    VECTOR = {ConditionalType.NEAR}
    STRING = {
        ConditionalType.MATCHES,
        ConditionalType.STARTS_WITH,
        ConditionalType.ENDS_WITH,
        ConditionalType.MATCHES_REGEX,
    }
    SCORED = {ConditionalType.NEAR, *STRING}
    # aggregations
    BOOLEAN = {AggregationType.EXISTENCE}
    SCALAR = {
        AggregationType.COUNT,
        AggregationType.SUM,
        AggregationType.AVERAGE,
        AggregationType.MIN,
        AggregationType.MAX,
        AggregationType.MEDIAN,
    }
    BUCKET = {AggregationType.HISTOGRAM}
    # sorts
    SORT = {SortType.ASCENDING, SortType.DESCENDING}


EXPRESSION_OPS_BY_KIND: dict[ExpressionKind, set[ExpressionType]] = {
    ExpressionKind.CONDITIONAL: {*ConditionalType},
    ExpressionKind.AGGREGATION: {*AggregationType},
    ExpressionKind.SORT: {*SortType},
}
EXPRESSION_KIND_BY_OP: dict[ExpressionType, ExpressionKind] = {
    op: kind for kind, ops in EXPRESSION_OPS_BY_KIND.items() for op in ops
}

CONDITIONAL_OP_BY_DJANGO_SIGN: dict[str, ConditionalType] = {
    "eq": ConditionalType.EQUALS,
    "ne": ConditionalType.NOT_EQUALS,
    "gt": ConditionalType.GREATER_THAN,
    "gte": ConditionalType.GREATER_THAN_OR_EQUALS,
    "lt": ConditionalType.LESS_THAN,
    "lte": ConditionalType.LESS_THAN_OR_EQUALS,
    "in": ConditionalType.IN,
    "nin": ConditionalType.NOT_IN,
}
DJANGO_SIGN_BY_CONDITIONAL_OP: dict[ConditionalType, str] = {
    v: k for k, v in CONDITIONAL_OP_BY_DJANGO_SIGN.items()
}


@struct_(StructType.AGGREGATION_RESULT)
class AggregationResult(Struct):
    """The result of an aggregation expression."""

    op: AggregationType = p_regular(30, require=True)
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
            op = CONDITIONAL_OP_BY_DJANGO_SIGN.get(op_str)
            if op is None:
                raise TypeError(
                    f"unsupported conditional operator {op_str!r} (allowed: {list(CONDITIONAL_OP_BY_DJANGO_SIGN)})"
                )
        else:
            key, op = arg, ConditionalType.EQUALS

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
            if op == ConditionalType.EQUALS:
                op = ConditionalType.NOT_EXISTS
            elif op == ConditionalType.NOT_EQUALS:
                op = ConditionalType.EXISTS
        _check_type_supports(target.type_info, op)
        clauses.append(
            Expression(
                type=op,
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
                op = SortType.DESCENDING
                field_key = item[1:]
            else:
                op = SortType.ASCENDING
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
            _check_type_supports(item, SortType.ASCENDING)
            item = S(SortType.ASCENDING, field=item)
        elif isinstance(item, Property):
            _check_type_supports(item.type_info, SortType.ASCENDING)
            item = S(SortType.ASCENDING, property=item)
        if not isinstance(item, Expression) or item.kind != ExpressionKind.SORT:
            raise TypeError(f"expected Sort or str, got {item!r}")
        coerced.append(item)
    if not coerced:
        return None
    return coerced


def _lower_expression_value(typ: "TypeBase", value: Any) -> Any:
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
    if isinstance(value, (NodeReference, NodeReferenceData)):
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
    elif typ.primitive_type == PrimitiveType.DURATION:
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
    value = _lower_expression_value(prop.type_info, value)
    return value


NODE_PROPERTY_KEY_BY_FIELD_TYPE: dict[FieldType, str] = {
    FieldType.INPUT: "inputs_packed",
    FieldType.OUTPUT: "outputs_packed",
    FieldType.MEMBER: "value_packed",
    FieldType.VARIABLE: "variables_packed",
}


def _get_node_field_expression_value(node: Node | AnyNodeData, field: "Field") -> Any:
    if isinstance(node, Node):
        value = getattr(node, field.name)
    else:
        field_key = NODE_PROPERTY_KEY_BY_FIELD_TYPE.get(field.type)
        if field_key is None:
            return None
        value_packed = getattr(node, field_key, None)
        if value_packed is not None:
            value_packed = unpack_proto_json(value_packed)
            value = value_packed.get(field.name) if type(value_packed) is dict else None
        else:
            return None
    return _lower_expression_value(field, value)


def evaluate_conditional(cond: Expression, node: Node | AnyNodeData) -> bool:
    """Evaluates the conditional expression on a Node / packed node data."""
    assert cond.kind == ExpressionKind.CONDITIONAL, f"expected Conditional, got {cond!r}"
    # decompose compound expressions
    if cond.type in ExpressionTypes.COMPOUND:
        if not cond.clauses:
            return True  # empty compound is True :EmptyCompoundConditional
        if cond.type == ConditionalType.NOT:
            return not evaluate_conditional(cond.clauses[0], node)
        elif cond.type == ConditionalType.AND:
            return all(evaluate_conditional(clause, node) for clause in cond.clauses)
        elif cond.type == ConditionalType.OR:
            return any(evaluate_conditional(clause, node) for clause in cond.clauses)

    # get target and value
    prop = cond.property
    field = cond.field
    if prop is not None:
        if prop.reference_wired_ptr is not None:
            prop = prop.reference_wired_ptr
        typ = prop.type_info
        node_value = _get_node_prop_expression_value(node, prop)
    elif field is not None:
        typ = field
        node_value = _get_node_field_expression_value(node, field)
    else:
        raise RuntimeError(f"expected Conditional with property or field, got {cond!r}")

    cond_value = cond.value
    cond_value = _lower_expression_value(typ, cond_value)

    # basic comparison
    if cond.type == ConditionalType.EQUALS:
        return node_value == cond_value
    elif cond.type == ConditionalType.NOT_EQUALS:
        return node_value != cond_value
    elif cond.type == ConditionalType.GREATER_THAN:
        return node_value > cond_value
    elif cond.type == ConditionalType.GREATER_THAN_OR_EQUALS:
        return node_value >= cond_value
    elif cond.type == ConditionalType.LESS_THAN:
        return node_value < cond_value
    elif cond.type == ConditionalType.LESS_THAN_OR_EQUALS:
        return node_value <= cond_value
    # string comparison
    elif cond.type == ConditionalType.MATCHES:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and cond_value in node_value
    elif cond.type == ConditionalType.STARTS_WITH:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and node_value.startswith(cond_value)
    elif cond.type == ConditionalType.ENDS_WITH:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and node_value.endswith(cond_value)
    elif cond.type == ConditionalType.MATCHES_REGEX:
        assert isinstance(cond_value, str), f"expected str value, got {cond_value!r}"
        return node_value is not None and regex.match(cond_value, node_value) is not None
    # containment
    elif cond.type == ConditionalType.CONTAINS:
        return isinstance(node_value, Collection) and cond_value in node_value
    elif cond.type == ConditionalType.NOT_CONTAINS:
        return isinstance(node_value, Collection) and cond_value not in node_value
    elif cond.type == ConditionalType.IN:
        assert isinstance(cond_value, Collection), f"expected Collection value, got {cond_value!r}"
        return node_value in cond_value
    elif cond.type == ConditionalType.NOT_IN:
        assert isinstance(cond_value, Collection), f"expected Collection value, got {cond_value!r}"
        return node_value not in cond_value
    # existence
    elif cond.type == ConditionalType.EXISTS:
        return bool(node_value)
    elif cond.type == ConditionalType.NOT_EXISTS:
        return not bool(node_value)
    # vector
    elif cond.type == ConditionalType.NEAR:
        raise RuntimeError(f"cannot evaluate vector expression {cond!r}")
    # unknown
    else:
        raise RuntimeError(f"unsupported conditional {cond!r}")


def evaluate_sort(sorts: Sequence[Expression], a: Node | AnyNodeData, b: Node | AnyNodeData) -> int:
    """Compares the two values based on the given sort expressions."""
    for sort in sorts:
        prop = sort.property
        field = sort.field
        typ = None
        if prop is not None:
            if prop.reference_wired_ptr is not None:
                prop = prop.reference_wired_ptr
            typ = prop.type_info
            a_value = _get_node_prop_expression_value(a, prop)
            b_value = _get_node_prop_expression_value(b, prop)
        elif field is not None:
            typ = field
            a_value = _get_node_field_expression_value(a, field)
            b_value = _get_node_field_expression_value(b, field)
        else:
            continue  # invalid sort

        if a_value == b_value:
            continue
        if typ is not None and typ.primitive_type is not None:
            cmp = -1 if a_value < b_value else 1
            if sort.type == SortType.DESCENDING:
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
    values.sort(key=functools.cmp_to_key(lambda a, b: evaluate_sort(sorts, a, b)))
    return values


# single-letter convenience constructors
def E(  # noqa: N802
    op: ExpressionType, *, _expect_kind: type[ExpressionKind] | None = None, **kwargs
) -> Expression:
    if _expect_kind is not None and op.kind != _expect_kind:
        raise TypeError(f"expected {_expect_kind}, got {op} ({op.kind})")
    kwargs = {k: v for k, v in kwargs.items() if v is not None and k in Expression.__properties__}
    return Expression(type=op, **kwargs)


C = functools.partial(E, _expect_t=ExpressionKind.CONDITIONAL)
S = functools.partial(E, _expect_t=ExpressionKind.SORT)
A = functools.partial(E, _expect_t=ExpressionKind.AGGREGATION)


#
# Type query ops
# :ExpressionSupport
#


class UnsupportedExpressionError(ValueError):
    def __init__(self, type: "TypeBase", thing: Any):
        super().__init__(f"{type!r} does not support {thing!r}")


def type_supports_expression(typ: "TypeBase", op: ExpressionType) -> bool:
    """Checks if the given type supports the given expression operator."""
    if op.kind == ExpressionKind.SORT:
        if typ.primitive_type is not None and (
            typ.primitive_type.is_numeric
            or typ.primitive_type
            in (PrimitiveType.STRING, PrimitiveType.DATETIME, PrimitiveType.DURATION)
        ):
            return True
    elif op.kind == ExpressionKind.CONDITIONAL:
        if (
            op in ExpressionTypes.COMPOUND
            or op in ExpressionTypes.EXISTENCE
            or op in ExpressionTypes.EXACT
            or op in ExpressionTypes.SET
        ):
            return True
        if typ.primitive_type is not None:
            if typ.primitive_type.is_numeric and op in ExpressionTypes.RANGE:
                return True
            if typ.primitive_type == PrimitiveType.STRING and (
                op in ExpressionTypes.STRING or op in ExpressionTypes.RANGE
            ):
                return True
    elif op.kind == ExpressionKind.AGGREGATION:
        return True  # ???
    return False


def _check_type_supports(typ: "TypeBase", op: ExpressionType):
    """Asserts that the field supports the given expression operator."""
    if not type_supports_expression(typ, op):
        raise UnsupportedExpressionError(typ, op)


NodeT = TypeVar("NodeT", bound="Node")
NodeDataT = TypeVar("NodeDataT", bound=AnyNodeData)
FieldOrProperty = Union["Field", "Property", Any]
NodeTypeOrClass = Union[NodeType, type[Node]]


def _require_expression_op(op: ExpressionType):
    def decorator(func):
        @functools.wraps(func)
        def wrapper(self: "_IntoQuery", *args, **kwargs):
            _check_type_supports(self.type_info, op)
            return func(self, *args, **kwargs)

        return wrapper

    return decorator


def _to_conditional(op: ConditionalType, target: Union["Field", "Property"], value: Any = None):
    if isinstance(target, Property):
        return C(op, field=None, property=target, value=value)
    else:
        return C(op, field=target, property=None, value=value)


def _to_sort(op: SortType, target: Union["Field", "Property"]):
    if isinstance(target, Property):
        return S(op, field=None, property=target)
    else:
        return S(op, field=target, property=None)


class _IntoQuery:
    """
    Base for field-like  on a field-like class.
    We define this here to use it for Property and Field.
    """

    @property
    def type_info(self) -> "TypeBase":
        raise NotImplementedError(f"{self!r} does not implement type")

    #
    # Conditional
    #

    # comparison

    @_require_expression_op(ConditionalType.EQUALS)
    def is_equal(self: Any, value: Any) -> "Expression":
        if value is None:
            return self.not_exists()
        return _to_conditional(ConditionalType.EQUALS, self, value=value)

    @_require_expression_op(ConditionalType.NOT_EQUALS)
    def not_equal(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.NOT_EQUALS, self, value=value)

    @_require_expression_op(ConditionalType.GREATER_THAN)
    def greater_than(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.GREATER_THAN, self, value=value)

    @_require_expression_op(ConditionalType.GREATER_THAN_OR_EQUALS)
    def greater_than_or_equals(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.GREATER_THAN_OR_EQUALS, self, value=value)

    @_require_expression_op(ConditionalType.LESS_THAN)
    def less_than(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.LESS_THAN, self, value=value)

    @_require_expression_op(ConditionalType.LESS_THAN_OR_EQUALS)
    def less_than_or_equals(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.LESS_THAN_OR_EQUALS, self, value=value)

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

    @_require_expression_op(ConditionalType.MATCHES_REGEX)
    def matches_regex(self: Any, value: str | regex.Pattern) -> "Expression":
        if isinstance(value, regex.Pattern):
            value = value.pattern
        return _to_conditional(ConditionalType.MATCHES_REGEX, self, value=value)

    @_require_expression_op(ConditionalType.STARTS_WITH)
    def starts_with(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalType.STARTS_WITH, self, value=value)

    startswith = starts_with

    @_require_expression_op(ConditionalType.ENDS_WITH)
    def ends_with(self: Any, value: str) -> "Expression":
        return _to_conditional(ConditionalType.ENDS_WITH, self, value=value)

    endswith = ends_with

    # collections

    @_require_expression_op(ConditionalType.IN)
    def in_(self: Any, *values: list[Any]) -> "Expression":
        return _to_conditional(ConditionalType.IN, self, value=values)

    @_require_expression_op(ConditionalType.NOT_IN)
    def not_in(self: Any, *values: list[Any]) -> "Expression":
        return _to_conditional(ConditionalType.NOT_IN, self, value=values)

    @_require_expression_op(ConditionalType.CONTAINS)
    def contains(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.CONTAINS, self, value=value)

    @_require_expression_op(ConditionalType.NOT_CONTAINS)
    def not_contains(self: Any, value: Any) -> "Expression":
        return _to_conditional(ConditionalType.NOT_CONTAINS, self, value=value)

    # existence

    @_require_expression_op(ConditionalType.EXISTS)
    def exists(self: Any) -> "Expression":
        return _to_conditional(ConditionalType.EXISTS, self)

    @_require_expression_op(ConditionalType.NOT_EXISTS)
    def not_exists(self: Any) -> "Expression":
        return _to_conditional(ConditionalType.NOT_EXISTS, self)

    # knn

    @_require_expression_op(ConditionalType.NEAR)
    def near(self: Any, value: list[float]) -> "Expression":
        return _to_conditional(ConditionalType.NEAR, self, value=value)

    #
    # Sort
    #

    @_require_expression_op(SortType.ASCENDING)
    def asc(self: Any) -> "Expression":
        return _to_sort(SortType.ASCENDING, self)

    ascending = asc

    @_require_expression_op(SortType.DESCENDING)
    def desc(self: Any) -> "Expression":
        return _to_sort(SortType.DESCENDING, self)

    descending = desc

    #
    # Aggregation
    #

    ...


#
# Value / value mappings & transformations
#


@struct_(StructType.VALUE)
class Value(Struct):
    """A generic typed 'freeform' value."""

    name: str | None = p_regular(32, constraint=NAME_CONSTRAINT)
    text: Optional["Text"] = p_regular(
        33, default=None, require=False, array=False, struct=StructType.TEXT
    )
    value_type: "TypeInfo" = p_regular(35, struct=StructType.TYPE_INFO)
    value_packed: Any = p_value_packed(36)
    value: Any = p_value_runtime(
        36, kind=ObjectKind.MEMBER, typ=lambda self: cast("Value", self).value_type
    )


@enum_(EnumType.COMPUTED_VALUE_KIND)
class ComputedValueKind(IdEnum):
    REFERENCE = 1
    EXPRESSION = 2
    CODE = 3


ComputedSourceIn = Union[PathIn, Expression, "Code"]


@struct_(StructType.COMPUTED_VALUE)
class ComputedValue(Struct):
    """
    A computed value for a certain Property/Field on a Node.
    The value is computed and set according to the context (and may be re-computed later,
     for instance at the start of a Run or inside an instanced View in a UI).
    The property/field corresponds to the last element of the path (so we know the type).
    """

    # meta
    kind: ComputedValueKind = p_regular(30)
    # scope...?

    # path to set at
    target_path: "Path | None" = p_regular(41, struct=StructType.PATH)

    # value to set
    source_path: "Path | None" = p_regular(51, struct=StructType.PATH)
    source_expression: "Expression | None" = p_regular(52, struct=StructType.EXPRESSION)
    source_code: "Code | None" = p_regular(53, struct=StructType.CODE)

    # flags
    is_active: bool = p_regular(60, default=True)

    def __content_str__(self) -> str:
        return f"{self.target_path} <- {self.source_path}"

    @staticmethod
    def new(
        target: PathIn,
        *,
        source: ComputedSourceIn,
        is_active: bool = True,
    ) -> "ComputedValue":
        from .code import Code

        target = to_path(target)
        if isinstance(source, Code):
            return ComputedValue(
                kind=ComputedValueKind.CODE,
                target_path=target,
                source_code=source,
                is_active=is_active,
            )
        elif isinstance(source, Expression):
            return ComputedValue(
                kind=ComputedValueKind.EXPRESSION,
                target_path=target,
                source_expression=source,
                is_active=is_active,
            )
        else:
            source = to_path(source)
            return ComputedValue(
                kind=ComputedValueKind.REFERENCE,
                target_path=target,
                source_path=source,
                is_active=is_active,
            )


@struct_(StructType.OBJECT_MAPPING)
class ObjectMapping(Struct):
    mappings: list["ComputedValue"] = p_regular(
        40, require=True, array=True, struct=StructType.COMPUTED_VALUE
    )
