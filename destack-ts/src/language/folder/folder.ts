import { Graph, IsTaggable, IsStarable, IsDeletable, IsFollowable, Team, MaterializationType, Spatial, StructFrozen, IsOwnable, Struct, EnumType, IsTracked, Role, Entity, QueryConnection, NodeReference, IsJoinable, Node, Scene, IsOrdered, NodeType, Session, Agent, Space, User, Organization, StructType, Icon, Supergraph, BuiltinObject } from '@/language';
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

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
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

  set mainScene(value: Scene | null) {
      if (value === null) {
          this.mainScenePtr = null;
      } else {
          this.mainScenePtr = value.toRef();
      }
  }
  ;
  mainScenePtr: NodeReference | null

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
    orderKey: string,
    ownedByPtr: NodeReference | null,
    type: FolderType,
    name: string,
    slug: string | null,
    icon: Icon | null,
    mainScenePtr: NodeReference | null,
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
    this.orderKey = orderKey;
    this.ownedByPtr = ownedByPtr;
    this.type = type;
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.mainScenePtr = mainScenePtr;
  }


  static create(options: {
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    type?: FolderType,
    name: string,
    slug?: string | null,
    icon?: Icon | null,
    mainScene?: Scene | NodeReference | null
  }): Folder {

    return new Folder(

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
    return new NodeReference(NodeType.FOLDER, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */