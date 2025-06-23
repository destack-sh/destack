import { activeSession, NodeType, TraitType } from "@/language/core";
import { NodeReference } from "@/language/core/common/relation";
import { Graph, QueryConnection, Session, Supergraph } from "@/language/core/runtime";
import { NodeTypeMapping, TraitTypeMapping } from "@/language/registry";
import { BuiltinObject } from "./object";

/** A Node is a collection of properties with an identity. */
export abstract class Node extends BuiltinObject {
  static readonly __isNode__: boolean = true;
  static readonly metatype: NodeType;
  static readonly __traits__: TraitType[];
  static readonly __rootType__: NodeType | null;
  static readonly __parentTypes__: NodeType[];
  static readonly __childTypes__: NodeType[];
  static readonly __ancestorTypes__: NodeType[];
  static readonly __descendantTypes__: NodeType[];

  readonly id: string;
  get parent(): Node | null {
    if (this.parentPtr === null) {
      return null;
    }
    return this._supergraph.get(this.parentPtr.id);
  }
  readonly parentPtr: NodeReference | null;

  // runtime
  _session: Session;
  _supergraph: Supergraph;
  _graph: Graph;
  _connection: QueryConnection | null;
  _hash: string | null;
  _ref: NodeReference | null;
  _isNew: boolean;
  _isAttached: boolean;
  _dirty: Record<string, any> | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    _session: Session | null,
    _supergraph: Supergraph | null,
    _graph: Graph | null,
    _connection: QueryConnection | null,
    _isNew: boolean,
    _isAttached: boolean,
  ) {
    super(_supergraph);
    this.id = id;
    this.parentPtr = parentPtr;
    this._session = _session ?? activeSession();
    this._supergraph = _supergraph ?? this._session.supergraph;
    this._graph = _graph;
    this._connection = _connection;
    this._hash = this.id;
    this._ref = null;
    this._dirty = null;
    this._isNew = _isNew;
    this._isAttached = _isAttached;
  }

  get metatype(): NodeType {
    return (this.constructor as typeof Node).metatype;
  }

  get __traits__(): TraitType[] {
    return (this.constructor as typeof Node).__traits__;
  }

  get __rootType__(): NodeType | null {
    return (this.constructor as typeof Node).__rootType__;
  }

  get __parentTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__parentTypes__;
  }

  get __childTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__childTypes__;
  }

  get __ancestorTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__ancestorTypes__;
  }

  get __descendantTypes__(): NodeType[] {
    return (this.constructor as typeof Node).__descendantTypes__;
  }

  get _pathKey(): string {
    throw new Error("not implemented");
  }

  get path(): string {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    throw new Error("not implemented");
  }

  toRef(): NodeReference {
    if (this._ref === null) {
      this._ref = this.__toRef__();
    }
    return this._ref;
  }

  /** Append a child to this Node. */
  addChild(child: Node, after?: Node, before?: Node): void {
    throw new Error("not implemented");
  }

  /** Append multiple children to this Node. */
  addChildren(children: Node[], after?: Node, before?: Node): void {
    throw new Error("not implemented");
  }

  /** Remove a child from this Node. */
  removeChild(child: Node): void {
    throw new Error("not implemented");
  }

  /** Get the children of this Node. */
  getChildren(node_type?: Node): Node[] {
    throw new Error("not implemented");
  }

  /** Get a specific child of this Node by name. */
  getChild(node_type: Node, name: string): Node | null {
    throw new Error("not implemented");
  }

  /** Get a specific child of this Node by name, or raises an error if not found. */
  child(node_type: Node, name: string): Node {
    throw new Error("not implemented");
  }

  /** Get the descendants of this Node. */
  getDescendants(node_type?: Node): Node[] {
    throw new Error("not implemented");
  }
}

/** A Node constructor. */
export type NodeClass = { new (...args: any[]): Node } & {
  metatype: NodeType;
  __traits__: TraitType[];
  __rootType__: NodeType | null;
  __parentTypes__: NodeType[];
  __childTypes__: NodeType[];
  __ancestorTypes__: NodeType[];
  __descendantTypes__: NodeType[];
};

/** Check if a value is a Node of a specific type. */
export function isNode<T extends NodeType>(value: any, nodeType?: T): value is NodeTypeMapping[T] {
  return value instanceof Node && (nodeType === undefined || value.metatype === nodeType);
}

/** Check if a value is a Node with a specific trait. */
export function isNodeWithTrait<T extends TraitType>(value: any, traitType: T): value is Node & TraitTypeMapping[T] {
  return value instanceof Node && value.__traits__.includes(traitType);
}
