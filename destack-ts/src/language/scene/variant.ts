import { Session, Spatial, Entity, IsDeletable, Organization, CustomViewDefinition, Layer, User, NodeReference, BuiltinObject, IsOwnable, Agent, MaterializationType, IsTracked, Scene, Graph, Role, Icon, Struct, Supergraph, Length, QueryConnection, NodeType, Team, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:9030 ==== */
export enum VariantType {
  DYNAMIC = 1,
  BREAKPOINT = 2,
  PLATFORM = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:9030 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:9031 ==== */
export enum VariantStateType {
  LOADING = 10,
  ERROR = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:9031 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9030 ==== */
export class Variant extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOwnable {
  readonly id: string;
  get parent(): Scene | Layer | CustomViewDefinition | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | Layer | CustomViewDefinition | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  type: VariantType;
  name: string;
  slug: string | null;
  icon: Icon | null;
  maxWidth: Length | null;
  maxHeight: Length | null;
  minWidth: Length | null;
  minHeight: Length | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    ownedByPtr: NodeReference | null,
    type: VariantType,
    name: string,
    slug: string | null,
    icon: Icon | null,
    maxWidth: Length | null,
    maxHeight: Length | null,
    minWidth: Length | null,
    minHeight: Length | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.ownedByPtr = ownedByPtr;
    this.type = type;
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.maxWidth = maxWidth;
    this.maxHeight = maxHeight;
    this.minWidth = minWidth;
    this.minHeight = minHeight;
  }


  static create(): Variant {

    return new Variant();
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.VARIANT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return this.slug or this.name;
  }

  get path(): string {
      const pathParts: string[] = [];
      let node: Node | null = this;
      while (node !== null) {
          pathParts.push(node._pathKey);
          node = node.parent;
      }
      if (!this._isAttached) {
          pathParts.push("<detached>");
      }
      return pathParts.reverse().join("/");
  }
}
/* ==== DESTACK_GENERATED_END:NODE:9030 ==== */