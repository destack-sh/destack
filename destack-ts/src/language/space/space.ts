import {
  Database,
  Entity,
  Folder,
  Global,
  Graph,
  Handle,
  Icon,
  IsFollowable,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsStarable,
  IsSubject,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Region,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1 ==== */
export enum SpaceStatus {
  CREATING = 1,
  QUEUED = 3,
  RUNNING = 10,
  PAUSED = 20,
}
/* ==== DESTACK_GENERATED_END:ENUM:1 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1 ==== */
export class Space
  extends Node
  implements Global, Spatial, Entity, IsTracked, IsOwnable, IsJoinable, IsStarable, IsFollowable
{
  static metatype: NodeType = NodeType.SPACE;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.JOINABLE,
    TraitType.STARABLE,
    TraitType.FOLLOWABLE,
  ];
  static __rootType__: NodeType | null = null;
  static __parentTypes__: NodeType[] = [];
  static __childTypes__: NodeType[] = [
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.EDIT_EVENT,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.CUSTOM_EVENT,
    NodeType.GAUGE_METRIC,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_METRIC,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.HISTOGRAM_METRIC,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.SNAPSHOT,
    NodeType.BRANCH,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.ENTITLEMENT,
    NodeType.INVITE_EVENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE_EVENT,
    NodeType.ROLE,
    NodeType.SANCTION_EVENT,
    NodeType.SANCTION,
    NodeType.FILE,
    NodeType.LINK,
    NodeType.ENVIRONMENT,
    NodeType.FOLDER,
    NodeType.DATABASE,
    NodeType.MACHINE,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.SERVICE,
    NodeType.TIMER_EVENT,
    NodeType.TIMER,
    NodeType.TRIGGER_EVENT,
    NodeType.TRIGGER,
    NodeType.LOG,
    NodeType.RUN_EVENT,
    NodeType.RUN,
    NodeType.SCENE_EVENT,
    NodeType.WINDOW,
    NodeType.FOLLOW,
    NodeType.NOTIFICATION_EVENT,
    NodeType.NOTIFICATION,
    NodeType.STAR,
    NodeType.FRIENDSHIP_INVITE_EVENT,
    NodeType.HANDLE,
    NodeType.THEME,
  ];
  static __ancestorTypes__: NodeType[] = [];
  static __descendantTypes__: NodeType[] = [
    NodeType.LOG,
    NodeType.ROLE,
    NodeType.ROLE_EVENT,
    NodeType.HANDLE,
    NodeType.GAUGE_METRIC,
    NodeType.GAUGE_MEASUREMENT,
    NodeType.COUNTER_METRIC,
    NodeType.COUNTER_MEASUREMENT,
    NodeType.PERMISSION,
    NodeType.HISTOGRAM_MEASUREMENT,
    NodeType.HISTOGRAM_METRIC,
    NodeType.EVENT_CURSOR,
    NodeType.SCREEN_CURSOR,
    NodeType.THREAD_CURSOR,
    NodeType.SANCTION,
    NodeType.FRIENDSHIP_INVITE_EVENT,
    NodeType.SANCTION_EVENT,
    NodeType.ENTITLEMENT,
    NodeType.ENTITLEMENT_EVENT,
    NodeType.AGENT,
    NodeType.CLIENT,
    NodeType.CUSTOM_EVENT_DEFINITION,
    NodeType.CUSTOM_EVENT,
    NodeType.EDIT_EVENT,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.THEME,
    NodeType.PALETTE,
    NodeType.COLOR_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.CANVAS,
    NodeType.SHADOW_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.SCENE_EVENT,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.VARIANT,
    NodeType.DATABASE,
    NodeType.THREAD_VIEW,
    NodeType.THREAD,
    NodeType.MESSAGE,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.ENVIRONMENT,
    NodeType.FOLLOW,
    NodeType.WIZARD_VIEW,
    NodeType.RUN,
    NodeType.RUN_EVENT,
    NodeType.SPAN,
    NodeType.MACHINE,
    NodeType.INTERRUPTION,
    NodeType.SCRIPT,
    NodeType.SERVICE,
    NodeType.CUSTOM_STRUCT_DEFINITION,
    NodeType.ACTION,
    NodeType.CUSTOM_ENUM_DEFINITION,
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.ROUTE,
    NodeType.FIELD,
    NodeType.TEXT_VIEW,
    NodeType.SNAPSHOT,
    NodeType.NOTIFICATION,
    NodeType.NOTIFICATION_EVENT,
    NodeType.OPTION,
    NodeType.TRIGGER,
    NodeType.TRIGGER_EVENT,
    NodeType.BRANCH,
    NodeType.FOLDER,
    NodeType.TIMER,
    NodeType.TIMER_EVENT,
    NodeType.FILE,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.MEMBERSHIP,
    NodeType.MEMBERSHIP_EVENT,
    NodeType.LINK,
    NodeType.INVITE,
    NodeType.INVITE_EVENT,
  ];

  get parent(): Node | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null | null;
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
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  get ownedBy(): (Node & IsOwner) | null | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null | null;
    }
    return null;
  }

  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: SpaceStatus;
  get handle(): Handle | null | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null | null;
    }
    return null;
  }
  readonly handlePtr: NodeReference | null;
  get systemFolder(): Folder | null | null {
    const nodePtr: NodeReference | null = this.systemFolderPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null | null;
    }
    return null;
  }
  readonly systemFolderPtr: NodeReference | null;
  get homeFolder(): Folder | null | null {
    const nodePtr: NodeReference | null = this.homeFolderPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null | null;
    }
    return null;
  }
  readonly homeFolderPtr: NodeReference | null;
  readonly region: Region;
  readonly galaxyName: string | null;
  get database(): Database | null | null {
    const nodePtr: NodeReference | null = this.databasePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Database | null | null;
    }
    return null;
  }
  readonly databasePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    name: string;
    slug: string;
    icon?: Icon | null;
    status: SpaceStatus;
    handle?: Handle | NodeReference | null;
    systemFolder?: Folder | NodeReference | null;
    homeFolder?: Folder | NodeReference | null;
    region: Region;
    galaxyName?: string | null;
    database?: Database | NodeReference | null;
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
      true,
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
      throw new Error(`Space.materialization is required`);
    }
    this.materialization = _materialization;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Space.name is required`);
    }
    this.name = _name;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Space.slug is required`);
    }
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _status = options.status;
    if (_status === null) {
      throw new Error(`Space.status is required`);
    }
    this.status = _status;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle instanceof Node) {
      _handle = _handle.toRef();
    }
    this.handlePtr = _handle;
    let _systemFolder = options.systemFolder ?? null;
    if (_systemFolder != null && _systemFolder instanceof Node) {
      _systemFolder = _systemFolder.toRef();
    }
    this.systemFolderPtr = _systemFolder;
    let _homeFolder = options.homeFolder ?? null;
    if (_homeFolder != null && _homeFolder instanceof Node) {
      _homeFolder = _homeFolder.toRef();
    }
    this.homeFolderPtr = _homeFolder;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Space.region is required`);
    }
    this.region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this.galaxyName = _galaxyName;
    let _database = options.database ?? null;
    if (_database != null && _database instanceof Node) {
      _database = _database.toRef();
    }
    this.databasePtr = _database;

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
      nodeType: NodeType.SPACE,
      id: this.id,
      spaceId: this.id,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
  }

  get path(): string {
    return this.slug ?? this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:1 ==== */
