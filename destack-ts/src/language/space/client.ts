import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  ClientType,
  Entity,
  Global,
  HasName,
  IsDeletable,
  IsSubject,
  MaterializationType,
  Node,
  NodeType,
  StructFrozen,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Machine } from "@destack/language/infra";
import { Cursor } from "@destack/language/logic";
import { registerNodeClass, registerStructClass } from "@destack/language/registry";
import { User } from "@destack/language/space";
import {
  ClientProto,
  ClientTypeProto,
  MaterializationTypeProto,
  OriginProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:50001 ==== */
/**
 * Origin of something.
 */
export class Origin extends StructFrozen {
  static metatype: StructType = StructType.ORIGIN;
  static __isFrozen__: boolean = true;

  /**
   * Origin.type
   */
  readonly type: ClientType;

  /**
   * Origin.id
   */
  readonly id: string | null;

  /**
   * Origin.ck
   */
  readonly ck: string | null;

  /**
   * Origin.nonce
   */
  readonly nonce: string | null;

  constructor(options: {
    type: ClientType;
    id?: string | null;
    ck?: string | null;
    nonce?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Origin.type is required`);
    }
    this.type = _type;
    let _id = options.id ?? null;
    this.id = _id;
    let _ck = options.ck ?? null;
    this.ck = _ck;
    let _nonce = options.nonce ?? null;
    this.nonce = _nonce;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.ck === other.ck)) {
      return false;
    }
    if (!(this.nonce === other.nonce)) {
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Origin.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Origin): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50001;
    objectValue["30"] = object.type;
    if (object.id != null) {
      objectValue["31"] = String(object.id);
    }
    if (object.ck != null) {
      objectValue["32"] = String(object.ck);
    }
    if (object.nonce != null) {
      objectValue["33"] = String(object.nonce);
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Origin {
    const idValue = objectValue["31"];
    const unpackedId = idValue != undefined ? String(idValue) : null;
    const ckValue = objectValue["32"];
    const unpackedCk = ckValue != undefined ? String(ckValue) : null;
    const nonceValue = objectValue["33"];
    const unpackedNonce = nonceValue != undefined ? String(nonceValue) : null;
    return new Origin({
      type: Number(objectValue["30"]),
      id: unpackedId,
      ck: unpackedCk,
      nonce: unpackedNonce,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Origin {
    return Origin.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): OriginProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Origin.__packProto__(this);
    }
    return this._proto as OriginProto;
  }

  static __packProto__(object: Origin): OriginProto {
    const objectProto: Partial<OriginProto> = { metatype: 50001 };
    objectProto.type = Number(object.type) as ClientTypeProto;
    if (object.id != null) {
      objectProto.id = String(object.id);
    }
    if (object.ck != null) {
      objectProto.ck = String(object.ck);
    }
    if (object.nonce != null) {
      objectProto.nonce = String(object.nonce);
    }
    return objectProto as OriginProto;
  }

  static __unpackProto__(
    objectProto: OriginProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Origin {
    return new Origin({
      type: Number(objectProto.type) as ClientType,
      id: objectProto.id != undefined ? String(objectProto.id) : null,
      ck: objectProto.ck != undefined ? String(objectProto.ck) : null,
      nonce: objectProto.nonce != undefined ? String(objectProto.nonce) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: OriginProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Origin {
    return Origin.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Origin {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = OriginProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ORIGIN, Origin);
/* ==== DESTACK_GENERATED_END:STRUCT:50001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:100 ==== */
/**
 * A Client to connect with the system.
 */
export class Client extends Node implements HasName, Global, Entity, IsDeletable {
  static metatype: NodeType = NodeType.CLIENT;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.AGENT, NodeType.USER];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [
    NodeType.AGENT,
    NodeType.FOLDER,
    NodeType.USER,
    NodeType.SPACE,
  ];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Client.parent
   */
  get parent(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Client.type
   */
  type: ClientType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * Client.machine
   */
  get machine(): Machine | null {
    const nodePtr: NodeReference | null = this.machinePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Machine | null;
    }
    return null;
  }
  set machine(node: Machine | null) {
    if (node === null) {
      this.machinePtr = null;
    } else {
      this.machinePtr = node.toRef();
    }
  }
  machinePtr: NodeReference | null;

  /**
   * Client.user
   */
  get user(): User | null {
    const nodePtr: NodeReference | null = this.userPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as User | null;
    }
    return null;
  }
  set user(node: User | null) {
    if (node === null) {
      this.userPtr = null;
    } else {
      this.userPtr = node.toRef();
    }
  }
  userPtr: NodeReference | null;

  /**
   * Client.deviceType
   */
  deviceType: string | null;

  /**
   * Client.deviceName
   */
  deviceName: string | null;

  /**
   * Client.operatingSystem
   */
  operatingSystem: string | null;

  /**
   * Client.browserName
   */
  browserName: string | null;

  /**
   * Client.browserVersion
   */
  browserVersion: string | null;

  /**
   * Client.accessToken
   */
  accessToken: string | null;

  /**
   * Client.seenAt
   */
  seenAt: Temporal.ZonedDateTime | null;

  /**
   * Client.loggedInAt
   */
  loggedInAt: Temporal.ZonedDateTime | null;

  /**
   * Client.cursor
   */
  get cursor(): (Node & Cursor) | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & Cursor) | null;
    }
    return null;
  }
  set cursor(node: (Node & Cursor) | null) {
    if (node === null) {
      this.cursorPtr = null;
    } else {
      this.cursorPtr = node.toRef();
    }
  }
  cursorPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: (Node & IsSubject) | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    type: ClientType;
    name: string;
    machine?: Machine | NodeReference | null;
    user?: User | NodeReference | null;
    deviceType?: string | null;
    deviceName?: string | null;
    operatingSystem?: string | null;
    browserName?: string | null;
    browserVersion?: string | null;
    accessToken?: string | null;
    seenAt?: Temporal.ZonedDateTime | null;
    loggedInAt?: Temporal.ZonedDateTime | null;
    cursor?: (Node & Cursor) | NodeReference | null;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Client.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Client.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Client.name is required`);
    }
    this.name = _name;
    let _machine = options.machine ?? null;
    if (_machine != null && _machine instanceof Node) {
      _machine = _machine.toRef();
    }
    this.machinePtr = _machine;
    let _user = options.user ?? null;
    if (_user != null && _user instanceof Node) {
      _user = _user.toRef();
    }
    this.userPtr = _user;
    let _deviceType = options.deviceType ?? null;
    this.deviceType = _deviceType;
    let _deviceName = options.deviceName ?? null;
    this.deviceName = _deviceName;
    let _operatingSystem = options.operatingSystem ?? null;
    this.operatingSystem = _operatingSystem;
    let _browserName = options.browserName ?? null;
    this.browserName = _browserName;
    let _browserVersion = options.browserVersion ?? null;
    this.browserVersion = _browserVersion;
    let _accessToken = options.accessToken ?? null;
    this.accessToken = _accessToken;
    let _seenAt = options.seenAt ?? null;
    this.seenAt = _seenAt;
    let _loggedInAt = options.loggedInAt ?? null;
    this.loggedInAt = _loggedInAt;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor instanceof Node) {
      _cursor = _cursor.toRef();
    }
    this.cursorPtr = _cursor;

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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.machinePtr?.id === other.machinePtr?.id)) {
      return false;
    }
    if (!(this.userPtr?.id === other.userPtr?.id)) {
      return false;
    }
    if (!(this.accessToken === other.accessToken)) {
      return false;
    }
    if (!(this.seenAt === other.seenAt)) {
      return false;
    }
    if (!(this.loggedInAt === other.loggedInAt)) {
      return false;
    }
    if (!(this.cursorPtr?.id === other.cursorPtr?.id)) {
      return false;
    }
    if (!(this.deviceType === other.deviceType)) {
      return false;
    }
    if (!(this.deviceName === other.deviceName)) {
      return false;
    }
    if (!(this.operatingSystem === other.operatingSystem)) {
      return false;
    }
    if (!(this.browserName === other.browserName)) {
      return false;
    }
    if (!(this.browserVersion === other.browserVersion)) {
      return false;
    }
    if (!(this.name === other.name)) {
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
      nodeType: NodeType.CLIENT,
      id: this.id,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
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

  toValue(): { [key: string]: any } {
    return Client.__packValue__(this);
  }

  static __packValue__(object: Client): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
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
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString();
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.machinePtr != null) {
      objectValue["36"] = object.machinePtr.toValue();
    }
    if (object.userPtr != null) {
      objectValue["37"] = object.userPtr.toValue();
    }
    if (object.deviceType != null) {
      objectValue["40"] = object.deviceType;
    }
    if (object.deviceName != null) {
      objectValue["41"] = object.deviceName;
    }
    if (object.operatingSystem != null) {
      objectValue["42"] = object.operatingSystem;
    }
    if (object.browserName != null) {
      objectValue["43"] = object.browserName;
    }
    if (object.browserVersion != null) {
      objectValue["44"] = object.browserVersion;
    }
    if (object.accessToken != null) {
      objectValue["50"] = object.accessToken;
    }
    if (object.seenAt != null) {
      objectValue["51"] = object.seenAt.toString();
    }
    if (object.loggedInAt != null) {
      objectValue["52"] = object.loggedInAt.toString();
    }
    if (object.cursorPtr != null) {
      objectValue["55"] = object.cursorPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const machinePtrValue = objectValue["36"];
    const unpackedMachinePtr =
      machinePtrValue != undefined
        ? NodeReference.fromValue(machinePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const userPtrValue = objectValue["37"];
    const unpackedUserPtr =
      userPtrValue != undefined
        ? NodeReference.fromValue(userPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const accessTokenValue = objectValue["50"];
    const unpackedAccessToken = accessTokenValue != undefined ? accessTokenValue : null;
    const seenAtValue = objectValue["51"];
    const unpackedSeenAt =
      seenAtValue != undefined ? Temporal.ZonedDateTime.from(seenAtValue) : null;
    const loggedInAtValue = objectValue["52"];
    const unpackedLoggedInAt =
      loggedInAtValue != undefined ? Temporal.ZonedDateTime.from(loggedInAtValue) : null;
    const cursorPtrValue = objectValue["55"];
    const unpackedCursorPtr =
      cursorPtrValue != undefined
        ? NodeReference.fromValue(cursorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deviceTypeValue = objectValue["40"];
    const unpackedDeviceType = deviceTypeValue != undefined ? deviceTypeValue : null;
    const deviceNameValue = objectValue["41"];
    const unpackedDeviceName = deviceNameValue != undefined ? deviceNameValue : null;
    const operatingSystemValue = objectValue["42"];
    const unpackedOperatingSystem = operatingSystemValue != undefined ? operatingSystemValue : null;
    const browserNameValue = objectValue["43"];
    const unpackedBrowserName = browserNameValue != undefined ? browserNameValue : null;
    const browserVersionValue = objectValue["44"];
    const unpackedBrowserVersion = browserVersionValue != undefined ? browserVersionValue : null;
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
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    return new Client({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      machine: unpackedMachinePtr,
      user: unpackedUserPtr,
      accessToken: unpackedAccessToken,
      seenAt: unpackedSeenAt,
      loggedInAt: unpackedLoggedInAt,
      cursor: unpackedCursorPtr,
      deviceType: unpackedDeviceType,
      deviceName: unpackedDeviceName,
      operatingSystem: unpackedOperatingSystem,
      browserName: unpackedBrowserName,
      browserVersion: unpackedBrowserVersion,
      name: objectValue["31"],
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
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
  ): Client {
    return Client.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ClientProto {
    return Client.__packProto__(this);
  }

  static __packProto__(object: Client): ClientProto {
    const objectProto: Partial<ClientProto> = { metatype: 100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
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
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.type = Number(object.type) as ClientTypeProto;
    objectProto.name = object.name;
    if (object.machinePtr != null) {
      objectProto.machinePtr = object.machinePtr.toProto();
    }
    if (object.userPtr != null) {
      objectProto.userPtr = object.userPtr.toProto();
    }
    if (object.deviceType != null) {
      objectProto.deviceType = object.deviceType;
    }
    if (object.deviceName != null) {
      objectProto.deviceName = object.deviceName;
    }
    if (object.operatingSystem != null) {
      objectProto.operatingSystem = object.operatingSystem;
    }
    if (object.browserName != null) {
      objectProto.browserName = object.browserName;
    }
    if (object.browserVersion != null) {
      objectProto.browserVersion = object.browserVersion;
    }
    if (object.accessToken != null) {
      objectProto.accessToken = object.accessToken;
    }
    if (object.seenAt != null) {
      objectProto.seenAt = packProtoTimestamp(object.seenAt);
    }
    if (object.loggedInAt != null) {
      objectProto.loggedInAt = packProtoTimestamp(object.loggedInAt);
    }
    if (object.cursorPtr != null) {
      objectProto.cursorPtr = object.cursorPtr.toProto();
    }
    return objectProto as ClientProto;
  }

  static __unpackProto__(
    objectProto: ClientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    return new Client({
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
      type: Number(objectProto.type) as ClientType,
      machine:
        objectProto.machinePtr != undefined
          ? NodeReference.fromProto(
              objectProto.machinePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      user:
        objectProto.userPtr != undefined
          ? NodeReference.fromProto(
              objectProto.userPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      accessToken: objectProto.accessToken != undefined ? objectProto.accessToken : null,
      seenAt: objectProto.seenAt != undefined ? unpackProtoTimestamp(objectProto.seenAt!) : null,
      loggedInAt:
        objectProto.loggedInAt != undefined ? unpackProtoTimestamp(objectProto.loggedInAt!) : null,
      cursor:
        objectProto.cursorPtr != undefined
          ? NodeReference.fromProto(
              objectProto.cursorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deviceType: objectProto.deviceType != undefined ? objectProto.deviceType : null,
      deviceName: objectProto.deviceName != undefined ? objectProto.deviceName : null,
      operatingSystem:
        objectProto.operatingSystem != undefined ? objectProto.operatingSystem : null,
      browserName: objectProto.browserName != undefined ? objectProto.browserName : null,
      browserVersion: objectProto.browserVersion != undefined ? objectProto.browserVersion : null,
      name: objectProto.name,
      id: String(objectProto.id),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ClientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Client {
    return Client.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Client {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ClientProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CLIENT, Client);
/* ==== DESTACK_GENERATED_END:NODE:100 ==== */
