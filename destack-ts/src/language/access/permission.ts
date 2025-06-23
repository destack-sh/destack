import {
  Agent,
  Entity,
  Folder,
  Graph,
  Icon,
  IsDeletable,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Organization,
  QueryConnection,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  Team,
  Thread,
  TraitType,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:530 ==== */
export enum PermissionType {
  GENERAL = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:530 ==== */

/* ==== DESTACK_GENERATED_START:NODE:530 ==== */
export class Permission extends Node implements Spatial, Entity, IsTracked, IsDeletable {
  static metatype: NodeType = NodeType.PERMISSION;
  static __traits__: TraitType[] = [TraitType.TRACKED, TraitType.SPATIAL, TraitType.ENTITY, TraitType.DELETABLE];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.FOLDER,
    NodeType.ORGANIZATION,
    NodeType.TEAM,
    NodeType.THREAD,
  ];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.FOLDER,
    NodeType.ORGANIZATION,
    NodeType.TEAM,
    NodeType.THREAD,
  ];
  static __descendantTypes__: NodeType[] = [];

  get parent(): Folder | Thread | Organization | Space | Team | Folder | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | Thread | Organization | Space | Team | Folder | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  type: PermissionType;
  name: string;
  slug: string | null;
  icon: Icon | null;

  constructor(options: {
    id?: string;
    parent?: Folder | Thread | Organization | Space | Team | Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    type: PermissionType;
    name: string;
    slug?: string | null;
    icon?: Icon | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // is_new
      options.id == null,
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Permission.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Permission.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Permission.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
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

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.PERMISSION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
/* ==== DESTACK_GENERATED_END:NODE:530 ==== */
