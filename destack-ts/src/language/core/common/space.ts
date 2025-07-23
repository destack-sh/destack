import { NodeType, Region, StructType } from "@destack/language/core/builtin/common";
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT } from "@destack/language/core/builtin/const";
import type {
  EnumDefinition,
  NodeDefinition,
  StructDefinition,
  TraitDefinition,
} from "@destack/language/core/builtin/definition";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsFollowable,
  IsJoinable,
  IsOwnable,
  IsStarable,
} from "@destack/language/core/builtin/trait";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import { BranchType, SnapshotType } from "@destack/language/core/common/time";
import type { Value } from "@destack/language/core/common/value";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  NODE_CLASS_BY_TYPE,
  STRUCT_CLASS_BY_TYPE,
  registerNodeClass,
} from "@destack/language/registry";
import type { Handle } from "@destack/language/universe";
import { hashBool, hashString } from "@destack/utils/hash";
import { uuid4 } from "@destack/utils/uuid";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1000 ==== */
/**
 * The Destack computational universe.
 */
export abstract class Universe extends Entity {
  static metatype: NodeType = NodeType.UNIVERSE;

  /**
   * The current version of Destack.
   */
  static readonly VERSION: string = "2025.07.21.0";

  /**
   * The float epsilon used for floating point comparisons.
   */
  static readonly EPSILON: number = 1e-6;

  /**
   * The beginning of time. (1970-01-01T00:00:00+00:00)
   */
  static readonly BEGINNING_OF_TIME: Temporal.ZonedDateTime = Temporal.Instant.from(
    "1970-01-01 00:00:00+00:00",
  ).toZonedDateTimeISO("UTC");

  /**
   * All Node definitions.
   */
  static readonly NODES: readonly NodeDefinition[] = undefined as any /* (deferred) */;

  /**
   * All Trait definitions.
   */
  static readonly TRAITS: readonly TraitDefinition[] = undefined as any /* (deferred) */;

  /**
   * All Struct definitions.
   */
  static readonly STRUCTS: readonly StructDefinition[] = undefined as any /* (deferred) */;

  /**
   * All Enum definitions.
   */
  static readonly ENUMS: readonly EnumDefinition[] = undefined as any /* (deferred) */;

  /**
   * The system Space ID.
   */
  static readonly SPACE_ID: string = "00000000-0000-0000-0000-000000000000";

  /**
   * The system Space.
   */
  static readonly SPACE: NodeReference = undefined as any /* (deferred) */;

  /**
   * The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)
   */
  static readonly META_SNAPSHOT_ID: string = "00000000-0000-0000-0000-000000000014";

  /**
   * The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)
   */
  static readonly META_BRANCH_ID: string = "00000000-0000-0000-0000-000000000015";

  /**
   * The 'root' Branch.id, the Branch all other Branches originate from.
   */
  static readonly ROOT_BRANCH_ID: string = "00000000-0000-0000-0000-00000000001f";

  /**
   * The 'head' Snapshot.id, the current active Snapshot.
   */
  static readonly HEAD_SNAPSHOT_ID: string = "00000000-0000-0000-0000-00000000001e";

  /**
   * God himself, the creator of the Universe.
   */
  static readonly ACTOR: NodeReference = undefined as any /* (deferred) */;

  /**
   * God's terminal, for when He needs to do something.
   */
  static readonly CLIENT: NodeReference = undefined as any /* (deferred) */;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  abstract get parent(): Entity | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  abstract get branch(): Branch | null;
  declare readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  abstract get precededBy(): Universe | null;
  declare readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByPtr: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByPtr(): NodeReference | null;
  abstract set ownedByPtr(value: NodeReference | null);

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  abstract get name(): string;
  abstract set name(value: string);

  /**
   * The absolute order key of this Entity in its parent.
   */
  declare readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  abstract get customValues(): { readonly [key: string]: Value };
  abstract set customValues(value: { readonly [key: string]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptPtr(): NodeReference | null;
  abstract set scriptPtr(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  abstract get source(): Script | null;
  declare readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  abstract get key(): string | null;
  abstract set key(value: string | null);

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.UNIVERSE, Universe);
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1100 ==== */
/**
 * A Space is the home of your personal software studio.
 */
export class Space extends Entity implements IsFollowable, IsJoinable, IsOwnable, IsStarable {
  static metatype: NodeType = NodeType.SPACE;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
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
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Space | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Space | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

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
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

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
  get updatedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

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
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptPtr = null;
    } else {
      this.scriptPtr = node.toRef();
    }
  }
  /**
   * The Script of this Entity.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
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
   * Space.slug
   */
  /**
   * Space.slug
   */
  get slug(): string {
    return this._slug;
  }
  set slug(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["slug"];
    this._session.updateSetProperty(this, prop, value);
    this._slug = value;
  }
  _slug: string;

  /**
   * Space.handle
   */
  get handle(): Handle | null {
    const nodePtr: NodeReference | null = this.handlePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Handle | null;
    }
    return null;
  }
  set handle(node: Handle | null) {
    if (node === null) {
      this.handlePtr = null;
    } else {
      this.handlePtr = node.toRef();
    }
  }
  /**
   * Space.handle
   */
  get handlePtr(): NodeReference | null {
    return this._handlePtr;
  }
  set handlePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["handle"];
    this._session.updateSetProperty(this, prop, value);
    this._handlePtr = value;
  }
  _handlePtr: NodeReference | null;

  /**
   * Space.region
   */
  /**
   * Space.region
   */
  get region(): Region {
    return this._region;
  }
  set region(value: Region) {
    const prop = (this.constructor as NodeClass).__properties__["region"];
    this._session.updateSetProperty(this, prop, value);
    this._region = value;
  }
  _region: Region;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Space | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: Entity | NodeReference;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    slug: string;
    handle?: Handle | NodeReference | null;
    region: Region;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null
        ? options.parent.constructor.name === "NodeReference"
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name !== "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = this.toRef();
    }
    if (_space === null) {
      throw new Error(`Space.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Space.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for Space`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Space.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Space`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Space.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Space";
    }
    if (_name === null) {
      throw new Error(`Space.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Space.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Space.slug is required`);
    }
    this._slug = _slug;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.constructor.name !== "NodeReference") {
      _handle = (_handle as Node).toRef();
    }
    this._handlePtr = _handle as NodeReference | null;
    let _region = options.region;
    if (_region === null) {
      throw new Error(`Space.region is required`);
    }
    this._region = _region;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = this._session.actorPtr;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(`Space.createdAt and Space.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name === "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name === "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : this._session.actorPtr;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (!(this._slug === other._slug)) {
      return false;
    }
    if (!(this._handlePtr?.id === other._handlePtr?.id)) {
      return false;
    }
    if (!(this._region === other._region)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    if (this._handlePtr != null) {
      h = (h * 31 + hashString(this._handlePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._region) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SPACE,
      id: this.id,
      spaceId: this.id,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
  }

  get path(): string {
    return this.slug ?? this.name;
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`slug=${`"${this.slug}"`}`);
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Space "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Create a new Space with a root Branch, meta Snapshot and head Snapshot. */
  static createSpace(options: {
    session: Session;
    id?: string;
    name: string;
    slug: string;
    region: Region;
    ownedBy: Entity | NodeReference;
  }): {
    space: Space;
    rootBranch: Branch;
    metaBranch: Branch;
    metaSnapshot: Snapshot;
    headSnapshot: Snapshot;
  } {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Branch = NODE_CLASS_BY_TYPE[NodeType.BRANCH] as typeof Branch;
    const _Snapshot = NODE_CLASS_BY_TYPE[NodeType.SNAPSHOT] as typeof Snapshot;
    const _Space = NODE_CLASS_BY_TYPE[NodeType.SPACE] as typeof Space;

    const { session, id, name, slug, region, ownedBy } = options;

    let ownedByPtr: NodeReference;
    if (ownedBy instanceof Entity) {
      ownedByPtr = ownedBy.__toRef__();
    } else {
      ownedByPtr = ownedBy;
    }

    const spaceId = id ?? uuid4();
    const epoch = session.epoch;
    const now = Temporal.Now.zonedDateTimeISO("UTC");

    const spacePtr = new _NodeReference({
      type: NodeType.SPACE,
      id: spaceId,
      spaceId,
      branchId: Universe.ROOT_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    // snapshots/branches live in the meta Branch/Snapshot
    const metaBranchPtr = new _NodeReference({
      type: NodeType.BRANCH,
      id: Universe.META_BRANCH_ID,
      spaceId,
      branchId: Universe.META_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const metaSnapshotPtr = new _NodeReference({
      type: NodeType.SNAPSHOT,
      id: Universe.META_SNAPSHOT_ID,
      spaceId,
      branchId: Universe.META_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const rootBranchPtr = new _NodeReference({
      type: NodeType.BRANCH,
      id: Universe.ROOT_BRANCH_ID,
      spaceId,
      branchId: Universe.ROOT_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const headSnapshotPtr = new _NodeReference({
      type: NodeType.SNAPSHOT,
      id: Universe.HEAD_SNAPSHOT_ID,
      spaceId,
      branchId: Universe.META_BRANCH_ID,
      snapshotId: Universe.META_BRANCH_ID,
    });

    const metaBranch = new _Branch({
      id: Universe.META_BRANCH_ID,
      type: BranchType.ROOT,
      name: "Meta",
      space: spacePtr,
      snapshot: metaSnapshotPtr,
      branch: metaBranchPtr,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByPtr,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByPtr,
    });

    const metaSnapshot = new _Snapshot({
      id: Universe.META_SNAPSHOT_ID,
      name: "Meta",
      space: spacePtr,
      branch: metaBranchPtr,
      snapshot: metaSnapshotPtr,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByPtr,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByPtr,
      type: SnapshotType.FULL,
    });

    const headSnapshot = new _Snapshot({
      id: Universe.HEAD_SNAPSHOT_ID,
      name: "Head",
      space: spacePtr,
      branch: metaBranchPtr,
      snapshot: metaSnapshotPtr,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByPtr,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByPtr,
      type: SnapshotType.FULL,
    });

    const rootBranch = new _Branch({
      id: Universe.ROOT_BRANCH_ID,
      name: "Root",
      space: spacePtr,
      snapshot: metaSnapshotPtr,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByPtr,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByPtr,
      type: BranchType.ROOT,
      branch: metaBranchPtr,
    });

    // space lives in the root Branch at head Snapshot
    const space = new _Space({
      id: spaceId,
      space: spacePtr,
      branch: rootBranchPtr,
      snapshot: headSnapshotPtr,
      name,
      slug,
      region,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByPtr,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByPtr,
      ownedBy: ownedByPtr,
    });

    session.create(space);
    session.create(metaBranch);
    session.create(metaSnapshot);
    session.create(rootBranch);
    session.create(headSnapshot);

    return { space, rootBranch, metaBranch, metaSnapshot, headSnapshot };
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SPACE, Space);
/* ==== DESTACK_GENERATED_END:NODE:1100 ==== */
