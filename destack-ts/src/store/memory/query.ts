import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  NodeDefinitionReference,
  NodeReference,
  PropertyReferenceType,
  Query,
  QueryResult,
  QueryType,
  ScalarType,
  Select,
  Sort,
  SortType,
  Value,
  toValue,
} from "@destack/language";

import { MemoryContext, MemoryRow } from "./core";
import { unpackNodeRow } from "./wiring";

const MAX_RECURSION_DEPTH = 100;

const NODE_REFERENCE_TYPE_KEY = String(NodeReference.property("type").id);
const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);
const NODE_REFERENCE_SPACE_ID_KEY = String(NodeReference.property("space_id").id);
const NODE_REFERENCE_DEFINITION_ID_KEY = String(NodeReference.property("definition_id").id);

/**
 * Evaluate an Expression against in-memory row data.
 */
function evaluateExpression(options: {
  context: MemoryContext;
  expression: Expression;
  row: MemoryRow;
}): any {
  const { context, expression, row } = options;

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
      const prop = attr.resolveOrError();
      if (prop.scalarType === ScalarType.NODE_REFERENCE) {
        const nodePtrPacked = row.value[String(prop.id)];
        return nodePtrPacked && nodePtrPacked[NODE_REFERENCE_ID_KEY]
          ? nodePtrPacked[NODE_REFERENCE_ID_KEY]
          : null;
      } else {
        return row.value[String(prop.id)];
      }
    } else {
      throw new Error(`unsupported attribute type: ${attr.type}`);
    }
  } else if (expression.type === ExpressionType.CONDITION) {
    if (!expression.condition) {
      throw new Error(`no condition for ${expression.repr()}`);
    }
    return evaluateCondition({ context, condition: expression.condition, row });
  } else if (expression.type === ExpressionType.FUNCTION) {
    if (!expression.function) {
      throw new Error(`no function for ${expression.repr()}`);
    }
    return evaluateFunction({ context, func: expression.function, row });
  } else if (expression.type === ExpressionType.AGGREGATION) {
    // Aggregations need to be handled at a higher level with multiple rows
    throw new Error(`aggregation cannot be evaluated on single row: ${expression.repr()}`);
  } else {
    throw new Error(`Unknown expression type: ${expression.type}`);
  }
}

/**
 * Evaluate a Condition against in-memory row data.
 */
function evaluateCondition(options: {
  context: MemoryContext;
  condition: Condition;
  row: MemoryRow;
}): boolean {
  const { context, condition, row } = options;

  // logical
  if (condition.type === ConditionalType.NOT) {
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    return !Boolean(leftVal);
  } else if (condition.type === ConditionalType.AND) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    if (!leftVal) {
      return false;
    }
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return Boolean(rightVal);
  } else if (condition.type === ConditionalType.OR) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    if (leftVal) {
      return true;
    }
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return Boolean(rightVal);
  }
  // comparison
  else if (condition.type === ConditionalType.EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal === rightVal;
  } else if (condition.type === ConditionalType.NOT_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal !== rightVal;
  } else if (condition.type === ConditionalType.GREATER_THAN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal > rightVal;
  } else if (condition.type === ConditionalType.GREATER_THAN_OR_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal >= rightVal;
  } else if (condition.type === ConditionalType.LESS_THAN) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal < rightVal;
  } else if (condition.type === ConditionalType.LESS_THAN_OR_EQUALS) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    return leftVal <= rightVal;
  }
  // string
  else if (condition.type === ConditionalType.MATCHES) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    if (leftVal == null || rightVal == null) {
      return false;
    }
    return String(leftVal).includes(String(rightVal));
  } else if (condition.type === ConditionalType.STARTS_WITH) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
    if (leftVal == null || rightVal == null) {
      return false;
    }
    return String(leftVal).startsWith(String(rightVal));
  } else if (condition.type === ConditionalType.ENDS_WITH) {
    if (!condition.right) {
      throw new Error(`no right for ${condition.repr()}`);
    }
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
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
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
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
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    const rightVal = evaluateExpression({ context, expression: condition.right, row });
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
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    return leftVal != null;
  } else if (condition.type === ConditionalType.NOT_EXISTS) {
    const leftVal = evaluateExpression({ context, expression: condition.left, row });
    return leftVal == null;
  } else {
    throw new Error(`Unknown condition type: ${condition.type}`);
  }
}

/**
 * Sort in-memory rows based on Sort criteria.
 */
function evaluateSort(options: {
  context: MemoryContext;
  sort: Sort[];
  rows: MemoryRow[];
}): MemoryRow[] {
  const { context, sort, rows } = options;

  if (sort.length === 0) {
    return rows;
  }

  function sortKey(row: MemoryRow): any[] {
    const keyValues: any[] = [];
    for (const s of sort) {
      const val = evaluateExpression({ context, expression: s.by, row });
      // Handle null values by putting them at the end
      const processedVal = val == null ? [1, null] : [0, val];
      keyValues.push(processedVal);
    }
    return keyValues;
  }

  const sortedRows = [...rows];
  sortedRows.sort((a, b) => {
    const aKey = sortKey(a);
    const bKey = sortKey(b);

    for (let i = 0; i < aKey.length; i++) {
      const aVal = aKey[i];
      const bVal = bKey[i];
      const isDesc = sort[i].type === SortType.DESCENDING;

      if (aVal[0] !== bVal[0]) {
        return aVal[0] - bVal[0];
      }

      if (aVal[1] < bVal[1]) {
        return isDesc ? 1 : -1;
      }
      if (aVal[1] > bVal[1]) {
        return isDesc ? -1 : 1;
      }
    }
    return 0;
  });

  return sortedRows;
}

/**
 * Evaluate a Function against in-memory row data.
 */
function evaluateFunction(options: {
  context: MemoryContext;
  func: Function;
  row: MemoryRow;
}): any {
  const { context, func, row } = options;

  const leftVal = evaluateExpression({ context, expression: func.left, row });
  if (func.type === FunctionType.ADD) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return leftVal + rightVal;
  } else if (func.type === FunctionType.SUBTRACT) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return leftVal - rightVal;
  } else if (func.type === FunctionType.MULTIPLY) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return leftVal * rightVal;
  } else if (func.type === FunctionType.DIVIDE) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return leftVal / rightVal;
  } else if (func.type === FunctionType.MODULO) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return leftVal % rightVal;
  } else if (func.type === FunctionType.POWER) {
    if (!func.right) {
      throw new Error(`no right for ${func.repr()}`);
    }
    const rightVal = evaluateExpression({ context, expression: func.right, row });
    return Math.pow(leftVal, rightVal);
  } else {
    throw new Error(`Unknown function type: ${func.type}`);
  }
}

/**
 * Evaluate an Aggregation against in-memory rows.
 */
function evaluateAggregation(options: {
  context: MemoryContext;
  aggregation: Aggregation;
  rows: MemoryRow[];
}): any {
  const { context, aggregation, rows } = options;

  if (aggregation.type === AggregationType.EXISTS) {
    return rows.length > 0;
  } else if (aggregation.type === AggregationType.COUNT) {
    return rows.length;
  } else if (aggregation.type === AggregationType.SUM) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let total = 0;
    for (const row of rows) {
      const val = evaluateExpression({ context, expression: aggregation.expression, row });
      if (val != null) {
        total += val;
      }
    }
    return total;
  } else if (aggregation.type === AggregationType.MIN) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let minVal = null;
    for (const row of rows) {
      const val = evaluateExpression({ context, expression: aggregation.expression, row });
      if (val != null && (minVal == null || val < minVal)) {
        minVal = val;
      }
    }
    return minVal;
  } else if (aggregation.type === AggregationType.MAX) {
    if (!aggregation.expression) {
      throw new Error(`no expression for ${aggregation.repr()}`);
    }
    let maxVal = null;
    for (const row of rows) {
      const val = evaluateExpression({ context, expression: aggregation.expression, row });
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
    for (const row of rows) {
      const val = evaluateExpression({ context, expression: aggregation.expression, row });
      if (val != null) {
        total += val;
        count++;
      }
    }
    return count > 0 ? total / count : null;
  } else {
    throw new Error(`Unknown aggregation type: ${aggregation.type}`);
  }
}

/**
 * Execute a node Query.
 */
function queryNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  sort: Sort[] | null;
  limit: number | null;
  offset: number | null;
  snapshotPath: string[];
  ignoreMulti?: boolean;
}): [Value[], NodeReference[]] {
  const {
    context,
    definition,
    select,
    where,
    sort,
    limit,
    offset,
    snapshotPath,
    ignoreMulti = false,
  } = options;

  // fan out trait definitions
  if (definition.isMulti && !ignoreMulti) {
    if (limit != null || offset != null) {
      throw new Error(`cannot limit/offset for multi definition: ${definition.repr()}`);
    }
    const definitions = context.resolve(definition);
    const allValues: Value[] = [];
    const allPtrs: NodeReference[] = [];
    for (const rel of definitions) {
      const [values, ptrs] = queryNode({
        context,
        definition: rel,
        select,
        where,
        sort,
        limit,
        offset,
        snapshotPath,
        ignoreMulti: true,
      });
      allValues.push(...values);
      allPtrs.push(...ptrs);
    }
    return [allValues, allPtrs];
  }

  const table = context.get(definition);
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  // filter
  let filteredRows: MemoryRow[] = [];
  if (where != null) {
    const snapshotRows = table.rowsBySnapshot.get(snapshotId) || new Map();
    for (const row of snapshotRows.values()) {
      if (evaluateCondition({ context, condition: where, row })) {
        filteredRows.push(row);
      }
    }
  } else {
    const snapshotRows = table.rowsBySnapshot.get(snapshotId) || new Map();
    filteredRows = Array.from(snapshotRows.values());
  }

  // sort
  if (sort && sort.length > 0) {
    filteredRows = evaluateSort({ context, sort, rows: filteredRows });
  }
  if (offset && offset > 0) {
    filteredRows = filteredRows.slice(offset);
  }
  if (limit && limit > 0) {
    filteredRows = filteredRows.slice(0, limit);
  }

  // convert to values
  const values: Value[] = [];
  const ptrs: NodeReference[] = [];
  for (const row of filteredRows) {
    values.push(unpackNodeRow(table, row));
    ptrs.push(row.ptr);
  }

  return [values, ptrs];
}

/**
 * Execute a scalar Query.
 */
function queryScalar(options: QueryScalarOptions): Value {
  const { context, definition, aggregation, where, snapshotPath } = options;
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  // handle multi-definitions
  let filteredRows: MemoryRow[] = [];
  if (definition.isMulti) {
    const definitions = context.resolve(definition);
    for (const rel of definitions) {
      const table = context.get(rel);
      const snapshotRows = table.rowsBySnapshot.get(snapshotId) || new Map();
      for (const row of snapshotRows.values()) {
        if (where == null || evaluateCondition({ context, condition: where, row })) {
          filteredRows.push(row);
        }
      }
    }
  } else {
    const table = context.get(definition);
    const snapshotRows = table.rowsBySnapshot.get(snapshotId) || new Map();
    for (const row of snapshotRows.values()) {
      if (where == null || evaluateCondition({ context, condition: where, row })) {
        filteredRows.push(row);
      }
    }
  }

  // execute
  const scalar = evaluateAggregation({ context, aggregation, rows: filteredRows });
  return toValue(scalar);
}

/**
 * Execute the Query (and any subqueries).
 */
export function executeQuery(options: {
  context: MemoryContext;
  query: Query;
  where: Condition | null;
}): QueryResult {
  const { context, query, where = null } = options;

  // combine wheres
  const combinedWhere =
    where != null && query.where != null ? query.where.and(where) : where || query.where;

  let result: QueryResult;
  let nodesPtrs: NodeReference[];

  // node
  if (query.type === QueryType.NODE) {
    const [nodes, ptrs] = queryNode({
      context,
      definition: query.definition,
      select: query.select,
      where: combinedWhere,
      sort: query.sort,
      limit: query.limit,
      offset: query.offset,
      snapshotPath: query.snapshotPath,
    });
    nodesPtrs = ptrs;
    result = new QueryResult({
      id: query.id,
      type: query.type,
      nodes: nodes,
    });
  }
  // scalar
  else if (query.type === QueryType.SCALAR) {
    if (!query.aggregation) {
      throw new Error(`no aggregation for scalar query: ${query.repr()}`);
    }
    const scalarResult = queryScalar({
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
      snapshotPath: query.snapshotPath,
    });
    nodesPtrs = [];
    result = new QueryResult({
      id: query.id,
      type: query.type,
      scalar: scalarResult,
    });
    if (query.aggregation.type === AggregationType.EXISTS) {
      result.exists = scalarResult.unpack() as boolean;
    } else if (query.aggregation.type === AggregationType.COUNT) {
      result.count = scalarResult.unpack() as number;
    }
  }
  // TODO: Add support for grouped queries
  else {
    throw new Error(`Query type ${query.type} not yet implemented`);
  }

  // TODO: execute subqueries
  result.subresults = [];

  return result;
}
