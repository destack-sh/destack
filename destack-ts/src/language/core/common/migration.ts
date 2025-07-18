import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { EnumType, NodeType, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_SNAPSHOT, ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import type { Snapshot } from "@destack/language/core/builtin/entity";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import { Event } from "@destack/language/core/builtin/event";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { IsActor, IsExtensible } from "@destack/language/core/builtin/trait";
import { BuiltinDefinition } from "@destack/language/core/common/definition";
import type { Icon } from "@destack/language/core/common/icon";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import {
  MaterializationProto,
  MigrationDefinitionProto,
  MigrationOperationDefinitionProto,
  MigrationOperationProto,
  MigrationProto,
  MigrationTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:31000 ==== */
/**
 * MigrationType
 */
export enum MigrationType {
  CREATE = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.MIGRATION_TYPE, MigrationType);
/* ==== DESTACK_GENERATED_END:ENUM:31000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:31000 ==== */
/**
 * Definition of a builtin Migration.
 */
export class MigrationDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.MIGRATION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * MigrationDefinition.type
   */
  readonly type: MigrationType;

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

  constructor(options: {
    id: number;
    type: MigrationType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
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
      throw new Error(`MigrationDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`MigrationDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`MigrationDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;

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
      propertyReprs.push(`type=${MigrationType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<MigrationDefinition ${propertyReprs.join(" ")}>`;
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
      this._value = MigrationDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: MigrationDefinition): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 31000;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new MigrationDefinition({
      type: Number(objectValue["100"]),
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
  ): MigrationDefinition {
    return MigrationDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MigrationDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = MigrationDefinition.__packProto__(this);
    }
    return this._proto as MigrationDefinitionProto;
  }

  static __packProto__(object: MigrationDefinition): MigrationDefinitionProto {
    const objectProto: Partial<MigrationDefinitionProto> = { metatype: 31000 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as MigrationTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    return objectProto as MigrationDefinitionProto;
  }

  static __unpackProto__(
    objectProto: MigrationDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new MigrationDefinition({
      type: Number(objectProto.type) as MigrationType,
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
    objectProto: MigrationDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationDefinition {
    return MigrationDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MigrationDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MigrationDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.MIGRATION_DEFINITION, MigrationDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:31000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:31100 ==== */
/**
 * Definition of a builtin MigrationOperation.
 */
export class MigrationOperationDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.MIGRATION_OPERATION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

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

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
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
      throw new Error(`MigrationOperationDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`MigrationOperationDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;

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
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<MigrationOperationDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
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
      this._value = MigrationOperationDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: MigrationOperationDefinition): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 31100;
    objectValue["2"] = object.id;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperationDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new MigrationOperationDefinition({
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
  ): MigrationOperationDefinition {
    return MigrationOperationDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MigrationOperationDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = MigrationOperationDefinition.__packProto__(this);
    }
    return this._proto as MigrationOperationDefinitionProto;
  }

  static __packProto__(object: MigrationOperationDefinition): MigrationOperationDefinitionProto {
    const objectProto: Partial<MigrationOperationDefinitionProto> = { metatype: 31100 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    return objectProto as MigrationOperationDefinitionProto;
  }

  static __unpackProto__(
    objectProto: MigrationOperationDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperationDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new MigrationOperationDefinition({
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
    objectProto: MigrationOperationDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperationDefinition {
    return MigrationOperationDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MigrationOperationDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MigrationOperationDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.MIGRATION_OPERATION_DEFINITION, MigrationOperationDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:31100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:31000 ==== */
/**
 * Migration of an Entity.
 */
export class Migration extends Entity {
  static metatype: NodeType = NodeType.MIGRATION;

  /**
   * Migration.parent
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
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Snapshot).
   */
  get precededBy(): Migration | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Migration | null;
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
   * Migration.type
   */
  /**
   * Migration.type
   */
  get type(): MigrationType {
    return this._type;
  }
  set type(value: MigrationType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: MigrationType;

  constructor(options: {
    id?: string;
    parent?: (Entity & IsExtensible) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Migration | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    name?: string;
    type: MigrationType;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for Migration`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Migration.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Migration.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Migration`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Migration.snapshot is required`);
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
      _name = "Migration";
    }
    if (_name === null) {
      throw new Error(`Migration.name is required`);
    }
    this._name = _name;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Migration.type is required`);
    }
    this._type = _type;

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
          `Migration.createdAt and Migration.updatedAt are required for existing Nodes`,
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
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
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
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
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
      type: NodeType.MIGRATION,
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
    propertyReprs.push(`type=${MigrationType[this.type]}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Migration "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Migration.__packValue__(this);
  }

  static __packValue__(object: Migration): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 31000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    objectValue["11"] = object.snapshotPtr.toValue();
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
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Migration {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
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
    return new Migration({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      materialization: Number(objectValue["10"]),
      snapshot: _NodeReference.fromValue(
        objectValue["11"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
  ): Migration {
    return Migration.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): MigrationProto {
    return Migration.__packProto__(this);
  }

  static __packProto__(object: Migration): MigrationProto {
    const objectProto: Partial<MigrationProto> = { metatype: 31000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
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
    objectProto.type = Number(object._type) as MigrationTypeProto;
    return objectProto as MigrationProto;
  }

  static __unpackProto__(
    objectProto: MigrationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Migration {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Migration({
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
      type: Number(objectProto.type) as MigrationType,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
    objectProto: MigrationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Migration {
    return Migration.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Migration {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MigrationProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MIGRATION, Migration);
/* ==== DESTACK_GENERATED_END:NODE:31000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:31100 ==== */
/**
 * MigrationOperation of an Entity.
 */
export class MigrationOperation extends Entity {
  static metatype: NodeType = NodeType.MIGRATION_OPERATION;

  /**
   * MigrationOperation.parent
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
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Snapshot).
   */
  get precededBy(): MigrationOperation | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as MigrationOperation | null;
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

  constructor(options: {
    id?: string;
    parent?: (Entity & IsExtensible) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference;
    precededBy?: MigrationOperation | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    name?: string;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for MigrationOperation`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`MigrationOperation.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`MigrationOperation.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for MigrationOperation`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`MigrationOperation.snapshot is required`);
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
      _name = "MigrationOperation";
    }
    if (_name === null) {
      throw new Error(`MigrationOperation.name is required`);
    }
    this._name = _name;

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
          `MigrationOperation.createdAt and MigrationOperation.updatedAt are required for existing Nodes`,
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
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
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
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
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
      type: NodeType.MIGRATION_OPERATION,
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<MigrationOperation "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return MigrationOperation.__packValue__(this);
  }

  static __packValue__(object: MigrationOperation): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 31100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    objectValue["11"] = object.snapshotPtr.toValue();
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
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperation {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
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
    return new MigrationOperation({
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      snapshot: _NodeReference.fromValue(
        objectValue["11"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
  ): MigrationOperation {
    return MigrationOperation.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MigrationOperationProto {
    return MigrationOperation.__packProto__(this);
  }

  static __packProto__(object: MigrationOperation): MigrationOperationProto {
    const objectProto: Partial<MigrationOperationProto> = { metatype: 31100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
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
    return objectProto as MigrationOperationProto;
  }

  static __unpackProto__(
    objectProto: MigrationOperationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperation {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new MigrationOperation({
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
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
    objectProto: MigrationOperationProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MigrationOperation {
    return MigrationOperation.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MigrationOperation {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MigrationOperationProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MIGRATION_OPERATION, MigrationOperation);
/* ==== DESTACK_GENERATED_END:NODE:31100 ==== */
