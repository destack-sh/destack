import {
  Agent,
  Entity,
  Graph,
  Icon,
  IsDeletable,
  IsFollowable,
  IsJoinable,
  IsOrdered,
  IsOwnable,
  IsStarable,
  IsTaggable,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Organization,
  QueryConnection,
  Role,
  Scene,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  Team,
  TraitType,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

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
export class Folder
  extends Node
  implements
    Spatial,
    Entity,
    IsTracked,
    IsDeletable,
    IsOrdered,
    IsOwnable,
    IsJoinable,
    IsTaggable,
    IsStarable,
    IsFollowable
{
  static metatype: NodeType = NodeType.FOLDER;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.JOINABLE,
    TraitType.ORDERED,
    TraitType.STARABLE,
    TraitType.FOLLOWABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __childTypes__: NodeType[] = [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.ENTITLEMENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SCENE,
    NodeType.FOLLOW,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
  ];
  static __ancestorTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.MESSAGE,
    NodeType.ROLE,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.PERMISSION,
    NodeType.CUSTOM_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.FOLLOW,
    NodeType.WIZARD_VIEW,
    NodeType.SANCTION,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.ENTITLEMENT,
    NodeType.LABEL_VIEW,
    NodeType.CANVAS,
    NodeType.SCENE,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.VARIANT,
    NodeType.BORDER_STYLE,
    NodeType.ACTION,
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.ROUTE,
    NodeType.FIELD,
    NodeType.AGENT,
    NodeType.TEXT_VIEW,
    NodeType.OPTION,
    NodeType.CLIENT,
    NodeType.THREAD_VIEW,
    NodeType.FOLDER,
    NodeType.PALETTE,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.MEMBERSHIP,
    NodeType.COLOR_STYLE,
    NodeType.FONT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.THREAD,
    NodeType.INVITE,
  ];

  get parent(): Space | Folder | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | Folder | null | null;
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
  ownedByPtr: NodeReference | null;
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
  mainScenePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null;
    type?: FolderType;
    name: string;
    slug?: string | null;
    icon?: Icon | null;
    mainScene?: Scene | NodeReference | null;
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
      throw new Error(`Folder.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Folder.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = FolderType.GENERAL;
    }
    if (_type === null) {
      throw new Error(`Folder.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Folder.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _mainScene = options.mainScene ?? null;
    if (_mainScene != null && _mainScene instanceof Node) {
      _mainScene = _mainScene.toRef();
    }
    this.mainScenePtr = _mainScene;
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
      nodeType: NodeType.FOLDER,
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
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */
