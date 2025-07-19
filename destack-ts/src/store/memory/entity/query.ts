import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  EdgeDirection,
  Expression,
  JoinType,
  NodeDefinitionReference,
  NodeReference,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  Select,
  Sort,
  SortType,
  Value,
  toValue,
} from "@destack/language";
import {
  ENTITY_DELETED_AT_KEY,
  ENTITY_PARENT_KEY,
  MAX_RECURSION_DEPTH,
  MemoryContext,
  NODE_REFERENCE_ID_KEY,
} from "@destack/store/memory/core";
import { MemoryEntityRow } from "@destack/store/memory/entity/core";
import { unpackEntityRow } from "@destack/store/memory/entity/wiring";
import {
  evaluateAggregation,
  evaluateCondition,
  evaluateExpression,
  evaluateSortKey,
  extractIdCondition,
} from "@destack/store/memory/evaluate";
import { assertNever, getLogger, getTracer, traceFunction } from "@destack/utils";

const logger = getLogger("memory.entity.query");
const tracer = getTracer("memory.entity.query");

/**
 * Filter rows based on where condition and snapshot.
 */
function _filterRows(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  includeDeleted?: boolean;
  snapshotPath: readonly string[];
  ignoreMulti?: boolean;
}): MemoryEntityRow[] {
  const {
    context,
    definition,
    where,
    snapshotPath,
    ignoreMulti = false,
    includeDeleted = false,
  } = options;
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  let filteredRows: MemoryEntityRow[];

  if (definition.isMulti && !ignoreMulti) {
    const definitions = context.resolve(definition);
    filteredRows = [];
    for (const def of definitions) {
      const table = context.getEntityTable(def);
      const snapshotRows: Map<string, MemoryEntityRow> =
        table.rowsBySnapshot.get(snapshotId) || new Map();
      for (const row of snapshotRows.values()) {
        if (
          (includeDeleted || !row.value[ENTITY_DELETED_AT_KEY]) &&
          (where === null || evaluateCondition({ value: row.value, condition: where }))
        ) {
          filteredRows.push(row);
        }
      }
    }
  } else {
    const table = context.getEntityTable(definition);
    if (where !== null) {
      const { isIdCondition, nodeIds } = extractIdCondition({ condition: where });
      if (isIdCondition) {
        filteredRows = [];
        for (const idVal of nodeIds) {
          const nodeKey = table.getNodeKey({ id: idVal, snapshotId });
          const row = table.rows.get(nodeKey.toString());
          if (
            row !== undefined &&
            (includeDeleted || !row.value[ENTITY_DELETED_AT_KEY]) &&
            (where === null || evaluateCondition({ value: row.value, condition: where }))
          ) {
            filteredRows.push(row);
          }
        }
      } else {
        filteredRows = [];
        const snapshotRows: Map<string, MemoryEntityRow> =
          table.rowsBySnapshot.get(snapshotId) || new Map();
        for (const row of snapshotRows.values()) {
          if (
            (includeDeleted || !row.value[ENTITY_DELETED_AT_KEY]) &&
            (where === null || evaluateCondition({ value: row.value, condition: where }))
          ) {
            filteredRows.push(row);
          }
        }
      }
    } else {
      const snapshotRows: Map<string, MemoryEntityRow> =
        table.rowsBySnapshot.get(snapshotId) || new Map();
      filteredRows = Array.from(snapshotRows.values()).filter(
        (row) => includeDeleted || !row.value[ENTITY_DELETED_AT_KEY],
      );
    }
  }

  return filteredRows;
}
const filterRows = traceFunction(tracer, "filter_rows", _filterRows);

/**
 * Execute a node Query.
 */
function _queryNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  sort: readonly Sort[] | null;
  includeDeleted?: boolean;
  limit: number | null;
  offset: number | null;
  snapshotPath: readonly string[];
  ignoreMulti?: boolean;
}): { nodes: Value[]; nodesPtrs: NodeReference[] } {
  const {
    context,
    definition,
    select,
    where,
    sort,
    includeDeleted = false,
    limit,
    offset,
    snapshotPath,
    ignoreMulti = false,
  } = options;

  // fan out multi definitions
  if (definition.isMulti && !ignoreMulti) {
    if (limit !== null || offset !== null) {
      throw new Error(`cannot limit/offset for multi definition: ${definition.repr()}`);
    }
    const definitions = context.resolve(definition);
    const allValues: Value[] = [];
    const allPtrs: NodeReference[] = [];
    for (const rel of definitions) {
      const { nodes, nodesPtrs } = queryNode({
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
      allValues.push(...nodes);
      allPtrs.push(...nodesPtrs);
    }
    return { nodes: allValues, nodesPtrs: allPtrs };
  }

  // filter
  let filteredRows = filterRows({
    context,
    definition,
    where,
    snapshotPath,
    ignoreMulti: true,
  });

  // sort
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

  // offset / limit
  if (offset) {
    filteredRows = filteredRows.slice(offset);
  }
  if (limit) {
    filteredRows = filteredRows.slice(0, limit);
  }

  // convert to values
  const values: Value[] = [];
  const nodesPtrs: NodeReference[] = [];
  for (const row of filteredRows) {
    values.push(unpackEntityRow(row));
    nodesPtrs.push(row.ptr);
  }

  return { nodes: values, nodesPtrs: nodesPtrs };
}
const queryNode = traceFunction(tracer, "query_node", _queryNode);

/**
 * Execute a scalar Query.
 */
function queryScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  includeDeleted?: boolean;
  snapshotPath: readonly string[];
}): Value {
  const { context, definition, aggregation, where, includeDeleted = false, snapshotPath } = options;

  // filter
  const filteredRows = filterRows({ context, definition, where, snapshotPath, includeDeleted });

  // execute
  const scalar = evaluateAggregation({ values: filteredRows.map((row) => row.value), aggregation });
  const scalarValue = toValue(scalar);
  return scalarValue;
}

/**
 * Execute a grouped node Query.
 */
function _queryGroupedNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  having: Condition | null;
  sort: readonly Sort[] | null;
  groupBy: readonly Expression[];
  limit: number | null;
  offset: number | null;
  includeDeleted?: boolean;
  snapshotPath: readonly string[];
}): Array<{ discriminator: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> {
  const {
    context,
    definition,
    select,
    where,
    having,
    sort,
    groupBy,
    limit,
    offset,
    includeDeleted = false,
    snapshotPath,
  } = options;

  if (definition.isMulti) {
    throw new Error("grouped node queries not supported for multi definitions");
  }

  // filter
  const filteredRows = filterRows({ context, definition, where, snapshotPath, includeDeleted });

  // group rows by group_by expressions
  const groups = new Map<string, MemoryEntityRow[]>();
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
  const results: Array<{ discriminator: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> = [];
  for (const [groupKey, groupRows] of groups) {
    if (having !== null && groupRows.length > 0) {
      if (!evaluateCondition({ value: groupRows[0].value, condition: having })) {
        continue;
      }
    }

    let processedRows = groupRows;
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

    if (offset) {
      processedRows = processedRows.slice(offset);
    }
    if (limit) {
      processedRows = processedRows.slice(0, limit);
    }

    const values: Value[] = [];
    const ptrs: NodeReference[] = [];
    for (const row of processedRows) {
      values.push(unpackEntityRow(row));
      ptrs.push(row.ptr);
    }

    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push({ discriminator: toValue(discriminator), nodes: values, nodesPtrs: ptrs });
  }

  return results;
}
const queryGroupedNode = traceFunction(tracer, "query_grouped_node", _queryGroupedNode);

/**
 * Execute a grouped scalar Query.
 */
function _queryGroupedScalar(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: readonly Expression[];
  includeDeleted?: boolean;
  snapshotPath: readonly string[];
}): Array<[Value, Value]> {
  const {
    context,
    definition,
    aggregation,
    where,
    having,
    groupBy,
    includeDeleted = false,
    snapshotPath,
  } = options;

  if (definition.isMulti) {
    throw new Error("grouped scalar queries not supported for multi definitions");
  }

  // filter rows based on where condition
  const filteredNodes = filterRows({ context, definition, where, snapshotPath, includeDeleted });

  // group rows by group_by expressions
  const groups = new Map<string, MemoryEntityRow[]>();
  for (const row of filteredNodes) {
    const groupKey = groupBy
      .map((expr) => JSON.stringify(evaluateExpression({ value: row.value, expression: expr })))
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
      if (!evaluateCondition({ value: groupRows[0].value, condition: having })) {
        continue;
      }
    }
    const aggResult = evaluateAggregation({
      values: groupRows.map((row) => row.value),
      aggregation,
    });

    // extract discriminator from first group key value
    const groupKeyValues = groupKey.split(":").map((k) => JSON.parse(k));
    const discriminator = groupKeyValues.length > 0 ? groupKeyValues[0] : null;
    results.push([toValue(discriminator), toValue(aggResult)]);
  }

  return results;
}
const queryGroupedScalar = traceFunction(tracer, "query_grouped_scalar", _queryGroupedScalar);

/**
 * Walk nodes in a specific direction with optional recursion.
 */
function _walkNode(options: {
  context: MemoryContext;
  definition: NodeDefinitionReference;
  nodesPtrs: readonly NodeReference[];
  direction: EdgeDirection;
  depth: number;
  includeDeleted?: boolean | Array<string>;
  snapshotPath: readonly string[];
}): { cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> } {
  const {
    context,
    definition,
    nodesPtrs,
    direction,
    depth,
    includeDeleted = false,
    snapshotPath,
  } = options;

  const nodesById = new Map<string, NodeReference>();
  const sourceIdByNodeId = new Map<string, string>();
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;
  const definitions = context.resolve(definition);

  // parent walk
  if (direction === EdgeDirection.PARENT) {
    // start with root nodes and walk up
    let currentDepth = 0;
    let currentParentIds = new Set<string>(nodesPtrs.map((ptr) => ptr.id));
    for (const ptr of nodesPtrs) {
      nodesById.set(ptr.id, ptr);
      sourceIdByNodeId.set(ptr.id, ptr.id);
    }

    while (currentDepth < depth) {
      if (currentParentIds.size === 0) {
        break;
      }

      const nextParentIds = new Set<string>();
      for (const def of definitions) {
        const table = context.getEntityTable(def);
        for (const nodeId of currentParentIds) {
          const nodeKey = table.getNodeKey({ id: nodeId, snapshotId });
          const row = table.rows.get(nodeKey);
          if (
            row != null &&
            row.parentPtr != null &&
            !nodesById.has(row.parentPtr.id) &&
            (includeDeleted === true ||
              !row.value[ENTITY_DELETED_AT_KEY] ||
              (Array.isArray(includeDeleted) &&
                includeDeleted.includes(row.value[ENTITY_DELETED_AT_KEY])))
          ) {
            sourceIdByNodeId.set(row.parentPtr.id, sourceIdByNodeId.get(nodeId)!);
            nodesById.set(row.parentPtr.id, row.parentPtr);
            nextParentIds.add(row.parentPtr.id);
          }
        }
      }

      currentParentIds = nextParentIds;
      currentDepth++;
    }
  }

  // child walk
  else if (direction === EdgeDirection.CHILD) {
    // start with root parents and walk down
    let currentDepth = 0;
    let currentParentIds = new Set<string>(nodesPtrs.map((ptr) => ptr.id));
    for (const ptr of nodesPtrs) {
      sourceIdByNodeId.set(ptr.id, ptr.id);
    }

    while (currentDepth < depth) {
      if (currentParentIds.size === 0) {
        break;
      }

      const nextParentIds = new Set<string>();
      for (const def of definitions) {
        const table = context.getEntityTable(def);
        for (const parentId of currentParentIds) {
          const parentKey = table.getNodeKey({ id: parentId, snapshotId });
          const children = table.rowsByParent.get(parentKey);
          if (children) {
            for (const row of children) {
              if (
                !nodesById.has(row.id) &&
                (includeDeleted === true ||
                  !row.value[ENTITY_DELETED_AT_KEY] ||
                  (Array.isArray(includeDeleted) &&
                    includeDeleted.includes(row.value[ENTITY_DELETED_AT_KEY])))
              ) {
                sourceIdByNodeId.set(row.id, sourceIdByNodeId.get(parentId)!);
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

  return { cascadedNodePtrs: Array.from(nodesById.values()), sourceIdByNodeId };
}
export const walkNode = traceFunction(tracer, "walk_node", _walkNode);

/**
 * Execute the specific Query "clause" (ignoring subqueries).
 */
function _queryClause(options: { context: MemoryContext; query: Query; where: Condition | null }): {
  result: QueryResult;
  nodesPtrs: NodeReference[];
} {
  const { context, query, where } = options;

  // combine wheres
  let combinedWhere: Condition | null;
  if (where !== null) {
    combinedWhere = query.where !== null ? query.where.and(where) : where;
  } else {
    combinedWhere = query.where;
  }

  let result: QueryResult;
  let nodesPtrs: NodeReference[];

  // node
  if (query.type === QueryType.NODE) {
    const { nodes, nodesPtrs: ptrs } = queryNode({
      context,
      definition: query.definition,
      select: query.select || null,
      where: combinedWhere,
      sort: query.sort || null,
      includeDeleted: query.includeDeleted,
      limit: query.limit || null,
      offset: query.offset || null,
      snapshotPath: query.snapshotPath || [],
    });
    result = new QueryResult({ id: query.id, type: query.type, nodes });
    nodesPtrs = ptrs;
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
      includeDeleted: query.includeDeleted,
      snapshotPath: query.snapshotPath || [],
    });
    nodesPtrs = [];
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
    const groupsValue = queryGroupedNode({
      context,
      definition: query.definition,
      select: query.select || null,
      where: combinedWhere,
      having: query.having || null,
      sort: query.sort || null,
      groupBy: query.groupBy,
      includeDeleted: query.includeDeleted,
      limit: query.limit || null,
      offset: query.offset || null,
      snapshotPath: query.snapshotPath || [],
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const { discriminator, nodes, nodesPtrs } of groupsValue) {
      const group = new QueryResultGroup({
        type: QueryType.NODE,
        discriminator,
        nodes,
      });
      groups.push(group);
      nodesPtrs.push(...nodesPtrs);
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
    const groupsValue = queryGroupedScalar({
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
      having: query.having || null,
      groupBy: query.groupBy,
      includeDeleted: query.includeDeleted,
      snapshotPath: query.snapshotPath || [],
    });
    nodesPtrs = [];
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

  // fallback
  else {
    assertNever(query.type);
  }

  return { result, nodesPtrs };
}
const queryClause = traceFunction(tracer, "query_clause", _queryClause);

/**
 * Execute a subquery to a main Query.
 */
function executeSubquery(options: {
  context: MemoryContext;
  result: QueryResult;
  nodesPtrs: readonly NodeReference[];
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
      if (nodeValue.value !== null && nodeValue.value[ENTITY_PARENT_KEY] !== undefined) {
        const parentPtrValue = nodeValue.value[ENTITY_PARENT_KEY];
        const parentId = parentPtrValue[NODE_REFERENCE_ID_KEY];
        if (parentsPtr.has(parentId.toString())) {
          continue;
        }
        const parentPtr = NodeReference.fromCson(parentPtrValue);
        parentsPtr.set(parentPtr.id.toString(), parentPtr);
      }
    }
    if (parentsPtr.size === 0) {
      return null; // nothing to query here
    }

    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const { cascadedNodePtrs: expandedNodesPtrs } = walkNode({
        context,
        definition: subquery.definition,
        nodesPtrs: Array.from(parentsPtr.values()),
        direction: EdgeDirection.PARENT,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        includeDeleted: subquery.includeDeleted,
        snapshotPath: subquery.snapshotPath,
      });
      const idProperty = subquery.definition.resolveProperty("id");
      const nodeIds = expandedNodesPtrs.map((n) => n.id);
      subqueryWhere = idProperty.in(...nodeIds);
    } else {
      const parentIds = Array.from(parentsPtr.keys());
      subqueryWhere = subquery.definition.resolveProperty("id").in(...parentIds);
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
      const { cascadedNodePtrs: expandedNodesPtrs } = walkNode({
        context,
        definition: subquery.definition,
        nodesPtrs,
        direction: EdgeDirection.CHILD,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        includeDeleted: subquery.includeDeleted,
        snapshotPath: subquery.snapshotPath,
      });
      const idProperty = subquery.definition.resolveProperty("id");
      const nodeIds = expandedNodesPtrs.map((n) => n.id);
      subqueryWhere = new Condition({
        type: ConditionalType.IN,
        left: Expression.of(idProperty),
        right: Expression.of(toValue(nodeIds)),
      });
    } else {
      subqueryWhere = subquery.definition.resolveProperty("parent").in(...nodesPtrs);
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
function _executeQuery(options: {
  context: MemoryContext;
  query: Query;
  where?: Condition;
}): QueryResult {
  const { context, query, where } = options;

  // execute main query
  const { result, nodesPtrs } = queryClause({ context, query, where: where || null });

  // execute subqueries
  const subresults: QueryResult[] = [];
  for (const subquery of query.subqueries || []) {
    const subresult = executeSubquery({ context, result, nodesPtrs, subquery });
    if (subresult !== null) {
      subresults.push(subresult);
    }
  }
  result.subresults = subresults;

  return result;
}
export const executeQuery = traceFunction(tracer, "execute_query", _executeQuery);
