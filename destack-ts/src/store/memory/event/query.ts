import {
  Aggregation,
  AggregationType,
  Condition,
  Expression,
  NodeDefinitionReference,
  NodeReference,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  Sort,
  SortType,
  Value,
  toValue,
} from "@destack/language";
import { MemoryContext } from "@destack/store/memory/core";
import {
  evaluateAggregation,
  evaluateCondition,
  evaluateExpression,
  evaluateSortKey,
  extractIdCondition,
} from "@destack/store/memory/evaluate";
import { MemoryEventRow } from "@destack/store/memory/event/core";
import { unpackEventRow } from "@destack/store/memory/event/wiring";
import { assertNever } from "@destack/utils";

/**
 * Execute a node Query for events.
 */
function queryEventNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  sort: readonly Sort[] | null;
  limit: number | null;
  offset: number | null;
}): [Value[], NodeReference[]] {
  const { context, definition, where, sort, limit, offset } = options;
  const table = context.getEventTable(definition);

  // filter rows
  let filteredRows: MemoryEventRow[];
  if (where !== null) {
    const [isIdQuery, nodeIds] = extractIdCondition({ condition: where });
    if (isIdQuery) {
      filteredRows = [];
      for (const idVal of nodeIds) {
        const row = table.rows.get(idVal);
        if (row !== undefined && evaluateCondition({ value: row.value, condition: where })) {
          filteredRows.push(row);
        }
      }
    } else {
      // filter from sorted list for better performance
      filteredRows = [];
      for (const row of table.rowsSorted) {
        if (evaluateCondition({ value: row.value, condition: where })) {
          filteredRows.push(row);
        }
      }
    }
  } else {
    // use pre-sorted list
    filteredRows = [...table.rowsSorted];
  }

  // sort if needed (override default created_at sort)
  if (sort && sort.length > 0) {
    const reverseFlags = sort.some((s) => s.type === SortType.DESCENDING);
    filteredRows.sort((a, b) => {
      const aKey = evaluateSortKey({ value: a.value, sort });
      const bKey = evaluateSortKey({ value: b.value, sort });

      for (let i = 0; i < aKey.length; i++) {
        if (aKey[i] < bKey[i]) return reverseFlags ? 1 : -1;
        if (aKey[i] > bKey[i]) return reverseFlags ? -1 : 1;
      }
      return 0;
    });
  }

  // apply offset and limit
  if (offset) {
    filteredRows = filteredRows.slice(offset);
  }
  if (limit) {
    filteredRows = filteredRows.slice(0, limit);
  }

  // convert to values
  const values: Value[] = [];
  const ptrs: NodeReference[] = [];
  for (const row of filteredRows) {
    values.push(unpackEventRow(row));
    ptrs.push(row.ptr);
  }

  return [values, ptrs];
}

/**
 * Execute a scalar Query for events.
 */
function queryEventScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
}): Value {
  const { context, definition, aggregation, where } = options;
  const table = context.getEventTable(definition);

  // collect values for aggregation
  let valuesForAgg: Array<Record<string, any>>;
  if (where !== null) {
    valuesForAgg = [];
    for (const row of table.rowsSorted) {
      if (evaluateCondition({ value: row.value, condition: where })) {
        valuesForAgg.push(row.value);
      }
    }
  } else {
    valuesForAgg = table.rowsSorted.map((row) => row.value);
  }

  // execute aggregation
  const scalar = evaluateAggregation({ values: valuesForAgg, aggregation });
  const scalarValue = toValue(scalar);

  return scalarValue;
}

/**
 * Execute a grouped node Query for events.
 */
function queryEventGroupedNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  having: Condition | null;
  sort: readonly Sort[] | null;
  groupBy: readonly Expression[];
  limit: number | null;
  offset: number | null;
}): Array<[Value, Value[], NodeReference[]]> {
  const { context, definition, where, having, sort, groupBy, limit, offset } = options;
  const table = context.getEventTable(definition);

  // filter
  let filteredRows: MemoryEventRow[];
  if (where !== null) {
    filteredRows = [];
    for (const row of table.rowsSorted) {
      if (evaluateCondition({ value: row.value, condition: where })) {
        filteredRows.push(row);
      }
    }
  } else {
    filteredRows = [...table.rowsSorted];
  }

  // group rows by group_by expressions
  const groups = new Map<string, MemoryEventRow[]>();
  for (const row of filteredRows) {
    const groupKey = groupBy
      .map((expr) => JSON.stringify(evaluateExpression({ value: row.value, expression: expr })))
      .join(":");

    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row);
  }

  // apply having filter and collect results
  const results: Array<[Value, Value[], NodeReference[]]> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having !== null && groupRows.length > 0) {
      if (!evaluateCondition({ value: groupRows[0].value, condition: having })) {
        continue;
      }
    }

    let processedRows = groupRows;

    // sort
    if (sort && sort.length > 0) {
      const reverseFlags = sort.some((s) => s.type === SortType.DESCENDING);
      processedRows.sort((a, b) => {
        const aKey = evaluateSortKey({ value: a.value, sort });
        const bKey = evaluateSortKey({ value: b.value, sort });
        for (let i = 0; i < aKey.length; i++) {
          if (aKey[i] < bKey[i]) return reverseFlags ? 1 : -1;
          if (aKey[i] > bKey[i]) return reverseFlags ? -1 : 1;
        }
        return 0;
      });
    }

    // offset / limit
    if (offset) {
      processedRows = processedRows.slice(offset);
    }
    if (limit) {
      processedRows = processedRows.slice(0, limit);
    }

    // convert to values
    const values: Value[] = [];
    const ptrs: NodeReference[] = [];
    for (const row of processedRows) {
      values.push(unpackEventRow(row));
      ptrs.push(row.ptr);
    }

    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push([toValue(discriminator), values, ptrs]);
  }

  return results;
}

/**
 * Execute a grouped scalar Query for events.
 */
function queryEventGroupedScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: readonly Expression[];
}): Array<[Value, Value]> {
  const { context, definition, aggregation, where, having, groupBy } = options;
  const table = context.getEventTable(definition);

  // filter
  let filteredRows: MemoryEventRow[];
  if (where !== null) {
    filteredRows = [];
    for (const row of table.rowsSorted) {
      if (evaluateCondition({ value: row.value, condition: where })) {
        filteredRows.push(row);
      }
    }
  } else {
    filteredRows = [...table.rowsSorted];
  }

  // group rows by group_by expressions
  const groups = new Map<string, Array<Record<string, any>>>();
  for (const row of filteredRows) {
    const groupKey = groupBy
      .map((expr) => JSON.stringify(evaluateExpression({ value: row.value, expression: expr })))
      .join(":");

    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row.value);
  }

  // apply having filter and aggregation to each group
  const results: Array<[Value, Value]> = [];
  for (const [groupKey, groupValues] of groups) {
    if (having !== null && groupValues.length > 0) {
      if (!evaluateCondition({ value: groupValues[0], condition: having })) {
        continue;
      }
    }
    const aggResult = evaluateAggregation({ values: groupValues, aggregation });

    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push([toValue(discriminator), toValue(aggResult)]);
  }

  return results;
}

/**
 * Execute an event query.
 */
export function executeQuery(context: MemoryContext, query: Query): QueryResult {
  let result: QueryResult;
  let nodesPtrs: NodeReference[] = [];

  // node
  if (query.type === QueryType.NODE) {
    const [nodes, ptrs] = queryEventNode({
      context,
      definition: query.definition,
      where: query.where || null,
      sort: query.sort || null,
      limit: query.limit || null,
      offset: query.offset || null,
    });
    nodesPtrs = ptrs;
    result = new QueryResult({ id: query.id, type: query.type, nodes });
  }

  // scalar
  else if (query.type === QueryType.SCALAR) {
    if (!query.aggregation) {
      throw new Error(`no aggregation for scalar query: ${query.repr()}`);
    }
    const scalarResult = queryEventScalar({
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: query.where || null,
    });
    result = new QueryResult({ id: query.id, type: query.type, scalar: scalarResult });

    if (query.aggregation.type === AggregationType.EXISTS) {
      result.exists = scalarResult.unpack();
    } else if (query.aggregation.type === AggregationType.COUNT) {
      result.count = scalarResult.unpack();
    }
  }

  // grouped node
  else if (query.type === QueryType.GROUPED_NODE) {
    if (!query.groupBy || query.groupBy.length === 0) {
      throw new Error(`no group_by for grouped node query: ${query.repr()}`);
    }
    const groupsValue = queryEventGroupedNode({
      context,
      definition: query.definition,
      where: query.where || null,
      having: query.having || null,
      sort: query.sort || null,
      groupBy: query.groupBy,
      limit: query.limit || null,
      offset: query.offset || null,
    });

    const groups: QueryResultGroup[] = [];
    for (const [groupDiscriminator, groupNodes, groupNodesPtrs] of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.NODE,
        discriminator: groupDiscriminator,
        nodes: groupNodes,
      });
      groups.push(group);
      nodesPtrs.push(...groupNodesPtrs);
    }
    result = new QueryResult({ id: query.id, type: query.type, groups });
  }

  // grouped scalar
  else if (query.type === QueryType.GROUPED_SCALAR) {
    if (!query.aggregation) {
      throw new Error(`no aggregation for grouped scalar query: ${query.repr()}`);
    }
    if (!query.groupBy || query.groupBy.length === 0) {
      throw new Error(`no group_by for grouped scalar query: ${query.repr()}`);
    }
    const groupsValue = queryEventGroupedScalar({
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: query.where || null,
      having: query.having || null,
      groupBy: query.groupBy,
    });

    const groups: QueryResultGroup[] = [];
    for (const [groupDiscriminator, groupScalarVal] of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.SCALAR,
        discriminator: groupDiscriminator,
        scalar: groupScalarVal,
      });
      groups.push(group);
    }
    result = new QueryResult({ id: query.id, type: query.type, groups });
  } else {
    assertNever(query.type);
  }

  // note: subqueries not supported for events
  if (query.subqueries && query.subqueries.length > 0) {
    throw new Error(`subqueries not supported for event query: ${query.repr()}`);
  }

  return result;
}
