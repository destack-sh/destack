import { NodeType, StructType } from "@destack/language/core/builtin/builtin";
import type { Region } from "@destack/language/core/builtin/common";
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT } from "@destack/language/core/builtin/const";
import type {
  EnumDefinition,
  NodeDefinition,
  StructDefinition,
  TraitDefinition,
} from "@destack/language/core/builtin/definition";
import { Entity, type Materialization } from "@destack/language/core/builtin/entity";
import type { Node, NodeClass } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsFollowable,
  IsJoinable,
  IsOwnable,
  IsStarable,
} from "@destack/language/core/builtin/trait";
import type { DateTime, Float64, UInt128, UUID } from "@destack/language/core/builtin/types";
import type { Value } from "@destack/language/core/builtin/value";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import { BranchType, SnapshotType } from "@destack/language/core/common/time";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  NODE_CLASS_BY_TYPE,
  registerNodeClass,
  STRUCT_CLASS_BY_TYPE,
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
  static readonly VERSION: string = "2025.07.26.0";

  /**
   * The float epsilon used for floating point comparisons.
   */
  static readonly EPSILON: Float64 = 1e-6;

  /**
   * The beginning of time. (1970-01-01T00:00:00+00:00)
   */
  static readonly BEGINNING_OF_TIME: DateTime = Temporal.Instant.from(
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
  static readonly SPACE_ID: UUID = "00000000-0000-0000-0000-000000000000";

  /**
   * The system Space.
   */
  static readonly SPACE: NodeReference = undefined as any /* (deferred) */;

  /**
   * The 'meta' Snapshot.id, the Snapshot containing time-related Entities (like Snapshots, Branches, etc.)
   */
  static readonly META_SNAPSHOT_ID: UUID = "00000000-0000-0000-0000-000000000014";

  /**
   * The 'meta' Branch.id, the Branch containing time-related Entities (like Snapshots, Branches, etc.)
   */
  static readonly META_BRANCH_ID: UUID = "00000000-0000-0000-0000-000000000015";

  /**
   * The 'root' Branch.id, the Branch all other Branches originate from.
   */
  static readonly ROOT_BRANCH_ID: UUID = "00000000-0000-0000-0000-00000000001f";

  /**
   * The 'head' Snapshot.id, the current active Snapshot.
   */
  static readonly HEAD_SNAPSHOT_ID: UUID = "00000000-0000-0000-0000-00000000001e";

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
  declare readonly parentRef: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spaceRef: NodeReference;

  /**
   * Entity.materialization
   */
  declare readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  abstract get definition(): Entity | null;
  declare readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  abstract get branch(): Branch | null;
  declare readonly branchRef: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotRef: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  abstract get precededBy(): Universe | null;
  declare readonly precededByRef: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  abstract get instance(): Entity | null;
  declare readonly instanceRef: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  declare readonly createdAt: DateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  declare readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  abstract get createdBy(): Entity | null;
  declare readonly createdByRef: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  declare readonly updatedAt: DateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  declare readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  abstract get updatedBy(): Entity | null;
  declare readonly updatedByRef: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  declare readonly deletedAt: DateTime | null;

  /**
   * Entity.ownedBy
   */
  abstract get ownedBy(): Entity | null;
  abstract set ownedBy(value: Entity | null);
  /**
   * Entity.ownedBy
   */
  abstract get ownedByRef(): NodeReference | null;
  abstract set ownedByRef(value: NodeReference | null);

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
  abstract get customValues(): { readonly [key: UUID]: Value };
  abstract set customValues(value: { readonly [key: UUID]: Value });

  /**
   * The Script of this Entity.
   */
  abstract get script(): Script | null;
  abstract set script(value: Script | null);
  /**
   * The Script of this Entity.
   */
  abstract get scriptRef(): NodeReference | null;
  abstract set scriptRef(value: NodeReference | null);

  /**
   * Whether this Entity can be instanced.
   */
  declare readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  abstract get source(): Script | null;
  declare readonly sourceRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.parentRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly parentRef: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodeRef: NodeReference | null = this.spaceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly spaceRef: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodeRef: NodeReference | null = this.definitionRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly definitionRef: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodeRef: NodeReference | null = this.branchRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Branch | null;
    }
    return null;
  }
  readonly branchRef: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodeRef: NodeReference | null = this.snapshotRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotRef: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Space | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Space | null;
    }
    return null;
  }
  readonly precededByRef: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodeRef: NodeReference | null = this.instanceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly instanceRef: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: DateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): Entity | null {
    const nodeRef: NodeReference | null = this.createdByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly createdByRef: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: DateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): Entity | null {
    const nodeRef: NodeReference | null = this.updatedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  readonly updatedByRef: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: DateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodeRef: NodeReference | null = this.ownedByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
    if (node === null) {
      this.ownedByRef = null;
    } else {
      this.ownedByRef = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
  get ownedByRef(): NodeReference | null {
    return this._ownedByRef;
  }
  set ownedByRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByRef = value;
  }
  _ownedByRef: NodeReference | null;

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
  get customValues(): { readonly [key: UUID]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: UUID]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: UUID]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodeRef: NodeReference | null = this.scriptRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptRef = null;
    } else {
      this.scriptRef = node.toRef();
    }
  }
  /**
   * The Script of this Entity.
   */
  get scriptRef(): NodeReference | null {
    return this._scriptRef;
  }
  set scriptRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptRef = value;
  }
  _scriptRef: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodeRef: NodeReference | null = this.sourceRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Script | null;
    }
    return null;
  }
  readonly sourceRef: NodeReference | null;

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
    const nodeRef: NodeReference | null = this.handleRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as Handle | null;
    }
    return null;
  }
  set handle(node: Handle | null) {
    if (node === null) {
      this.handleRef = null;
    } else {
      this.handleRef = node.toRef();
    }
  }
  /**
   * Space.handle
   */
  get handleRef(): NodeReference | null {
    return this._handleRef;
  }
  set handleRef(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["handle"];
    this._session.updateSetProperty(this, prop, value);
    this._handleRef = value;
  }
  _handleRef: NodeReference | null;

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
    id?: UUID;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Space | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: DateTime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: DateTime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: DateTime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: UUID]: Value };
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
      options.parent != null ? options.parent.toRef() : null,
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
    this.parentRef = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = this.toRef();
    }
    if (_space == null) {
      throw new Error(`Space.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`Space.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionRef = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for Space`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`Space.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for Space`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`Space.snapshot is required`);
    }
    this.snapshotRef = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByRef = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instanceRef = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByRef = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name == null) {
      _name = "Space";
    }
    if (_name == null) {
      throw new Error(`Space.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`Space.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues == null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptRef = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourceRef = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _slug = options.slug;
    if (_slug == null) {
      throw new Error(`Space.slug is required`);
    }
    this._slug = _slug;
    let _handle = options.handle ?? null;
    if (_handle != null && _handle.constructor.name !== "NodeReference") {
      _handle = (_handle as Node).toRef();
    }
    this._handleRef = _handle as NodeReference | null;
    let _region = options.region;
    if (_region == null) {
      throw new Error(`Space.region is required`);
    }
    this._region = _region;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByRef = this._session.actorRef;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByRef = this._session.actorRef;
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
      this.createdByRef =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorRef;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByRef =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorRef;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    if (!(this._slug === other._slug)) {
      return false;
    }
    if (!(this._handleRef?.id === other._handleRef?.id)) {
      return false;
    }
    if (!(this._region === other._region)) {
      return false;
    }
    if (!(this.definitionRef?.id === other.definitionRef?.id)) {
      return false;
    }
    if (!(this._ownedByRef?.id === other._ownedByRef?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (JSON.stringify(this._customValues) !== JSON.stringify(other._customValues)) {
      return false;
    }
    if (!(this._scriptRef?.id === other._scriptRef?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourceRef?.id === other.sourceRef?.id)) {
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
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    if (this._handleRef != null) {
      h = (h * 31 + hashString(this._handleRef.id)) & 0xffffffff;
    }
    h = (h * 31 + this._region) & 0xffffffff;
    if (this.parentRef != null) {
      h = (h * 31 + hashString(this.parentRef.id)) & 0xffffffff;
    }
    if (this.definitionRef != null) {
      h = (h * 31 + hashString(this.definitionRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByRef.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByRef.id)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByRef != null) {
      h = (h * 31 + hashString(this._ownedByRef.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptRef != null) {
      h = (h * 31 + hashString(this._scriptRef.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourceRef != null) {
      h = (h * 31 + hashString(this.sourceRef.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.SPACE,
      id: this.id,
      spaceId: this.id,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
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

    let ownedByRef: NodeReference;
    if (ownedBy instanceof Entity) {
      ownedByRef = ownedBy.__toRef__();
    } else {
      ownedByRef = ownedBy;
    }

    const spaceId = id ?? uuid4();
    const epoch = session.remoteEpoch;
    const now = Temporal.Now.zonedDateTimeISO("UTC");

    const spaceRef = new _NodeReference({
      type: NodeType.SPACE,
      id: spaceId,
      spaceId,
      branchId: Universe.ROOT_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    // snapshots/branches live in the meta Branch/Snapshot
    const metaBranchRef = new _NodeReference({
      type: NodeType.BRANCH,
      id: Universe.META_BRANCH_ID,
      spaceId,
      branchId: Universe.META_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const metaSnapshotRef = new _NodeReference({
      type: NodeType.SNAPSHOT,
      id: Universe.META_SNAPSHOT_ID,
      spaceId,
      branchId: Universe.META_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const rootBranchRef = new _NodeReference({
      type: NodeType.BRANCH,
      id: Universe.ROOT_BRANCH_ID,
      spaceId,
      branchId: Universe.ROOT_BRANCH_ID,
      snapshotId: Universe.META_SNAPSHOT_ID,
    });

    const headSnapshotRef = new _NodeReference({
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
      space: spaceRef,
      snapshot: metaSnapshotRef,
      branch: metaBranchRef,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByRef,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByRef,
    });

    const metaSnapshot = new _Snapshot({
      id: Universe.META_SNAPSHOT_ID,
      name: "Meta",
      space: spaceRef,
      branch: metaBranchRef,
      snapshot: metaSnapshotRef,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByRef,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByRef,
      type: SnapshotType.FULL,
    });

    const headSnapshot = new _Snapshot({
      id: Universe.HEAD_SNAPSHOT_ID,
      name: "Head",
      space: spaceRef,
      branch: metaBranchRef,
      snapshot: metaSnapshotRef,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByRef,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByRef,
      type: SnapshotType.FULL,
    });

    const rootBranch = new _Branch({
      id: Universe.ROOT_BRANCH_ID,
      name: "Root",
      space: spaceRef,
      snapshot: metaSnapshotRef,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByRef,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByRef,
      type: BranchType.ROOT,
      branch: metaBranchRef,
    });

    // space lives in the root Branch at head Snapshot
    const space = new _Space({
      id: spaceId,
      space: spaceRef,
      branch: rootBranchRef,
      snapshot: headSnapshotRef,
      name,
      slug,
      region,
      createdEpoch: epoch,
      createdAt: now,
      createdBy: ownedByRef,
      updatedEpoch: epoch,
      updatedAt: now,
      updatedBy: ownedByRef,
      ownedBy: ownedByRef,
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
