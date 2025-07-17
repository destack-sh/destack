import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { NodeType, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import type { Snapshot } from "@destack/language/core/builtin/entity";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import { Event } from "@destack/language/core/builtin/event";
import { ConstraintType, IndexType } from "@destack/language/core/builtin/meta";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference, PropertyReference } from "@destack/language/core/builtin/relation";
import type { IsActor, IsExtensible, IsTaggable } from "@destack/language/core/builtin/trait";
import { BuiltinDefinition } from "@destack/language/core/common/definition";
import type { Icon } from "@destack/language/core/common/icon";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import {
  ConstraintDefinitionProto,
  ConstraintProto,
  ConstraintTypeProto,
  IndexDefinitionProto,
  IndexProto,
  IndexTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:30200 ==== */
/**
 * Definition of a builtin Constraint.
 */
export class ConstraintDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.CONSTRAINT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * ConstraintDefinition.type
   */
  readonly type: ConstraintType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstraintDefinition.properties
   */
  readonly properties: readonly PropertyReference[];

  constructor(options: {
    id: number;
    type: ConstraintType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyReference[];
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
    let _id = options.id;
    if (_id === null) {
      throw new Error(`ConstraintDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`ConstraintDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ConstraintDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;

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
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ConstraintType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ConstraintDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = ConstraintDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ConstraintDefinition): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 30200;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ConstraintDefinition({
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    return ConstraintDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ConstraintDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ConstraintDefinition.__packProto__(this);
    }
    return this._proto as ConstraintDefinitionProto;
  }

  static __packProto__(object: ConstraintDefinition): ConstraintDefinitionProto {
    const objectProto: Partial<ConstraintDefinitionProto> = { metatype: 30200 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as ConstraintTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    return objectProto as ConstraintDefinitionProto;
  }

  static __unpackProto__(
    objectProto: ConstraintDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ConstraintDefinition({
      type: Number(objectProto.type) as ConstraintType,
      properties: unpackedProperties,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ConstraintDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    return ConstraintDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ConstraintDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConstraintDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONSTRAINT_DEFINITION, ConstraintDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:30200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:30200 ==== */
/**
 * Constraint of an Entity.
 */
export class Constraint extends Entity implements IsTaggable {
  static metatype: NodeType = NodeType.CONSTRAINT;

  /**
   * Constraint.parent
   */
  get parent(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
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
  get precededBy(): Constraint | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Constraint | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

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
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

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
   * The time this Entity was deleted (system time).
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

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
   * Constraint.type
   */
  /**
   * Constraint.type
   */
  get type(): ConstraintType {
    return this._type;
  }
  set type(value: ConstraintType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: ConstraintType;

  /**
   * Constraint.properties
   */
  /**
   * Constraint.properties
   */
  get properties(): readonly PropertyReference[] {
    return this._properties;
  }
  set properties(value: readonly PropertyReference[]) {
    const prop = (this.constructor as NodeClass).__properties__["properties"];
    this._session.updateSetProperty(this, prop, value);
    this._properties = value;
  }
  _properties: readonly PropertyReference[];

  constructor(options: {
    id?: string;
    parent?: (Entity & IsExtensible) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Constraint | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    name?: string;
    type: ConstraintType;
    properties?: readonly PropertyReference[];
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
        throw new Error(`Constraint has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Constraint has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Constraint.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Constraint.materialization is required`);
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
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Constraint";
    }
    if (_name === null) {
      throw new Error(`Constraint.name is required`);
    }
    this._name = _name;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Constraint.type is required`);
    }
    this._type = _type;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this._properties = _properties;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(
          `Constraint.createdAt and Constraint.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (this._properties.length != other._properties.length) {
      return false;
    }
    for (let i = 0; i < this._properties.length; i++) {
      if (!this._properties[i].equals(other._properties[i])) {
        return false;
      }
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
    if (this._properties && this._properties.length > 0) {
      for (const _item of this._properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
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
      type: NodeType.CONSTRAINT,
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
    propertyReprs.push(`type=${ConstraintType[this.type]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Constraint "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Constraint.__packValue__(this);
  }

  static __packValue__(object: Constraint): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 30200;
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
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["50"] = object._name;
    objectValue["100"] = object._type;
    if (object._properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Constraint {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new Constraint({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
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
  ): Constraint {
    return Constraint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ConstraintProto {
    return Constraint.__packProto__(this);
  }

  static __packProto__(object: Constraint): ConstraintProto {
    const objectProto: Partial<ConstraintProto> = { metatype: 30200 };
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
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.name = object._name;
    objectProto.type = Number(object._type) as ConstraintTypeProto;
    if (object._properties) {
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    return objectProto as ConstraintProto;
  }

  static __unpackProto__(
    objectProto: ConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Constraint {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Constraint({
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
      type: Number(objectProto.type) as ConstraintType,
      properties: unpackedProperties,
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
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
    objectProto: ConstraintProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Constraint {
    return Constraint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Constraint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConstraintProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CONSTRAINT, Constraint);
/* ==== DESTACK_GENERATED_END:NODE:30200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:30100 ==== */
/**
 * Definition of a builtin Index.
 */
export class IndexDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.INDEX_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * IndexDefinition.type
   */
  readonly type: IndexType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * IndexDefinition.properties
   */
  readonly properties: readonly PropertyReference[];

  /**
   * IndexDefinition.cover
   */
  readonly cover: readonly PropertyReference[];

  constructor(options: {
    id: number;
    type: IndexType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyReference[];
    cover?: readonly PropertyReference[];
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
    let _id = options.id;
    if (_id === null) {
      throw new Error(`IndexDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`IndexDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`IndexDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _cover = options.cover ?? null;
    if (_cover === null) {
      _cover = [];
    }
    this.cover = _cover;

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
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.cover.length != other.cover.length) {
      return false;
    }
    for (let i = 0; i < this.cover.length; i++) {
      if (!this.cover[i].equals(other.cover[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${IndexType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<IndexDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.cover && this.cover.length > 0) {
      for (const _item of this.cover) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = IndexDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: IndexDefinition): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 30100;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    if (object.cover.length > 0) {
      const packedCover: any[] = [];
      for (const item of object.cover) {
        packedCover.push(item.toValue());
      }
      objectValue["106"] = packedCover;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedCover: any[] = [];
    if (objectValue["106"] != undefined) {
      for (const item of objectValue["106"]) {
        unpackedCover.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new IndexDefinition({
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      cover: unpackedCover,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    return IndexDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): IndexDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = IndexDefinition.__packProto__(this);
    }
    return this._proto as IndexDefinitionProto;
  }

  static __packProto__(object: IndexDefinition): IndexDefinitionProto {
    const objectProto: Partial<IndexDefinitionProto> = { metatype: 30100 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as IndexTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.cover) {
      const packedCover: any[] = [];
      for (const item of object.cover) {
        packedCover.push(item.toProto());
      }
      objectProto.cover = packedCover;
    }
    return objectProto as IndexDefinitionProto;
  }

  static __unpackProto__(
    objectProto: IndexDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedCover: any[] = [];
    if (objectProto.cover) {
      for (const item of objectProto.cover) {
        unpackedCover.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new IndexDefinition({
      type: Number(objectProto.type) as IndexType,
      properties: unpackedProperties,
      cover: unpackedCover,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: IndexDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    return IndexDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): IndexDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = IndexDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.INDEX_DEFINITION, IndexDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:30100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:30100 ==== */
/**
 * Index of an Entity.
 */
export class Index extends Entity implements IsTaggable {
  static metatype: NodeType = NodeType.INDEX;

  /**
   * Index.parent
   */
  get parent(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
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
  get precededBy(): Index | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Index | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

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
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

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
   * The time this Entity was deleted (system time).
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

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
   * Index.type
   */
  /**
   * Index.type
   */
  get type(): IndexType {
    return this._type;
  }
  set type(value: IndexType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: IndexType;

  /**
   * Index.properties
   */
  /**
   * Index.properties
   */
  get properties(): readonly PropertyReference[] {
    return this._properties;
  }
  set properties(value: readonly PropertyReference[]) {
    const prop = (this.constructor as NodeClass).__properties__["properties"];
    this._session.updateSetProperty(this, prop, value);
    this._properties = value;
  }
  _properties: readonly PropertyReference[];

  constructor(options: {
    id?: string;
    parent?: (Entity & IsExtensible) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: Index | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    name?: string;
    type: IndexType;
    properties?: readonly PropertyReference[];
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
        throw new Error(`Index has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`Index has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Index.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Index.materialization is required`);
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
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Index";
    }
    if (_name === null) {
      throw new Error(`Index.name is required`);
    }
    this._name = _name;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Index.type is required`);
    }
    this._type = _type;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this._properties = _properties;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(`Index.createdAt and Index.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (this._properties.length != other._properties.length) {
      return false;
    }
    for (let i = 0; i < this._properties.length; i++) {
      if (!this._properties[i].equals(other._properties[i])) {
        return false;
      }
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
    if (this._properties && this._properties.length > 0) {
      for (const _item of this._properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
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
      type: NodeType.INDEX,
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
    propertyReprs.push(`type=${IndexType[this.type]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Index "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Index.__packValue__(this);
  }

  static __packValue__(object: Index): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 30100;
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
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["50"] = object._name;
    objectValue["100"] = object._type;
    if (object._properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Index {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new Index({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
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
  ): Index {
    return Index.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): IndexProto {
    return Index.__packProto__(this);
  }

  static __packProto__(object: Index): IndexProto {
    const objectProto: Partial<IndexProto> = { metatype: 30100 };
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
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.name = object._name;
    objectProto.type = Number(object._type) as IndexTypeProto;
    if (object._properties) {
      const packedProperties: any[] = [];
      for (const item of object._properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    return objectProto as IndexProto;
  }

  static __unpackProto__(
    objectProto: IndexProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Index {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Index({
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
      type: Number(objectProto.type) as IndexType,
      properties: unpackedProperties,
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
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
    objectProto: IndexProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Index {
    return Index.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Index {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = IndexProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INDEX, Index);
/* ==== DESTACK_GENERATED_END:NODE:30100 ==== */
