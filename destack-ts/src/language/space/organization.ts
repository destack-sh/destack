import {
  Entity,
  Global,
  Graph,
  Icon,
  IsJoinable,
  IsOwner,
  IsSubject,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Handle, Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:40 ==== */
export enum OrganizationStatus {
  CREATING = 1,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:40 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40 ==== */
export class Organization extends Node implements Global, Entity, IsTracked, IsJoinable, IsOwner {
  static metatype: NodeType = NodeType.ORGANIZATION;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.JOINABLE,
    TraitType.OWNER,
  ];
  static __rootType__: NodeType | null = null;
  static __parentTypes__: NodeType[] = [];
  static __childTypes__: NodeType[] = [
    NodeType.ENTITLEMENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE,
    NodeType.SANCTION,
  ];
  static __ancestorTypes__: NodeType[] = [];
  static __descendantTypes__: NodeType[] = [
    NodeType.ENTITLEMENT,
    NodeType.ROLE,
    NodeType.PERMISSION,
    NodeType.MEMBERSHIP,
    NodeType.SANCTION,
    NodeType.INVITE,
  ];

  get parent(): Node | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null | null;
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
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: OrganizationStatus;
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;
  get handle(): Handle | null | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null | null;
    }
    return null;
  }
  readonly handlePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    name: string;
    slug: string;
    icon?: Icon | null;
    status?: OrganizationStatus;
    space: Space | NodeReference;
    handle?: Handle | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`Organization.materialization is required`);
    }
    this.materialization = _materialization;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Organization.name is required`);
    }
    this.name = _name;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Organization.slug is required`);
    }
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = OrganizationStatus.CREATING;
    }
    if (_status === null) {
      throw new Error(`Organization.status is required`);
    }
    this.status = _status;
    let _space = options.space;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Organization.space is required`);
    }
    this.spacePtr = _space;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle instanceof Node) {
      _handle = _handle.toRef();
    }
    this.handlePtr = _handle;

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
      nodeType: NodeType.ORGANIZATION,
      id: this.id,
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
}
/* ==== DESTACK_GENERATED_END:NODE:40 ==== */
