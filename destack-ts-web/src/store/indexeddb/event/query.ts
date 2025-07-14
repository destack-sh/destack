import {
  EVENT_CREATED_AT_KEY,
  EVENT_SNAPSHOT_KEY,
  IndexedDBContext,
  NODE_ID_KEY,
  NODE_REFERENCE_ID_KEY,
  NULL_SENTINEL,
} from "@destack-web/store/indexeddb/core";
import { unpackEventRow } from "@destack-web/store/indexeddb/event/wiring";
import { getIndexName } from "@destack-web/store/indexeddb/map";
import { assertNever } from "@destack/utils";
import {
  Aggregation,
  AggregationType,
  Condition,
  evaluateAggregation,
  evaluateCondition,
  evaluateExpression,
  evaluateSortKey,
  Expression,
  extractIdCondition,
  NodeDefinitionReference,
  NodeReference,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  Sort,
  SortType,
  toValue,
  Value,
} from "destack";
import { IDBPTransaction } from "idb";

type _IndexedDBEventRow = {
  nodePtr: NodeReference;
  value: Value;
};

/**
 * Filter event rows based on conditions.
 */
async function filterEventRows(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
}): Promise<_IndexedDBEventRow[]> {
  const { tx, context, definition, where } = options;
  
  const eventTable = context.getEventTable(definition);
  const store = tx.objectStore(eventTable.name);
  
  let filteredRows: _IndexedDBEventRow[] = [];
  
  if (where !== null) {
    const { isIdCondition, nodeIds } = extractIdCondition({ condition: where });
    
    if (isIdCondition) {
      // fetch specific events by id
      const rows = (
        await Promise.all(
          nodeIds.map((idVal) => store.get(idVal))
        )
      ).filter((row) => row != null);
      
      for (const row of rows) {
        const value = unpackEventRow(row);
        const nodePtr = NodeReference.fromValue(row);
        if (evaluateCondition({ value: value.value, condition: where })) {
          filteredRows.push({ nodePtr, value });
        }
      }
    } else {
      // scan all events - use created_at index for performance
      const index = store.index(getIndexName(eventTable, EVENT_CREATED_AT_KEY));
      const rows = await index.getAll();
      for (const row of rows) {
        const value = unpackEventRow(row);
        const nodePtr = NodeReference.fromValue(row);
        if (evaluateCondition({ value: value.value, condition: where })) {
          filteredRows.push({ nodePtr, value });
        }
      }
    }
  } else {
    // get all events - use created_at index for default ordering
    const index = store.index(getIndexName(eventTable, EVENT_CREATED_AT_KEY));
    const rows = await index.getAll();
    for (const row of rows) {
      const value = unpackEventRow(row);
      const nodePtr = NodeReference.fromValue(row);
      filteredRows.push({ nodePtr, value });
    }
  }
  
  return filteredRows;
}

/**
 * Execute an event node Query.
 */
async function queryEventNode(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  sort: readonly Sort[] | null;
  limit: number | null;
  offset: number | null;
}): Promise<{ nodes: Value[]; nodesPtrs: NodeReference[] }> {
  const { tx, context, definition, where, sort, limit, offset } = options;
  
  // filter
  const filteredRows = await filterEventRows({ tx, context, definition, where });
  
  // sort (override default created_at sort if needed)
  if (sort && sort.length > 0) {
    const reverseFlags = sort.some((s) => s.type === SortType.DESCENDING);
    filteredRows.sort((a, b) => {
      const aKey = evaluateSortKey({ value: a.value.value, sort });
      const bKey = evaluateSortKey({ value: b.value.value, sort });
      for (let i = 0; i < aKey.length; i++) {
        if (aKey[i] < bKey[i]) return reverseFlags ? 1 : -1;
        if (aKey[i] > bKey[i]) return reverseFlags ? -1 : 1;
      }
      return 0;
    });
  }
  
  // apply offset and limit
  let processedRows = filteredRows;
  if (offset) {
    processedRows = processedRows.slice(offset);
  }
  if (limit) {
    processedRows = processedRows.slice(0, limit);
  }
  
  // convert to values
  const values: Value[] = [];
  const nodesPtrs: NodeReference[] = [];
  for (const row of processedRows) {
    values.push(row.value);
    nodesPtrs.push(row.nodePtr);
  }
  
  return { nodes: values, nodesPtrs };
}

/**
 * Execute an event scalar Query.
 */
async function queryEventScalar(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
}): Promise<Value> {
  const { tx, context, definition, aggregation, where } = options;
  
  // filter
  const filteredRows = await filterEventRows({ tx, context, definition, where });
  
  // execute aggregation
  const scalar = evaluateAggregation({
    values: filteredRows.map((row) => row.value.value),
    aggregation,
  });
  const scalarValue = toValue(scalar);

  return scalarValue;
}

/**
 * Execute an event grouped node Query.
 */
async function queryEventGroupedNode(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  having: Condition | null;
  sort: readonly Sort[] | null;
  groupBy: readonly Expression[];
  limit: number | null;
  offset: number | null;
}): Promise<Array<{ discriminator: Value; nodes: Value[]; nodesPtrs: NodeReference[] }>> {
  const { tx, context, definition, where, having, sort, groupBy, limit, offset } = options;
  
  // filter rows
  const filteredRows = await filterEventRows({ tx, context, definition, where });
  
  // group rows by group_by expressions
  const groups = new Map<string, _IndexedDBEventRow[]>();
  for (const row of filteredRows) {
    const groupKey = groupBy
      .map((expr) =>
        JSON.stringify(evaluateExpression({ value: row.value.value, expression: expr }))
      )
      .join(":");
    
    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row);
  }
  
  // apply having filter and collect results
  const results: Array<{ discriminator: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having !== null && groupRows.length > 0) {
      if (!evaluateCondition({ value: groupRows[0].value.value, condition: having })) {
        continue;
      }
    }
    
    let processedRows = groupRows;

		// sort
    if (sort && sort.length > 0) {
      const reverseFlags = sort.some((s) => s.type === SortType.DESCENDING);
      processedRows.sort((a, b) => {
        const aKey = evaluateSortKey({ value: a.value.value, sort });
        const bKey = evaluateSortKey({ value: b.value.value, sort });
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
      values.push(row.value);
      ptrs.push(row.nodePtr);
    }
    
    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push({ discriminator: toValue(discriminator), nodes: values, nodesPtrs: ptrs });
  }
  
  return results;
}

/**
 * Execute an event grouped scalar Query.
 */
async function queryEventGroupedScalar(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: readonly Expression[];
}): Promise<Array<[Value, Value]>> {
  const { tx, context, definition, aggregation, where, having, groupBy } = options;
  
  // filter rows
  const filteredRows = await filterEventRows({ tx, context, definition, where });
  
  // group rows by group_by expressions
  const groups = new Map<string, _IndexedDBEventRow[]>();
  for (const row of filteredRows) {
    const groupKey = groupBy
      .map((expr) =>
        JSON.stringify(evaluateExpression({ value: row.value.value, expression: expr }))
      )
      .join(":");
    
    if (!groups.has(groupKey)) {
      groups.set(groupKey, []);
    }
    groups.get(groupKey)!.push(row);
  }
  
  // apply having filter and aggregation to each group
  const results: Array<[Value, Value]> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having !== null && groupRows.length > 0) {
      if (!evaluateCondition({ value: groupRows[0].value.value, condition: having })) {
        continue;
      }
    }
    
    const aggResult = evaluateAggregation({
      values: groupRows.map((row) => row.value.value),
      aggregation,
    });
    
    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push([toValue(discriminator), toValue(aggResult)]);
  }
  
  return results;
}

/**
 * Execute an event Query.
 */
export async function executeQuery(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  query: Query;
}): Promise<QueryResult> {
  const { tx, context, query } = options;
  
  let result: QueryResult;
  let nodesPtrs: NodeReference[] = [];
  
  // node
  if (query.type === QueryType.NODE) {
    const { nodes, nodesPtrs: ptrs } = await queryEventNode({
      tx,
      context,
      definition: query.definition,
      where: query.where,
      sort: query.sort,
      limit: query.limit,
      offset: query.offset,
    });
    result = new QueryResult({ id: query.id, type: query.type, nodes });
    nodesPtrs = ptrs;
  }
  
  // scalar
  else if (query.type === QueryType.SCALAR) {
    if (!query.aggregation) {
      throw new Error(`no aggregation for scalar query: ${query.repr()}`);
    }
    const scalarResult = await queryEventScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: query.where,
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
    const groupsValue = await queryEventGroupedNode({
      tx,
      context,
      definition: query.definition,
      where: query.where,
      having: query.having,
      sort: query.sort,
      groupBy: query.groupBy,
      limit: query.limit,
      offset: query.offset,
    });
    const groups: QueryResultGroup[] = [];
    for (const { discriminator, nodes, nodesPtrs: ptrs } of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.NODE,
        discriminator,
        nodes,
      });
      groups.push(group);
      nodesPtrs.push(...ptrs);
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
    const groupsValue = await queryEventGroupedScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: query.where,
      having: query.having,
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
  }
  
  else {
    assertNever(query.type);
  }
  
  // NOTE: subqueries not supported for events
  if (query.subqueries) {
    throw new Error(`subqueries not supported for event query: ${query.repr()}`);
  }
  
  return result;
}
