import { Agent, BuiltinObject, ACTIVE_SESSION, Organization, Session, Scene, StructFrozen, Graph, Struct, activeSession, StructType, NodeType, Role, Icon, Supergraph, QueryConnection, Length, NodeReference, MaterializationType, IsOwnable, User, Spatial, Team, Space, CustomViewDefinition, IsDeletable, Entity, EnumType, Layer, Node, IsTracked } from '@/language';
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
  get parent(): Scene | Layer | CustomViewDefinition | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | Layer | CustomViewDefinition | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
      if (node === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = node.toRef();
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

  constructor(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    type: VariantType,
    name: string,
    slug?: string | null,
    icon?: Icon | null,
    maxWidth?: Length | null,
    maxHeight?: Length | null,
    minWidth?: Length | null,
    minHeight?: Length | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.type = options.type;
    this.name = options.name;
    this.slug = options.slug ?? null;
    this.icon = options.icon ?? null;
    this.maxWidth = options.maxWidth ?? null;
    this.maxHeight = options.maxHeight ?? null;
    this.minWidth = options.minWidth ?? null;
    this.minHeight = options.minHeight ?? null;
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
      return this.slug ?? this.name;
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