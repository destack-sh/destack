import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  IsSpatial,
  IsSubject,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  EnumType,
  Graph,
  Node,
  NodeReference,
  NodeType,
  Resource,
  ResourceStatus,
  StructType,
} from "@destack/language/core";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import type { Client, Space } from "@destack/language/space";
import { MachineProto, MachineTypeProto, ResourceStatusProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:7600 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:7600 ==== */

/* ==== DESTACK_GENERATED_START:NODE:7600 ==== */
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
   * Machine.type
   */
  type: MachineType;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /**
   * Machine.version
   */
  version: string;

  /**
   * Machine.externalName
   */
  readonly externalName: string | null;

  /**
   * Machine.externalId
   */
  readonly externalId: string | null;

  /**
   * Machine.imageId
   */
  readonly imageId: string | null;

  /**
   * Machine.grpcUrl
   */
  readonly grpcUrl: string | null;

  /**
   * Machine.vncUrl
   */
  readonly vncUrl: string | null;

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
  clientPtr: NodeReference | null;

  /**
   * vCPU count
   */
  readonly cpu: number;

  /**
   * GB
   */
  readonly ram: number;

  /**
   * Machine.width
   */
  readonly width: number;

  /**
   * Machine.height
   */
  readonly height: number;

  /**
   * Machine.isHeadless
   */
  readonly isHeadless: boolean;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    type?: MachineType;
    status?: ResourceStatus;
    targetStatus?: Temporal.ZonedDateTime | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = MachineType.RUNTIME;
    }
    if (_type === null) {
      throw new Error(`Machine.type is required`);
    }
    this.type = _type;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = ResourceStatus.PENDING;
    }
    if (_status === null) {
      throw new Error(`Machine.status is required`);
    }
    this.status = _status;
    let _targetStatus = options.targetStatus ?? null;
    this.targetStatus = _targetStatus;
    let _version = options.version ?? null;
    if (_version === null) {
      _version = "2025.06.30.0";
    }
    if (_version === null) {
      throw new Error(`Machine.version is required`);
    }
    this.version = _version;
    let _externalName = options.externalName ?? null;
    this.externalName = _externalName;
    let _externalId = options.externalId ?? null;
    this.externalId = _externalId;
    let _imageId = options.imageId ?? null;
    this.imageId = _imageId;
    let _grpcUrl = options.grpcUrl ?? null;
    this.grpcUrl = _grpcUrl;
    let _vncUrl = options.vncUrl ?? null;
    this.vncUrl = _vncUrl;
    let _client = options.client ?? null;
    if (_client != null && _client instanceof Node) {
      _client = _client.toRef();
    }
    this.clientPtr = _client;
    let _cpu = options.cpu ?? null;
    if (_cpu === null) {
      _cpu = 1.0;
    }
    if (_cpu === null) {
      throw new Error(`Machine.cpu is required`);
    }
    this.cpu = _cpu;
    let _ram = options.ram ?? null;
    if (_ram === null) {
      _ram = 1.0;
    }
    if (_ram === null) {
      throw new Error(`Machine.ram is required`);
    }
    this.ram = _ram;
    let _width = options.width ?? null;
    if (_width === null) {
      _width = 1280;
    }
    if (_width === null) {
      throw new Error(`Machine.width is required`);
    }
    this.width = _width;
    let _height = options.height ?? null;
    if (_height === null) {
      _height = 960;
    }
    if (_height === null) {
      throw new Error(`Machine.height is required`);
    }
    this.height = _height;
    let _isHeadless = options.isHeadless ?? null;
    if (_isHeadless === null) {
      _isHeadless = false;
    }
    if (_isHeadless === null) {
      throw new Error(`Machine.isHeadless is required`);
    }
    this.isHeadless = _isHeadless;

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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.version === other.version)) {
      return false;
    }
    if (!(this.externalName === other.externalName)) {
      return false;
    }
    if (!(this.externalId === other.externalId)) {
      return false;
    }
    if (!(this.imageId === other.imageId)) {
      return false;
    }
    if (!(this.grpcUrl === other.grpcUrl)) {
      return false;
    }
    if (!(this.vncUrl === other.vncUrl)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.cpu === other.cpu || Math.abs(this.cpu - other.cpu) < 1e-10)) {
      return false;
    }
    if (!(this.ram === other.ram || Math.abs(this.ram - other.ram) < 1e-10)) {
      return false;
    }
    if (!(this.width === other.width)) {
      return false;
    }
    if (!(this.height === other.height)) {
      return false;
    }
    if (!(this.isHeadless === other.isHeadless)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.targetStatus === other.targetStatus)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.version)) & 0xffffffff;
    if (this.externalName !== null) {
      h = (h * 31 + hashString(this.externalName)) & 0xffffffff;
    }
    if (this.externalId !== null) {
      h = (h * 31 + hashString(this.externalId)) & 0xffffffff;
    }
    if (this.imageId !== null) {
      h = (h * 31 + hashString(this.imageId)) & 0xffffffff;
    }
    if (this.grpcUrl !== null) {
      h = (h * 31 + hashString(this.grpcUrl)) & 0xffffffff;
    }
    if (this.vncUrl !== null) {
      h = (h * 31 + hashString(this.vncUrl)) & 0xffffffff;
    }
    if (this.clientPtr !== null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashFloat(this.cpu)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.ram)) & 0xffffffff;
    h = (h * 31 + hashInt(this.width)) & 0xffffffff;
    h = (h * 31 + hashInt(this.height)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isHeadless)) & 0xffffffff;
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.targetStatus !== null) {
      h = (h * 31 + hashString(this.targetStatus.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.MACHINE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
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
    objectValue["1"] = 7600;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["30"] = object.type;
    objectValue["40"] = object.status;
    if (object.targetStatus != null) {
      objectValue["41"] = object.targetStatus.toString({ timeZoneName: "never" });
    }
    objectValue["60"] = object.version;
    if (object.externalName != null) {
      objectValue["62"] = object.externalName;
    }
    if (object.externalId != null) {
      objectValue["63"] = object.externalId;
    }
    if (object.imageId != null) {
      objectValue["64"] = object.imageId;
    }
    if (object.grpcUrl != null) {
      objectValue["65"] = object.grpcUrl;
    }
    if (object.vncUrl != null) {
      objectValue["66"] = object.vncUrl;
    }
    if (object.clientPtr != null) {
      objectValue["69"] = object.clientPtr.toValue();
    }
    objectValue["70"] = object.cpu;
    objectValue["71"] = object.ram;
    objectValue["75"] = object.width;
    objectValue["76"] = object.height;
    objectValue["77"] = object.isHeadless;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
    const externalNameValue = objectValue["62"];
    const unpackedExternalName = externalNameValue != undefined ? externalNameValue : null;
    const externalIdValue = objectValue["63"];
    const unpackedExternalId = externalIdValue != undefined ? externalIdValue : null;
    const imageIdValue = objectValue["64"];
    const unpackedImageId = imageIdValue != undefined ? imageIdValue : null;
    const grpcUrlValue = objectValue["65"];
    const unpackedGrpcUrl = grpcUrlValue != undefined ? grpcUrlValue : null;
    const vncUrlValue = objectValue["66"];
    const unpackedVncUrl = vncUrlValue != undefined ? vncUrlValue : null;
    const clientPtrValue = objectValue["69"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? NodeReference.fromValue(clientPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetStatusValue = objectValue["41"];
    const unpackedTargetStatus =
      targetStatusValue != undefined
        ? Temporal.Instant.from(targetStatusValue).toZonedDateTimeISO("UTC")
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
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
    return new Machine({
      type: Number(objectValue["30"]),
      version: objectValue["60"],
      externalName: unpackedExternalName,
      externalId: unpackedExternalId,
      imageId: unpackedImageId,
      grpcUrl: unpackedGrpcUrl,
      vncUrl: unpackedVncUrl,
      client: unpackedClientPtr,
      cpu: objectValue["70"],
      ram: objectValue["71"],
      width: Number(objectValue["75"]),
      height: Number(objectValue["76"]),
      isHeadless: objectValue["77"],
      space: unpackedSpacePtr,
      status: Number(objectValue["40"]),
      targetStatus: unpackedTargetStatus,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
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
    const objectProto: Partial<MachineProto> = { metatype: 7600 };
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
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.type = Number(object.type) as MachineTypeProto;
    objectProto.status = Number(object.status) as ResourceStatusProto;
    if (object.targetStatus != null) {
      objectProto.targetStatus = packProtoTimestamp(object.targetStatus);
    }
    objectProto.version = object.version;
    if (object.externalName != null) {
      objectProto.externalName = object.externalName;
    }
    if (object.externalId != null) {
      objectProto.externalId = object.externalId;
    }
    if (object.imageId != null) {
      objectProto.imageId = object.imageId;
    }
    if (object.grpcUrl != null) {
      objectProto.grpcUrl = object.grpcUrl;
    }
    if (object.vncUrl != null) {
      objectProto.vncUrl = object.vncUrl;
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    objectProto.cpu = object.cpu;
    objectProto.ram = object.ram;
    objectProto.width = object.width;
    objectProto.height = object.height;
    objectProto.isHeadless = object.isHeadless;
    return objectProto as MachineProto;
  }

  static __unpackProto__(
    objectProto: MachineProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Machine {
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
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as ResourceStatus,
      targetStatus:
        objectProto.targetStatus != undefined
          ? unpackProtoTimestamp(objectProto.targetStatus!)
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
/* ==== DESTACK_GENERATED_END:NODE:7600 ==== */
