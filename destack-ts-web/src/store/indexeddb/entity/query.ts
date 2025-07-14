import {
  ENTITY_SNAPSHOT_KEY,
  IndexedDBContext,
  ENTITY_PARENT_KEY,
  NODE_REFERENCE_ID_KEY,
  NULL_SENTINEL,
} from "@destack-web/store/indexeddb/core";
import { unpackEntityRow } from "@destack-web/store/indexeddb/entity/wiring";
import { getEntityKey, getIndexName } from "@destack-web/store/indexeddb/map";
import { assertNever } from "@destack/utils";
import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  EdgeDirection,
  evaluateAggregation,
  evaluateCondition,
  evaluateExpression,
  evaluateSortKey,
  Expression,
  extractIdCondition,
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
  toValue,
  Value,
} from "destack";
import { IDBPTransaction } from "idb";

const MAX_RECURSION_DEPTH = 10;

type _IndexedDBEntityRow = {
  nodePtr: NodeReference;
  value: Value;
};
/**
 * Filter rows based on conditions and snapshot.
 */
async function filterRows(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  where: Condition | null;
  snapshotPath: readonly string[];
  ignoreMulti?: boolean;
}): Promise<_IndexedDBEntityRow[]> {
  const { tx, context, definition, where, snapshotPath, ignoreMulti = false } = options;
  const snapshotId = snapshotPath.length > 0 ? snapshotPath[snapshotPath.length - 1] : null;

  let filteredRows: _IndexedDBEntityRow[] = [];

  if (definition.isMulti && !ignoreMulti) {
    // fan out multi definitions
    const definitions = context.resolve(definition);
    for (const def of definitions) {
      const table = context.getEntityTable(def);
      const store = tx.objectStore(table.name);
      const index = store.index(getIndexName(table, ENTITY_SNAPSHOT_KEY));
      const rows = await index.getAll(IDBKeyRange.only(snapshotId ?? NULL_SENTINEL));
      for (const row of rows) {
        const { nodePtr, value } = unpackEntityRow(row);
        if (where === null || evaluateCondition({ value: value.value, condition: where })) {
          filteredRows.push({ nodePtr, value });
        }
      }
    }
  } else {
    // collect from single definition
    const table = context.getEntityTable(definition);
    const store = tx.objectStore(table.name);
    if (where !== null) {
      const { isIdCondition, nodeIds } = extractIdCondition({ condition: where });
      if (isIdCondition) {
        const keys = nodeIds.map((idVal) => getEntityKey(idVal, snapshotId));
        const rows = (await Promise.all(keys.map((key) => store.get(key)))).filter(
          (row) => row != null,
        );
        for (const row of rows) {
          const { nodePtr, value } = unpackEntityRow(row);
          if (evaluateCondition({ value: value.value, condition: where })) {
            filteredRows.push({ nodePtr, value });
          }
        }
      } else {
        // need to scan all rows for this snapshot
        const index = store.index(getIndexName(table, ENTITY_SNAPSHOT_KEY));
        const rows = await index.getAll(IDBKeyRange.only(snapshotId ?? NULL_SENTINEL));
        for (const row of rows) {
          const { nodePtr, value } = unpackEntityRow(row);
          if (evaluateCondition({ value: value.value, condition: where })) {
            filteredRows.push({ nodePtr, value });
          }
        }
      }
    } else {
      // get all rows for this snapshot
      const index = store.index(getIndexName(table, ENTITY_SNAPSHOT_KEY));
      const rows = await index.getAll(IDBKeyRange.only(snapshotId ?? NULL_SENTINEL));
      for (const row of rows) {
        const { nodePtr, value } = unpackEntityRow(row);
        filteredRows.push({ nodePtr, value });
      }
    }
  }

  return filteredRows;
}

/**
 * Execute a node Query.
 */
async function queryNode(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  sort: readonly Sort[] | null;
  limit: number | null;
  offset: number | null;
  snapshotPath: readonly string[];
  ignoreMulti?: boolean;
}): Promise<{ nodes: Value[]; nodesPtrs: NodeReference[] }> {
  const {
    tx,
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
    if (limit !== null || offset !== null) {
      throw new Error(`cannot limit/offset for multi definition: ${definition.repr()}`);
    }
    const definitions = context.resolve(definition);
    const allValues: Value[] = [];
    const allPtrs: NodeReference[] = [];
    for (const rel of definitions) {
      const { nodes, nodesPtrs } = await queryNode({
        tx,
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
  const filteredRows = await filterRows({ tx, context, definition, where, snapshotPath });

  // sort
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

  // offset / limit
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

  return { nodes: values, nodesPtrs: nodesPtrs };
}

/**
 * Execute a scalar Query.
 */
async function queryScalar(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  snapshotPath: readonly string[];
}): Promise<Value> {
  const { tx, context, definition, aggregation, where, snapshotPath } = options;

  // filter
  const filteredRows = await filterRows({ tx, context, definition, where, snapshotPath });

  // execute
  const scalar = evaluateAggregation({
    values: filteredRows.map((row) => row.value.value),
    aggregation,
  });
  const scalarValue = toValue(scalar);
  return scalarValue;
}

/**
 * Execute a grouped node Query.
 */
async function queryGroupedNode(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  having: Condition | null;
  sort: readonly Sort[] | null;
  groupBy: readonly Expression[];
  limit: number | null;
  offset: number | null;
  snapshotPath: readonly string[];
}): Promise<Array<{ discriminator: Value; nodes: Value[]; nodesPtrs: NodeReference[] }>> {
  const {
    tx,
    context,
    definition,
    select,
    where,
    having,
    sort,
    groupBy,
    limit,
    offset,
    snapshotPath,
  } = options;

  if (definition.isMulti) {
    throw new Error("grouped node queries not supported for multi definitions");
  }

  // filter
  const filteredRows = await filterRows({ tx, context, definition, where, snapshotPath });

  // group rows by group_by expressions
  const groups = new Map<string, _IndexedDBEntityRow[]>();
  for (const row of filteredRows) {
    const groupKey = groupBy
      .map((expr) =>
        JSON.stringify(evaluateExpression({ value: row.value.value, expression: expr })),
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

    if (offset) {
      processedRows = processedRows.slice(offset);
    }
    if (limit) {
      processedRows = processedRows.slice(0, limit);
    }

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
 * Execute a grouped scalar Query.
 */
async function queryGroupedScalar(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: readonly Expression[];
  snapshotPath: readonly string[];
}): Promise<Array<[Value, Value]>> {
  const { tx, context, definition, aggregation, where, having, groupBy, snapshotPath } = options;

  if (definition.isMulti) {
    throw new Error("grouped scalar queries not supported for multi definitions");
  }

  // filter rows based on where condition
  const filteredNodes = await filterRows({ tx, context, definition, where, snapshotPath });

  // group rows by group_by expressions
  const groups = new Map<string, _IndexedDBEntityRow[]>();
  for (const row of filteredNodes) {
    const groupKey = groupBy
      .map((expr) =>
        JSON.stringify(evaluateExpression({ value: row.value.value, expression: expr })),
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
 * Walk nodes in a specific direction with optional recursion.
 */
export async function walkNode(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  definition: NodeDefinitionReference;
  nodesPtrs: readonly NodeReference[];
  direction: EdgeDirection;
  depth: number;
  where: Condition | null;
  snapshotPath: readonly string[];
}): Promise<{ cascadedNodePtrs: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodesPtrs, direction, depth, where, snapshotPath } = options;

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
      const getPromises: Promise<{ nodeId: string; parentNodePtr: NodeReference } | null>[] = [];
      for (const def of definitions) {
        const table = context.getEntityTable(def);
        const store = tx.objectStore(table.name);

        for (const nodeId of currentParentIds) {
          const key = getEntityKey(nodeId, snapshotId);
          getPromises.push(
            store.get(key).then((row) => {
              if (row != null) {
                const { value } = unpackEntityRow(row);
                const parentPtr = value.value[ENTITY_PARENT_KEY];
                if (
                  parentPtr != null &&
                  !nodesById.has(parentPtr[NODE_REFERENCE_ID_KEY]) &&
                  (where == null || evaluateCondition({ value: value.value, condition: where }))
                ) {
                  const parentNodePtr = NodeReference.fromValue(parentPtr);
                  return { nodeId, parentNodePtr };
                }
              }
              return null;
            }),
          );
        }
      }
      const getResults = await Promise.all(getPromises);
      for (const getResult of getResults) {
        if (getResult != null) {
          const { nodeId, parentNodePtr } = getResult;
          sourceIdByNodeId.set(parentNodePtr.id, sourceIdByNodeId.get(nodeId)!);
          nodesById.set(parentNodePtr.id, parentNodePtr);
          nextParentIds.add(parentNodePtr.id);
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
      const getPromises: Promise<{ parentId: string; childNodePtrs: NodeReference[] } | null>[] =
        [];
      for (const def of definitions) {
        const table = context.getEntityTable(def);
        const store = tx.objectStore(table.name);
        const index = store.index(getIndexName(table, ENTITY_PARENT_KEY));

        for (const parentId of currentParentIds) {
          const range = IDBKeyRange.only(parentId);
          getPromises.push(
            index.getAll(range).then((rows) => {
              const childNodePtrs: NodeReference[] = [];
              for (const row of rows) {
                const { nodePtr, value } = unpackEntityRow(row);
                if (
                  !nodesById.has(nodePtr.id) &&
                  (where == null || evaluateCondition({ value: value.value, condition: where }))
                ) {
                  childNodePtrs.push(nodePtr);
                }
              }
              return childNodePtrs.length > 0 ? { parentId, childNodePtrs } : null;
            }),
          );
        }
      }
      const getResults = await Promise.all(getPromises);
      for (const getResult of getResults) {
        if (getResult != null) {
          const { parentId, childNodePtrs } = getResult;
          for (const nodePtr of childNodePtrs) {
            sourceIdByNodeId.set(nodePtr.id, sourceIdByNodeId.get(parentId)!);
            nodesById.set(nodePtr.id, nodePtr);
            nextParentIds.add(nodePtr.id);
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

/**
 * Execute the specific Query "clause" (ignoring subqueries).
 */
async function queryClause(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  query: Query;
  where: Condition | null;
}): Promise<{ result: QueryResult; nodesPtrs: NodeReference[] }> {
  const { tx, context, query, where } = options;

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
    const { nodes, nodesPtrs: ptrs } = await queryNode({
      tx,
      context,
      definition: query.definition,
      select: query.select || null,
      where: combinedWhere,
      sort: query.sort || null,
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
    const scalarResult = await queryScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
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
    const groupsValue = await queryGroupedNode({
      tx,
      context,
      definition: query.definition,
      select: query.select || null,
      where: combinedWhere,
      having: query.having || null,
      sort: query.sort || null,
      groupBy: query.groupBy,
      limit: query.limit || null,
      offset: query.offset || null,
      snapshotPath: query.snapshotPath || [],
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const { discriminator, nodes, nodesPtrs } of groupsValue) {
      const group = new QueryResultGroup({ type: QueryType.NODE, discriminator, nodes });
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
    const groupsValue = await queryGroupedScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
      having: query.having || null,
      groupBy: query.groupBy,
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

/**
 * Execute a subquery to a main Query.
 */
async function executeSubquery(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  result: QueryResult;
  nodesPtrs: readonly NodeReference[];
  subquery: Query;
}): Promise<QueryResult | null> {
  const { tx, context, result, nodesPtrs, subquery } = options;

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
        const parentPtr = NodeReference.fromValue(parentPtrValue);
        parentsPtr.set(parentPtr.id.toString(), parentPtr);
      }
    }
    if (parentsPtr.size === 0) {
      return null; // nothing to query here
    }

    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const { cascadedNodePtrs: expandedNodesPtrs } = await walkNode({
        tx,
        context,
        definition: subquery.definition,
        nodesPtrs: Array.from(parentsPtr.values()),
        direction: EdgeDirection.PARENT,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        where: subquery.where,
        snapshotPath: subquery.snapshotPath,
      });
      const idProperty = subquery.definition.resolvePropertyOrError("id");
      const nodeIds = expandedNodesPtrs.map((n) => n.id);
      subqueryWhere = idProperty.in(...nodeIds);
    } else {
      const parentIds = Array.from(parentsPtr.keys());
      subqueryWhere = subquery.definition.resolvePropertyOrError("id").in(...parentIds);
    }

    // execute subquery
    const subresult = await executeQuery({
      tx,
      context,
      query: subquery,
      where: subqueryWhere,
    });
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
      const { cascadedNodePtrs: expandedNodesPtrs } = await walkNode({
        tx,
        context,
        definition: subquery.definition,
        nodesPtrs,
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
      const nodeIds = nodesPtrs.map((n) => n.id);
      subqueryWhere = subquery.definition.resolvePropertyOrError("parent").in(...nodeIds);
    }

    // execute subquery
    const subresult = await executeQuery({
      tx,
      context,
      query: subquery,
      where: subqueryWhere,
    });
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
export async function executeQuery(options: {
  tx: IDBPTransaction<unknown, string[], "readonly" | "readwrite">;
  context: IndexedDBContext;
  query: Query;
  where?: Condition;
}): Promise<QueryResult> {
  const { tx, context, query, where } = options;

  // execute main query
  const { result, nodesPtrs } = await queryClause({ tx, context, query, where: where || null });

  // execute subqueries
  const subresults: QueryResult[] = [];
  for (const subquery of query.subqueries || []) {
    const subresult = await executeSubquery({ tx, context, result, nodesPtrs, subquery });
    if (subresult !== null) {
      subresults.push(subresult);
    }
  }
  result.subresults = subresults;

  return result;
}
