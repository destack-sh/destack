import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { EnumType, NodeType, Region, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT } from "@destack/language/core/builtin/const";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsActor,
  IsFollowable,
  IsJoinable,
  IsOwnable,
  IsStarable,
} from "@destack/language/core/builtin/trait";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import { BranchType, SnapshotType } from "@destack/language/core/common/time";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Database } from "@destack/language/infrastructure";
import type { Script } from "@destack/language/logic";
import {
  NODE_CLASS_BY_TYPE,
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Folder } from "@destack/language/space";
import type { Handle } from "@destack/language/universe";
import { MaterializationProto, RegionProto, SpaceProto, SpaceStatusProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { uuid4 } from "@destack/utils/uuid";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:100000 ==== */
/**
 * SpaceStatus
 */
export enum SpaceStatus {
  CREATING = 1,
  ACTIVE = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SPACE_STATUS, SpaceStatus);
/* ==== DESTACK_GENERATED_END:ENUM:100000 ==== */

/** Create a new Space with a root Branch and Snapshot. */
export function createSpace(options: {
  session: Session;
  id?: string;
  name?: string;
  slug?: string;
}): {
  space: Space;
  branch: Branch;
  snapshot: Snapshot;
} {
  const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
  const _Branch = NODE_CLASS_BY_TYPE[NodeType.BRANCH] as typeof Branch;
  const _Snapshot = NODE_CLASS_BY_TYPE[NodeType.SNAPSHOT] as typeof Snapshot;
  const _Space = NODE_CLASS_BY_TYPE[NodeType.SPACE] as typeof Space;

  const { session, id, name, slug } = options;
  const epoch = session.epoch;
  const now = Temporal.Now.zonedDateTimeISO("UTC");

  const spaceId = id ?? uuid4();
  const spacePtr = new _NodeReference({
    type: NodeType.SPACE,
    id: spaceId,
    spaceId,
  });

  const branchId = uuid4();
  const branchPtr = new _NodeReference({
    type: NodeType.BRANCH,
    id: branchId,
    spaceId,
  });

  const snapshotId = uuid4();
  const snapshotPtr = new _NodeReference({
    type: NodeType.SNAPSHOT,
    id: snapshotId,
    spaceId,
    branchId,
  });

  const branch = new _Branch({
    id: branchId,
    name: "Main",
    space: spacePtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
    type: BranchType.ROOT,
    branch: branchPtr,
    snapshot: snapshotPtr,
  });
  session.create(branch);
  const snapshot = new _Snapshot({
    id: snapshotId,
    name: "Root",
    space: spacePtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
    type: SnapshotType.FULL,
    branch: branchPtr,
  });
  session.create(snapshot);
  const space = new _Space({
    id: spaceId,
    name: name ?? "Space",
    slug: slug ?? "space",
    status: SpaceStatus.ACTIVE,
    region: Region.ZURICH,
    branch: branchPtr,
    snapshot: snapshotPtr,
    createdEpoch: epoch,
    createdAt: now,
    updatedEpoch: epoch,
    updatedAt: now,
  });
  session.create(space);

  return { space, branch, snapshot };
}

/* ==== DESTACK_GENERATED_START:NODE:1000 ==== */
/**
 * A Space is the home of your personal software studio.
 */
export class Space extends Entity implements IsFollowable, IsJoinable, IsOwnable, IsStarable {
  static metatype: NodeType = NodeType.SPACE;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Space | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
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
   * Entity.name
   */
  /**
   * Entity.name
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
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
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
  /**
   * The Script of this Entity.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

  /**
   * Space.slug
   */
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Handle | null;
    }
    return null;
  }
  set handle(node: Handle | null) {
    if (node === null) {
      this.handlePtr = null;
    } else {
      this.handlePtr = node.toRef();
    }
  }
  /**
   * Space.handle
   */
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
    }
    return null;
  }
  set systemFolder(node: Folder | null) {
    if (node === null) {
      this.systemFolderPtr = null;
    } else {
      this.systemFolderPtr = node.toRef();
    }
  }
  /**
   * The system Folder.
   */
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
    }
    return null;
  }
  set homeFolder(node: Folder | null) {
    if (node === null) {
      this.homeFolderPtr = null;
    } else {
      this.homeFolderPtr = node.toRef();
    }
  }
  /**
   * The home Folder.
   */
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Database | null;
    }
    return null;
  }
  set database(node: Database | null) {
    if (node === null) {
      this.databasePtr = null;
    } else {
      this.databasePtr = node.toRef();
    }
  }
  /**
   * Space.database
   */
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
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Space | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
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
        ? options.parent.constructor.name == "NodeReference"
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
      // _isNew
      options.id == null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name != "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = this.toRef();
    }
    if (_space === null) {
      throw new Error(`Space.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Space.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for Space`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Space.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Space`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Space.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name != "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name != "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Space";
    }
    if (_name === null) {
      throw new Error(`Space.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Space.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name != "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name != "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
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
    if (_handle != null && _handle.constructor.name != "NodeReference") {
      _handle = (_handle as Node).toRef();
    }
    this._handlePtr = _handle as NodeReference | null;
    let _systemFolder = options.systemFolder ?? null;
    if (_systemFolder != null && _systemFolder.constructor.name != "NodeReference") {
      _systemFolder = (_systemFolder as Node).toRef();
    }
    this._systemFolderPtr = _systemFolder as NodeReference | null;
    let _homeFolder = options.homeFolder ?? null;
    if (_homeFolder != null && _homeFolder.constructor.name != "NodeReference") {
      _homeFolder = (_homeFolder as Node).toRef();
    }
    this._homeFolderPtr = _homeFolder as NodeReference | null;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Space.region is required`);
    }
    this._region = _region;
    let _galaxyName = options.galaxyName ?? null;
    this._galaxyName = _galaxyName;
    let _database = options.database ?? null;
    if (_database != null && _database.constructor.name != "NodeReference") {
      _database = (_database as Node).toRef();
    }
    this._databasePtr = _database as NodeReference | null;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(`Space.createdAt and Space.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name == "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    h = (h * 31 + this._status) & 0xffffffff;
    if (this._handlePtr != null) {
      h = (h * 31 + hashString(this._handlePtr.id)) & 0xffffffff;
    }
    if (this._systemFolderPtr != null) {
      h = (h * 31 + hashString(this._systemFolderPtr.id)) & 0xffffffff;
    }
    if (this._homeFolderPtr != null) {
      h = (h * 31 + hashString(this._homeFolderPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._region) & 0xffffffff;
    if (this._galaxyName != null) {
      h = (h * 31 + hashString(this._galaxyName)) & 0xffffffff;
    }
    if (this._databasePtr != null) {
      h = (h * 31 + hashString(this._databasePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
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
    propertyReprs.push(`slug=${`"${this.slug}"`}`);
    propertyReprs.push(`status=${SpaceStatus[this.status]}`);
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Space "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return Space.__packCson__(this);
  }

  static __packCson__(object: Space): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1000;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    objectCson["102"] = object._slug;
    objectCson["110"] = object._status;
    if (object._handlePtr != null) {
      objectCson["111"] = object._handlePtr.toCson();
    }
    if (object._systemFolderPtr != null) {
      objectCson["112"] = object._systemFolderPtr.toCson();
    }
    if (object._homeFolderPtr != null) {
      objectCson["113"] = object._homeFolderPtr.toCson();
    }
    objectCson["120"] = object._region;
    if (object._galaxyName != null) {
      objectCson["121"] = object._galaxyName;
    }
    if (object._databasePtr != null) {
      objectCson["122"] = object._databasePtr.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const handlePtrValue = objectCson["111"];
    const unpackedHandlePtr =
      handlePtrValue != undefined
        ? _NodeReference.fromCson(handlePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const systemFolderPtrValue = objectCson["112"];
    const unpackedSystemFolderPtr =
      systemFolderPtrValue != undefined
        ? _NodeReference.fromCson(systemFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const homeFolderPtrValue = objectCson["113"];
    const unpackedHomeFolderPtr =
      homeFolderPtrValue != undefined
        ? _NodeReference.fromCson(homeFolderPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const galaxyNameValue = objectCson["121"];
    const unpackedGalaxyName = galaxyNameValue != undefined ? galaxyNameValue : null;
    const databasePtrValue = objectCson["122"];
    const unpackedDatabasePtr =
      databasePtrValue != undefined
        ? _NodeReference.fromCson(databasePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new Space({
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      slug: objectCson["102"],
      status: Number(objectCson["110"]),
      handle: unpackedHandlePtr,
      systemFolder: unpackedSystemFolderPtr,
      homeFolder: unpackedHomeFolderPtr,
      region: Number(objectCson["120"]),
      galaxyName: unpackedGalaxyName,
      database: unpackedDatabasePtr,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Space {
    return Space.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): SpaceProto {
    return Space.__packProto__(this);
  }

  static __packProto__(object: Space): SpaceProto {
    const objectProto: Partial<SpaceProto> = { metatype: 1000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Space({
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
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
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */
