import {
  Agent,
  ClientType,
  Entity,
  EventCursor,
  Global,
  Graph,
  IsDeletable,
  IsTracked,
  Machine,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  ScreenCursor,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  ThreadCursor,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:50001 ==== */
export class Origin extends StructFrozen {
  readonly type: ClientType;
  readonly id: string | null;
  readonly ck: string | null;
  readonly nonce: string | null;

  constructor(options: {
    type: ClientType;
    id?: string | null;
    ck?: string | null;
    nonce?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.id = options.id ?? null;
    this.ck = options.ck ?? null;
    this.nonce = options.nonce ?? null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
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
  readonly parentPtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
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
  machinePtr: NodeReference | null;
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
  userPtr: NodeReference | null;
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
  cursorPtr: NodeReference | null;

  constructor(options: {
    id: string;
    parent?: Agent | User | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
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
    cursor?: EventCursor | ScreenCursor | ThreadCursor | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.deletedAt = options.deletedAt ?? null;
    this.type = options.type;
    this.name = options.name;
    this.machinePtr =
      options.machine != null
        ? options.machine.metatype == StructType.NODE_REFERENCE
          ? (options.machine as NodeReference)
          : (options.machine as Node).toRef()
        : null;
    this.userPtr =
      options.user != null
        ? options.user.metatype == StructType.NODE_REFERENCE
          ? (options.user as NodeReference)
          : (options.user as Node).toRef()
        : null;
    this.deviceType = options.deviceType ?? null;
    this.deviceName = options.deviceName ?? null;
    this.operatingSystem = options.operatingSystem ?? null;
    this.browserName = options.browserName ?? null;
    this.browserVersion = options.browserVersion ?? null;
    this.accessToken = options.accessToken ?? null;
    this.seenAt = options.seenAt ?? null;
    this.loggedInAt = options.loggedInAt ?? null;
    this.cursorPtr =
      options.cursor != null
        ? options.cursor.metatype == StructType.NODE_REFERENCE
          ? (options.cursor as NodeReference)
          : (options.cursor as Node).toRef()
        : null;
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
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
}
/* ==== DESTACK_GENERATED_END:NODE:100 ==== */
