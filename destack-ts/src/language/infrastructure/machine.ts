import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  CustomEntityDefinition,
  CustomEventDefinition,
  Graph,
  IsSpatial,
  IsSubject,
  NodeDefinitionReference,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
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
export class Machine extends Resource implements IsSpatial {
  static metatype: NodeType = NodeType.MACHINE;

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
   * The definitionthis CustomEntity is an instance of.
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  readonly baseType: NodeDefinitionReference | null;

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
  get predecessor(): Machine | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Machine | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): Machine | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Machine | null;
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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): Map<string, Value> {
    return this._customValues;
  }
  set customValues(value: Map<string, Value>) {
    const oldValue = this._customValues;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["customValues"] === undefined) {
      this._dirty["customValues"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._customValues = value;
  }
  _customValues: Map<string, Value>;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
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
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const oldValue = this._scriptPtr;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["scriptPtr"] === undefined) {
      this._dirty["scriptPtr"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Resource.status
   */
  get status(): ResourceStatus {
    return this._status;
  }
  set status(value: ResourceStatus) {
    const oldValue = this._status;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["status"] === undefined) {
      this._dirty["status"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._status = value;
  }
  _status: ResourceStatus;

  /**
   * Machine.type
   */
  get type(): MachineType {
    return this._type;
  }
  set type(value: MachineType) {
    const oldValue = this._type;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["type"] === undefined) {
      this._dirty["type"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._type = value;
  }
  _type: MachineType;

  /**
   * Machine.version
   */
  get version(): string {
    return this._version;
  }
  set version(value: string) {
    const oldValue = this._version;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["version"] === undefined) {
      this._dirty["version"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._version = value;
  }
  _version: string;

  /**
   * Machine.externalName
   */
  get externalName(): string | null {
    return this._externalName;
  }
  set externalName(value: string | null) {
    const oldValue = this._externalName;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["externalName"] === undefined) {
      this._dirty["externalName"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._externalName = value;
  }
  _externalName: string | null;

  /**
   * Machine.externalId
   */
  get externalId(): string | null {
    return this._externalId;
  }
  set externalId(value: string | null) {
    const oldValue = this._externalId;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["externalId"] === undefined) {
      this._dirty["externalId"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._externalId = value;
  }
  _externalId: string | null;

  /**
   * Machine.imageId
   */
  get imageId(): string | null {
    return this._imageId;
  }
  set imageId(value: string | null) {
    const oldValue = this._imageId;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["imageId"] === undefined) {
      this._dirty["imageId"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._imageId = value;
  }
  _imageId: string | null;

  /**
   * Machine.grpcUrl
   */
  get grpcUrl(): string | null {
    return this._grpcUrl;
  }
  set grpcUrl(value: string | null) {
    const oldValue = this._grpcUrl;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["grpcUrl"] === undefined) {
      this._dirty["grpcUrl"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._grpcUrl = value;
  }
  _grpcUrl: string | null;

  /**
   * Machine.vncUrl
   */
  get vncUrl(): string | null {
    return this._vncUrl;
  }
  set vncUrl(value: string | null) {
    const oldValue = this._vncUrl;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["vncUrl"] === undefined) {
      this._dirty["vncUrl"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._vncUrl = value;
  }
  _vncUrl: string | null;

  /**
   * Machine.client
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr !== null) {
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
  get clientPtr(): NodeReference | null {
    return this._clientPtr;
  }
  set clientPtr(value: NodeReference | null) {
    const oldValue = this._clientPtr;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["clientPtr"] === undefined) {
      this._dirty["clientPtr"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._clientPtr = value;
  }
  _clientPtr: NodeReference | null;

  /**
   * vCPU count
   */
  get cpu(): number {
    return this._cpu;
  }
  set cpu(value: number) {
    const oldValue = this._cpu;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["cpu"] === undefined) {
      this._dirty["cpu"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._cpu = value;
  }
  _cpu: number;

  /**
   * GB
   */
  get ram(): number {
    return this._ram;
  }
  set ram(value: number) {
    const oldValue = this._ram;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["ram"] === undefined) {
      this._dirty["ram"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._ram = value;
  }
  _ram: number;

  /**
   * Machine.width
   */
  get width(): number {
    return this._width;
  }
  set width(value: number) {
    const oldValue = this._width;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["width"] === undefined) {
      this._dirty["width"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._width = value;
  }
  _width: number;

  /**
   * Machine.height
   */
  get height(): number {
    return this._height;
  }
  set height(value: number) {
    const oldValue = this._height;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["height"] === undefined) {
      this._dirty["height"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._height = value;
  }
  _height: number;

  /**
   * Machine.isHeadless
   */
  get isHeadless(): boolean {
    return this._isHeadless;
  }
  set isHeadless(value: boolean) {
    const oldValue = this._isHeadless;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isHeadless"] === undefined) {
      this._dirty["isHeadless"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this._isHeadless = value;
  }
  _isHeadless: boolean;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    definition?: CustomEntityDefinition | CustomEventDefinition | NodeReference | null;
    baseType?: NodeDefinitionReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Machine | NodeReference | null;
    template?: Machine | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: Map<string, Value>;
    script?: Script | NodeReference | null;
    status?: ResourceStatus;
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
      // is_attached
      options.id != null || options._graph != null,
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
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 32 /* Materialization.FULL */;
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`Machine.status is required`);
    }
    this._status = _status;
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
      _version = "2025.07.05.1";
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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
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
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues.get(key)!.equals(other._customValues.get(key)!)) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + hashString(this._version)) & 0xffffffff;
    if (this._externalName !== null) {
      h = (h * 31 + hashString(this._externalName)) & 0xffffffff;
    }
    if (this._externalId !== null) {
      h = (h * 31 + hashString(this._externalId)) & 0xffffffff;
    }
    if (this._imageId !== null) {
      h = (h * 31 + hashString(this._imageId)) & 0xffffffff;
    }
    if (this._grpcUrl !== null) {
      h = (h * 31 + hashString(this._grpcUrl)) & 0xffffffff;
    }
    if (this._vncUrl !== null) {
      h = (h * 31 + hashString(this._vncUrl)) & 0xffffffff;
    }
    if (this._clientPtr !== null) {
      h = (h * 31 + hashString(this._clientPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashFloat(this._cpu)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._ram)) & 0xffffffff;
    h = (h * 31 + hashInt(this._width)) & 0xffffffff;
    h = (h * 31 + hashInt(this._height)) & 0xffffffff;
    h = (h * 31 + hashBool(this._isHeadless)) & 0xffffffff;
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
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
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr !== null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }

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
    return "Machine[id={this.id}]";
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

  repr(): string {
    return `<Machine '${this.path}'>`;
  }

  toValue(): { [key: string]: any } {
    return Machine.__packValue__(this);
  }

  static __packValue__(object: Machine): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 140100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    if (object.baseType != null) {
      objectValue["7"] = object.baseType.toValue();
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
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object._customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectValue["70"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object._status;
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
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
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
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["7"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _NodeDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
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
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const scriptPtrValue = objectValue["70"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Machine({
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
      space: unpackedSpacePtr,
      status: Number(objectValue["90"]),
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      deletedAt: unpackedDeletedAt,
      definition: unpackedDefinitionPtr,
      baseType: unpackedBaseType,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
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
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
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
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object._customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.status = Number(object._status) as ResourceStatusProto;
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Machine({
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
      status: Number(objectProto.status) as ResourceStatus,
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      baseType:
        objectProto.baseType != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.baseType!,
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
