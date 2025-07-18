import {
  Aggregation,
  AggregationType,
  Condition,
  NodeDefinitionReference,
  NodeReference,
  Query,
  QueryResult,
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
  evaluateSortKey,
  extractIdCondition,
} from "@destack/store/memory/evaluate";
import { MemoryEventRow } from "@destack/store/memory/event/core";
import { unpackEventRow } from "@destack/store/memory/event/wiring";
import { assertNever, getLogger, getTracer, traceFunction } from "@destack/utils";

const logger = getLogger("memory.event.query");
const tracer = getTracer("memory.event.query");

/**
 * Filter event rows based on where condition.
 */
function filterRows(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
}): MemoryEventRow[] {
  const { context, definition, where } = options;
  const table = context.getEventTable(definition);

  let filteredRows: MemoryEventRow[];
  if (where !== null) {
    const { isIdCondition, nodeIds } = extractIdCondition({ condition: where });
    if (isIdCondition) {
      filteredRows = [];
      for (const idVal of nodeIds) {
        const row = table.rows.get(idVal);
        if (row !== undefined && evaluateCondition({ value: row.value.value, condition: where })) {
          filteredRows.push(row);
        }
      }
    } else {
      filteredRows = [];
      for (const row of table.rowsSorted) {
        if (evaluateCondition({ value: row.value.value, condition: where })) {
          filteredRows.push(row);
        }
      }
    }
  } else {
    filteredRows = [...table.rowsSorted];
  }

  return filteredRows;
}

/**
 * Execute a node Query for events.
 */
function _queryEventNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  sort: readonly Sort[] | null;
  limit: number | null;
  offset: number | null;
}): [Value[], NodeReference[]] {
  const { context, definition, where, sort, limit, offset } = options;

  // filter rows
  let filteredRows = filterRows({ context, definition, where });

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

const queryEventNode = traceFunction(tracer, "query_event_node", _queryEventNode);

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

  // filter rows
  const filteredRows = filterRows({ context, definition, where });

  // execute aggregation
  const scalar = evaluateAggregation({
    values: filteredRows.map((row) => row.value.value),
    aggregation,
  });
  const scalarValue = toValue(scalar);

  return scalarValue;
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
      where: query.where,
      sort: query.sort,
      limit: query.limit,
      offset: query.offset,
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
    throw new Error(`grouped node queries not supported for events: ${query.repr()}`);
  }

  // grouped scalar
  else if (query.type === QueryType.GROUPED_SCALAR) {
    throw new Error(`grouped scalar queries not supported for events: ${query.repr()}`);
  }

  //
  else {
    assertNever(query.type);
  }

  // NOTE: subqueries not supported for events
  if (query.subqueries && query.subqueries.length > 0) {
    throw new Error(`subqueries not supported for event query: ${query.repr()}`);
  }

  return result;
}
