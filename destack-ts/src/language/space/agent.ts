import { IsFollowable, IsScriptable, BuiltinObject, ACTIVE_SESSION, Folder, Session, StructFrozen, ThreadCursor, Graph, Struct, ScreenCursor, activeSession, StructType, NodeType, IsSubject, EventCursor, Icon, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Spatial, Space, IsDeletable, Entity, Script, EnumType, IsOwner, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:600 ==== */
export class Agent extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsSubject, IsOwner, IsScriptable, IsFollowable {
  readonly id: string;
  get parent(): Folder | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null | null;
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
  name: string;
  slug: string;
  icon: Icon | null;
  get cursor(): EventCursor | ScreenCursor | ThreadCursor | null | null {
      const nodePtr: NodeReference | null = this.cursorPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as EventCursor | ScreenCursor | ThreadCursor | null | null;
      }
      return null;
  }

  set cursor(node: EventCursor | ScreenCursor | ThreadCursor | null) {
      if (node === null) {
          this.cursorPtr = null;
      } else {
          this.cursorPtr = node.toRef();
      }
  }
  ;
  cursorPtr: NodeReference | null
  get script(): Script | null | null {
      const nodePtr: NodeReference | null = this.scriptPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }

  set script(node: Script | null) {
      if (node === null) {
          this.scriptPtr = null;
      } else {
          this.scriptPtr = node.toRef();
      }
  }
  ;
  scriptPtr: NodeReference | null

  constructor(options: {
    name: string,
    slug: string,
    icon?: Icon | null,
    cursor?: EventCursor | ScreenCursor | ThreadCursor | NodeReference | null,
    script?: Script | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
    this.slug = options.slug;
    this.icon = options.icon ?? null;
    this.cursorPtr = options.cursor != null ? (options.cursor.metatype == StructType.NODE_REFERENCE ? (options.cursor as NodeReference) : (options.cursor as Node).toRef()) : null;
    this.scriptPtr = options.script != null ? (options.script.metatype == StructType.NODE_REFERENCE ? (options.script as NodeReference) : (options.script as Node).toRef()) : null;
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
    return new NodeReference(NodeType.AGENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:600 ==== */