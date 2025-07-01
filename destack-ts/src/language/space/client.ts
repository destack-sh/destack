import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  HasName,
  IsDeletable,
  IsGlobal,
  IsSubject,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  ClientType,
  Entity,
  Graph,
  Node,
  NodeReference,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Machine } from "@destack/language/infra";
import type { Cursor } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import type { User } from "@destack/language/space";
import { ClientProto, ClientTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:10700 ==== */
/**
 * A Client to connect with the system.
 */
export class Client extends Entity implements HasName, IsGlobal, IsDeletable {
  static metatype: NodeType = NodeType.CLIENT;

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
  get cursor(): Cursor | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Cursor | null;
    }
    return null;
  }
  set cursor(node: Cursor | null) {
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
    cursor?: Cursor | NodeReference | null;
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
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.machinePtr !== null) {
      h = (h * 31 + hashString(this.machinePtr.id)) & 0xffffffff;
    }
    if (this.userPtr !== null) {
      h = (h * 31 + hashString(this.userPtr.id)) & 0xffffffff;
    }
    if (this.accessToken !== null) {
      h = (h * 31 + hashString(this.accessToken)) & 0xffffffff;
    }
    if (this.seenAt !== null) {
      h = (h * 31 + hashString(this.seenAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.loggedInAt !== null) {
      h = (h * 31 + hashString(this.loggedInAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.cursorPtr !== null) {
      h = (h * 31 + hashString(this.cursorPtr.id)) & 0xffffffff;
    }
    if (this.deviceType !== null) {
      h = (h * 31 + hashString(this.deviceType)) & 0xffffffff;
    }
    if (this.deviceName !== null) {
      h = (h * 31 + hashString(this.deviceName)) & 0xffffffff;
    }
    if (this.operatingSystem !== null) {
      h = (h * 31 + hashString(this.operatingSystem)) & 0xffffffff;
    }
    if (this.browserName !== null) {
      h = (h * 31 + hashString(this.browserName)) & 0xffffffff;
    }
    if (this.browserVersion !== null) {
      h = (h * 31 + hashString(this.browserVersion)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`name=${this.name}`);
    return `<Client '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Client.__packValue__(this);
  }

  static __packValue__(object: Client): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 10700;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
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
      objectValue["51"] = object.seenAt.toString({ timeZoneName: "never" });
    }
    if (object.loggedInAt != null) {
      objectValue["52"] = object.loggedInAt.toString({ timeZoneName: "never" });
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
      seenAtValue != undefined
        ? Temporal.Instant.from(seenAtValue).toZonedDateTimeISO("UTC")
        : null;
    const loggedInAtValue = objectValue["52"];
    const unpackedLoggedInAt =
      loggedInAtValue != undefined
        ? Temporal.Instant.from(loggedInAtValue).toZonedDateTimeISO("UTC")
        : null;
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
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
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
  ): Client {
    return Client.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ClientProto {
    return Client.__packProto__(this);
  }

  static __packProto__(object: Client): ClientProto {
    const objectProto: Partial<ClientProto> = { metatype: 10700 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
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
      id: String(objectProto.id),
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
/* ==== DESTACK_GENERATED_END:NODE:10700 ==== */
