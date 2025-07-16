import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  Icon,
  IsActor,
  IsJoinable,
  IsSourceable,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Folder } from "@destack/language/space";
import type { Space } from "@destack/language/universe";
import { MaterializationProto, PermissionProto, PermissionTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:300300 ==== */
/**
 * PermissionType
 */
export enum PermissionType {
  GENERAL = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PERMISSION_TYPE, PermissionType);
/* ==== DESTACK_GENERATED_END:ENUM:300300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:360300 ==== */
/**
 * A Permission for something.
 */
export class Permission extends Entity implements IsSourceable {
  static metatype: NodeType = NodeType.PERMISSION;

  /**
   * Permission.parent
   */
  get parent(): (Entity & IsJoinable) | Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsJoinable) | Folder | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get precededBy(): Permission | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Permission | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * Entity.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

  /**
   * Permission.type
   */
  /**
   * Permission.type
   */
  get type(): PermissionType {
    return this._type;
  }
  set type(value: PermissionType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: PermissionType;

  /**
   * Permission.icon
   */
  /**
   * Permission.icon
   */
  get icon(): Icon | null {
    return this._icon;
  }
  set icon(value: Icon | null) {
    const prop = (this.constructor as NodeClass).__properties__["icon"];
    this._session.updateSetProperty(this, prop, value);
    this._icon = value;
  }
  _icon: Icon | null;

  constructor(options: {
    id?: string;
    parent?: (Entity & IsJoinable) | Folder | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Permission | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    name?: string;
    source?: Script | NodeReference | null;
    key?: string | null;
    type: PermissionType;
    icon?: Icon | null;
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
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`Permission has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Permission has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Permission.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Permission.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Permission.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Permission";
    }
    if (_name === null) {
      throw new Error(`Permission.name is required`);
    }
    this._name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Permission.type is required`);
    }
    this._type = _type;
    let _icon = options.icon ?? null;
    this._icon = _icon;

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
          `Permission.createdAt and Permission.updatedAt are required for existing Nodes`,
        );
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
    if (
      (this._icon == null) !== (other._icon == null) ||
      (this._icon != null && !this._icon.equals(other._icon))
    ) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._icon != null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.PERMISSION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${PermissionType[this.type]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Permission "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Permission.__packValue__(this);
  }

  static __packValue__(object: Permission): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 360300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
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
      objectValue["24"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["27"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    if (object._key != null) {
      objectValue["70"] = object._key;
    }
    objectValue["100"] = object._type;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Permission {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["70"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
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
    const deletedAtValue = objectValue["24"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new Permission({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      icon: unpackedIcon,
      source: unpackedSourcePtr,
      key: unpackedKey,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      orderKey: objectValue["27"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Permission {
    return Permission.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): PermissionProto {
    return Permission.__packProto__(this);
  }

  static __packProto__(object: Permission): PermissionProto {
    const objectProto: Partial<PermissionProto> = { metatype: 360300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
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
    objectProto.orderKey = object.orderKey;
    objectProto.name = object._name;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.type = Number(object._type) as PermissionTypeProto;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
    }
    return objectProto as PermissionProto;
  }

  static __unpackProto__(
    objectProto: PermissionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Permission {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new Permission({
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
      type: Number(objectProto.type) as PermissionType,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
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
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: PermissionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Permission {
    return Permission.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Permission {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PermissionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.PERMISSION, Permission);
/* ==== DESTACK_GENERATED_END:NODE:360300 ==== */
