import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, ThreadCursor, Graph, ClientType, Machine, Struct, ScreenCursor, activeSession, Global, StructType, NodeType, EventCursor, Supergraph, QueryConnection, NodeReference, MaterializationType, User, IsDeletable, Entity, EnumType, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:STRUCT:50001 ==== */
export class Origin extends StructFrozen {
  readonly type: ClientType;
  readonly id: string | null;
  readonly ck: string | null;
  readonly nonce: string | null;

  constructor(options: {
    type: ClientType,
    id?: string | null,
    ck?: string | null,
    nonce?: string | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.id = options.id ?? null;
    this.ck = options.ck ?? null;
    this.nonce = options.nonce ?? null;
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

  set machine(node: Machine | null) {
      if (node === null) {
          this.machinePtr = null;
      } else {
          this.machinePtr = node.toRef();
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

  set user(node: User | null) {
      if (node === null) {
          this.userPtr = null;
      } else {
          this.userPtr = node.toRef();
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

  set cursor(node: EventCursor | ScreenCursor | ThreadCursor | null) {
      if (node === null) {
          this.cursorPtr = null;
      } else {
          this.cursorPtr = node.toRef();
      }
  }
  ;
  cursorPtr: NodeReference | null

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.name = options.name;
    this.machinePtr = options.machine != null ? (options.machine.metatype == StructType.NODE_REFERENCE ? (options.machine as NodeReference) : (options.machine as Node).toRef()) : null;
    this.userPtr = options.user != null ? (options.user.metatype == StructType.NODE_REFERENCE ? (options.user as NodeReference) : (options.user as Node).toRef()) : null;
    this.deviceType = options.deviceType ?? null;
    this.deviceName = options.deviceName ?? null;
    this.operatingSystem = options.operatingSystem ?? null;
    this.browserName = options.browserName ?? null;
    this.browserVersion = options.browserVersion ?? null;
    this.accessToken = options.accessToken ?? null;
    this.seenAt = options.seenAt ?? null;
    this.loggedInAt = options.loggedInAt ?? null;
    this.cursorPtr = options.cursor != null ? (options.cursor.metatype == StructType.NODE_REFERENCE ? (options.cursor as NodeReference) : (options.cursor as Node).toRef()) : null;
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