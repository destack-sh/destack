import { assertNever } from "@destack/utils";
import { SQL } from "bun";
import {
  Aggregation,
  AggregationType,
  Condition,
  ConditionalType,
  EdgeDirection,
  Entity,
  Expression,
  ExpressionType,
  Function,
  FunctionType,
  JoinType,
  Node,
  NodeDefinitionReference,
  NodeReference,
  NodeType,
  PropertyReference,
  PropertyReferenceType,
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  ScalarType,
  Select,
  Sort,
  SortType,
  toValue,
  Value,
} from "destack";
import { PostgresContext } from "./core";
import { packColumn, unpackNodeRow } from "./wiring";

const MAX_RECURSION_DEPTH = 1_000;
const NODE_ID_KEY = String(Node.property("id").id);
const ENTITY_PARENT_KEY = String(Entity.property("parent").id);
const NODE_REFERENCE_ID_KEY = String(NodeReference.property("id").id);

/** Compile a Value into a SQL expression. */
function compileValue(options: {
  context: PostgresContext;
  argumentsOut: any[];
  value: Value;
}): string {
  const { context, argumentsOut, value } = options;

  if (value.type.scalarType === ScalarType.NODE_REFERENCE) {
    // unravel reference column into id
    if (value.value == null) {
      throw new Error(`no value for ${value.repr()}`);
    }
    const valueId = value.value[NODE_REFERENCE_ID_KEY];
    argumentsOut.push(valueId);
    return `$${argumentsOut.length}`;
  } else {
    const valuePacked = packColumn({ type: value.type, value: value.value });
    argumentsOut.push(valuePacked);
    return `$${argumentsOut.length}`;
  }
}

/** Compile a Select into a SQL SELECT clause. */
function compileSelect(options: {
  context: PostgresContext;
  argumentsOut: any[];
  select: Select;
}): string {
  throw new Error("not implemented");
}

/** Compile an Attribute into a SQL expression. */
function compileAttribute(options: {
  context: PostgresContext;
  argumentsOut: any[];
  attribute: PropertyReference;
}): string {
  const { context, argumentsOut, attribute } = options;

  if (attribute.type === PropertyReferenceType.BUILTIN) {
    const prop = attribute.resolve();
    if (prop.scalarType === ScalarType.NODE_REFERENCE) {
      // unravel reference column into id
      return `"${prop.id}_${NODE_REFERENCE_ID_KEY}"`;
    } else {
      return `"${prop.id}"`;
    }
  } else if (attribute.type === PropertyReferenceType.CUSTOM) {
    throw new Error(`cannot compile custom attribute: ${attribute.repr()}`);
  } else {
    assertNever(attribute.type);
  }
}

/** Compile a Condition into a SQL WHERE clause. */
function compileCondition(options: {
  context: PostgresContext;
  argumentsOut: any[];
  condition: Condition;
}): string {
  const { context, argumentsOut, condition } = options;

  // logical
  if (condition.type === ConditionalType.NOT) {
    return `NOT (${compileExpression({ context, argumentsOut, expression: condition.left })})`;
  } else if (condition.type === ConditionalType.AND) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `(${compileExpression({ context, argumentsOut, expression: condition.left })} AND ${compileExpression({ context, argumentsOut, expression: condition.right })})`;
  } else if (condition.type === ConditionalType.OR) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `(${compileExpression({ context, argumentsOut, expression: condition.left })} OR ${compileExpression({ context, argumentsOut, expression: condition.right })})`;
  }
  // comparison
  else if (condition.type === ConditionalType.EQUALS) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} = ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.NOT_EQUALS) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} != ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.GREATER_THAN) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} > ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.GREATER_THAN_OR_EQUALS) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} >= ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.LESS_THAN) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} < ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.LESS_THAN_OR_EQUALS) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} <= ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  }
  // string
  else if (condition.type === ConditionalType.MATCHES) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} LIKE ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.STARTS_WITH) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} LIKE '%' + ${compileExpression({ context, argumentsOut, expression: condition.right })}`;
  } else if (condition.type === ConditionalType.ENDS_WITH) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} LIKE ${compileExpression({ context, argumentsOut, expression: condition.right })} + '%'`;
  }
  // collections
  else if (condition.type === ConditionalType.IN) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} = ANY(${compileExpression({ context, argumentsOut, expression: condition.right })})`;
  } else if (condition.type === ConditionalType.NOT_IN) {
    if (condition.right == null) throw new Error(`no right for ${condition.repr()}`);
    return `${compileExpression({ context, argumentsOut, expression: condition.left })} != ANY(${compileExpression({ context, argumentsOut, expression: condition.right })})`;
  }
  // existence
  else if (condition.type === ConditionalType.EXISTS) {
    return `(${compileExpression({ context, argumentsOut, expression: condition.left })}) IS NOT NULL`;
  } else if (condition.type === ConditionalType.NOT_EXISTS) {
    return `(${compileExpression({ context, argumentsOut, expression: condition.left })}) IS NULL`;
  } else {
    assertNever(condition.type);
  }
}

/** Compile a Sort into a SQL ORDER BY clause. */
function compileSort(options: {
  context: PostgresContext;
  argumentsOut: any[];
  sort: readonly Sort[];
}): string {
  const { context, argumentsOut, sort } = options;

  const sortParts: string[] = [];
  for (const s of sort) {
    const exprSql = compileExpression({ context, argumentsOut, expression: s.by });
    if (s.type === SortType.ASCENDING) {
      sortParts.push(`${exprSql} ASC`);
    } else if (s.type === SortType.DESCENDING) {
      sortParts.push(`${exprSql} DESC`);
    } else {
      assertNever(s.type);
    }
  }

  return sortParts.join(", ");
}

/** Compile a Function into a SQL function call. */
function compileFunction(options: {
  context: PostgresContext;
  argumentsOut: any[];
  func: Function;
}): string {
  const { context, argumentsOut, func } = options;

  const leftSql = compileExpression({ context, argumentsOut, expression: func.left });

  if (func.type === FunctionType.ADD) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `(${leftSql} + ${rightSql})`;
  } else if (func.type === FunctionType.SUBTRACT) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `(${leftSql} - ${rightSql})`;
  } else if (func.type === FunctionType.MULTIPLY) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `(${leftSql} * ${rightSql})`;
  } else if (func.type === FunctionType.DIVIDE) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `(${leftSql} / ${rightSql})`;
  } else if (func.type === FunctionType.MODULO) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `(${leftSql} % ${rightSql})`;
  } else if (func.type === FunctionType.POWER) {
    if (func.right == null) throw new Error(`no right operand for ${func.repr()}`);
    const rightSql = compileExpression({ context, argumentsOut, expression: func.right });
    return `POWER(${leftSql}, ${rightSql})`;
  } else {
    assertNever(func.type);
  }
}

/** Compile an Aggregation into a SQL aggregation function call. */
function compileAggregation(options: {
  context: PostgresContext;
  argumentsOut: any[];
  aggregation: Aggregation;
}): string {
  const { context, argumentsOut, aggregation } = options;

  if (aggregation.type === AggregationType.EXISTS) {
    return "1";
  } else if (aggregation.type === AggregationType.COUNT) {
    if (aggregation.expression != null) {
      const exprSql = compileExpression({
        context,
        argumentsOut,
        expression: aggregation.expression,
      });
      return `COUNT(${exprSql})`;
    } else {
      return "COUNT(*)";
    }
  } else if (aggregation.type === AggregationType.SUM) {
    if (aggregation.expression == null) throw new Error(`no expression for ${aggregation.repr()}`);
    const exprSql = compileExpression({
      context,
      argumentsOut,
      expression: aggregation.expression,
    });
    return `SUM(${exprSql})`;
  } else if (aggregation.type === AggregationType.MIN) {
    if (aggregation.expression == null) throw new Error(`no expression for ${aggregation.repr()}`);
    const exprSql = compileExpression({
      context,
      argumentsOut,
      expression: aggregation.expression,
    });
    return `MIN(${exprSql})`;
  } else if (aggregation.type === AggregationType.MAX) {
    if (aggregation.expression == null) throw new Error(`no expression for ${aggregation.repr()}`);
    const exprSql = compileExpression({
      context,
      argumentsOut,
      expression: aggregation.expression,
    });
    return `MAX(${exprSql})`;
  } else if (aggregation.type === AggregationType.AVERAGE) {
    if (aggregation.expression == null) throw new Error(`no expression for ${aggregation.repr()}`);
    const exprSql = compileExpression({
      context,
      argumentsOut,
      expression: aggregation.expression,
    });
    return `AVG(${exprSql})`;
  } else {
    assertNever(aggregation.type);
  }
}

/** Compile an Expression into a SQL expression. */
function compileExpression(options: {
  context: PostgresContext;
  argumentsOut: any[];
  expression: Expression;
}): string {
  const { context, argumentsOut, expression } = options;

  if (expression.type === ExpressionType.LITERAL) {
    if (expression.literal == null) throw new Error(`no literal for ${expression.repr()}`);
    return compileValue({ context, argumentsOut, value: expression.literal });
  } else if (expression.type === ExpressionType.ATTRIBUTE) {
    if (expression.attribute == null) throw new Error(`no attribute for ${expression.repr()}`);
    return compileAttribute({ context, argumentsOut, attribute: expression.attribute });
  } else if (expression.type === ExpressionType.CONDITION) {
    if (expression.condition == null) throw new Error(`no condition for ${expression.repr()}`);
    return compileCondition({ context, argumentsOut, condition: expression.condition });
  } else if (expression.type === ExpressionType.FUNCTION) {
    if (expression.function == null) throw new Error(`no function for ${expression.repr()}`);
    return compileFunction({ context, argumentsOut, func: expression.function });
  } else if (expression.type === ExpressionType.AGGREGATION) {
    if (expression.aggregation == null) throw new Error(`no aggregation for ${expression.repr()}`);
    return compileAggregation({ context, argumentsOut, aggregation: expression.aggregation });
  } else {
    assertNever(expression.type);
  }
}

/** Get the cascaded Nodes for a query. */
export async function walkNode(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  nodesPtrs: NodeReference[];
  direction: EdgeDirection;
  depth: number;
  where: Condition | null;
}): Promise<{ nodes: NodeReference[]; sourceIdByNodeId: Map<string, string> }> {
  const { tx, context, definition, nodesPtrs, direction, depth, where } = options;

  let rootsIds: string[] = [];
  let rootsParentsIds: string[] = [];

  if (direction === EdgeDirection.PARENT) {
    rootsIds = nodesPtrs.map((n) => n.id);
  } else {
    rootsParentsIds = nodesPtrs.map((n) => n.id);
  }

  const arguments_: any[] = [rootsIds, rootsParentsIds, depth];
  const whereSql =
    where != null
      ? compileCondition({ context, argumentsOut: arguments_, condition: where })
      : "TRUE";

  // parent walk
  if (direction === EdgeDirection.PARENT) {
    if (definition.isMulti) {
      // fan out definition
      const tables = context.resolve(definition).map((r) => context.getEntityTable(r));
      // build a single UNION of all node tables
      const unionParts = tables.map(
        (tbl) =>
          `SELECT "${NODE_ID_KEY}", "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}", ${tbl.nodeType}::int AS node_type FROM "${tbl.name}"`,
      );
      const unionSubquery = unionParts.join(" UNION ALL ");
      // anchor term
      const baseSql = `
SELECT "${NODE_ID_KEY}",
    "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
    1 AS depth,
    node_type,
    "${NODE_ID_KEY}" AS source_id
FROM (
    ${unionSubquery}
) roots
WHERE ("${NODE_ID_KEY}" = ANY($1) OR "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}" = ANY($2))
AND ${whereSql}`;
      // recursive term
      const recursiveSql = `
SELECT p."${NODE_ID_KEY}",
    p."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
    t.depth + 1 AS depth,
    p.node_type,
    t.source_id
FROM (
    ${unionSubquery}
) p
JOIN tree t ON p."${NODE_ID_KEY}" = t."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}"
WHERE t.depth < $3
AND ${whereSql}`;
      // final statement
      const stmt = `
WITH RECURSIVE tree AS (
    ${baseSql}
    UNION ALL
    ${recursiveSql}
)
SELECT "${NODE_ID_KEY}", "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}", depth, node_type, source_id
FROM tree;`;

      const resultRows = await tx.unsafe(stmt, arguments_);
      const resultNodesPtrs: NodeReference[] = [];
      const sourceIdByNodeId = new Map<string, string>();
      for (const nodePtr of nodesPtrs) {
        sourceIdByNodeId.set(nodePtr.id, nodePtr.id);
      }
      for (const row of resultRows) {
        const nodeType = row.node_type as NodeType;
        const nodePtr = new NodeReference({ type: nodeType, id: row[NODE_ID_KEY] });
        resultNodesPtrs.push(nodePtr);
        sourceIdByNodeId.set(nodePtr.id, row.source_id);
      }
      return { nodes: resultNodesPtrs, sourceIdByNodeId };
    } else {
      const table = context.getEntityTable(definition);
      const stmt = `
WITH RECURSIVE tree AS (
    SELECT  "${NODE_ID_KEY}",
            "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
            1 AS depth,
            "${NODE_ID_KEY}" AS source_id
    FROM    "${table.name}"
    WHERE   ("${NODE_ID_KEY}" = ANY($1) OR "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}" = ANY($2)) AND ${whereSql}

    UNION ALL

    SELECT  p."${NODE_ID_KEY}",
            p."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
            t.depth + 1,
            t.source_id
    FROM    "${table.name}"  AS p
    JOIN    tree      AS t ON p."${NODE_ID_KEY}" = t."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}"
    WHERE   t.depth < $3 AND ${whereSql}
)
SELECT  "${NODE_ID_KEY}",
        "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
        depth,
        source_id
FROM    tree;`;

      const resultRows = await tx.unsafe(stmt, arguments_);
      const resultNodesPtrs: NodeReference[] = [];
      const sourceIdByNodeId = new Map<string, string>();
      for (const nodePtr of nodesPtrs) {
        sourceIdByNodeId.set(nodePtr.id, nodePtr.id);
      }
      for (const row of resultRows) {
        const nodePtr = new NodeReference({ type: table.nodeType, id: row[NODE_ID_KEY] });
        resultNodesPtrs.push(nodePtr);
        sourceIdByNodeId.set(nodePtr.id, row.source_id);
      }
      return { nodes: resultNodesPtrs, sourceIdByNodeId };
    }
  }
  // child walk
  else if (direction === EdgeDirection.CHILD) {
    // fan out definition
    const tables = context.resolve(definition).map((r) => context.getEntityTable(r));

    // build a single UNION of all node tables
    const unionParts = tables.map(
      (tbl) =>
        `SELECT "${NODE_ID_KEY}", "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}", ${tbl.nodeType}::int AS node_type FROM "${tbl.name}"`,
    );
    const unionSubquery = unionParts.join(" UNION ALL ");
    // anchor term
    const baseSql = `
SELECT "${NODE_ID_KEY}",
    "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
    1 AS depth,
    node_type,
    "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}" AS source_id
FROM (
    ${unionSubquery}
) roots
WHERE ("${NODE_ID_KEY}" = ANY($1) OR "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}" = ANY($2))
AND ${whereSql}`;
    // recursive term
    const recursiveSql = `
SELECT c."${NODE_ID_KEY}",
    c."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}",
    t.depth + 1 AS depth,
    c.node_type,
    t.source_id
FROM (
    ${unionSubquery}
) c
JOIN tree t ON c."${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}" = t."${NODE_ID_KEY}"
WHERE t.depth < $3
AND ${whereSql}`;
    // final statement
    const stmt = `
WITH RECURSIVE tree AS (
    ${baseSql}
    UNION ALL
    ${recursiveSql}
)
SELECT "${NODE_ID_KEY}", "${ENTITY_PARENT_KEY}_${NODE_REFERENCE_ID_KEY}", depth, node_type, source_id
FROM tree;`;

    const resultRows = await tx.unsafe(stmt, arguments_);
    const resultNodesPtrs: NodeReference[] = [];
    const sourceIdByNodeId = new Map<string, string>();
    for (const nodePtr of nodesPtrs) {
      sourceIdByNodeId.set(nodePtr.id, nodePtr.id);
    }
    for (const row of resultRows) {
      const nodeType = row.node_type as NodeType;
      const nodePtr = new NodeReference({ type: nodeType, id: row[NODE_ID_KEY] });
      resultNodesPtrs.push(nodePtr);
      sourceIdByNodeId.set(nodePtr.id, row.source_id);
    }
    return { nodes: resultNodesPtrs, sourceIdByNodeId };
  }
  // side walk
  else if (direction === EdgeDirection.SIDE) {
    throw new Error(`cannot walk ${definition.repr()} in direction: ${direction}`);
  } else {
    assertNever(direction);
  }
}

/** Execute a node Query. */
async function queryNode(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  sort: readonly Sort[] | null;
  limit: number | null;
  offset: number | null;
  ignoreMulti?: boolean;
}): Promise<{ nodes: Value[]; nodesPtrs: NodeReference[] }> {
  const { tx, context, definition, select, where, sort, limit, offset, ignoreMulti } = options;

  // fan out multi definitions
  if (definition.isMulti && !ignoreMulti) {
    if (limit != null || offset != null) {
      throw new Error(`cannot limit/offset for multi definition: ${definition.repr()}`);
    }
    const subdefinitions = context.resolve(definition);
    const nodesValue: Value[] = [];
    const nodesPtr: NodeReference[] = [];
    for (const subdefinition of subdefinitions) {
      const { nodes: subnodesValue, nodesPtrs: subnodesPtr } = await queryNode({
        tx,
        context,
        definition: subdefinition,
        select,
        where,
        sort,
        limit,
        offset,
        ignoreMulti: true,
      });
      nodesValue.push(...subnodesValue);
      nodesPtr.push(...subnodesPtr);
    }
    return { nodes: nodesValue, nodesPtrs: nodesPtr };
  }

  // build statement
  const table = context.getEntityTable(definition);
  const arguments_: any[] = [];
  const stmtParts: string[] = ["SELECT"];
  if (select) {
    stmtParts.push(compileSelect({ context, argumentsOut: arguments_, select }));
  } else {
    stmtParts.push(table.columns.map((col) => `"${col.name}"`).join(", "));
  }
  stmtParts.push(`FROM "${table.name}"`);
  if (where != null) {
    stmtParts.push(
      `WHERE ${compileCondition({ context, argumentsOut: arguments_, condition: where })}`,
    );
  }
  if (sort && sort.length > 0) {
    stmtParts.push(`ORDER BY ${compileSort({ context, argumentsOut: arguments_, sort })}`);
  }
  if (limit != null) {
    stmtParts.push(`LIMIT ${limit}`);
  }
  if (offset != null) {
    stmtParts.push(`OFFSET ${offset}`);
  }
  const stmt = stmtParts.join("\n");

  // execute
  const nodesRow = await tx.unsafe(stmt, arguments_);
  const nodesValue: Value[] = [];
  const nodesPtr: NodeReference[] = [];
  for (const row of nodesRow) {
    const { value, ptr } = unpackNodeRow({ table, row });
    nodesValue.push(value);
    nodesPtr.push(ptr);
  }

  return { nodes: nodesValue, nodesPtrs: nodesPtr };
}

/** Execute a scalar Query. */
async function queryScalar(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
}): Promise<{ result: Value }> {
  const { tx, context, definition, aggregation, where } = options;

  if (definition.isMulti) {
    throw new Error(`cannot query scalar on multi definition: ${definition.repr()}`);
  }

  // build statement
  const table = context.getEntityTable(definition);
  const arguments_: any[] = [];
  const stmtParts: string[] = [
    "SELECT",
    compileAggregation({ context, argumentsOut: arguments_, aggregation }),
    `FROM "${table.name}"`,
  ];
  if (where != null) {
    stmtParts.push(
      `WHERE ${compileCondition({ context, argumentsOut: arguments_, condition: where })}`,
    );
  }
  if (aggregation.type === AggregationType.EXISTS) {
    stmtParts.push("LIMIT 1");
  }
  const stmt = stmtParts.join("\n");

  // execute
  const scalarRow = await tx.unsafe(stmt, arguments_);
  let scalarValue: Value;
  if (aggregation.type === AggregationType.EXISTS) {
    scalarValue = toValue(scalarRow.length > 0);
  } else if (scalarRow.length === 0 || scalarRow[0] == null) {
    scalarValue = toValue(aggregation.type === AggregationType.COUNT ? 0 : 0.0);
  } else {
    const firstCol = Object.values(scalarRow[0])[0];
    scalarValue = toValue(Number(firstCol));
  }

  return { result: scalarValue };
}

/** Execute a grouped node Query. */
async function queryGroupedNode(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  having: Condition | null;
  sort: readonly Sort[] | null;
  groupBy: readonly Expression[];
  limit: number | null;
  offset: number | null;
}): Promise<{ results: Array<{ group: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> }> {
  const { tx, context, definition, select, where, having, sort, groupBy, limit, offset } = options;

  if (definition.isMulti) {
    throw new Error(`cannot query grouped node on multi definition: ${definition.repr()}`);
  }

  // build statement to get groups
  const table = context.getEntityTable(definition);
  const groupArguments: any[] = [];
  const groupByParts = groupBy.map((expr) =>
    compileExpression({ context, argumentsOut: groupArguments, expression: expr }),
  );
  const groupByClause = groupByParts.join(", ");
  const stmtParts: string[] = [
    "SELECT",
    `${groupByClause}, ARRAY_AGG("${NODE_ID_KEY}" ORDER BY "${NODE_ID_KEY}") as grouped_ids`,
    `FROM "${table.name}"`,
  ];
  if (where != null) {
    stmtParts.push(
      `WHERE ${compileCondition({ context, argumentsOut: groupArguments, condition: where })}`,
    );
  }
  stmtParts.push(`GROUP BY ${groupByClause}`);
  if (having != null) {
    stmtParts.push(
      `HAVING ${compileCondition({ context, argumentsOut: groupArguments, condition: having })}`,
    );
  }
  if (sort && sort.length > 0) {
    stmtParts.push(`ORDER BY ${compileSort({ context, argumentsOut: groupArguments, sort })}`);
  }
  if (limit != null) {
    stmtParts.push(`LIMIT ${limit}`);
  }
  if (offset != null) {
    stmtParts.push(`OFFSET ${offset}`);
  }
  const groupStmt = stmtParts.join("\n");

  // execute statement to get groups
  const groupRows = await tx.unsafe(groupStmt, groupArguments);
  const groupByCount = groupBy.length;
  const nodesId: string[] = [];
  const nodesIdByDiscriminator = new Map<any, string[]>();
  for (const row of groupRows) {
    const groupValues = Object.values(row).slice(0, groupByCount);
    const discriminator = groupByCount === 1 ? groupValues[0] : groupValues;
    const groupIds = row.grouped_ids as string[];
    nodesIdByDiscriminator.set(discriminator, groupIds);
    nodesId.push(...groupIds);
  }

  // build statement to get nodes
  const nodeArguments: any[] = [];
  const nodeStmtParts: string[] = ["SELECT"];
  if (select) {
    nodeStmtParts.push(compileSelect({ context, argumentsOut: nodeArguments, select }));
  } else {
    const columnsClause = table.columns.map((col) => `"${col.name}"`).join(", ");
    nodeStmtParts.push(columnsClause);
  }
  nodeStmtParts.push(`FROM "${table.name}"`);
  nodeStmtParts.push(`WHERE "${NODE_ID_KEY}" = ANY($${nodeArguments.length + 1})`);
  nodeArguments.push(nodesId);
  const nodeStmt = nodeStmtParts.join("\n");

  // execute statement to get nodes
  const nodeRows = await tx.unsafe(nodeStmt, nodeArguments);
  const nodesById = new Map<string, { value: Value; ptr: NodeReference }>();
  for (const row of nodeRows) {
    const { value, ptr } = unpackNodeRow({ table, row });
    nodesById.set(ptr.id, { value, ptr });
  }

  // assemble results
  const results: Array<{ group: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> = [];
  for (const [discriminator, groupNodeIds] of nodesIdByDiscriminator) {
    const groupDiscriminator = toValue(discriminator);
    const groupNodesValue: Value[] = [];
    const groupNodesPtr: NodeReference[] = [];
    for (const nodeId of groupNodeIds) {
      const node = nodesById.get(nodeId);
      if (!node) throw new Error(`node ${nodeId} not found`);
      groupNodesValue.push(node.value);
      groupNodesPtr.push(node.ptr);
    }
    results.push({ group: groupDiscriminator, nodes: groupNodesValue, nodesPtrs: groupNodesPtr });
  }

  return { results };
}

/** Execute a grouped scalar Query. */
async function queryGroupedScalar(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: readonly Expression[];
}): Promise<{ results: Array<{ group: Value; scalar: Value }> }> {
  const { tx, context, definition, aggregation, where, having, groupBy } = options;

  if (definition.isMulti) {
    throw new Error(`cannot query grouped scalar on multi definition: ${definition.repr()}`);
  }

  // build statement
  const table = context.getEntityTable(definition);
  const arguments_: any[] = [];
  // build GROUP BY clause
  const groupByParts = groupBy.map((expr) =>
    compileExpression({ context, argumentsOut: arguments_, expression: expr }),
  );
  const groupByClause = groupByParts.join(", ");
  const stmtParts: string[] = [
    "SELECT",
    `${groupByClause}, ${compileAggregation({ context, argumentsOut: arguments_, aggregation })}`,
    `FROM "${table.name}"`,
  ];
  if (where != null) {
    stmtParts.push(
      `WHERE ${compileCondition({ context, argumentsOut: arguments_, condition: where })}`,
    );
  }
  stmtParts.push(`GROUP BY ${groupByClause}`);
  if (having != null) {
    stmtParts.push(
      `HAVING ${compileCondition({ context, argumentsOut: arguments_, condition: having })}`,
    );
  }
  const stmt = stmtParts.join("\n");

  // execute
  const rows = await tx.unsafe(stmt, arguments_);
  const results: Array<{ group: Value; scalar: Value }> = [];
  for (const row of rows) {
    // first columns are group_by values, last column is aggregation result
    const rowValues = Object.values(row);
    const groupValues = rowValues.slice(0, -1);
    const scalarValue = Number(rowValues[rowValues.length - 1]);
    // single vs multiple group by expressions
    let groupDiscriminator: Value;
    if (groupBy.length === 1) {
      groupDiscriminator = toValue(groupValues[0]);
    } else {
      // for multiple group by expressions, create a tuple
      groupDiscriminator = toValue(groupValues);
    }
    // aggregation result
    let scalarResult: Value;
    if (aggregation.type === AggregationType.EXISTS) {
      scalarResult = toValue(scalarValue != null && scalarValue);
    } else if (scalarValue == null) {
      scalarResult = toValue(aggregation.type === AggregationType.COUNT ? 0 : 0.0);
    } else {
      scalarResult = toValue(scalarValue);
    }
    results.push({ group: groupDiscriminator, scalar: scalarResult });
  }

  return { results };
}

/** Execute the specific Query "clause" (ignoring subqueries). */
async function queryClause(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
  where: Condition | null;
}): Promise<{ result: QueryResult; nodesPtrs: NodeReference[] }> {
  const { tx, context, query, where } = options;

  // combine wheres
  let combinedWhere: Condition | null;
  if (where != null) {
    combinedWhere = query.where != null ? query.where.and(where) : where;
  } else {
    combinedWhere = query.where;
  }

  let result: QueryResult;
  let nodesPtrs: NodeReference[];

  // node
  if (query.type === QueryType.NODE) {
    const { nodes, nodesPtrs: nodePtrs } = await queryNode({
      tx,
      context,
      definition: query.definition,
      select: query.select,
      where: combinedWhere,
      sort: query.sort,
      limit: query.limit,
      offset: query.offset,
    });
    nodesPtrs = nodePtrs;
    result = new QueryResult({ id: query.id, type: query.type, nodes });
  }
  // scalar
  else if (query.type === QueryType.SCALAR) {
    if (query.aggregation == null) throw new Error(`no aggregation for ${query.repr()}`);
    const { result: scalar } = await queryScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
    });
    nodesPtrs = [];
    result = new QueryResult({ id: query.id, type: query.type, scalar });
    if (query.aggregation.type === AggregationType.EXISTS) {
      result.exists = scalar.unpack() as boolean;
    } else if (query.aggregation.type === AggregationType.COUNT) {
      result.count = scalar.unpack() as number;
    }
  }
  // grouped node
  else if (query.type === QueryType.GROUPED_NODE) {
    if (query.groupBy == null) throw new Error(`no groupBy for ${query.repr()}`);
    const { results: groupsValue } = await queryGroupedNode({
      tx,
      context,
      definition: query.definition,
      select: query.select,
      where: combinedWhere,
      having: query.having,
      sort: query.sort,
      groupBy: query.groupBy,
      limit: query.limit,
      offset: query.offset,
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const {
      group: groupDiscriminator,
      nodes: groupNodes,
      nodesPtrs: groupNodesPtrs,
    } of groupsValue) {
      const group = new QueryResultGroup({
        type: query.type,
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
    if (query.groupBy == null) throw new Error(`no groupBy for ${query.repr()}`);
    if (query.aggregation == null) throw new Error(`no aggregation for ${query.repr()}`);
    const { results: groupsValue } = await queryGroupedScalar({
      tx,
      context,
      definition: query.definition,
      aggregation: query.aggregation,
      where: combinedWhere,
      having: query.having,
      groupBy: query.groupBy,
    });
    nodesPtrs = [];
    const groups: QueryResultGroup[] = [];
    for (const { group: groupDiscriminator, scalar: groupScalar } of groupsValue) {
      const group = new QueryResultGroup({
        type: query.type,
        discriminator: groupDiscriminator,
        scalar: groupScalar,
      });
      groups.push(group);
    }
    result = new QueryResult({ id: query.id, type: query.type, groups });
  } else {
    assertNever(query.type);
  }

  return { result, nodesPtrs };
}

/** Execute a Subquery. */
async function executeSubquery(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
  nodesPtrs: NodeReference[];
  where: Condition | null;
}): Promise<{ result: QueryResult }> {
  const { tx, context, query: subquery, nodesPtrs, where } = options;

  if (subquery.join == null) throw new Error(`no join for subquery ${subquery.repr()}`);

  // parent join
  if (subquery.join.type === JoinType.PARENT) {
    // collect/walk
    const parentsPtr = new Map<string, NodeReference>();
    for (const nodeValue of (options as any).result.nodes) {
      if (nodeValue.value != null && nodeValue.value[ENTITY_PARENT_KEY] != null) {
        const parentPtrValue = nodeValue.value[ENTITY_PARENT_KEY];
        const parentId = parentPtrValue[NODE_REFERENCE_ID_KEY];
        if (parentsPtr.has(parentId)) {
          continue;
        }
        const parentPtr = NodeReference.fromValue(parentPtrValue);
        parentsPtr.set(parentPtr.id, parentPtr);
      }
    }
    if (parentsPtr.size === 0) {
      return { result: new QueryResult({ id: subquery.id, type: subquery.type, nodes: [] }) };
    }
    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const { nodes: expandedNodesPtrs } = await walkNode({
        tx,
        context,
        definition: subquery.definition,
        nodesPtrs: Array.from(parentsPtr.values()),
        direction: EdgeDirection.PARENT,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        where: subquery.where,
      });
      subqueryWhere = subquery.definition
        .resolveProperty("id")
        .in(...expandedNodesPtrs.map((n) => n.id));
    } else {
      subqueryWhere = subquery.definition
        .resolveProperty("id")
        .in(...Array.from(parentsPtr.keys()));
    }
    // subquery
    const { result: subresult } = await executeQuery({
      tx,
      context,
      query: subquery,
      where: subqueryWhere,
    });
    return { result: subresult };
  }
  // child join
  else if (subquery.join.type === JoinType.CHILD) {
    // collect/walk
    if (nodesPtrs.length === 0) {
      return { result: new QueryResult({ id: subquery.id, type: subquery.type, nodes: [] }) };
    }
    let subqueryWhere: Condition;
    if (subquery.join.recursive) {
      const { nodes: expandedNodesPtrs } = await walkNode({
        tx,
        context,
        definition: subquery.definition,
        nodesPtrs,
        direction: EdgeDirection.CHILD,
        depth: subquery.join.depth || MAX_RECURSION_DEPTH,
        where: subquery.where,
      });
      subqueryWhere = subquery.definition
        .resolveProperty("id")
        .in(...expandedNodesPtrs.map((n) => n.id));
    } else {
      subqueryWhere = subquery.definition
        .resolveProperty("parent")
        .in(...nodesPtrs.map((n) => n.id));
    }
    // subquery
    const { result: subresult } = await executeQuery({
      tx,
      context,
      query: subquery,
      where: subqueryWhere,
    });
    return { result: subresult };
  }
  // left join
  else if (subquery.join.type === JoinType.LEFT) {
    throw new Error("not implemented");
  } else {
    assertNever(subquery.join.type);
  }
}

/** Execute a Query. */
export async function executeQuery(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
  where?: Condition | null;
}): Promise<{ result: QueryResult }> {
  const { tx, context, query, where = null } = options;

  // main query clause
  const { result, nodesPtrs } = await queryClause({ tx, context, query, where });

  // subqueries (sequentially)
  // NOTE :Performance: execute postgres statements in parallel?
  const subresults: QueryResult[] = [];
  for (const subquery of query.subqueries) {
    const extendedOptions = { ...options, result, query: subquery, nodesPtrs, where: null };
    const { result: subresult } = await executeSubquery(extendedOptions as any);
    if (subresult != null) {
      subresults.push(subresult);
    }
  }
  result.subresults = subresults;

  return { result };
}
