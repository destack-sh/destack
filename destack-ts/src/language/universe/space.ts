import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsFollowable,
  IsGlobal,
  IsJoinable,
  IsOwnable,
  IsOwner,
  IsSpatial,
  IsStarable,
  IsSubject,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Materialization,
  Node,
  NodeType,
  Region,
  StructType,
} from "@destack/language/core";
import type { Database } from "@destack/language/infrastructure";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Folder } from "@destack/language/space";
import type { Handle } from "@destack/language/universe/handle";
import { MaterializationProto, RegionProto, SpaceProto, SpaceStatusProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:20001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:20001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:20100 ==== */
/**
 * A Space is the home of your personal software studio.
 */
export class Space
  extends Entity
  implements IsGlobal, IsFollowable, IsJoinable, IsOwnable, IsStarable, IsSpatial
{
  static metatype: NodeType = NodeType.SPACE;

  /**
   * Entity.parent
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
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Space | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): Space | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree (not the template tree).
   */
  get instanceRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instanceRootPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instanceRootPtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
   */
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
   */
  get updatedBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Entity & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsOwner) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

  /**
   * Space.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * Space.slug
   */
  get slug(): string {
    return this._slug;
  }
  set slug(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["slug"];
    this._session.updateSetProperty(this, prop, value);
    this._slug = value;
  }
  _slug: string;

  /**
   * Space.status
   */
  get status(): SpaceStatus {
    return this._status;
  }
  set status(value: SpaceStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: SpaceStatus;

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
  get handlePtr(): NodeReference | null {
    return this._handlePtr;
  }
  set handlePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["handle"];
    this._session.updateSetProperty(this, prop, value);
    this._handlePtr = value;
  }
  _handlePtr: NodeReference | null;

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
  get systemFolderPtr(): NodeReference | null {
    return this._systemFolderPtr;
  }
  set systemFolderPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["system_folder"];
    this._session.updateSetProperty(this, prop, value);
    this._systemFolderPtr = value;
  }
  _systemFolderPtr: NodeReference | null;

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
  get homeFolderPtr(): NodeReference | null {
    return this._homeFolderPtr;
  }
  set homeFolderPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["home_folder"];
    this._session.updateSetProperty(this, prop, value);
    this._homeFolderPtr = value;
  }
  _homeFolderPtr: NodeReference | null;

  /**
   * Space.region
   */
  get region(): Region {
    return this._region;
  }
  set region(value: Region) {
    const prop = (this.constructor as NodeClass).__properties__["region"];
    this._session.updateSetProperty(this, prop, value);
    this._region = value;
  }
  _region: Region;

  /**
   * Space.galaxyName
   */
  get galaxyName(): string | null {
    return this._galaxyName;
  }
  set galaxyName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["galaxy_name"];
    this._session.updateSetProperty(this, prop, value);
    this._galaxyName = value;
  }
  _galaxyName: string | null;

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
  get databasePtr(): NodeReference | null {
    return this._databasePtr;
  }
  set databasePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["database"];
    this._session.updateSetProperty(this, prop, value);
    this._databasePtr = value;
  }
  _databasePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Space | NodeReference | null;
    template?: Space | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsSubject) | NodeReference | null;
    ownedBy?: (Entity & IsOwner) | NodeReference | null;
    name: string;
    slug: string;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 32 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`Space.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _template = options.template ?? null;
    if (_template != null && _template.metatype != StructType.NODE_REFERENCE) {
      _template = (_template as Node).toRef();
    }
    this.templatePtr = _template;
    let _instanceRoot = options.instanceRoot ?? null;
    if (_instanceRoot != null && _instanceRoot.metatype != StructType.NODE_REFERENCE) {
      _instanceRoot = (_instanceRoot as Node).toRef();
    }
    this.instanceRootPtr = _instanceRoot;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Space.name is required`);
    }
    this._name = _name;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Space.slug is required`);
    }
    this._slug = _slug;
    let _status = options.status;
    if (_status === null) {
      throw new Error(`Space.status is required`);
    }
    this._status = _status;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.metatype != StructType.NODE_REFERENCE) {
      _handle = (_handle as Node).toRef();
    }
    this._handlePtr = _handle;
    let _systemFolder = options.systemFolder ?? null;
    if (_systemFolder != null && _systemFolder.metatype != StructType.NODE_REFERENCE) {
      _systemFolder = (_systemFolder as Node).toRef();
    }
    this._systemFolderPtr = _systemFolder;
    let _homeFolder = options.homeFolder ?? null;
    if (_homeFolder != null && _homeFolder.metatype != StructType.NODE_REFERENCE) {
      _homeFolder = (_homeFolder as Node).toRef();
    }
    this._homeFolderPtr = _homeFolder;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Space.region is required`);
    }
    this._region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this._galaxyName = _galaxyName;
    let _database = options.database ?? null;
    if (_database != null && _database.metatype != StructType.NODE_REFERENCE) {
      _database = (_database as Node).toRef();
    }
    this._databasePtr = _database;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`Space.createdAt and Space.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.metatype == StructType.NODE_REFERENCE
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this._slug === other._slug)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this._handlePtr?.id === other._handlePtr?.id)) {
      return false;
    }
    if (!(this._systemFolderPtr?.id === other._systemFolderPtr?.id)) {
      return false;
    }
    if (!(this._homeFolderPtr?.id === other._homeFolderPtr?.id)) {
      return false;
    }
    if (!(this._region === other._region)) {
      return false;
    }
    if (!(this._galaxyName === other._galaxyName)) {
      return false;
    }
    if (!(this._databasePtr?.id === other._databasePtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this.templatePtr?.id === other.templatePtr?.id)) {
      return false;
    }
    if (!(this.instanceRootPtr?.id === other.instanceRootPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    h = (h * 31 + this._status) & 0xffffffff;
    if (this._handlePtr !== null) {
      h = (h * 31 + hashString(this._handlePtr.id)) & 0xffffffff;
    }
    if (this._systemFolderPtr !== null) {
      h = (h * 31 + hashString(this._systemFolderPtr.id)) & 0xffffffff;
    }
    if (this._homeFolderPtr !== null) {
      h = (h * 31 + hashString(this._homeFolderPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._region) & 0xffffffff;
    if (this._galaxyName !== null) {
      h = (h * 31 + hashString(this._galaxyName)) & 0xffffffff;
    }
    if (this._databasePtr !== null) {
      h = (h * 31 + hashString(this._databasePtr.id)) & 0xffffffff;
    }
    if (this._ownedByPtr !== null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
    }
    if (this.instanceRootPtr !== null) {
      h = (h * 31 + hashString(this.instanceRootPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SPACE,
      id: this.id,
      spaceId: this.id,
      snapshotId: this.snapshotPtr?.id ?? null,
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`name=${this.name}`);
    propertyReprs.push(`slug=${this.slug}`);
    propertyReprs.push(`status=${SpaceStatus[this.status]}`);
    if (this.ownedBy !== null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    return `<Space '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Space.__packValue__(this);
  }

  static __packValue__(object: Space): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 20100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    if (object.templatePtr != null) {
      objectValue["13"] = object.templatePtr.toValue();
    }
    if (object.instanceRootPtr != null) {
      objectValue["14"] = object.instanceRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object._ownedByPtr != null) {
      objectValue["28"] = object._ownedByPtr.toValue();
    }
    objectValue["101"] = object._name;
    objectValue["102"] = object._slug;
    objectValue["110"] = object._status;
    if (object._handlePtr != null) {
      objectValue["111"] = object._handlePtr.toValue();
    }
    if (object._systemFolderPtr != null) {
      objectValue["112"] = object._systemFolderPtr.toValue();
    }
    if (object._homeFolderPtr != null) {
      objectValue["113"] = object._homeFolderPtr.toValue();
    }
    objectValue["120"] = object._region;
    if (object._galaxyName != null) {
      objectValue["121"] = object._galaxyName;
    }
    if (object._databasePtr != null) {
      objectValue["122"] = object._databasePtr.toValue();
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const handlePtrValue = objectValue["111"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? _NodeReference.fromValue(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const systemFolderPtrValue = objectValue["112"];
    const unpackedSystemFolderPtr =
      systemFolderPtrValue != undefined
        ? _NodeReference.fromValue(systemFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const homeFolderPtrValue = objectValue["113"];
    const unpackedHomeFolderPtr =
      homeFolderPtrValue != undefined
        ? _NodeReference.fromValue(homeFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const galaxyNameValue = objectValue["121"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const databasePtrValue = objectValue["122"];
    const unpackedDatabasePtr =
      databasePtrValue != undefined
        ? _NodeReference.fromValue(databasePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["28"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const templatePtrValue = objectValue["13"];
    const unpackedTemplatePtr =
      templatePtrValue != undefined
        ? _NodeReference.fromValue(templatePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instanceRootPtrValue = objectValue["14"];
    const unpackedInstanceRootPtr =
      instanceRootPtrValue != undefined
        ? _NodeReference.fromValue(instanceRootPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Space({
      name: objectValue["101"],
      slug: objectValue["102"],
      status: Number(objectValue["110"]),
      handle: unpackedHandlePtr,
      systemFolder: unpackedSystemFolderPtr,
      homeFolder: unpackedHomeFolderPtr,
      region: Number(objectValue["120"]),
      galaxyName: unpackedGalaxyName,
      database: unpackedDatabasePtr,
      ownedBy: unpackedOwnedByPtr,
      space: unpackedSpacePtr,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
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
    const objectProto: Partial<SpaceProto> = { metatype: 20100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    if (object.templatePtr != null) {
      objectProto.templatePtr = object.templatePtr.toProto();
    }
    if (object.instanceRootPtr != null) {
      objectProto.instanceRootPtr = object.instanceRootPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.slug = object._slug;
    objectProto.status = Number(object._status) as SpaceStatusProto;
    if (object._handlePtr != null) {
      objectProto.handlePtr = object._handlePtr.toProto();
    }
    if (object._systemFolderPtr != null) {
      objectProto.systemFolderPtr = object._systemFolderPtr.toProto();
    }
    if (object._homeFolderPtr != null) {
      objectProto.homeFolderPtr = object._homeFolderPtr.toProto();
    }
    objectProto.region = Number(object._region) as RegionProto;
    if (object._galaxyName != null) {
      objectProto.galaxyName = object._galaxyName;
    }
    if (object._databasePtr != null) {
      objectProto.databasePtr = object._databasePtr.toProto();
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Space({
      name: objectProto.name,
      slug: objectProto.slug,
      status: Number(objectProto.status) as SpaceStatus,
      handle:
        objectProto.handlePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.handlePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      systemFolder:
        objectProto.systemFolderPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.systemFolderPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      homeFolder:
        objectProto.homeFolderPtr != undefined
          ? _NodeReference.fromProto(
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
          ? _NodeReference.fromProto(
              objectProto.databasePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      template:
        objectProto.templatePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.templatePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instanceRoot:
        objectProto.instanceRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instanceRootPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
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
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
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
/* ==== DESTACK_GENERATED_END:NODE:20100 ==== */
