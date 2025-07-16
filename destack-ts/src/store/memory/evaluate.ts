import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  PropertyReferenceType,
  ScalarType,
  Sort,
  TypeCardinality,
} from "@destack/language";
import { NODE_ID_ID, NODE_REFERENCE_ID_KEY } from "@destack/store/memory/core";
import { assertNever } from "@destack/utils";

/**
 * Evaluate an Expression against value data.
 */
export function evaluateExpression(options: {
  value: Record<string, any>;
  expression: Expression;
}): any {
  const { value, expression } = options;
  if (expression.type === ExpressionType.LITERAL) {
    if (!expression.literal) {
      throw new Error(`no literal for ${expression.repr()}`);
    }
    if (expression.literal.type.scalarType === ScalarType.NODE_REFERENCE) {
      return expression.literal.value && expression.literal.value[NODE_REFERENCE_ID_KEY]
        ? expression.literal.value[NODE_REFERENCE_ID_KEY]
        : null;
    } else {
      return expression.literal.value;
    }
  } else if (expression.type === ExpressionType.ATTRIBUTE) {
    if (!expression.attribute) {
      throw new Error(`no attribute for ${expression.repr()}`);
    }
    const attr = expression.attribute;
    if (attr.type === PropertyReferenceType.BUILTIN) {
      const prop = attr.resolve();
      if (prop.scalarType === ScalarType.NODE_REFERENCE) {
        const nodePtrPacked = value[String(prop.id)];
        return nodePtrPacked && nodePtrPacked[NODE_REFERENCE_ID_KEY]
          ? nodePtrPacked[NODE_REFERENCE_ID_KEY]
          : null;
      } else {
        return value[String(prop.id)];
      }
    } else {
      throw new Error(`unsupported attribute type: ${attr.type}`);
    }
  } else if (expression.type === ExpressionType.CONDITION) {
    if (!expression.condition) {
      throw new Error(`no condition for ${expression.repr()}`);
    }
    return evaluateCondition({ value, condition: expression.condition });
  } else if (expression.type === ExpressionType.FUNCTION) {
    if (!expression.function) {
      throw new Error(`no function for ${expression.repr()}`);
    }
    return evaluateFunction({ value, func: expression.function });
  } else if (expression.type === ExpressionType.AGGREGATION) {
    // Aggregations need to be handled at a higher level with multiple values
    throw new Error(`aggregation cannot be evaluated on single value: ${expression.repr()}`);
  } else {
    assertNever(expression.type);
  }
}

/**
 * Evaluate a Condition against value data.
 */
export function evaluateCondition(options: {
  value: Record<string, any>;
  condition: Condition;
}): boolean {
  const { value, condition } = options;
  // logical
  if (condition.type === ConditionalType.NOT) {
    const leftVal = evaluateExpression({ value, expression: condition.left });
    return !Boolean(leftVal);
  } else if (condition.type === ConditionalType.AND) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    if (!leftVal) {
      return false;
    }
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return Boolean(rightVal);
  } else if (condition.type === ConditionalType.OR) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    if (leftVal) {
      return true;
    }
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return Boolean(rightVal);
  }
  // comparison
  else if (condition.type === ConditionalType.EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal === rightVal;
  } else if (condition.type === ConditionalType.NOT_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal !== rightVal;
  } else if (condition.type === ConditionalType.GREATER_THAN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal > rightVal;
  } else if (condition.type === ConditionalType.GREATER_THAN_OR_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal >= rightVal;
  } else if (condition.type === ConditionalType.LESS_THAN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal < rightVal;
  } else if (condition.type === ConditionalType.LESS_THAN_OR_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    return leftVal <= rightVal;
  }
  // string
  else if (condition.type === ConditionalType.MATCHES) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    if (leftVal == null || rightVal == null) {
      return false;
    }
    return String(leftVal).includes(String(rightVal));
  } else if (condition.type === ConditionalType.STARTS_WITH) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    if (leftVal == null || rightVal == null) {
      return false;
    }
    return String(leftVal).startsWith(String(rightVal));
  } else if (condition.type === ConditionalType.ENDS_WITH) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    if (leftVal == null || rightVal == null) {
      return false;
    }
    return String(leftVal).endsWith(String(rightVal));
  }
  // collections
  else if (condition.type === ConditionalType.IN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    if (rightVal == null) {
      return false;
    }
    if (Array.isArray(rightVal)) {
      return rightVal.includes(leftVal);
    }
    return leftVal === rightVal;
  } else if (condition.type === ConditionalType.NOT_IN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ value, expression: condition.left });
    const rightVal = evaluateExpression({ value, expression: condition.right });
    if (rightVal == null) {
      return true;
    }
    if (Array.isArray(rightVal)) {
      return !rightVal.includes(leftVal);
    }
    return leftVal !== rightVal;
  }
  // existence
  else if (condition.type === ConditionalType.EXISTS) {
    const leftVal = evaluateExpression({ value, expression: condition.left });
    return leftVal != null;
  } else if (condition.type === ConditionalType.NOT_EXISTS) {
    const leftVal = evaluateExpression({ value, expression: condition.left });
    return leftVal == null;
  } else {
    assertNever(condition.type);
  }
}

/**
 * Get sort key for a value based on Sort criteria.
 */
export function evaluateSortKey(options: {
  value: Record<string, any>;
  sort: readonly Sort[];
}): any[] {
  const { value, sort } = options;
  if (!sort || sort.length === 0) {
    return [];
  }

  const keyValues: any[] = [];
  for (const s of sort) {
    let val = evaluateExpression({ value, expression: s.by });
    // handle null/undefined values by putting them at the end
    val = val == null ? [1, null] : [0, val];
    keyValues.push(val);
  }
  return keyValues;
}

/**
 * Evaluate a Function against value data.
 */
export function evaluateFunction(options: { value: Record<string, any>; func: Function }): any {
  const { value, func } = options;
  const leftVal = evaluateExpression({ value, expression: func.left });
  if (func.type === FunctionType.ADD) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return leftVal + rightVal;
  } else if (func.type === FunctionType.SUBTRACT) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return leftVal - rightVal;
  } else if (func.type === FunctionType.MULTIPLY) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return leftVal * rightVal;
  } else if (func.type === FunctionType.DIVIDE) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return leftVal / rightVal;
  } else if (func.type === FunctionType.MODULO) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return leftVal % rightVal;
  } else if (func.type === FunctionType.POWER) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ value, expression: func.right });
    return Math.pow(leftVal, rightVal);
  } else {
    assertNever(func.type);
  }
}

/**
 * Evaluate an Aggregation against values.
 */
export function evaluateAggregation(options: {
  values: Array<Record<string, any>>;
  aggregation: Aggregation;
}): any {
  const { values, aggregation } = options;
  if (aggregation.type === AggregationType.EXISTS) {
    return values.length > 0;
  } else if (aggregation.type === AggregationType.COUNT) {
    return values.length;
  } else if (aggregation.type === AggregationType.SUM) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let total = 0;
    for (const value of values) {
      const val = evaluateExpression({ value, expression: aggregation.expression });
      if (val != null) {
        total += val;
      }
    }
    return total;
  } else if (aggregation.type === AggregationType.MIN) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let minVal: any = null;
    for (const value of values) {
      const val = evaluateExpression({ value, expression: aggregation.expression });
      if (val != null && (minVal == null || val < minVal)) {
        minVal = val;
      }
    }
    return minVal;
  } else if (aggregation.type === AggregationType.MAX) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let maxVal: any = null;
    for (const value of values) {
      const val = evaluateExpression({ value, expression: aggregation.expression });
      if (val != null && (maxVal == null || val > maxVal)) {
        maxVal = val;
      }
    }
    return maxVal;
  } else if (aggregation.type === AggregationType.AVERAGE) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let total = 0;
    let count = 0;
    for (const value of values) {
      const val = evaluateExpression({ value, expression: aggregation.expression });
      if (val != null) {
        total += val;
        count += 1;
      }
    }
    return count > 0 ? total / count : null;
  } else {
    assertNever(aggregation.type);
  }
}

/**
 * Check if condition is id = value or id IN values and return the value(s).
 */
export function extractIdCondition(options: { condition: Condition }): {
  isIdCondition: boolean;
  nodeIds: string[];
} {
  const { condition } = options;
  if (
    (condition.type === ConditionalType.EQUALS || condition.type === ConditionalType.IN) &&
    condition.left &&
    condition.left.type === ExpressionType.ATTRIBUTE &&
    condition.left.attribute &&
    condition.left.attribute.type === PropertyReferenceType.BUILTIN &&
    condition.left.attribute.id === NODE_ID_ID &&
    condition.right &&
    condition.right.type === ExpressionType.LITERAL &&
    condition.right.literal
  ) {
    const literal = condition.right.literal;
    if (literal.type.cardinality === TypeCardinality.SCALAR) {
      if (literal.type.scalarType === ScalarType.PRIMITIVE) {
        return { isIdCondition: true, nodeIds: [literal.unpack() as string] };
      } else {
        const value = literal.unpack();
        return { isIdCondition: true, nodeIds: [value.id] };
      }
    } else if (literal.type.cardinality === TypeCardinality.LIST) {
      if (literal.type.scalarType === ScalarType.PRIMITIVE) {
        return { isIdCondition: true, nodeIds: literal.unpack() as string[] };
      } else {
        const values = literal.unpack() as any[];
        return { isIdCondition: true, nodeIds: values.map((ptr) => ptr.id) };
      }
    }
  }

  return { isIdCondition: false, nodeIds: [] };
}
