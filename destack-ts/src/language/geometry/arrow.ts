import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import type { Offset2 } from "@destack/language/geometry/relative";
import { Anchor } from "@destack/language/geometry/relative";
import { Shape2D } from "@destack/language/geometry/shape";
import type { Vector2 } from "@destack/language/geometry/vector";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Stroke } from "@destack/language/style";
import {
  AnchorProto,
  Arrow2DProto,
  ArrowHeadTypeProto,
  ArrowShape2DProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2401200 ==== */
/**
 * ArrowHeadType
 */
export enum ArrowHeadType {
  ARROW = 1,
  TRIANGLE = 2,
  DOT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ARROW_HEAD_TYPE, ArrowHeadType);
/* ==== DESTACK_GENERATED_END:ENUM:2401200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2411200 ==== */
/**
 * An Arrow is a shape that represents an arrow.
 */
export class Arrow2D extends StructFrozen {
  static metatype: StructType = StructType.ARROW2D;
  static __isFrozen__: boolean = true;

  /**
   * Arrow2D.startType
   */
  readonly startType: ArrowHeadType;

  /**
   * Arrow2D.start
   */
  readonly start: Vector2;

  /**
   * Arrow2D.endType
   */
  readonly endType: ArrowHeadType;

  /**
   * Arrow2D.end
   */
  readonly end: Vector2;

  constructor(options: {
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`Arrow2D.startType is required`);
    }
    this.startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`Arrow2D.start is required`);
    }
    this.start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`Arrow2D.endType is required`);
    }
    this.endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`Arrow2D.end is required`);
    }
    this.end = _end;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.startType === other.startType)) {
      return false;
    }
    if (!this.start.equals(other.start)) {
      return false;
    }
    if (!(this.endType === other.endType)) {
      return false;
    }
    if (!this.end.equals(other.end)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Arrow2D>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.startType) & 0xffffffff;
    h = (h * 31 + this.start.hash()) & 0xffffffff;
    h = (h * 31 + this.endType) & 0xffffffff;
    h = (h * 31 + this.end.hash()) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Arrow2D.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Arrow2D): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2411200;
    objectCson["200"] = object.startType;
    objectCson["201"] = object.start.toCson();
    objectCson["210"] = object.endType;
    objectCson["211"] = object.end.toCson();
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow2D {
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new Arrow2D({
      startType: Number(objectCson["200"]),
      start: _Vector2.fromCson(objectCson["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectCson["210"]),
      end: _Vector2.fromCson(objectCson["211"], _session, _supergraph, _graph, _connection),
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow2D {
    return Arrow2D.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): Arrow2DProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Arrow2D.__packProto__(this);
    }
    return this._proto as Arrow2DProto;
  }

  static __packProto__(object: Arrow2D): Arrow2DProto {
    const objectProto: Partial<Arrow2DProto> = { metatype: 2411200 };
    objectProto.startType = Number(object.startType) as ArrowHeadTypeProto;
    objectProto.start = object.start.toProto();
    objectProto.endType = Number(object.endType) as ArrowHeadTypeProto;
    objectProto.end = object.end.toProto();
    return objectProto as Arrow2DProto;
  }

  static __unpackProto__(
    objectProto: Arrow2DProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow2D {
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new Arrow2D({
      startType: Number(objectProto.startType) as ArrowHeadType,
      start: _Vector2.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: Arrow2DProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow2D {
    return Arrow2D.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Arrow2D {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = Arrow2DProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ARROW2D, Arrow2D);
/* ==== DESTACK_GENERATED_END:STRUCT:2411200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2410200 ==== */
/**
 * An ArrowShape is a shape that represents an arrow.
 */
export class ArrowShape2D extends Shape2D {
  static metatype: NodeType = NodeType.ARROW_SHAPE2D;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
      return this._supergraph.get(nodePtr.id) as Branch | null;
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
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): ArrowShape2D | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as ArrowShape2D | null;
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
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
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
      return this._supergraph.get(nodePtr.id) as Script | null;
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
   * ArrowShape2D.startType
   */
  /**
   * ArrowShape2D.startType
   */
  get startType(): ArrowHeadType {
    return this._startType;
  }
  set startType(value: ArrowHeadType) {
    const prop = (this.constructor as NodeClass).__properties__["start_type"];
    this._session.updateSetProperty(this, prop, value);
    this._startType = value;
  }
  _startType: ArrowHeadType;

  /**
   * ArrowShape2D.start
   */
  /**
   * ArrowShape2D.start
   */
  get start(): Vector2 {
    return this._start;
  }
  set start(value: Vector2) {
    const prop = (this.constructor as NodeClass).__properties__["start"];
    this._session.updateSetProperty(this, prop, value);
    this._start = value;
  }
  _start: Vector2;

  /**
   * ArrowShape2D.endType
   */
  /**
   * ArrowShape2D.endType
   */
  get endType(): ArrowHeadType {
    return this._endType;
  }
  set endType(value: ArrowHeadType) {
    const prop = (this.constructor as NodeClass).__properties__["end_type"];
    this._session.updateSetProperty(this, prop, value);
    this._endType = value;
  }
  _endType: ArrowHeadType;

  /**
   * ArrowShape2D.end
   */
  /**
   * ArrowShape2D.end
   */
  get end(): Vector2 {
    return this._end;
  }
  set end(value: Vector2) {
    const prop = (this.constructor as NodeClass).__properties__["end"];
    this._session.updateSetProperty(this, prop, value);
    this._end = value;
  }
  _end: Vector2;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: ArrowShape2D | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
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
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
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
        ? options.parent.constructor.name == "NodeReference"
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
      // _isNew
      options.id == null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name != "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for ArrowShape2D`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ArrowShape2D.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`ArrowShape2D.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for ArrowShape2D`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`ArrowShape2D.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for ArrowShape2D`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`ArrowShape2D.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name != "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name != "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "ArrowShape2D";
    }
    if (_name === null) {
      throw new Error(`ArrowShape2D.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ArrowShape2D.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name != "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name != "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
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
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`ArrowShape2D.startType is required`);
    }
    this._startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`ArrowShape2D.start is required`);
    }
    this._start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`ArrowShape2D.endType is required`);
    }
    this._endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`ArrowShape2D.end is required`);
    }
    this._end = _end;

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
          `ArrowShape2D.createdAt and ArrowShape2D.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name == "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._startType === other._startType)) {
      return false;
    }
    if (!this._start.equals(other._start)) {
      return false;
    }
    if (!(this._endType === other._endType)) {
      return false;
    }
    if (!this._end.equals(other._end)) {
      return false;
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
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._startType) & 0xffffffff;
    h = (h * 31 + this._start.hash()) & 0xffffffff;
    h = (h * 31 + this._endType) & 0xffffffff;
    h = (h * 31 + this._end.hash()) & 0xffffffff;
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
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.ARROW_SHAPE2D,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
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
    if (this.stroke != null) {
      propertyReprs.push(`stroke=${this.stroke.repr()}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<ArrowShape2D "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return ArrowShape2D.__packCson__(this);
  }

  static __packCson__(object: ArrowShape2D): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2410200;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    if (object._position != null) {
      objectCson["110"] = object._position.toCson();
    }
    if (object._offset != null) {
      objectCson["111"] = object._offset.toCson();
    }
    if (object._scale != null) {
      objectCson["112"] = object._scale.toCson();
    }
    if (object._rotation != null) {
      objectCson["113"] = object._rotation.toCson();
    }
    if (object._skew != null) {
      objectCson["114"] = object._skew.toCson();
    }
    if (object._origin != null) {
      objectCson["115"] = object._origin.toCson();
    }
    if (object._anchor != null) {
      objectCson["116"] = object._anchor;
    }
    if (object._stroke != null) {
      objectCson["180"] = object._stroke.toCson();
    }
    objectCson["200"] = object._startType;
    objectCson["201"] = object._start.toCson();
    objectCson["210"] = object._endType;
    objectCson["211"] = object._end.toCson();
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape2D {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Stroke = STRUCT_CLASS_BY_TYPE[StructType.STROKE] as typeof Stroke;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Offset2 = STRUCT_CLASS_BY_TYPE[StructType.OFFSET2] as typeof Offset2;
    const strokeValue = objectCson["180"];
    const unpackedStroke =
      strokeValue != undefined
        ? _Stroke.fromCson(strokeValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectCson["110"];
    const unpackedPosition =
      positionValue != undefined
        ? _Vector2.fromCson(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const offsetValue = objectCson["111"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Offset2.fromCson(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectCson["112"];
    const unpackedScale =
      scaleValue != undefined
        ? _Vector2.fromCson(scaleValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectCson["113"];
    const unpackedRotation =
      rotationValue != undefined
        ? _Vector2.fromCson(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectCson["114"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2.fromCson(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const originValue = objectCson["115"];
    const unpackedOrigin =
      originValue != undefined
        ? _Vector2.fromCson(originValue, _session, _supergraph, _graph, _connection)
        : null;
    const anchorValue = objectCson["116"];
    const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : null;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new ArrowShape2D({
      startType: Number(objectCson["200"]),
      start: _Vector2.fromCson(objectCson["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectCson["210"]),
      end: _Vector2.fromCson(objectCson["211"], _session, _supergraph, _graph, _connection),
      stroke: unpackedStroke,
      position: unpackedPosition,
      offset: unpackedOffset,
      scale: unpackedScale,
      rotation: unpackedRotation,
      skew: unpackedSkew,
      origin: unpackedOrigin,
      anchor: unpackedAnchor,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape2D {
    return ArrowShape2D.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): ArrowShape2DProto {
    return ArrowShape2D.__packProto__(this);
  }

  static __packProto__(object: ArrowShape2D): ArrowShape2DProto {
    const objectProto: Partial<ArrowShape2DProto> = { metatype: 2410200 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
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
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    if (object._position != null) {
      objectProto.position = object._position.toProto();
    }
    if (object._offset != null) {
      objectProto.offset = object._offset.toProto();
    }
    if (object._scale != null) {
      objectProto.scale = object._scale.toProto();
    }
    if (object._rotation != null) {
      objectProto.rotation = object._rotation.toProto();
    }
    if (object._skew != null) {
      objectProto.skew = object._skew.toProto();
    }
    if (object._origin != null) {
      objectProto.origin = object._origin.toProto();
    }
    if (object._anchor != null) {
      objectProto.anchor = Number(object._anchor) as AnchorProto;
    }
    if (object._stroke != null) {
      objectProto.stroke = object._stroke.toProto();
    }
    objectProto.startType = Number(object._startType) as ArrowHeadTypeProto;
    objectProto.start = object._start.toProto();
    objectProto.endType = Number(object._endType) as ArrowHeadTypeProto;
    objectProto.end = object._end.toProto();
    return objectProto as ArrowShape2DProto;
  }

  static __unpackProto__(
    objectProto: ArrowShape2DProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape2D {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Stroke = STRUCT_CLASS_BY_TYPE[StructType.STROKE] as typeof Stroke;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Offset2 = STRUCT_CLASS_BY_TYPE[StructType.OFFSET2] as typeof Offset2;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ArrowShape2D({
      startType: Number(objectProto.startType) as ArrowHeadType,
      start: _Vector2.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
      stroke:
        objectProto.stroke != undefined
          ? _Stroke.fromProto(objectProto.stroke!, _session, _supergraph, _graph, _connection)
          : null,
      position:
        objectProto.position != undefined
          ? _Vector2.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      offset:
        objectProto.offset != undefined
          ? _Offset2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale:
        objectProto.scale != undefined
          ? _Vector2.fromProto(objectProto.scale!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? _Vector2.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      origin:
        objectProto.origin != undefined
          ? _Vector2.fromProto(objectProto.origin!, _session, _supergraph, _graph, _connection)
          : null,
      anchor: objectProto.anchor != undefined ? (Number(objectProto.anchor) as Anchor) : null,
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
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
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
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
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
    objectProto: ArrowShape2DProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape2D {
    return ArrowShape2D.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ArrowShape2D {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ArrowShape2DProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ARROW_SHAPE2D, ArrowShape2D);
/* ==== DESTACK_GENERATED_END:NODE:2410200 ==== */
