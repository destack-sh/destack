import { Graph, IsFollowable, IsDeletable, MaterializationType, Spatial, StructFrozen, IsScriptable, Struct, EnumType, ThreadCursor, Script, IsTracked, Entity, QueryConnection, NodeReference, ScreenCursor, Node, IsSubject, Folder, NodeType, Session, IsOwner, Space, User, StructType, Icon, Supergraph, EventCursor, BuiltinObject } from '@/language';
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

  set cursor(value: EventCursor | ScreenCursor | ThreadCursor | null) {
      if (value === null) {
          this.cursorPtr = null;
      } else {
          this.cursorPtr = value.toRef();
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

  set script(value: Script | null) {
      if (value === null) {
          this.scriptPtr = null;
      } else {
          this.scriptPtr = value.toRef();
      }
  }
  ;
  scriptPtr: NodeReference | null

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
    name: string,
    slug: string,
    icon: Icon | null,
    cursorPtr: NodeReference | null,
    scriptPtr: NodeReference | null,
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
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.cursorPtr = cursorPtr;
    this.scriptPtr = scriptPtr;
  }


  static create(options: {
    name: string,
    slug: string,
    icon?: Icon | null,
    cursor?: EventCursor | ScreenCursor | ThreadCursor | NodeReference | null,
    script?: Script | NodeReference | null
  }): Agent {

    return new Agent(

    );
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
/* ==== DESTACK_GENERATED_END:NODE:600 ==== */