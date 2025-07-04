import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  EdgeDirection,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  JoinType,
  NodeDefinitionReference,
	Node,
  NodeReference,
  PropertyReferenceType,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  ScalarType,
  Select,
  Sort,
  SortType,
  Value,
  toValue,
} from "@destack/language";

import { assertNever } from "@destack/utils";
import { MemoryContext, MemoryRow } from "./core";
import { unpackNodeRow } from "./wiring";

const MAX_RECURSION_DEPTH = 100;

const NODE_PARENT_KEY = String(Node.property("parent").id);

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
    assertNever(expression.type);
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
    assertNever(condition.type);
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
    assertNever(func.type);
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
    assertNever(aggregation.type);
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

  // fan out multi definitions
  if (definition.isMulti && !ignoreMulti) {
    if (limit != null || offset != null) {
      throw new Error(`cannot limit/offset for multi definition: ${definition.repr()}`);
    }
    const subdefinitions = context.resolve(definition);
    const allValues: Value[] = [];
    const allPtrs: NodeReference[] = [];
    for (const subdefinition of subdefinitions) {
      const [values, ptrs] = queryNode({
        context,
        definition: subdefinition,
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
    const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
    for (const row of snapshotRows) {
      if (evaluateCondition({ context, condition: where, row })) {
        filteredRows.push(row);
      }
    }
  } else {
    const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
    filteredRows = Array.from(snapshotRows);
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
function queryScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where?: Condition | null;
  snapshotPath: string[];
}): Value {
  const { context, definition, aggregation, where, snapshotPath } = options;
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  // collect rows
  let filteredRows: MemoryRow[] = [];
  if (definition.isMulti) {
    const definitions = context.resolve(definition);
    for (const rel of definitions) {
      const table = context.get(rel);
      const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
      for (const row of snapshotRows) {
        if (where == null || evaluateCondition({ context, condition: where, row })) {
          filteredRows.push(row);
        }
      }
    }
  } else {
    const table = context.get(definition);
    const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
    for (const row of snapshotRows) {
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
 * Execute a grouped node Query.
 */
function queryGroupedNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  select?: Select | null;
  where?: Condition | null;
  having?: Condition | null;
  sort?: Sort[] | null;
  groupBy: Expression[];
  limit?: number | null;
  offset?: number | null;
  snapshotPath: string[];
}): Array<{ discriminator: Value; nodes: Value[]; nodePtrs: NodeReference[] }> {
  const { context, definition, select, where, having, sort, groupBy, limit, offset, snapshotPath } =
    options;

  if (definition.isMulti) {
    throw new Error("grouped node queries not supported for multi definitions");
  }

  const table = context.get(definition);
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  // filter
  let filteredRows: MemoryRow[] = [];
  if (where != null) {
    // filter rows based on where condition
    const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
    for (const row of snapshotRows) {
      if (evaluateCondition({ context, condition: where, row })) {
        filteredRows.push(row);
      }
    }
  } else {
    const snapshotRows = table.rowsBySnapshot.get(snapshotId)?.values() || [];
    filteredRows = Array.from(snapshotRows);
  }

  // group rows by groupBy expressions
  const groups = new Map<string, MemoryRow[]>();
  for (const row of filteredRows) {
    const groupKeyValues: any[] = [];
    for (const expr of groupBy) {
      const val = evaluateExpression({ context, expression: expr, row });
      groupKeyValues.push(val);
    }
    const groupKey = JSON.stringify(groupKeyValues);
    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row);
  }

  // apply having filter and collect results
  const results: Array<{ discriminator: Value; nodes: Value[]; nodePtrs: NodeReference[] }> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having != null && groupRows.length > 0) {
      if (!evaluateCondition({ context, condition: having, row: groupRows[0] })) {
        continue;
      }
    }

    let processedRows = groupRows;
    if (sort) {
      processedRows = evaluateSort({ context, sort, rows: processedRows });
    }
    if (offset && offset > 0) {
      processedRows = processedRows.slice(offset);
    }
    if (limit && limit > 0) {
      processedRows = processedRows.slice(0, limit);
    }

    const nodes: Value[] = [];
    const nodePtrs: NodeReference[] = [];
    for (const row of processedRows) {
      nodes.push(unpackNodeRow(table, row));
      nodePtrs.push(row.ptr);
    }

    const groupKeyValues = JSON.parse(groupKey);
    const discriminator = groupKeyValues.length > 0 ? toValue(groupKeyValues[0]) : toValue(null);
    results.push({ discriminator, nodes, nodePtrs });
  }

  return results;
}

/**
 * Execute a grouped scalar Query.
 */
function queryGroupedScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: Expression[];
  snapshotPath: string[];
}): Array<{ discriminator: Value; scalar: Value }> {
  const { context, definition, aggregation, where, having, groupBy, snapshotPath } = options;

  if (definition.isMulti) {
    throw new Error("grouped scalar queries not supported for multi definitions");
  }

  const table = context.get(definition);

  // filter rows based on where condition
  const filteredNodes: MemoryRow[] = [];
  for (const row of table.rows.values()) {
    if (where == null || evaluateCondition({ context, condition: where, row })) {
      filteredNodes.push(row);
    }
  }

  // group rows by groupBy expressions
  const groups = new Map<string, MemoryRow[]>();
  for (const row of filteredNodes) {
    const groupKeyValues: any[] = [];
    for (const expr of groupBy) {
      const val = evaluateExpression({ context, expression: expr, row });
      groupKeyValues.push(val);
    }
    const groupKey = JSON.stringify(groupKeyValues);
    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row);
  }

  // apply having filter and aggregation to each group
  const results: Array<{ discriminator: Value; scalar: Value }> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having != null && groupRows.length > 0) {
      if (!evaluateCondition({ context, condition: having, row: groupRows[0] })) {
        continue;
      }
    }
    const aggResult = evaluateAggregation({ context, aggregation, rows: groupRows });
    const groupKeyValues = JSON.parse(groupKey);
    const discriminator = groupKeyValues.length > 0 ? toValue(groupKeyValues[0]) : toValue(null);
    results.push({ discriminator, scalar: toValue(aggResult) });
  }

  return results;
}

/**
 * Walk nodes in a specific direction with optional recursion.
 */
function walkNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  rootsPtrs: NodeReference[];
  rootsParentsPtrs: NodeReference[];
  direction: EdgeDirection;
  depth: number;
  where: Condition | null;
  snapshotPath: string[];
}): NodeReference[] {
  const {
    context,
    definition,
    rootsPtrs,
    rootsParentsPtrs,
    direction,
    depth,
    where,
    snapshotPath,
  } = options;

  const nodesById = new Map<string, NodeReference>();
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;
  const definitions = context.resolve(definition);

  // parent walk
  if (direction === EdgeDirection.PARENT) {
    // start with root nodes and walk up
    let currentDepth = 0;
    let currentNodeIds = new Set<string>(rootsPtrs.map((ptr) => ptr.id));

    while (currentDepth < depth) {
      if (currentNodeIds.size === 0) {
        break;
      }

      const nextNodeIds = new Set<string>();
      for (const def of definitions) {
        const table = context.get(def);
        for (const nodeId of currentNodeIds) {
          const nodeKey = table.createKey({ id: nodeId, snapshotId });
          const row = table.rows.get(nodeKey);
          if (
            row != null &&
            row.parentPtr != null &&
            !nodesById.has(row.parentPtr.id) &&
            (where == null || evaluateCondition({ context, condition: where, row }))
          ) {
            nodesById.set(row.parentPtr.id, row.parentPtr);
            nextNodeIds.add(row.parentPtr.id);
          }
        }
      }

      currentNodeIds = nextNodeIds;
      currentDepth++;
    }
  }

  // child walk
  else if (direction === EdgeDirection.CHILD) {
    // start with root parents and walk down
    let currentDepth = 0;
    let currentParentIds = new Set<string>(rootsParentsPtrs.map((ptr) => ptr.id));

    while (currentDepth < depth) {
      if (currentParentIds.size === 0) {
        break;
      }

      const nextParentIds = new Set<string>();
      for (const def of definitions) {
        const table = context.get(def);
        for (const parentId of currentParentIds) {
          const parentKey = table.createKey({ id: parentId, snapshotId });
          const children = table.rowsByParent.get(parentKey);
          if (children) {
            for (const row of children) {
              if (
                !nodesById.has(row.id) &&
                (where == null || evaluateCondition({ context, condition: where, row }))
              ) {
                nodesById.set(row.id, row.ptr);
                nextParentIds.add(row.id);
              }
            }
          }
        }
      }

      currentParentIds = nextParentIds;
      currentDepth++;
    }
  }

  // side walk
  else if (direction === EdgeDirection.SIDE) {
    throw new Error(`cannot walk ${definition.repr()} in direction: ${direction}`);
  }

  //
  else {
    assertNever(direction);
  }

  return Array.from(nodesById.values());
}

/**
 * Execute the specific Query "clause" (ignoring subqueries).
 */
function queryClause(options: {
  context: MemoryContext;
  query: Query;
  where: Condition | null;
}): [QueryResult, NodeReference[]] {
  const { context, query, where } = options;

  // combine wheres
  const combinedWhere =
    where != null && query.where != null ? query.where.and(where) : where || query.where;

  let result: QueryResult;
  let nodesPtrs: NodeReference[] = [];

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

  // grouped node
  else if (query.type === QueryType.GROUPED_NODE) {
    if (!query.groupBy || query.groupBy.length === 0) {
      throw new Error(`no groupBy for grouped node query: ${query.repr()}`);
    }
    const groupsValue = queryGroupedNode({
      context,
      definition: query.definition,
      select: query.select,
      where: combinedWhere,
      having: query.having,
      sort: query.sort,
      groupBy: query.groupBy,
      limit: query.limit,
      offset: query.offset,
      snapshotPath: query.snapshotPath,
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const { discriminator, nodes, nodePtrs } of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.NODE,
        discriminator,
        nodes,
      });
      groups.push(group);
      nodesPtrs.push(...nodePtrs);
    }
    result = new QueryResult({
      id: query.id,
      type: query.type,
      groups,
    });
  }

  // grouped scalar
  else if (query.type === QueryType.GROUPED_SCALAR) {
    if (!query.aggregation) {
      throw new Error(`no aggregation for grouped scalar query: ${query.repr()}`);
    }
    if (!query.groupBy || query.groupBy.length === 0) {
      throw new Error(`no groupBy for grouped scalar query: ${query.repr()}`);
    }
    const groupsValue = queryGroupedScalar({
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
      having: query.having,
      groupBy: query.groupBy,
      snapshotPath: query.snapshotPath,
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const { discriminator, scalar } of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.SCALAR,
        discriminator,
        scalar,
      });
      groups.push(group);
    }
    result = new QueryResult({
      id: query.id,
      type: query.type,
      groups,
    });
  } else {
    assertNever(query.type);
  }

  return [result, nodesPtrs];
}

/**
 * Execute a subquery to a main Query.
 */
function executeSubquery(options: {
  context: MemoryContext;
  result: QueryResult;
  nodesPtrs: NodeReference[];
  subquery: Query;
}): QueryResult | null {
  const { context, result, nodesPtrs, subquery } = options;

  if (nodesPtrs.length === 0) {
    return null;
  }

  if (!subquery.join) {
    throw new Error(`no join for subquery ${subquery.repr()}`);
  }

  // parent join
  if (subquery.join.type === JoinType.PARENT) {
    // collect/walk
    const parentsPtr = new Map<string, NodeReference>();
    for (const nodeValue of result.nodes || []) {
      if (nodeValue.value != null) {
        const parentPtrValue = nodeValue.value[NODE_PARENT_KEY];
        if (parentPtrValue != null) {
          const parentId = parentPtrValue[NODE_REFERENCE_ID_KEY];
          if (!parentsPtr.has(parentId)) {
            const parentPtr = NodeReference.fromValue(parentPtrValue);
            parentsPtr.set(parentPtr.id, parentPtr);
          }
        }
      }
    }
    if (parentsPtr.size === 0) {
      return null; // nothing to query here
    }

    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const expandedNodesPtrs = walkNode({
        context,
        definition: subquery.definition,
        rootsPtrs: Array.from(parentsPtr.values()),
        rootsParentsPtrs: [],
        direction: EdgeDirection.PARENT,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        where: subquery.where,
        snapshotPath: subquery.snapshotPath,
      });
      const idProperty = subquery.definition.resolvePropertyOrError("id");
      const nodeIds = expandedNodesPtrs.map((n) => n.id);
      subqueryWhere = idProperty.in(...nodeIds);
    } else {
      const idProperty = subquery.definition.resolvePropertyOrError("id");
      const parentIds = Array.from(parentsPtr.keys());
      subqueryWhere = idProperty.in(...parentIds);
    }

    // execute subquery
    const subresult = executeQuery({ context, query: subquery, where: subqueryWhere });
    return subresult;
  }

  // child join
  else if (subquery.join.type === JoinType.CHILD) {
    // collect/walk
    if (nodesPtrs.length === 0) {
      return null; // nothing to query here
    }

    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const expandedNodesPtrs = walkNode({
        context,
        definition: subquery.definition,
        rootsPtrs: [],
        rootsParentsPtrs: nodesPtrs,
        direction: EdgeDirection.CHILD,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        where: subquery.where,
        snapshotPath: subquery.snapshotPath,
      });
      const idProperty = subquery.definition.resolvePropertyOrError("id");
      const nodeIds = expandedNodesPtrs.map((n) => n.id);
      subqueryWhere = new Condition({
        type: ConditionalType.IN,
        left: Expression.of(idProperty),
        right: Expression.of(toValue(nodeIds)),
      });
    } else {
      const parentProperty = subquery.definition.resolvePropertyOrError("parent");
      const nodeIds = nodesPtrs.map((n) => n.id);
      subqueryWhere = parentProperty.in(...nodeIds);
    }

    // execute subquery
    const subresult = executeQuery({ context, query: subquery, where: subqueryWhere });
    return subresult;
  }

  // left join
  else if (subquery.join.type === JoinType.LEFT) {
    throw new Error(`left join not implemented for subquery: ${subquery.repr()}`);
  } else {
    assertNever(subquery.join.type);
  }
}

/**
 * Execute the Query (and any subqueries).
 */
export function executeQuery(options: {
  context: MemoryContext;
  query: Query;
  where?: Condition | null;
}): QueryResult {
  const { context, query, where = null } = options;

  // execute main query
  const [result, nodesPtrs] = queryClause({ context, query, where });

  // execute subqueries
  const subresults: QueryResult[] = [];
  for (const subquery of query.subqueries || []) {
    const subresult = executeSubquery({ context, result, nodesPtrs, subquery });
    if (subresult != null) {
      subresults.push(subresult);
    }
  }
  result.subresults = subresults;

  return result;
}
