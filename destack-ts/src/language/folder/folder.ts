import { Agent, IsTaggable, IsFollowable, BuiltinObject, ACTIVE_SESSION, Organization, Session, Scene, StructFrozen, IsOrdered, Graph, Struct, activeSession, StructType, IsStarable, NodeType, Role, Icon, Supergraph, QueryConnection, NodeReference, MaterializationType, IsOwnable, User, Spatial, Team, Space, IsDeletable, Entity, EnumType, IsJoinable, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:1000 ==== */
export enum FolderType {
  SYSTEM = 1,
  HOME = 2,
  GENERAL = 3,
  MODULE = 4,
  APP = 5,
}
/* ==== DESTACK_GENERATED_END:ENUM:1000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1000 ==== */
export class Folder extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsOwnable, IsJoinable, IsTaggable, IsStarable, IsFollowable {
  readonly id: string;
  get parent(): Space | Folder | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | Folder | null | null;
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
  readonly orderKey: string;
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
  type: FolderType;
  name: string;
  slug: string | null;
  icon: Icon | null;
  get mainScene(): Scene | null | null {
      const nodePtr: NodeReference | null = this.mainScenePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | null | null;
      }
      return null;
  }

  set mainScene(node: Scene | null) {
      if (node === null) {
          this.mainScenePtr = null;
      } else {
          this.mainScenePtr = node.toRef();
      }
  }
  ;
  mainScenePtr: NodeReference | null

  constructor(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    type?: FolderType,
    name: string,
    slug?: string | null,
    icon?: Icon | null,
    mainScene?: Scene | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.type = options.type ?? FolderType.GENERAL;
    this.name = options.name;
    this.slug = options.slug ?? null;
    this.icon = options.icon ?? null;
    this.mainScenePtr = options.mainScene != null ? (options.mainScene.metatype == StructType.NODE_REFERENCE ? (options.mainScene as NodeReference) : (options.mainScene as Node).toRef()) : null;
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
    return new NodeReference(NodeType.FOLDER, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */