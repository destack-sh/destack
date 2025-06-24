import {
  Entity,
  Global,
  Graph,
  HasIcon,
  HasName,
  HasSlug,
  Icon,
  IsJoinable,
  IsOwner,
  IsSubject,
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
/**
 * OrganizationStatus
 */
export enum OrganizationStatus {
  CREATING = 1,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:40 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40 ==== */
/**
 * An Organization with Users and Teams.
 */
export class Organization extends Node implements Global, Entity, HasSlug, HasIcon, HasName, IsOwner, IsJoinable {
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
   * HasName.name
   */
  name: string;

  /**
   * Organization.slug
   */
  slug: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * Organization.status
   */
  readonly status: OrganizationStatus;

  /**
   * Organization.space
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Organization.handle
   */
  get handle(): Handle | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Handle | null;
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

  toValue(): { [key: string]: any } {
    return Organization.__packValue__(this);
  }

  static __packValue__(object: Organization): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 40;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["31"] = object.name;
    objectValue["33"] = object.slug;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    objectValue["40"] = object.status;
    objectValue["50"] = object.spacePtr.toValue();
    if (object.handlePtr !== null) {
      objectValue["51"] = object.handlePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Organization {
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const handleValue = objectValue["51"];
    const unpackedHandle =
      handleValue !== undefined
        ? NodeReference.fromValue(handleValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Organization({
      slug: objectValue["33"],
      status: Number(objectValue["40"]),
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      icon: unpackedIcon,
      name: objectValue["31"],
      space: NodeReference.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      handle: unpackedHandle,
      parent: unpackedParent,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): Organization {
    return Organization.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:40 ==== */
