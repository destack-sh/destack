import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, Graph, Struct, activeSession, Global, StructType, NodeType, Icon, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Entity, EnumType, IsOwner, IsJoinable, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:50 ==== */
export class Team extends Node implements Global, Entity, IsTracked, IsJoinable, IsOwner {
  readonly id: string;
  get parent(): Node | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
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
  name: string;
  slug: string | null;
  icon: Icon | null;

  constructor(options: {
    name: string,
    slug?: string | null,
    icon?: Icon | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
    this.slug = options.slug ?? null;
    this.icon = options.icon ?? null;
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
    return new NodeReference(NodeType.TEAM, this.id, null, null, this._supergraph);

  get _pathKey(): string {
      return this.slug ?? this.name;
  }

  get path(): string {
      return this.slug ?? this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:50 ==== */