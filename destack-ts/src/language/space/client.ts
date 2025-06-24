import {
  ClientType,
  Entity,
  Global,
  Graph,
  IsDeletable,
  IsSubject,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Machine } from "@destack/language/infra";
import { Cursor } from "@destack/language/logic";
import { User } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:50001 ==== */
export class Origin extends StructFrozen {
  static metatype: StructType = StructType.ORIGIN;
  static __isFrozen__: boolean = true;

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
    // ...
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
  static metatype: NodeType = NodeType.CLIENT;
  static __traits__: TraitType[] = [TraitType.GLOBAL, TraitType.ENTITY, TraitType.TRACKED, TraitType.DELETABLE];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.AGENT, NodeType.USER];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.AGENT, NodeType.FOLDER, NodeType.USER, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  get parent(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
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
  get cursor(): (Node & Cursor) | null | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & Cursor) | null | null;
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
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
