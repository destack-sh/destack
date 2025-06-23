import { AttributeReference, RelationReference, Session, Struct, StructFrozen, Supergraph, Value } from "@/language";

/* ==== DESTACK_GENERATED_START:ENUM:108 ==== */
export enum FunctionType {
  ADD = 1,
  SUBTRACT = 2,
  MULTIPLY = 3,
  DIVIDE = 4,
  MODULO = 5,
  POWER = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:108 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:103 ==== */
export enum ConditionalType {
  NOT = 1,
  AND = 2,
  OR = 3,
  EQUALS = 10,
  NOT_EQUALS = 11,
  GREATER_THAN = 12,
  GREATER_THAN_OR_EQUALS = 13,
  LESS_THAN = 14,
  LESS_THAN_OR_EQUALS = 15,
  MATCHES = 20,
  STARTS_WITH = 21,
  ENDS_WITH = 22,
  IN = 30,
  NOT_IN = 31,
  EXISTS = 40,
  NOT_EXISTS = 41,
}
/* ==== DESTACK_GENERATED_END:ENUM:103 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:104 ==== */
export enum AggregationType {
  EXISTS = 1,
  COUNT = 2,
  SUM = 3,
  MIN = 4,
  MAX = 5,
  AVERAGE = 6,
}
/* ==== DESTACK_GENERATED_END:ENUM:104 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:109 ==== */
export enum ExpressionType {
  LITERAL = 1,
  ATTRIBUTE = 2,
  CONDITION = 3,
  FUNCTION = 4,
  AGGREGATION = 5,
}
/* ==== DESTACK_GENERATED_END:ENUM:109 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:106 ==== */
export enum SortType {
  ASCENDING = 1,
  DESCENDING = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:106 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:105 ==== */
export enum SortMode {
  MAX = 1,
  MIN = 2,
  AVERAGE = 3,
  SUM = 4,
  MEDIAN = 5,
}
/* ==== DESTACK_GENERATED_END:ENUM:105 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:107 ==== */
export enum JoinType {
  LEFT = 1,
  PARENT = 10,
  CHILD = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:107 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:120 ==== */
export enum QueryType {
  NODE = 1,
  SCALAR = 2,
  GROUPED_NODE = 10,
  GROUPED_SCALAR = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:120 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:121 ==== */
export enum QueryUpdateType {
  FULL_RESULT = 1,
  PARTIAL_RESULT = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:121 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50101 ==== */
export class Function extends StructFrozen {
  readonly type: FunctionType;
  readonly left: Expression;
  readonly right: Expression | null;

  constructor(options: {
    type: FunctionType;
    left: Expression;
    right?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.left = options.left;
    this.right = options.right ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50104 ==== */
export class Condition extends StructFrozen {
  readonly type: ConditionalType;
  readonly left: Expression;
  readonly right: Expression | null;

  constructor(options: {
    type: ConditionalType;
    left: Expression;
    right?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.left = options.left;
    this.right = options.right ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50104 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50103 ==== */
export class Aggregation extends StructFrozen {
  readonly type: AggregationType;
  readonly expression: Expression | null;

  constructor(options: {
    type: AggregationType;
    expression?: Expression | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.expression = options.expression ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50100 ==== */
export class Expression extends StructFrozen {
  readonly type: ExpressionType;
  readonly literal: Value | null;
  readonly attribute: AttributeReference | null;
  readonly condition: Condition | null;
  readonly function: Function | null;
  readonly aggregation: Aggregation | null;

  constructor(options: {
    type: ExpressionType;
    literal?: Value | null;
    attribute?: AttributeReference | null;
    condition?: Condition | null;
    function?: Function | null;
    aggregation?: Aggregation | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.literal = options.literal ?? null;
    this.attribute = options.attribute ?? null;
    this.condition = options.condition ?? null;
    this.function = options.function ?? null;
    this.aggregation = options.aggregation ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50105 ==== */
export class Sort extends StructFrozen {
  readonly type: SortType;
  readonly by: Expression;
  readonly mode: SortMode | null;

  constructor(options: {
    type: SortType;
    by: Expression;
    mode?: SortMode | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.by = options.by;
    this.mode = options.mode ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50105 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50106 ==== */
export class Select extends StructFrozen {
  readonly attributes: Array<AttributeReference>;

  constructor(options: {
    attributes?: Array<AttributeReference>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.attributes = options.attributes ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50106 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50102 ==== */
export class Join extends StructFrozen {
  readonly type: JoinType;
  readonly relation: RelationReference | null;
  readonly recursive: boolean;
  readonly depth: number | null;
  readonly on: Condition | null;

  constructor(options: {
    type: JoinType;
    relation?: RelationReference | null;
    recursive?: boolean;
    depth?: number | null;
    on?: Condition | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.relation = options.relation ?? null;
    this.recursive = options.recursive ?? false;
    this.depth = options.depth ?? null;
    this.on = options.on ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50102 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50110 ==== */
export class Query extends StructFrozen {
  readonly id: string;
  readonly type: QueryType;
  readonly name: string;
  readonly relation: RelationReference;
  readonly join: Join | null;
  readonly select: Select | null;
  readonly subqueries: Array<Query>;
  readonly where: Condition | null;
  readonly having: Condition | null;
  readonly groupBy: Array<Expression>;
  readonly aggregation: Aggregation | null;
  readonly sort: Array<Sort>;
  readonly limit: number | null;
  readonly offset: number | null;

  constructor(options: {
    id?: string;
    type: QueryType;
    name: string;
    relation: RelationReference;
    join?: Join | null;
    select?: Select | null;
    subqueries?: Array<Query>;
    where?: Condition | null;
    having?: Condition | null;
    groupBy?: Array<Expression>;
    aggregation?: Aggregation | null;
    sort?: Array<Sort>;
    limit?: number | null;
    offset?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.name = options.name;
    this.relation = options.relation;
    this.join = options.join ?? null;
    this.select = options.select ?? null;
    this.subqueries = options.subqueries ?? [];
    this.where = options.where ?? null;
    this.having = options.having ?? null;
    this.groupBy = options.groupBy ?? [];
    this.aggregation = options.aggregation ?? null;
    this.sort = options.sort ?? [];
    this.limit = options.limit ?? null;
    this.offset = options.offset ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50110 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50114 ==== */
export class Histogram extends StructFrozen {
  readonly buckets: Array<Value>;
  readonly counts: Array<number>;

  constructor(options: {
    buckets?: Array<Value>;
    counts?: Array<number>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.buckets = options.buckets ?? [];
    this.counts = options.counts ?? [];
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50114 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50111 ==== */
export class QueryResult extends Struct {
  id: string;
  type: QueryType;
  groups: Array<QueryResultGroup>;
  subresults: Array<QueryResult>;
  nodes: Array<Value>;
  count: number | null;
  exists: boolean | null;
  scalar: Value | null;

  constructor(options: {
    id: string;
    type: QueryType;
    groups?: Array<QueryResultGroup>;
    subresults?: Array<QueryResult>;
    nodes?: Array<Value>;
    count?: number | null;
    exists?: boolean | null;
    scalar?: Value | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.id = options.id;
    this.type = options.type;
    this.groups = options.groups ?? [];
    this.subresults = options.subresults ?? [];
    this.nodes = options.nodes ?? [];
    this.count = options.count ?? null;
    this.exists = options.exists ?? null;
    this.scalar = options.scalar ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50111 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50112 ==== */
export class QueryResultGroup extends Struct {
  type: QueryType;
  discriminator: Value;
  nodes: Array<Value>;
  count: number | null;
  exists: boolean | null;
  scalar: Value | null;

  constructor(options: {
    type: QueryType;
    discriminator: Value;
    nodes?: Array<Value>;
    count?: number | null;
    exists?: boolean | null;
    scalar?: Value | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.discriminator = options.discriminator;
    this.nodes = options.nodes ?? [];
    this.count = options.count ?? null;
    this.exists = options.exists ?? null;
    this.scalar = options.scalar ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50112 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50113 ==== */
export class QueryUpdate extends StructFrozen {
  readonly type: QueryUpdateType;
  readonly result: QueryResult | null;

  constructor(options: {
    type: QueryUpdateType;
    result?: QueryResult | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.result = options.result ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50113 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2571 ==== */
export class Selection extends StructFrozen {
  constructor(options: { _session?: Session | null; _supergraph?: Supergraph | null }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2571 ==== */
