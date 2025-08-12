import type {
  Branch,
  DateTime,
  NodeClass,
  NodeReference,
  PackedObjectCache,
  Session,
  Snapshot,
  Space,
  UInt128,
  UUID,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  ImmutableStruct,
  StructType,
} from "@destack/language/core";
import type { Anchor, Offset2 } from "@destack/language/geometry/relative";
import { Shape2D } from "@destack/language/geometry/shape";
import type { Vector2 } from "@destack/language/geometry/vector";
import type { Script } from "@destack/language/logic";
import {
  registerNodeClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import type { Stroke } from "@destack/language/style";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:2411100 ==== */
/**
 * A Path is a path of multiple points.
 */
export class Path2D extends ImmutableStruct {
  static metatype: StructType = StructType.PATH2D;
  static __isFrozen__: boolean = true;

  /**
   * Path2D.points
   */
  readonly points: readonly Vector2[];

  constructor(options: {
    points?: readonly Vector2[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _points = options.points ?? null;
    if (_points == null) {
      _points = [];
    }
    this.points = _points;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.points.length != other.points.length) {
      return false;
    }
    for (let i = 0; i < this.points.length; i++) {
      if (!this.points[i].equals(other.points[i])) {
        return false;
      }
    }

    return true;
  }

  repr(): string {
    return `<Path2D>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.points && this.points.length > 0) {
      for (const _item of this.points) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PATH2D, Path2D);
/* ==== DESTACK_GENERATED_END:STRUCT:2411100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2411100 ==== */
/**
 * A PathShape is a shape that represents a path of multiple points.
 */
export class PathShape2D extends Shape2D {
  static metatype: NodeType = NodeType.PATH_SHAPE2D;

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
  get precededBy(): PathShape2D | null {
    const nodeRef: NodeReference | null = this.precededByRef;
    if (nodeRef != null) {
      return this._session.graph.get(nodeRef) as PathShape2D | null;
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
   * Entity2D.position
   */
  /**
   * Entity2D.position
   */
  get position(): Vector2 | null {
    return this._position;
  }
  set position(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: Vector2 | null;

  /**
   * Entity2D.offset
   */
  /**
   * Entity2D.offset
   */
  get offset(): Offset2 | null {
    return this._offset;
  }
  set offset(value: Offset2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["offset"];
    this._session.updateSetProperty(this, prop, value);
    this._offset = value;
  }
  _offset: Offset2 | null;

  /**
   * Entity2D.scale
   */
  /**
   * Entity2D.scale
   */
  get scale(): Vector2 | null {
    return this._scale;
  }
  set scale(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["scale"];
    this._session.updateSetProperty(this, prop, value);
    this._scale = value;
  }
  _scale: Vector2 | null;

  /**
   * Entity2D.rotation
   */
  /**
   * Entity2D.rotation
   */
  get rotation(): Vector2 | null {
    return this._rotation;
  }
  set rotation(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["rotation"];
    this._session.updateSetProperty(this, prop, value);
    this._rotation = value;
  }
  _rotation: Vector2 | null;

  /**
   * Entity2D.skew
   */
  /**
   * Entity2D.skew
   */
  get skew(): Vector2 | null {
    return this._skew;
  }
  set skew(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["skew"];
    this._session.updateSetProperty(this, prop, value);
    this._skew = value;
  }
  _skew: Vector2 | null;

  /**
   * Entity2D.origin
   */
  /**
   * Entity2D.origin
   */
  get origin(): Vector2 | null {
    return this._origin;
  }
  set origin(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["origin"];
    this._session.updateSetProperty(this, prop, value);
    this._origin = value;
  }
  _origin: Vector2 | null;

  /**
   * Entity2D.anchor
   */
  /**
   * Entity2D.anchor
   */
  get anchor(): Anchor | null {
    return this._anchor;
  }
  set anchor(value: Anchor | null) {
    const prop = (this.constructor as NodeClass).__properties__["anchor"];
    this._session.updateSetProperty(this, prop, value);
    this._anchor = value;
  }
  _anchor: Anchor | null;

  /**
   * Shape2D.stroke
   */
  /**
   * Shape2D.stroke
   */
  get stroke(): Stroke | null {
    return this._stroke;
  }
  set stroke(value: Stroke | null) {
    const prop = (this.constructor as NodeClass).__properties__["stroke"];
    this._session.updateSetProperty(this, prop, value);
    this._stroke = value;
  }
  _stroke: Stroke | null;

  /**
   * PathShape2D.points
   */
  /**
   * PathShape2D.points
   */
  get points(): readonly Vector2[] {
    return this._points;
  }
  set points(value: readonly Vector2[]) {
    const prop = (this.constructor as NodeClass).__properties__["points"];
    this._session.updateSetProperty(this, prop, value);
    this._points = value;
  }
  _points: readonly Vector2[];

  constructor(options: {
    id?: UUID;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: PathShape2D | NodeReference | null;
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
    position?: Vector2 | null;
    offset?: Offset2 | null;
    scale?: Vector2 | null;
    rotation?: Vector2 | null;
    skew?: Vector2 | null;
    origin?: Vector2 | null;
    anchor?: Anchor | null;
    stroke?: Stroke | null;
    points?: readonly Vector2[];
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
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for PathShape2D`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`PathShape2D.space is required`);
    }
    this.spaceRef = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`PathShape2D.materialization is required`);
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
        throw new Error(`no active Branch for PathShape2D`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`PathShape2D.branch is required`);
    }
    this.branchRef = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for PathShape2D`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`PathShape2D.snapshot is required`);
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
      _name = "PathShape2D";
    }
    if (_name == null) {
      throw new Error(`PathShape2D.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`PathShape2D.orderKey is required`);
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
    let _position = options.position ?? null;
    this._position = _position;
    let _offset = options.offset ?? null;
    this._offset = _offset;
    let _scale = options.scale ?? null;
    this._scale = _scale;
    let _rotation = options.rotation ?? null;
    this._rotation = _rotation;
    let _skew = options.skew ?? null;
    this._skew = _skew;
    let _origin = options.origin ?? null;
    this._origin = _origin;
    let _anchor = options.anchor ?? null;
    this._anchor = _anchor;
    let _stroke = options.stroke ?? null;
    this._stroke = _stroke;
    let _points = options.points ?? null;
    if (_points == null) {
      _points = [];
    }
    this._points = _points;

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
        throw new Error(
          `PathShape2D.createdAt and PathShape2D.updatedAt are required for existing Nodes`,
        );
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
    if (this._points.length != other._points.length) {
      return false;
    }
    for (let i = 0; i < this._points.length; i++) {
      if (!this._points[i].equals(other._points[i])) {
        return false;
      }
    }

    if (
      (this._stroke == null) !== (other._stroke == null) ||
      (this._stroke != null && !this._stroke.equals(other._stroke))
    ) {
      return false;
    }
    if (
      (this._position == null) !== (other._position == null) ||
      (this._position != null && !this._position.equals(other._position))
    ) {
      return false;
    }
    if (
      (this._offset == null) !== (other._offset == null) ||
      (this._offset != null && !this._offset.equals(other._offset))
    ) {
      return false;
    }
    if (
      (this._scale == null) !== (other._scale == null) ||
      (this._scale != null && !this._scale.equals(other._scale))
    ) {
      return false;
    }
    if (
      (this._rotation == null) !== (other._rotation == null) ||
      (this._rotation != null && !this._rotation.equals(other._rotation))
    ) {
      return false;
    }
    if (
      (this._skew == null) !== (other._skew == null) ||
      (this._skew != null && !this._skew.equals(other._skew))
    ) {
      return false;
    }
    if (
      (this._origin == null) !== (other._origin == null) ||
      (this._origin != null && !this._origin.equals(other._origin))
    ) {
      return false;
    }
    if (!(this._anchor === other._anchor)) {
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
    if (!(this.spaceRef.id === other.spaceRef.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this._points && this._points.length > 0) {
      for (const _item of this._points) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this._stroke != null) {
      h = (h * 31 + this._stroke.hash()) & 0xffffffff;
    }
    if (this._position != null) {
      h = (h * 31 + this._position.hash()) & 0xffffffff;
    }
    if (this._offset != null) {
      h = (h * 31 + this._offset.hash()) & 0xffffffff;
    }
    if (this._scale != null) {
      h = (h * 31 + this._scale.hash()) & 0xffffffff;
    }
    if (this._rotation != null) {
      h = (h * 31 + this._rotation.hash()) & 0xffffffff;
    }
    if (this._skew != null) {
      h = (h * 31 + this._skew.hash()) & 0xffffffff;
    }
    if (this._origin != null) {
      h = (h * 31 + this._origin.hash()) & 0xffffffff;
    }
    if (this._anchor != null) {
      h = (h * 31 + this._anchor) & 0xffffffff;
    }
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
    h = (h * 31 + hashString(this.spaceRef.id)) & 0xffffffff;

    return h;
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.PATH_SHAPE2D,
      id: this.id,
      spaceId: this.spaceRef.id,
      definitionId: this.definitionRef?.id ?? null,
      branchId: this.branchRef.id,
      snapshotId: this.snapshotRef.id,
      _session: this._session,
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
    if (this.stroke != null) {
      propertyReprs.push(`stroke=${this.stroke.repr()}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<PathShape2D "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  /* ... */
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.PATH_SHAPE2D, PathShape2D);
/* ==== DESTACK_GENERATED_END:NODE:2411100 ==== */
