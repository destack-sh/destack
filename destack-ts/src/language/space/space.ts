import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  Entity,
  EnumType,
  Global,
  HasIcon,
  HasName,
  HasSlug,
  IsFollowable,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsStarable,
  IsSubject,
  MaterializationType,
  Node,
  NodeType,
  Region,
  Spatial,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Icon } from "@destack/language/core/common";
import { Folder } from "@destack/language/folder";
import { Database } from "@destack/language/infra";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Handle } from "@destack/language/space";
import {
  MaterializationTypeProto,
  RegionProto,
  SpaceProto,
  SpaceStatusProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1 ==== */
/**
 * SpaceStatus
 */
export enum SpaceStatus {
  CREATING = 1,
  QUEUED = 3,
  RUNNING = 10,
  PAUSED = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SPACE_STATUS, SpaceStatus);
/* ==== DESTACK_GENERATED_END:ENUM:1 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1 ==== */
/**
 * A Space is the home of your personal software studio.
 */
export class Space
  extends Node
  implements
    Global,
    Entity,
    HasName,
    HasSlug,
    HasIcon,
    IsFollowable,
    IsJoinable,
    IsOwnable,
    IsStarable,
    Spatial
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
    NodeType.POINTER_DOWN_EVENT,
    NodeType.POINTER_UP_EVENT,
    NodeType.POINTER_MOVE_EVENT,
    NodeType.POINTER_ENTER_EVENT,
    NodeType.POINTER_OVER_EVENT,
    NodeType.POINTER_LEAVE_EVENT,
    NodeType.LONG_PRESS_EVENT,
    NodeType.LEFT_CLICK_EVENT,
    NodeType.RIGHT_CLICK_EVENT,
    NodeType.MIDDLE_CLICK_EVENT,
    NodeType.DOUBLE_CLICK_EVENT,
    NodeType.WHEEL_EVENT,
    NodeType.KEY_DOWN_EVENT,
    NodeType.KEY_UP_EVENT,
    NodeType.KEY_PRESS_EVENT,
    NodeType.DRAG_START_EVENT,
    NodeType.DRAG_END_EVENT,
    NodeType.DRAG_OVER_EVENT,
    NodeType.DRAG_ENTER_EVENT,
    NodeType.DRAG_LEAVE_EVENT,
    NodeType.DROP_EVENT,
    NodeType.COPY_EVENT,
    NodeType.CUT_EVENT,
    NodeType.PASTE_EVENT,
    NodeType.FOCUS_IN_EVENT,
    NodeType.FOCUS_OUT_EVENT,
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
    NodeType.POINTER_DOWN_EVENT,
    NodeType.POINTER_UP_EVENT,
    NodeType.POINTER_MOVE_EVENT,
    NodeType.POINTER_ENTER_EVENT,
    NodeType.POINTER_OVER_EVENT,
    NodeType.POINTER_LEAVE_EVENT,
    NodeType.LONG_PRESS_EVENT,
    NodeType.FRAME_VIEW,
    NodeType.LEFT_CLICK_EVENT,
    NodeType.RIGHT_CLICK_EVENT,
    NodeType.WINDOW,
    NodeType.DOUBLE_CLICK_EVENT,
    NodeType.WHEEL_EVENT,
    NodeType.MIDDLE_CLICK_EVENT,
    NodeType.LABEL_VIEW,
    NodeType.KEY_DOWN_EVENT,
    NodeType.KEY_UP_EVENT,
    NodeType.SCENE,
    NodeType.SCENE_EVENT,
    NodeType.KEY_PRESS_EVENT,
    NodeType.SPLIT_VIEW,
    NodeType.DRAG_START_EVENT,
    NodeType.DRAG_END_EVENT,
    NodeType.LAYER,
    NodeType.DRAG_ENTER_EVENT,
    NodeType.DRAG_LEAVE_EVENT,
    NodeType.DROP_EVENT,
    NodeType.DRAG_OVER_EVENT,
    NodeType.COPY_EVENT,
    NodeType.CUT_EVENT,
    NodeType.VARIANT,
    NodeType.PASTE_EVENT,
    NodeType.DATABASE,
    NodeType.FOCUS_IN_EVENT,
    NodeType.FOCUS_OUT_EVENT,
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
    NodeType.CUSTOM_PROPERTY,
    NodeType.TEXT_VIEW,
    NodeType.SNAPSHOT,
    NodeType.NOTIFICATION,
    NodeType.NOTIFICATION_EVENT,
    NodeType.CUSTOM_OPTION,
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

  /**
   * Trait.parent
   */
  get parent(): Node | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
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

  /**
   * Space.name
   */
  name: string;

  /**
   * Space.slug
   */
  slug: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * Space.status
   */
  readonly status: SpaceStatus;

  /**
   * Space.handle
   */
  get handle(): Handle | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null;
    }
    return null;
  }
  readonly handlePtr: NodeReference | null;

  /**
   * The system Folder.
   */
  get systemFolder(): Folder | null {
    const nodePtr: NodeReference | null = this.systemFolderPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
    }
    return null;
  }
  readonly systemFolderPtr: NodeReference | null;

  /**
   * The home Folder.
   */
  get homeFolder(): Folder | null {
    const nodePtr: NodeReference | null = this.homeFolderPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
    }
    return null;
  }
  readonly homeFolderPtr: NodeReference | null;

  /**
   * Space.region
   */
  readonly region: Region;

  /**
   * Space.galaxyName
   */
  readonly galaxyName: string | null;

  /**
   * Space.database
   */
  get database(): Database | null {
    const nodePtr: NodeReference | null = this.databasePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Database | null;
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
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.slug === other.slug)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.handlePtr?.id === other.handlePtr?.id)) {
      return false;
    }
    if (!(this.systemFolderPtr?.id === other.systemFolderPtr?.id)) {
      return false;
    }
    if (!(this.homeFolderPtr?.id === other.homeFolderPtr?.id)) {
      return false;
    }
    if (!(this.region === other.region)) {
      return false;
    }
    if (!(this.galaxyName === other.galaxyName)) {
      return false;
    }
    if (!(this.databasePtr?.id === other.databasePtr?.id)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.ownedByPtr?.id === other.ownedByPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
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

  toValue(): { [key: string]: any } {
    return Space.__packValue__(this);
  }

  static __packValue__(object: Space): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.ownedByPtr != null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    objectValue["31"] = object.name;
    objectValue["33"] = object.slug;
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    objectValue["40"] = object.status;
    if (object.handlePtr != null) {
      objectValue["41"] = object.handlePtr.toValue();
    }
    if (object.systemFolderPtr != null) {
      objectValue["42"] = object.systemFolderPtr.toValue();
    }
    if (object.homeFolderPtr != null) {
      objectValue["43"] = object.homeFolderPtr.toValue();
    }
    objectValue["50"] = object.region;
    if (object.galaxyName != null) {
      objectValue["51"] = object.galaxyName;
    }
    if (object.databasePtr != null) {
      objectValue["55"] = object.databasePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    const handlePtrValue = objectValue["41"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? NodeReference.fromValue(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const systemFolderPtrValue = objectValue["42"];
    const unpackedSystemFolderPtr =
      systemFolderPtrValue != undefined
        ? NodeReference.fromValue(systemFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const homeFolderPtrValue = objectValue["43"];
    const unpackedHomeFolderPtr =
      homeFolderPtrValue != undefined
        ? NodeReference.fromValue(homeFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const galaxyNameValue = objectValue["51"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const databasePtrValue = objectValue["55"];
    const unpackedDatabasePtr =
      databasePtrValue != undefined
        ? NodeReference.fromValue(databasePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["25"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Space({
      name: objectValue["31"],
      slug: objectValue["33"],
      status: Number(objectValue["40"]),
      handle: unpackedHandlePtr,
      systemFolder: unpackedSystemFolderPtr,
      homeFolder: unpackedHomeFolderPtr,
      region: Number(objectValue["50"]),
      galaxyName: unpackedGalaxyName,
      database: unpackedDatabasePtr,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      icon: unpackedIcon,
      ownedBy: unpackedOwnedByPtr,
      space: unpackedSpacePtr,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    return Space.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): SpaceProto {
    return Space.__packProto__(this);
  }

  static __packProto__(object: Space): SpaceProto {
    const objectProto: Partial<SpaceProto> = { metatype: 1 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationTypeProto;
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.ownedByPtr != null) {
      objectProto.ownedByPtr = object.ownedByPtr.toProto();
    }
    objectProto.name = object.name;
    objectProto.slug = object.slug;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    objectProto.status = Number(object.status) as SpaceStatusProto;
    if (object.handlePtr != null) {
      objectProto.handlePtr = object.handlePtr.toProto();
    }
    if (object.systemFolderPtr != null) {
      objectProto.systemFolderPtr = object.systemFolderPtr.toProto();
    }
    if (object.homeFolderPtr != null) {
      objectProto.homeFolderPtr = object.homeFolderPtr.toProto();
    }
    objectProto.region = Number(object.region) as RegionProto;
    if (object.galaxyName != null) {
      objectProto.galaxyName = object.galaxyName;
    }
    if (object.databasePtr != null) {
      objectProto.databasePtr = object.databasePtr.toProto();
    }
    return objectProto as SpaceProto;
  }

  static __unpackProto__(
    objectProto: SpaceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    return new Space({
      name: objectProto.name,
      slug: objectProto.slug,
      status: Number(objectProto.status) as SpaceStatus,
      handle:
        objectProto.handlePtr != undefined
          ? NodeReference.fromProto(
              objectProto.handlePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      systemFolder:
        objectProto.systemFolderPtr != undefined
          ? NodeReference.fromProto(
              objectProto.systemFolderPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      homeFolder:
        objectProto.homeFolderPtr != undefined
          ? NodeReference.fromProto(
              objectProto.homeFolderPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      region: Number(objectProto.region) as Region,
      galaxyName: objectProto.galaxyName != undefined ? objectProto.galaxyName : null,
      database:
        objectProto.databasePtr != undefined
          ? NodeReference.fromProto(
              objectProto.databasePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      icon:
        objectProto.icon != undefined
          ? Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: SpaceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    return Space.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Space {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SpaceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SPACE, Space);
/* ==== DESTACK_GENERATED_END:NODE:1 ==== */
