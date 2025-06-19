import { ClientType, EnumType, StructFrozen, Agent, EventCursor, StructType, QueryConnection, NodeType, Machine, Supergraph, Session, Global, Node, IsDeletable, IsTracked, ScreenCursor, Struct, MaterializationType, Graph, User, Entity, ThreadCursor, BuiltinObject, NodeReference } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:50001 ==== */
export class Origin extends StructFrozen {
  readonly type: ClientType;
  readonly id: string | null;
  readonly ck: string | null;
  readonly nonce: string | null;

  constructor(
    type: ClientType,
    id: string | null,
    ck: string | null,
    nonce: string | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.id = id;
    this.ck = ck;
    this.nonce = nonce;
  }


  static create(options: {
    type: ClientType,
    id?: string | null,
    ck?: string | null,
    nonce?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Origin {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Origin(
      options.type,
      options.id ?? null,
      options.ck ?? null,
      options.nonce ?? null,
      supergraph
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:50001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:100 ==== */
export class Client extends Node implements Global, Entity, IsTracked, IsDeletable {
  readonly id: string;
  get parent(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly deletedAt: Temporal.ZonedDateTime | null;
  type: ClientType;
  name: string;
  get machine(): Machine | null | null {
      const nodePtr: NodeReference | null = this.machinePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Machine | null | null;
      }
      return null;
  }

  set machine(value: Machine | null) {
      if (value === null) {
          this.machinePtr = null;
      } else {
          this.machinePtr = value.toRef();
      }
  }
  ;
  machinePtr: NodeReference | null
  get user(): User | null | null {
      const nodePtr: NodeReference | null = this.userPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as User | null | null;
      }
      return null;
  }

  set user(value: User | null) {
      if (value === null) {
          this.userPtr = null;
      } else {
          this.userPtr = value.toRef();
      }
  }
  ;
  userPtr: NodeReference | null
  deviceType: string | null;
  deviceName: string | null;
  operatingSystem: string | null;
  browserName: string | null;
  browserVersion: string | null;
  accessToken: string | null;
  seenAt: Temporal.ZonedDateTime | null;
  loggedInAt: Temporal.ZonedDateTime | null;
  get cursor(): EventCursor | ScreenCursor | ThreadCursor | null | null {
      const nodePtr: NodeReference | null = this.cursorPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as EventCursor | ScreenCursor | ThreadCursor | null | null;
      }
      return null;
  }

  set cursor(value: EventCursor | ScreenCursor | ThreadCursor | null) {
      if (value === null) {
          this.cursorPtr = null;
      } else {
          this.cursorPtr = value.toRef();
      }
  }
  ;
  cursorPtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    type: ClientType,
    name: string,
    machinePtr: NodeReference | null,
    userPtr: NodeReference | null,
    deviceType: string | null,
    deviceName: string | null,
    operatingSystem: string | null,
    browserName: string | null,
    browserVersion: string | null,
    accessToken: string | null,
    seenAt: Temporal.ZonedDateTime | null,
    loggedInAt: Temporal.ZonedDateTime | null,
    cursorPtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.type = type;
    this.name = name;
    this.machinePtr = machinePtr;
    this.userPtr = userPtr;
    this.deviceType = deviceType;
    this.deviceName = deviceName;
    this.operatingSystem = operatingSystem;
    this.browserName = browserName;
    this.browserVersion = browserVersion;
    this.accessToken = accessToken;
    this.seenAt = seenAt;
    this.loggedInAt = loggedInAt;
    this.cursorPtr = cursorPtr;
  }


  static create(options: {
    type: ClientType,
    name: string,
    machine?: Machine | NodeReference | null,
    user?: User | NodeReference | null,
    deviceType?: string | null,
    deviceName?: string | null,
    operatingSystem?: string | null,
    browserName?: string | null,
    browserVersion?: string | null,
    accessToken?: string | null,
    seenAt?: Temporal.ZonedDateTime | null,
    loggedInAt?: Temporal.ZonedDateTime | null,
    cursor?: EventCursor | ScreenCursor | ThreadCursor | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Client {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Client(
      options.type,
      options.name,
      options.machine != null ? (options.machine.metatype == StructType.NODE_REFERENCE ? options.machine : options.machine.toRef()) : null,
      options.user != null ? (options.user.metatype == StructType.NODE_REFERENCE ? options.user : options.user.toRef()) : null,
      options.deviceType ?? null,
      options.deviceName ?? null,
      options.operatingSystem ?? null,
      options.browserName ?? null,
      options.browserVersion ?? null,
      options.accessToken ?? null,
      options.seenAt ?? null,
      options.loggedInAt ?? null,
      options.cursor != null ? (options.cursor.metatype == StructType.NODE_REFERENCE ? options.cursor : options.cursor.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.CLIENT, this.id, null, null, this._supergraph);

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
}
/* ==== DESTACK_GENERATED_END:NODE:100 ==== */