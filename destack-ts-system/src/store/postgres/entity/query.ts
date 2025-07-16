import { SQL } from "bun";
import {
  Query,
  QueryResult,
  Value,
  Select,
  PropertyReference,
  Condition,
  Sort,
  Function,
  Aggregation,
  Expression,
  NodeReference,
  Entity,
} from "destack";
import { PostgresContext } from "./core";

const MAX_RECURSION_DEPTH = 1_000;
const NODE_ID_KEY = "id"; // simplified for stub
const ENTITY_PARENT_KEY = "parent"; // simplified for stub
const NODE_REFERENCE_ID_KEY = "id"; // simplified for stub

/** Compile a Value into a SQL expression. */
function compileValue(context: PostgresContext, argumentsOut: any[], value: Value): string {
  throw new Error("not implemented");
}

/** Compile a Select into a SQL SELECT clause. */
function compileSelect(context: PostgresContext, argumentsOut: any[], select: Select): string {
  throw new Error("not implemented");
}

/** Compile an Attribute into a SQL expression. */
function compileAttribute(
  context: PostgresContext,
  argumentsOut: any[],
  attribute: PropertyReference
): string {
  throw new Error("not implemented");
}

/** Compile a Condition into a SQL WHERE clause. */
function compileCondition(
  context: PostgresContext,
  argumentsOut: any[],
  condition: Condition
): string {
  throw new Error("not implemented");
}

/** Compile a Sort into a SQL ORDER BY clause. */
function compileSort(context: PostgresContext, argumentsOut: any[], sort: Sort[]): string {
  throw new Error("not implemented");
}

/** Compile a Function into a SQL function call. */
function compileFunction(
  context: PostgresContext,
  argumentsOut: any[],
  func: Function
): string {
  throw new Error("not implemented");
}

/** Compile an Aggregation into a SQL aggregation function call. */
function compileAggregation(
  context: PostgresContext,
  argumentsOut: any[],
  aggregation: Aggregation
): string {
  throw new Error("not implemented");
}

/** Compile an Expression into a SQL expression. */
function compileExpression(
  context: PostgresContext,
  argumentsOut: any[],
  expression: Expression
): string {
  throw new Error("not implemented");
}

/** Walk a Node. */
async function walkNode(
  conn: SQL,
  context: PostgresContext,
  parent: Entity | null,
  parentRef: NodeReference | null,
  edgeDirection: "child" | "parent",
  query: Query,
  visited: Set<string>,
  depth: number
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Query a Node. */
async function queryNode(
  conn: SQL,
  context: PostgresContext,
  query: Query,
  argumentsOut: any[]
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Query a Scalar. */
async function queryScalar(
  conn: SQL,
  context: PostgresContext,
  query: Query,
  argumentsOut: any[]
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Query a Grouped Node. */
async function queryGroupedNode(
  conn: SQL,
  context: PostgresContext,
  query: Query,
  argumentsOut: any[]
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Query a Grouped Scalar. */
async function queryGroupedScalar(
  conn: SQL,
  context: PostgresContext,
  query: Query,
  argumentsOut: any[]
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Query a Clause. */
async function queryClause(
  conn: SQL,
  context: PostgresContext,
  parent: Entity | null,
  parentRef: NodeReference | null,
  edgeDirection: "child" | "parent",
  queryClause: Query,
  visited: Set<string>,
  depth: number
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Execute a Subquery. */
async function executeSubquery(
  conn: SQL,
  context: PostgresContext,
  parent: Entity | null,
  parentRef: NodeReference | null,
  edgeDirection: "child" | "parent",
  query: Query,
  visited: Set<string>,
  depth: number
): Promise<QueryResult> {
  throw new Error("not implemented");
}

/** Execute a Query. */
export async function executeQuery(
  conn: SQL,
  context: PostgresContext,
  query: Query
): Promise<QueryResult> {
  throw new Error("not implemented");
} 