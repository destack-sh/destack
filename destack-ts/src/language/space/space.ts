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
  NodeReference,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import { Entity, EnumType, Node, NodeType, Region, StructType } from "@destack/language/core";
import type { Folder } from "@destack/language/folder";
import type { Database } from "@destack/language/infra";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Handle } from "@destack/language/space/handle";
import { RegionProto, SpaceProto, SpaceStatusProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:10001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:10001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:10000 ==== */
/**
 * A Space is the home of your personal software studio.
 */
export class Space
  extends Entity
  implements IsGlobal, IsFollowable, IsJoinable, IsOwnable, IsStarable, IsSpatial
{
  static metatype: NodeType = NodeType.SPACE;

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
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
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
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
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
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
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
    let _status = options.status;
    if (_status === null) {
      throw new Error(`Space.status is required`);
    }
    this.status = _status;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.metatype != StructType.NODE_REFERENCE) {
      _handle = (_handle as Node).toRef();
    }
    this.handlePtr = _handle;
    let _systemFolder = options.systemFolder ?? null;
    if (_systemFolder != null && _systemFolder.metatype != StructType.NODE_REFERENCE) {
      _systemFolder = (_systemFolder as Node).toRef();
    }
    this.systemFolderPtr = _systemFolder;
    let _homeFolder = options.homeFolder ?? null;
    if (_homeFolder != null && _homeFolder.metatype != StructType.NODE_REFERENCE) {
      _homeFolder = (_homeFolder as Node).toRef();
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
    if (_database != null && _database.metatype != StructType.NODE_REFERENCE) {
      _database = (_database as Node).toRef();
    }
    this.databasePtr = _database;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
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
    if (!(this.ownedByPtr?.id === other.ownedByPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.slug)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.handlePtr !== null) {
      h = (h * 31 + hashString(this.handlePtr.id)) & 0xffffffff;
    }
    if (this.systemFolderPtr !== null) {
      h = (h * 31 + hashString(this.systemFolderPtr.id)) & 0xffffffff;
    }
    if (this.homeFolderPtr !== null) {
      h = (h * 31 + hashString(this.homeFolderPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.region) & 0xffffffff;
    if (this.galaxyName !== null) {
      h = (h * 31 + hashString(this.galaxyName)) & 0xffffffff;
    }
    if (this.databasePtr !== null) {
      h = (h * 31 + hashString(this.databasePtr.id)) & 0xffffffff;
    }
    if (this.ownedByPtr !== null) {
      h = (h * 31 + hashString(this.ownedByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }

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
    objectValue["1"] = 10000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.ownedByPtr != null) {
      objectValue["28"] = object.ownedByPtr.toValue();
    }
    objectValue["101"] = object.name;
    objectValue["102"] = object.slug;
    objectValue["110"] = object.status;
    if (object.handlePtr != null) {
      objectValue["111"] = object.handlePtr.toValue();
    }
    if (object.systemFolderPtr != null) {
      objectValue["112"] = object.systemFolderPtr.toValue();
    }
    if (object.homeFolderPtr != null) {
      objectValue["113"] = object.homeFolderPtr.toValue();
    }
    objectValue["120"] = object.region;
    if (object.galaxyName != null) {
      objectValue["121"] = object.galaxyName;
    }
    if (object.databasePtr != null) {
      objectValue["122"] = object.databasePtr.toValue();
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
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
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
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
    const objectProto: Partial<SpaceProto> = { metatype: 10000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
/* ==== DESTACK_GENERATED_END:NODE:10000 ==== */
