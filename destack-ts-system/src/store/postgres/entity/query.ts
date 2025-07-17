import { SQL } from "bun";
import {
  Aggregation,
  Condition,
  EdgeDirection,
  Expression,
  Function,
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
  Query,
  QueryResult,
  Select,
  Sort,
  Value,
} from "destack";
import { PostgresContext } from "./core";

const MAX_RECURSION_DEPTH = 1_000;
const NODE_ID_KEY = "id"; // simplified for stub
const ENTITY_PARENT_KEY = "parent"; // simplified for stub
const NODE_REFERENCE_ID_KEY = "id"; // simplified for stub

/** Compile a Value into a SQL expression. */
function compileValue(options: {
  context: PostgresContext;
  argumentsOut: any[];
  value: Value;
}): string {
  throw new Error("not implemented");
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
  throw new Error("not implemented");
}

/** Compile a Condition into a SQL WHERE clause. */
function compileCondition(options: {
  context: PostgresContext;
  argumentsOut: any[];
  condition: Condition;
}): string {
  throw new Error("not implemented");
}

/** Compile a Sort into a SQL ORDER BY clause. */
function compileSort(options: {
  context: PostgresContext;
  argumentsOut: any[];
  sort: Sort[];
}): string {
  throw new Error("not implemented");
}

/** Compile a Function into a SQL function call. */
function compileFunction(options: {
  context: PostgresContext;
  argumentsOut: any[];
  func: Function;
}): string {
  throw new Error("not implemented");
}

/** Compile an Aggregation into a SQL aggregation function call. */
function compileAggregation(options: {
  context: PostgresContext;
  argumentsOut: any[];
  aggregation: Aggregation;
}): string {
  throw new Error("not implemented");
}

/** Compile an Expression into a SQL expression. */
function compileExpression(options: {
  context: PostgresContext;
  argumentsOut: any[];
  expression: Expression;
}): string {
  throw new Error("not implemented");
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
  throw new Error("not implemented");
}

/** Execute a node Query. */
async function queryNode(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  sort: Sort[] | null;
  limit: number | null;
  offset: number | null;
  ignoreMulti?: boolean;
}): Promise<{ nodes: Value[]; nodesPtrs: NodeReference[] }> {
  throw new Error("not implemented");
}

/** Execute a scalar Query. */
async function queryScalar(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
}): Promise<{ result: Value }> {
  throw new Error("not implemented");
}

/** Execute a grouped node Query. */
async function queryGroupedNode(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  select: Select | null;
  where: Condition | null;
  having: Condition | null;
  sort: Sort[] | null;
  groupBy: Expression[];
  limit: number | null;
  offset: number | null;
}): Promise<{ results: Array<{ group: Value; nodes: Value[]; nodesPtrs: NodeReference[] }> }> {
  throw new Error("not implemented");
}

/** Execute a grouped scalar Query. */
async function queryGroupedScalar(options: {
  tx: SQL;
  context: PostgresContext;
  definition: NodeDefinitionReference;
  aggregation: Aggregation;
  where: Condition | null;
  having: Condition | null;
  groupBy: Expression[];
}): Promise<{ results: Array<{ group: Value; scalar: Value }> }> {
  throw new Error("not implemented");
}

/** Execute the specific Query "clause" (ignoring subqueries). */
async function queryClause(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
  where: Condition | null;
}): Promise<{ result: QueryResult; nodesPtrs: NodeReference[] }> {
  throw new Error("not implemented");
}

/** Execute a Subquery. */
async function executeSubquery(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
  nodesPtrs: NodeReference[];
  where: Condition | null;
}): Promise<{ result: QueryResult }> {
  throw new Error("not implemented");
}

/** Execute a Query. */
export async function executeQuery(options: {
  tx: SQL;
  context: PostgresContext;
  query: Query;
}): Promise<{ result: QueryResult }> {
  throw new Error("not implemented");
}
