import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsActor,
  IsExtensible,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  Resource,
  ResourceStatus,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import {
  MachineProto,
  MachineTypeProto,
  MaterializationProto,
  ResourceStatusProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:140100 ==== */
/**
 * MachineType
 */
export enum MachineType {
  RUNTIME = 10,
  UBUNTU = 1000,
  MAC = 1100,
  WINDOWS = 1200,
  CUSTOM = 9000,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MACHINE_TYPE, MachineType);
/* ==== DESTACK_GENERATED_END:ENUM:140100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:140100 ==== */
/**
 * A Machine provides physical compute.
 * NOTE :RichComputing: Machines also need Deployments/Endpoints/...?
 */
export class Machine extends Resource {
  static metatype: NodeType = NodeType.MACHINE;

  /**
   * Machine.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
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
   * The definition this CustomEntity is an instance of.
   */
  get definition(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

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
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Machine | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Machine | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

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
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
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
   * Resource.status
   */
  /**
   * Resource.status
   */
  get status(): ResourceStatus {
    return this._status;
  }
  set status(value: ResourceStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: ResourceStatus;

  /**
   * The main / root Script of this Node.
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
   * The main / root Script of this Node.
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
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * Machine.type
   */
  /**
   * Machine.type
   */
  get type(): MachineType {
    return this._type;
  }
  set type(value: MachineType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: MachineType;

  /**
   * Machine.version
   */
  /**
   * Machine.version
   */
  get version(): string {
    return this._version;
  }
  set version(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["version"];
    this._session.updateSetProperty(this, prop, value);
    this._version = value;
  }
  _version: string;

  /**
   * Machine.externalName
   */
  /**
   * Machine.externalName
   */
  get externalName(): string | null {
    return this._externalName;
  }
  set externalName(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["external_name"];
    this._session.updateSetProperty(this, prop, value);
    this._externalName = value;
  }
  _externalName: string | null;

  /**
   * Machine.externalId
   */
  /**
   * Machine.externalId
   */
  get externalId(): string | null {
    return this._externalId;
  }
  set externalId(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["external_id"];
    this._session.updateSetProperty(this, prop, value);
    this._externalId = value;
  }
  _externalId: string | null;

  /**
   * Machine.imageId
   */
  /**
   * Machine.imageId
   */
  get imageId(): string | null {
    return this._imageId;
  }
  set imageId(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["image_id"];
    this._session.updateSetProperty(this, prop, value);
    this._imageId = value;
  }
  _imageId: string | null;

  /**
   * Machine.grpcUrl
   */
  /**
   * Machine.grpcUrl
   */
  get grpcUrl(): string | null {
    return this._grpcUrl;
  }
  set grpcUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["grpc_url"];
    this._session.updateSetProperty(this, prop, value);
    this._grpcUrl = value;
  }
  _grpcUrl: string | null;

  /**
   * Machine.vncUrl
   */
  /**
   * Machine.vncUrl
   */
  get vncUrl(): string | null {
    return this._vncUrl;
  }
  set vncUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["vnc_url"];
    this._session.updateSetProperty(this, prop, value);
    this._vncUrl = value;
  }
  _vncUrl: string | null;

  /**
   * Machine.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  set client(node: Client | null) {
    if (node === null) {
      this.clientPtr = null;
    } else {
      this.clientPtr = node.toRef();
    }
  }
  /**
   * Machine.client
   */
  get clientPtr(): NodeReference | null {
    return this._clientPtr;
  }
  set clientPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["client"];
    this._session.updateSetProperty(this, prop, value);
    this._clientPtr = value;
  }
  _clientPtr: NodeReference | null;

  /**
   * vCPU count
   */
  /**
   * vCPU count
   */
  get cpu(): number {
    return this._cpu;
  }
  set cpu(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["cpu"];
    this._session.updateSetProperty(this, prop, value);
    this._cpu = value;
  }
  _cpu: number;

  /**
   * GB
   */
  /**
   * GB
   */
  get ram(): number {
    return this._ram;
  }
  set ram(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["ram"];
    this._session.updateSetProperty(this, prop, value);
    this._ram = value;
  }
  _ram: number;

  /**
   * Machine.width
   */
  /**
   * Machine.width
   */
  get width(): number {
    return this._width;
  }
  set width(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: number;

  /**
   * Machine.height
   */
  /**
   * Machine.height
   */
  get height(): number {
    return this._height;
  }
  set height(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: number;

  /**
   * Machine.isHeadless
   */
  /**
   * Machine.isHeadless
   */
  get isHeadless(): boolean {
    return this._isHeadless;
  }
  set isHeadless(value: boolean) {
    const prop = (this.constructor as NodeClass).__properties__["is_headless"];
    this._session.updateSetProperty(this, prop, value);
    this._isHeadless = value;
  }
  _isHeadless: boolean;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    definition?: (Entity & IsExtensible) | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Machine | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    status?: ResourceStatus;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type?: MachineType;
    version?: string;
    externalName?: string | null;
    externalId?: string | null;
    imageId?: string | null;
    grpcUrl?: string | null;
    vncUrl?: string | null;
    client?: Client | NodeReference | null;
    cpu?: number;
    ram?: number;
    width?: number;
    height?: number;
    isHeadless?: boolean;
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
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`Machine has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Machine has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Machine.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Machine.materialization is required`);
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`Machine.status is required`);
    }
    this._status = _status;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`Machine.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* MachineType.RUNTIME */;
    }
    if (_type === null) {
      throw new Error(`Machine.type is required`);
    }
    this._type = _type;
    let _version = options.version ?? null;
    if (_version === null) {
      _version = "2025.07.14.2";
    }
    if (_version === null) {
      throw new Error(`Machine.version is required`);
    }
    this._version = _version;
    let _externalName = options.externalName ?? null;
    this._externalName = _externalName;
    let _externalId = options.externalId ?? null;
    this._externalId = _externalId;
    let _imageId = options.imageId ?? null;
    this._imageId = _imageId;
    let _grpcUrl = options.grpcUrl ?? null;
    this._grpcUrl = _grpcUrl;
    let _vncUrl = options.vncUrl ?? null;
    this._vncUrl = _vncUrl;
    let _client = options.client ?? null;
    if (_client != null && _client.metatype != StructType.NODE_REFERENCE) {
      _client = (_client as Node).toRef();
    }
    this._clientPtr = _client;
    let _cpu = options.cpu ?? null;
    if (_cpu === null) {
      _cpu = 1.0;
    }
    if (_cpu === null) {
      throw new Error(`Machine.cpu is required`);
    }
    this._cpu = _cpu;
    let _ram = options.ram ?? null;
    if (_ram === null) {
      _ram = 1.0;
    }
    if (_ram === null) {
      throw new Error(`Machine.ram is required`);
    }
    this._ram = _ram;
    let _width = options.width ?? null;
    if (_width === null) {
      _width = 1280;
    }
    if (_width === null) {
      throw new Error(`Machine.width is required`);
    }
    this._width = _width;
    let _height = options.height ?? null;
    if (_height === null) {
      _height = 960;
    }
    if (_height === null) {
      throw new Error(`Machine.height is required`);
    }
    this._height = _height;
    let _isHeadless = options.isHeadless ?? null;
    if (_isHeadless === null) {
      _isHeadless = false;
    }
    if (_isHeadless === null) {
      throw new Error(`Machine.isHeadless is required`);
    }
    this._isHeadless = _isHeadless;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`Machine.createdAt and Machine.updatedAt are required for existing Nodes`);
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._version === other._version)) {
      return false;
    }
    if (!(this._externalName === other._externalName)) {
      return false;
    }
    if (!(this._externalId === other._externalId)) {
      return false;
    }
    if (!(this._imageId === other._imageId)) {
      return false;
    }
    if (!(this._grpcUrl === other._grpcUrl)) {
      return false;
    }
    if (!(this._vncUrl === other._vncUrl)) {
      return false;
    }
    if (!(this._clientPtr?.id === other._clientPtr?.id)) {
      return false;
    }
    if (!(this._cpu === other._cpu || Math.abs(this._cpu - other._cpu) < 1e-10)) {
      return false;
    }
    if (!(this._ram === other._ram || Math.abs(this._ram - other._ram) < 1e-10)) {
      return false;
    }
    if (!(this._width === other._width)) {
      return false;
    }
    if (!(this._height === other._height)) {
      return false;
    }
    if (!(this._isHeadless === other._isHeadless)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
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
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + hashString(this._version)) & 0xffffffff;
    if (this._externalName != null) {
      h = (h * 31 + hashString(this._externalName)) & 0xffffffff;
    }
    if (this._externalId != null) {
      h = (h * 31 + hashString(this._externalId)) & 0xffffffff;
    }
    if (this._imageId != null) {
      h = (h * 31 + hashString(this._imageId)) & 0xffffffff;
    }
    if (this._grpcUrl != null) {
      h = (h * 31 + hashString(this._grpcUrl)) & 0xffffffff;
    }
    if (this._vncUrl != null) {
      h = (h * 31 + hashString(this._vncUrl)) & 0xffffffff;
    }
    if (this._clientPtr != null) {
      h = (h * 31 + hashString(this._clientPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashFloat(this._cpu)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._ram)) & 0xffffffff;
    h = (h * 31 + hashInt(this._width)) & 0xffffffff;
    h = (h * 31 + hashInt(this._height)) & 0xffffffff;
    h = (h * 31 + hashBool(this._isHeadless)) & 0xffffffff;
    h = (h * 31 + this._status) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr != null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.MACHINE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `Machine[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    return `<Machine "${this.path}">`;
  }

  toValue(): { readonly [key: string]: any } {
    return Machine.__packValue__(this);
  }

  static __packValue__(object: Machine): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 140100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["40"] = object._status;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
    objectValue["110"] = object._version;
    if (object._externalName != null) {
      objectValue["112"] = object._externalName;
    }
    if (object._externalId != null) {
      objectValue["113"] = object._externalId;
    }
    if (object._imageId != null) {
      objectValue["114"] = object._imageId;
    }
    if (object._grpcUrl != null) {
      objectValue["115"] = object._grpcUrl;
    }
    if (object._vncUrl != null) {
      objectValue["116"] = object._vncUrl;
    }
    if (object._clientPtr != null) {
      objectValue["119"] = object._clientPtr.toValue();
    }
    objectValue["120"] = object._cpu;
    objectValue["121"] = object._ram;
    objectValue["122"] = object._width;
    objectValue["123"] = object._height;
    objectValue["124"] = object._isHeadless;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const externalNameValue = objectValue["112"];
    const unpackedExternalName = externalNameValue != undefined ? externalNameValue : null;
    const externalIdValue = objectValue["113"];
    const unpackedExternalId = externalIdValue != undefined ? externalIdValue : null;
    const imageIdValue = objectValue["114"];
    const unpackedImageId = imageIdValue != undefined ? imageIdValue : null;
    const grpcUrlValue = objectValue["115"];
    const unpackedGrpcUrl = grpcUrlValue != undefined ? grpcUrlValue : null;
    const vncUrlValue = objectValue["116"];
    const unpackedVncUrl = vncUrlValue != undefined ? vncUrlValue : null;
    const clientPtrValue = objectValue["119"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
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
    const unpackedCustomValues = {} as any;
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Machine({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      version: objectValue["110"],
      externalName: unpackedExternalName,
      externalId: unpackedExternalId,
      imageId: unpackedImageId,
      grpcUrl: unpackedGrpcUrl,
      vncUrl: unpackedVncUrl,
      client: unpackedClientPtr,
      cpu: objectValue["120"],
      ram: objectValue["121"],
      width: Number(objectValue["122"]),
      height: Number(objectValue["123"]),
      isHeadless: objectValue["124"],
      status: Number(objectValue["40"]),
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      deletedAt: unpackedDeletedAt,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    return Machine.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): MachineProto {
    return Machine.__packProto__(this);
  }

  static __packProto__(object: Machine): MachineProto {
    const objectProto: Partial<MachineProto> = { metatype: 140100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object._status) as ResourceStatusProto;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as MachineTypeProto;
    objectProto.version = object._version;
    if (object._externalName != null) {
      objectProto.externalName = object._externalName;
    }
    if (object._externalId != null) {
      objectProto.externalId = object._externalId;
    }
    if (object._imageId != null) {
      objectProto.imageId = object._imageId;
    }
    if (object._grpcUrl != null) {
      objectProto.grpcUrl = object._grpcUrl;
    }
    if (object._vncUrl != null) {
      objectProto.vncUrl = object._vncUrl;
    }
    if (object._clientPtr != null) {
      objectProto.clientPtr = object._clientPtr.toProto();
    }
    objectProto.cpu = object._cpu;
    objectProto.ram = object._ram;
    objectProto.width = object._width;
    objectProto.height = object._height;
    objectProto.isHeadless = object._isHeadless;
    return objectProto as MachineProto;
  }

  static __unpackProto__(
    objectProto: MachineProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Machine({
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
      type: Number(objectProto.type) as MachineType,
      version: objectProto.version,
      externalName: objectProto.externalName != undefined ? objectProto.externalName : null,
      externalId: objectProto.externalId != undefined ? objectProto.externalId : null,
      imageId: objectProto.imageId != undefined ? objectProto.imageId : null,
      grpcUrl: objectProto.grpcUrl != undefined ? objectProto.grpcUrl : null,
      vncUrl: objectProto.vncUrl != undefined ? objectProto.vncUrl : null,
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.clientPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      cpu: objectProto.cpu,
      ram: objectProto.ram,
      width: Number(objectProto.width),
      height: Number(objectProto.height),
      isHeadless: objectProto.isHeadless,
      status: Number(objectProto.status) as ResourceStatus,
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
      isExtensible: objectProto.isExtensible,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MachineProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    return Machine.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Machine {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MachineProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MACHINE, Machine);
/* ==== DESTACK_GENERATED_END:NODE:140100 ==== */
