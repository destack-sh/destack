from collections.abc import Sequence
from typing import Any, assert_never

import structlog
from opentelemetry import trace

from destack.language import (
    Aggregation,
    AggregationType,
    Condition,
    ConditionalType,
    Expression,
    ExpressionType,
    Function,
    FunctionType,
    Node,
    NodeReference,
    PropertyReferenceType,
    ScalarType,
    Sort,
    TypeCardinality,
)
from destack.utils.uuid import UUID

tracer = trace.get_tracer(__name__)
logger = structlog.get_logger(__name__)

MAX_RECURSION_DEPTH = 100

NODE_PARENT_KEY = str(Node.property("parent").id)
NODE_ID_ID = Node.property("id").id
NODE_ID_KEY = str(Node.property("id").id)

NODE_REFERENCE_TYPE_KEY = str(NodeReference.property("type").id)
NODE_REFERENCE_ID_KEY = str(NodeReference.property("id").id)
NODE_REFERENCE_SPACE_ID_KEY = str(NodeReference.property("space_id").id)
NODE_REFERENCE_DEFINITION_ID_KEY = str(NodeReference.property("definition_id").id)


def evaluate_expression(value: dict[str, Any], expression: Expression) -> Any:
    """Evaluate an Expression against value data."""
    if expression.type == ExpressionType.LITERAL:
        assert expression.literal is not None, f"no literal for {expression!r}"
        if expression.literal.type.scalar_type == ScalarType.NODE_REFERENCE:
            return (
                expression.literal.value[NODE_REFERENCE_ID_KEY]
                if expression.literal.value is not None
                else None
            )
        else:
            return expression.literal.value
    elif expression.type == ExpressionType.ATTRIBUTE:
        assert expression.attribute is not None, f"no attribute for {expression!r}"
        attr = expression.attribute
        if attr.type == PropertyReferenceType.BUILTIN:
            prop = attr.resolve_or_error()
            if prop.scalar_type == ScalarType.NODE_REFERENCE:
                node_ptr_packed = value.get(str(prop.id))
                return (
                    node_ptr_packed[NODE_REFERENCE_ID_KEY] if node_ptr_packed is not None else None
                )
            else:
                return value.get(str(prop.id))
        else:
            raise NotImplementedError(f"unsupported attribute type: {attr.type}")
    elif expression.type == ExpressionType.CONDITION:
        assert expression.condition is not None, f"no condition for {expression!r}"
        return evaluate_condition(value, expression.condition)
    elif expression.type == ExpressionType.FUNCTION:
        assert expression.function is not None, f"no function for {expression!r}"
        return evaluate_function(value, expression.function)
    elif expression.type == ExpressionType.AGGREGATION:
        # Aggregations need to be handled at a higher level with multiple values
        raise RuntimeError(f"aggregation cannot be evaluated on single value: {expression!r}")
    else:
        assert_never(expression.type)


def evaluate_condition(value: dict[str, Any], condition: Condition) -> bool:
    """Evaluate a Condition against value data."""
    # logical
    if condition.type == ConditionalType.NOT:
        left_val = evaluate_expression(value, condition.left)
        return not bool(left_val)
    elif condition.type == ConditionalType.AND:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        if not left_val:
            return False
        right_val = evaluate_expression(value, condition.right)
        return bool(right_val)
    elif condition.type == ConditionalType.OR:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        if left_val:
            return True
        right_val = evaluate_expression(value, condition.right)
        return bool(right_val)
    # comparison
    elif condition.type == ConditionalType.EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val == right_val
    elif condition.type == ConditionalType.NOT_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val != right_val
    elif condition.type == ConditionalType.GREATER_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val > right_val
    elif condition.type == ConditionalType.GREATER_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val >= right_val
    elif condition.type == ConditionalType.LESS_THAN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val < right_val
    elif condition.type == ConditionalType.LESS_THAN_OR_EQUALS:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        return left_val <= right_val
    # string
    elif condition.type == ConditionalType.MATCHES:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        if left_val is None or right_val is None:
            return False
        # Simple pattern matching (could be enhanced with regex)
        return str(right_val) in str(left_val)
    elif condition.type == ConditionalType.STARTS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        if left_val is None or right_val is None:
            return False
        return str(left_val).startswith(str(right_val))
    elif condition.type == ConditionalType.ENDS_WITH:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        if left_val is None or right_val is None:
            return False
        return str(left_val).endswith(str(right_val))
    # collections
    elif condition.type == ConditionalType.IN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        if right_val is None:
            return False
        if isinstance(right_val, (list, tuple)):
            return left_val in right_val
        return left_val == right_val
    elif condition.type == ConditionalType.NOT_IN:
        assert condition.right is not None, f"no right for {condition!r}"
        left_val = evaluate_expression(value, condition.left)
        right_val = evaluate_expression(value, condition.right)
        if right_val is None:
            return True
        if isinstance(right_val, (list, tuple)):
            return left_val not in right_val
        return left_val != right_val
    # existence
    elif condition.type == ConditionalType.EXISTS:
        left_val = evaluate_expression(value, condition.left)
        return left_val is not None
    elif condition.type == ConditionalType.NOT_EXISTS:
        left_val = evaluate_expression(value, condition.left)
        return left_val is None
    else:
        assert_never(condition.type)


def evaluate_sort_key(value: dict[str, Any], sort: Sequence[Sort]) -> tuple:
    """Get sort key for a value based on Sort criteria."""
    if not sort:
        return ()

    key_values: list[Any] = []
    for s in sort:
        val = evaluate_expression(value, s.by)
        # handle None values by putting them at the end
        val = (1, None) if val is None else (0, val)
        key_values.append(val)
    return tuple(key_values)


def evaluate_function(value: dict[str, Any], function: Function) -> Any:
    """Evaluate a Function against value data."""
    left_val = evaluate_expression(value, function.left)
    if function.type == FunctionType.ADD:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val + right_val
    elif function.type == FunctionType.SUBTRACT:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val - right_val
    elif function.type == FunctionType.MULTIPLY:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val * right_val
    elif function.type == FunctionType.DIVIDE:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val / right_val
    elif function.type == FunctionType.MODULO:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val % right_val
    elif function.type == FunctionType.POWER:
        assert function.right is not None, f"no right for {function!r}"
        right_val = evaluate_expression(value, function.right)
        return left_val**right_val
    else:
        assert_never(function.type)


def evaluate_aggregation(values: list[dict[str, Any]], aggregation: Aggregation) -> Any:
    """Evaluate an Aggregation against values."""
    if aggregation.type == AggregationType.EXISTS:
        return len(values) > 0
    elif aggregation.type == AggregationType.COUNT:
        return len(values)
    elif aggregation.type == AggregationType.SUM:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        total = 0
        for value in values:
            val = evaluate_expression(value, aggregation.expression)
            if val is not None:
                total += val
        return total
    elif aggregation.type == AggregationType.MIN:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        min_val = None
        for value in values:
            val = evaluate_expression(value, aggregation.expression)
            if val is not None and (min_val is None or val < min_val):
                min_val = val
        return min_val
    elif aggregation.type == AggregationType.MAX:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        max_val = None
        for value in values:
            val = evaluate_expression(value, aggregation.expression)
            if val is not None and (max_val is None or val > max_val):
                max_val = val
        return max_val
    elif aggregation.type == AggregationType.AVERAGE:
        assert aggregation.expression is not None, f"no expression for {aggregation!r}"
        total = 0
        count = 0
        for value in values:
            val = evaluate_expression(value, aggregation.expression)
            if val is not None:
                total += val
                count += 1
        return total / count if count > 0 else None
    else:
        assert_never(aggregation.type)


def _extract_id_condition(condition: Condition) -> tuple[bool, Sequence[UUID]]:
    """Check if condition is id = value or id IN values and return the value(s)."""
    if (
        (condition.type == ConditionalType.EQUALS or condition.type == ConditionalType.IN)
        and (left := condition.left) is not None
        and left.type == ExpressionType.ATTRIBUTE
        and (attribute := left.attribute) is not None
        and attribute.type == PropertyReferenceType.BUILTIN
        and (attribute.id == NODE_ID_ID)
        and (right := condition.right) is not None
        and right.type == ExpressionType.LITERAL
    ):
        assert right.literal is not None, f"no literal for {right!r}"
        if right.literal.type.cardinality == TypeCardinality.SCALAR:
            if right.literal.type.scalar_type == ScalarType.PRIMITIVE:
                return True, (right.literal.unpack(),)
            else:
                return True, (right.literal.unpack().id,)
        elif right.literal.type.cardinality == TypeCardinality.LIST:
            if right.literal.type.scalar_type == ScalarType.PRIMITIVE:
                return True, right.literal.unpack()
            else:
                return True, [ptr.id for ptr in right.literal.unpack()]

    return False, ()
