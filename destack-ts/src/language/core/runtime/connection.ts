import { Node } from "@destack/language/core/builtin";
import {
  Query,
  QueryResult,
  QueryResultGroup,
  QueryType,
  QueryUpdate,
} from "@destack/language/core/common/query";
import { unpackValue, Value } from "@destack/language/core/common/value";
import { Graph } from "@destack/language/core/runtime/graph";
import { Session } from "@destack/language/core/runtime/session";
import { Store } from "@destack/language/core/runtime/store";

/** A container for some (part of a) QueryResult. */
export class QueryContainer<T extends Node = Node> {
  readonly connection: QueryConnection;
  readonly type: QueryType;
  readonly query: Query;

  result: QueryResult | QueryResultGroup | null;
  nodes: T[];
  discriminator: Value | null;
  children: QueryContainer[];

  constructor(options: {
    connection: QueryConnection;
    type: QueryType;
    query: Query;
    result: QueryResult | QueryResultGroup | null;
    discriminator: Value | null;
  }) {
    this.connection = options.connection;
    this.type = options.type;
    this.query = options.query;
    this.result = options.result;
    this.nodes = [];
    this.discriminator = options.discriminator;
    this.children = [];
  }

  repr(): string {
    const contentParts: string[] = [`query=${this.query.repr()}`];
    if (this.result != null) {
      contentParts.push(`result=${this.result.repr()}`);
    }
    return `<QueryResultContainer ${contentParts.join(", ")}>`;
  }

  /** Add a new result to the subresult. */
  _addResult(result: QueryResult | QueryResultGroup, query: Query): void {
    const session = this.connection.session;
    const supergraph = session.supergraph;
    const graph = this.connection.graph;

    // nodes
    for (const nodeValue of result.nodes) {
      const node = unpackValue(nodeValue.value, nodeValue.type, {
        _session: session,
        _graph: graph,
        _supergraph: supergraph,
        _connection: this.connection,
      });
      if (!(node instanceof Node)) {
        throw new Error(`expected Node, got ${node?.constructor?.name} in ${this.repr()}`);
      }
      this.nodes.push(node as T);
    }

    if (result instanceof QueryResult) {
      // subgroups
      for (const group of result.groups) {
        let subtype: QueryType;
        if (query.type === QueryType.GROUPED_NODE) {
          subtype = QueryType.NODE;
        } else if (query.type === QueryType.GROUPED_SCALAR) {
          subtype = QueryType.SCALAR;
        } else {
          throw new Error(`unexpected query type: ${query.type}`);
        }
        const subcontainer = new QueryContainer({
          connection: this.connection,
          type: subtype,
          query: query,
          result: group,
          discriminator: group.discriminator,
        });
        this.children.push(subcontainer);
        subcontainer._addResult(group, query);
      }
      // subqueries
      for (let i = 0; i < result.subresults.length; i++) {
        const subresult = result.subresults[i];
        const subquery = query.subqueries[i];
        const subcontainer = new QueryContainer({
          connection: this.connection,
          type: subquery.type,
          query: subquery,
          result: subresult,
          discriminator: null,
        });
        this.children.push(subcontainer);
        subcontainer._addResult(subresult, subquery);
      }
    }
  }

  /** Update the subresult with a new result. */
  _updateResult(result: QueryUpdate, query: Query): void {
    throw new Error("not implemented");
  }

  /** Get the main Node (if any). */
  toOneOrNone(): T | null {
    const nodes = this.nodes;
    if (this.type !== QueryType.NODE) {
      throw new Error(`not a node Query: ${this.query.repr()}`);
    } else if (this.result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else if (nodes.length > 1) {
      throw new Error(
        `expected 0-1 root, got ${nodes.length} in ${this.repr()}: ${nodes.map((n) => n.repr()).join(", ")}`,
      );
    } else {
      return nodes[0] ?? null;
    }
  }

  /** Get the main Node (error if none). */
  toOne(): T {
    const nodes = this.nodes;
    if (this.type !== QueryType.NODE) {
      throw new Error(`not a node Query: ${this.query.repr()}`);
    } else if (this.result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else if (nodes.length !== 1) {
      throw new Error(
        `expected 1 root, got ${nodes.length} in ${this.repr()}: ${nodes.map((n) => n.repr()).join(", ")}`,
      );
    } else {
      return nodes[0];
    }
  }

  /** Get the list of main Nodes. */
  toList(): T[] {
    const nodes = this.nodes;
    if (this.type !== QueryType.NODE) {
      throw new Error(`not a node Query: ${this.query.repr()}`);
    } else if (this.result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else {
      return nodes;
    }
  }

  /** Get the count. */
  toCount(): number {
    const result = this.result;
    if (this.type !== QueryType.SCALAR) {
      throw new Error(`not a scalar Query: ${this.query.repr()}`);
    } else if (result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else if (result.count == null) {
      throw new Error(`no count in ${this.repr()}`);
    } else {
      return result.count;
    }
  }

  /** Get whether any results exist. */
  toExists(): boolean {
    const result = this.result;
    if (this.type !== QueryType.SCALAR) {
      throw new Error(`not a scalar Query: ${this.query.repr()}`);
    } else if (result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else if (result.exists == null) {
      throw new Error(`no exists in ${this.repr()}`);
    }
    return result.exists;
  }

  /** Get the scalar value. */
  toScalar(): any {
    const result = this.result;
    if (this.type !== QueryType.SCALAR) {
      throw new Error(`not a scalar Query: ${this.query.repr()}`);
    } else if (result == null) {
      throw new Error(`no result for ${this.repr()}`);
    } else if (result.scalar == null) {
      throw new Error(`no scalar in ${this.repr()}`);
    }
    return result.scalar.unpack();
  }

  /** Get the scalar value by group. */
  toScalarByGroup(): Record<string, any> {
    const children = this.children;
    if (this.type !== QueryType.GROUPED_SCALAR) {
      throw new Error(`not a grouped scalar Query: ${this.query.repr()}`);
    }
    const scalarByGroup: Record<string, any> = {};
    for (const subcontainer of children) {
      if (subcontainer.discriminator == null) {
        continue;
      }
      const discriminator = subcontainer.discriminator.unpack();
      scalarByGroup[discriminator] = subcontainer.toScalar();
    }
    return scalarByGroup;
  }

  /** Get the list of main Nodes by group. */
  toListByGroup(): Record<string, T[]> {
    const children = this.children;
    if (this.type !== QueryType.GROUPED_NODE) {
      throw new Error(`not a grouped node Query: ${this.query.repr()}`);
    }
    const listByGroup: Record<string, T[]> = {};
    for (const subcontainer of children) {
      if (subcontainer.discriminator == null) {
        continue;
      }
      const discriminator = subcontainer.discriminator.unpack();
      listByGroup[discriminator] = subcontainer.toList() as T[];
    }
    return listByGroup;
  }

  /** Get a subresult by name or id. */
  get(key: string): QueryContainer {
    for (const subcontainer of this.children) {
      if (subcontainer.query.name === key || subcontainer.query.id === key) {
        return subcontainer;
      }
    }
    throw new Error(`no subresult for ${key} in ${this.repr()}`);
  }
}

/** A connection to a Query and its result. */
export class QueryConnection<T extends Node = Node> extends QueryContainer<T> {
  readonly store: Store;
  readonly session: Session;
  readonly graph: Graph;

  constructor(options: { query: Query; store: Store; session: Session }) {
    super({
      connection: null as any, // assigned below
      type: options.query.type,
      query: options.query,
      result: null,
      discriminator: null,
    });
    // @ts-expect-error(readonly): can't pass this into super
    this.connection = this;
    this.store = options.store;
    this.session = options.session;
    this.graph = this.session.supergraph.createPolyGraph();
  }

  repr(): string {
    return `<QueryConnection query=${this.query.repr()}>`;
  }

  /** Execute the Query. */
  async execute(): Promise<void> {
    this.result = await this.store.query(this.query);
    this._addResult(this.result, this.query);
  }

  close(): void {
    throw new Error("not implemented");
  }

  async waitClosed(): Promise<void> {
    throw new Error("not implemented");
  }
}
