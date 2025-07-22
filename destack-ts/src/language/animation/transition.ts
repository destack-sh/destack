import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  GraphConnection,
  IsActor,
  NodeClass,
  NodeReference,
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
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Style } from "@destack/language/style";
import {
  MaterializationProto,
  SpringTypeProto,
  TransitionProto,
  TransitionStyleProto,
  TransitionTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2200000 ==== */
/**
 * TransitionType
 */
export enum TransitionType {
  TWEEN = 10,
  SPRING = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TRANSITION_TYPE, TransitionType);
/* ==== DESTACK_GENERATED_END:ENUM:2200000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2200001 ==== */
/**
 * SpringType
 */
export enum SpringType {
  TIME = 1,
  PHYSICAL = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SPRING_TYPE, SpringType);
/* ==== DESTACK_GENERATED_END:ENUM:2200001 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2200000 ==== */
/**
 * A transition value.
 */
export class Transition extends StructFrozen {
  static metatype: StructType = StructType.TRANSITION;
  static __isFrozen__: boolean = true;

  /**
   * Transition.type
   */
  readonly type: TransitionType;

  /**
   * Transition.style
   */
  get style(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
      if (this._graph === null) {
        return null;
      }
      return this._graph.get(nodePtr.id) as TransitionStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Transition.delay
   */
  readonly delay: number | null;

  /**
   * Transition.duration
   */
  readonly duration: number | null;

  /**
   * Transition.ease
   */
  readonly ease: readonly number[];

  /**
   * Transition.stiffness
   */
  readonly stiffness: number | null;

  /**
   * Transition.damping
   */
  readonly damping: number | null;

  /**
   * Transition.mass
   */
  readonly mass: number | null;

  /**
   * Transition.bounce
   */
  readonly bounce: number | null;

  /**
   * Transition.springType
   */
  readonly springType: SpringType | null;

  constructor(options: {
    type?: TransitionType;
    style?: TransitionStyle | NodeReference | null;
    delay?: number | null;
    duration?: number | null;
    ease?: readonly number[];
    stiffness?: number | null;
    damping?: number | null;
    mass?: number | null;
    bounce?: number | null;
    springType?: SpringType | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
    );

    // properties
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* TransitionType.TWEEN */;
    }
    if (_type === null) {
      throw new Error(`Transition.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name != "NodeReference") {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style as NodeReference | null;
    let _delay = options.delay ?? null;
    this.delay = _delay;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _ease = options.ease ?? null;
    if (_ease === null) {
      _ease = [];
    }
    this.ease = _ease;
    let _stiffness = options.stiffness ?? null;
    this.stiffness = _stiffness;
    let _damping = options.damping ?? null;
    this.damping = _damping;
    let _mass = options.mass ?? null;
    this.mass = _mass;
    let _bounce = options.bounce ?? null;
    this.bounce = _bounce;
    let _springType = options.springType ?? null;
    this.springType = _springType;

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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    if (
      (this.delay == null) !== (other.delay == null) ||
      (this.delay != null &&
        !(this.delay === other.delay || Math.abs(this.delay - other.delay) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null &&
        !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
    ) {
      return false;
    }
    if (this.ease.length != other.ease.length) {
      return false;
    }
    for (let i = 0; i < this.ease.length; i++) {
      if (!(this.ease[i] === other.ease[i] || Math.abs(this.ease[i] - other.ease[i]) < 1e-10)) {
        return false;
      }
    }
    if (
      (this.stiffness == null) !== (other.stiffness == null) ||
      (this.stiffness != null &&
        !(this.stiffness === other.stiffness || Math.abs(this.stiffness - other.stiffness) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.damping == null) !== (other.damping == null) ||
      (this.damping != null &&
        !(this.damping === other.damping || Math.abs(this.damping - other.damping) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.mass == null) !== (other.mass == null) ||
      (this.mass != null && !(this.mass === other.mass || Math.abs(this.mass - other.mass) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.bounce == null) !== (other.bounce == null) ||
      (this.bounce != null &&
        !(this.bounce === other.bounce || Math.abs(this.bounce - other.bounce) < 1e-10))
    ) {
      return false;
    }
    if (!(this.springType === other.springType)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${TransitionType[this.type]}`);
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.delay != null) {
        propertyReprs.push(`delay=${this.delay}`);
      }
      if (this.duration != null) {
        propertyReprs.push(`duration=${this.duration}`);
      }
      if (this.ease.length > 0) {
        propertyReprs.push(`ease=${this.ease.map((_item) => _item).join(", ")}`);
      }
      if (this.stiffness != null) {
        propertyReprs.push(`stiffness=${this.stiffness}`);
      }
      if (this.damping != null) {
        propertyReprs.push(`damping=${this.damping}`);
      }
      if (this.mass != null) {
        propertyReprs.push(`mass=${this.mass}`);
      }
      if (this.bounce != null) {
        propertyReprs.push(`bounce=${this.bounce}`);
      }
      if (this.springType != null) {
        propertyReprs.push(`springType=${SpringType[this.springType]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Transition ${propertyReprs.join(" ")}>`;
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
    if (this.stylePtr != null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.delay != null) {
      h = (h * 31 + hashFloat(this.delay)) & 0xffffffff;
    }
    if (this.duration != null) {
      h = (h * 31 + hashFloat(this.duration)) & 0xffffffff;
    }
    if (this.ease && this.ease.length > 0) {
      for (const _item of this.ease) {
        h = (h * 31 + hashFloat(_item)) & 0xffffffff;
      }
    }
    if (this.stiffness != null) {
      h = (h * 31 + hashFloat(this.stiffness)) & 0xffffffff;
    }
    if (this.damping != null) {
      h = (h * 31 + hashFloat(this.damping)) & 0xffffffff;
    }
    if (this.mass != null) {
      h = (h * 31 + hashFloat(this.mass)) & 0xffffffff;
    }
    if (this.bounce != null) {
      h = (h * 31 + hashFloat(this.bounce)) & 0xffffffff;
    }
    if (this.springType != null) {
      h = (h * 31 + this.springType) & 0xffffffff;
    }
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
      this._cson = Transition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Transition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2200000;
    objectCson["100"] = object.type;
    if (object.stylePtr != null) {
      objectCson["101"] = object.stylePtr.toCson();
    }
    if (object.delay != null) {
      objectCson["102"] = object.delay;
    }
    if (object.duration != null) {
      objectCson["103"] = object.duration;
    }
    if (object.ease.length > 0) {
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectCson["104"] = packedEase;
    }
    if (object.stiffness != null) {
      objectCson["105"] = object.stiffness;
    }
    if (object.damping != null) {
      objectCson["106"] = object.damping;
    }
    if (object.mass != null) {
      objectCson["107"] = object.mass;
    }
    if (object.bounce != null) {
      objectCson["108"] = object.bounce;
    }
    if (object.springType != null) {
      objectCson["109"] = object.springType;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Transition {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const stylePtrValue = objectCson["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromCson(stylePtrValue, _session, _graph, _connection)
        : null;
    const delayValue = objectCson["102"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectCson["103"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectCson["104"] != undefined) {
      for (const item of objectCson["104"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectCson["105"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectCson["106"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectCson["107"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectCson["108"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectCson["109"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
    return new Transition({
      type: Number(objectCson["100"]),
      style: unpackedStylePtr,
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Transition {
    return Transition.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): TransitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Transition.__packProto__(this);
    }
    return this._proto as TransitionProto;
  }

  static __packProto__(object: Transition): TransitionProto {
    const objectProto: Partial<TransitionProto> = { metatype: 2200000 };
    objectProto.type = Number(object.type) as TransitionTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.delay != null) {
      objectProto.delay = object.delay;
    }
    if (object.duration != null) {
      objectProto.duration = object.duration;
    }
    if (object.ease) {
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectProto.ease = packedEase;
    }
    if (object.stiffness != null) {
      objectProto.stiffness = object.stiffness;
    }
    if (object.damping != null) {
      objectProto.damping = object.damping;
    }
    if (object.mass != null) {
      objectProto.mass = object.mass;
    }
    if (object.bounce != null) {
      objectProto.bounce = object.bounce;
    }
    if (object.springType != null) {
      objectProto.springType = Number(object.springType) as SpringTypeProto;
    }
    return objectProto as TransitionProto;
  }

  static __unpackProto__(
    objectProto: TransitionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Transition {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedEase: any[] = [];
    if (objectProto.ease) {
      for (const item of objectProto.ease) {
        unpackedEase.push(item);
      }
    }
    return new Transition({
      type: Number(objectProto.type) as TransitionType,
      style:
        objectProto.stylePtr != undefined
          ? _NodeReference.fromProto(objectProto.stylePtr!, _session, _graph, _graph, _connection)
          : null,
      delay: objectProto.delay != undefined ? objectProto.delay : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      ease: unpackedEase,
      stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
      damping: objectProto.damping != undefined ? objectProto.damping : null,
      mass: objectProto.mass != undefined ? objectProto.mass : null,
      bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
      springType:
        objectProto.springType != undefined ? (Number(objectProto.springType) as SpringType) : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: TransitionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Transition {
    return Transition.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Transition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TransitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TRANSITION, Transition);
/* ==== DESTACK_GENERATED_END:STRUCT:2200000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2200000 ==== */
/**
 * A transition style.
 */
export class TransitionStyle extends Style {
  static metatype: NodeType = NodeType.TRANSITION_STYLE;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as Space | null;
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
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as Branch | null;
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
      return this._graph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as TransitionStyle | null;
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
      return this._graph.get(nodePtr.id) as Entity | null;
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
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
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
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
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
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
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
      return this._graph.get(nodePtr.id) as Script | null;
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
      return this._graph.get(nodePtr.id) as Script | null;
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
   * TransitionStyle.type
   */
  /**
   * TransitionStyle.type
   */
  get type(): TransitionType {
    return this._type;
  }
  set type(value: TransitionType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: TransitionType;

  /**
   * TransitionStyle.delay
   */
  /**
   * TransitionStyle.delay
   */
  get delay(): number | null {
    return this._delay;
  }
  set delay(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["delay"];
    this._session.updateSetProperty(this, prop, value);
    this._delay = value;
  }
  _delay: number | null;

  /**
   * TransitionStyle.duration
   */
  /**
   * TransitionStyle.duration
   */
  get duration(): number | null {
    return this._duration;
  }
  set duration(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["duration"];
    this._session.updateSetProperty(this, prop, value);
    this._duration = value;
  }
  _duration: number | null;

  /**
   * TransitionStyle.ease
   */
  /**
   * TransitionStyle.ease
   */
  get ease(): readonly number[] {
    return this._ease;
  }
  set ease(value: readonly number[]) {
    const prop = (this.constructor as NodeClass).__properties__["ease"];
    this._session.updateSetProperty(this, prop, value);
    this._ease = value;
  }
  _ease: readonly number[];

  /**
   * TransitionStyle.stiffness
   */
  /**
   * TransitionStyle.stiffness
   */
  get stiffness(): number | null {
    return this._stiffness;
  }
  set stiffness(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["stiffness"];
    this._session.updateSetProperty(this, prop, value);
    this._stiffness = value;
  }
  _stiffness: number | null;

  /**
   * TransitionStyle.damping
   */
  /**
   * TransitionStyle.damping
   */
  get damping(): number | null {
    return this._damping;
  }
  set damping(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["damping"];
    this._session.updateSetProperty(this, prop, value);
    this._damping = value;
  }
  _damping: number | null;

  /**
   * TransitionStyle.mass
   */
  /**
   * TransitionStyle.mass
   */
  get mass(): number | null {
    return this._mass;
  }
  set mass(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["mass"];
    this._session.updateSetProperty(this, prop, value);
    this._mass = value;
  }
  _mass: number | null;

  /**
   * TransitionStyle.bounce
   */
  /**
   * TransitionStyle.bounce
   */
  get bounce(): number | null {
    return this._bounce;
  }
  set bounce(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["bounce"];
    this._session.updateSetProperty(this, prop, value);
    this._bounce = value;
  }
  _bounce: number | null;

  /**
   * TransitionStyle.springType
   */
  /**
   * TransitionStyle.springType
   */
  get springType(): SpringType | null {
    return this._springType;
  }
  set springType(value: SpringType | null) {
    const prop = (this.constructor as NodeClass).__properties__["spring_type"];
    this._session.updateSetProperty(this, prop, value);
    this._springType = value;
  }
  _springType: SpringType | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: TransitionStyle | NodeReference | null;
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
    type?: TransitionType;
    delay?: number | null;
    duration?: number | null;
    ease?: readonly number[];
    stiffness?: number | null;
    damping?: number | null;
    mass?: number | null;
    bounce?: number | null;
    springType?: SpringType | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
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
      options._graph ?? null,
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
        throw new Error(`no active Space for TransitionStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`TransitionStyle.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`TransitionStyle.materialization is required`);
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
        throw new Error(`no active Branch for TransitionStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`TransitionStyle.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for TransitionStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`TransitionStyle.snapshot is required`);
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
      _name = "TransitionStyle";
    }
    if (_name === null) {
      throw new Error(`TransitionStyle.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`TransitionStyle.orderKey is required`);
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* TransitionType.TWEEN */;
    }
    if (_type === null) {
      throw new Error(`TransitionStyle.type is required`);
    }
    this._type = _type;
    let _delay = options.delay ?? null;
    this._delay = _delay;
    let _duration = options.duration ?? null;
    this._duration = _duration;
    let _ease = options.ease ?? null;
    if (_ease === null) {
      _ease = [];
    }
    this._ease = _ease;
    let _stiffness = options.stiffness ?? null;
    this._stiffness = _stiffness;
    let _damping = options.damping ?? null;
    this._damping = _damping;
    let _mass = options.mass ?? null;
    this._mass = _mass;
    let _bounce = options.bounce ?? null;
    this._bounce = _bounce;
    let _springType = options.springType ?? null;
    this._springType = _springType;

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
          `TransitionStyle.createdAt and TransitionStyle.updatedAt are required for existing Nodes`,
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (
      (this._delay == null) !== (other._delay == null) ||
      (this._delay != null &&
        !(this._delay === other._delay || Math.abs(this._delay - other._delay) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._duration == null) !== (other._duration == null) ||
      (this._duration != null &&
        !(this._duration === other._duration || Math.abs(this._duration - other._duration) < 1e-10))
    ) {
      return false;
    }
    if (this._ease.length != other._ease.length) {
      return false;
    }
    for (let i = 0; i < this._ease.length; i++) {
      if (!(this._ease[i] === other._ease[i] || Math.abs(this._ease[i] - other._ease[i]) < 1e-10)) {
        return false;
      }
    }
    if (
      (this._stiffness == null) !== (other._stiffness == null) ||
      (this._stiffness != null &&
        !(
          this._stiffness === other._stiffness ||
          Math.abs(this._stiffness - other._stiffness) < 1e-10
        ))
    ) {
      return false;
    }
    if (
      (this._damping == null) !== (other._damping == null) ||
      (this._damping != null &&
        !(this._damping === other._damping || Math.abs(this._damping - other._damping) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._mass == null) !== (other._mass == null) ||
      (this._mass != null &&
        !(this._mass === other._mass || Math.abs(this._mass - other._mass) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._bounce == null) !== (other._bounce == null) ||
      (this._bounce != null &&
        !(this._bounce === other._bounce || Math.abs(this._bounce - other._bounce) < 1e-10))
    ) {
      return false;
    }
    if (!(this._springType === other._springType)) {
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
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._delay != null) {
      h = (h * 31 + hashFloat(this._delay)) & 0xffffffff;
    }
    if (this._duration != null) {
      h = (h * 31 + hashFloat(this._duration)) & 0xffffffff;
    }
    if (this._ease && this._ease.length > 0) {
      for (const _item of this._ease) {
        h = (h * 31 + hashFloat(_item)) & 0xffffffff;
      }
    }
    if (this._stiffness != null) {
      h = (h * 31 + hashFloat(this._stiffness)) & 0xffffffff;
    }
    if (this._damping != null) {
      h = (h * 31 + hashFloat(this._damping)) & 0xffffffff;
    }
    if (this._mass != null) {
      h = (h * 31 + hashFloat(this._mass)) & 0xffffffff;
    }
    if (this._bounce != null) {
      h = (h * 31 + hashFloat(this._bounce)) & 0xffffffff;
    }
    if (this._springType != null) {
      h = (h * 31 + this._springType) & 0xffffffff;
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
      type: NodeType.TRANSITION_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
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
    propertyReprs.push(`type=${TransitionType[this.type]}`);
    if (this.delay != null) {
      propertyReprs.push(`delay=${this.delay}`);
    }
    if (this.duration != null) {
      propertyReprs.push(`duration=${this.duration}`);
    }
    if (this.ease.length > 0) {
      propertyReprs.push(`ease=${this.ease.map((_item) => _item).join(", ")}`);
    }
    if (this.stiffness != null) {
      propertyReprs.push(`stiffness=${this.stiffness}`);
    }
    if (this.damping != null) {
      propertyReprs.push(`damping=${this.damping}`);
    }
    if (this.mass != null) {
      propertyReprs.push(`mass=${this.mass}`);
    }
    if (this.bounce != null) {
      propertyReprs.push(`bounce=${this.bounce}`);
    }
    if (this.springType != null) {
      propertyReprs.push(`springType=${SpringType[this.springType]}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<TransitionStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return TransitionStyle.__packCson__(this);
  }

  static __packCson__(object: TransitionStyle): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2200000;
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
    objectCson["100"] = object._type;
    if (object._delay != null) {
      objectCson["102"] = object._delay;
    }
    if (object._duration != null) {
      objectCson["103"] = object._duration;
    }
    if (object._ease.length > 0) {
      const packedEase: any[] = [];
      for (const item of object._ease) {
        packedEase.push(item);
      }
      objectCson["104"] = packedEase;
    }
    if (object._stiffness != null) {
      objectCson["105"] = object._stiffness;
    }
    if (object._damping != null) {
      objectCson["106"] = object._damping;
    }
    if (object._mass != null) {
      objectCson["107"] = object._mass;
    }
    if (object._bounce != null) {
      objectCson["108"] = object._bounce;
    }
    if (object._springType != null) {
      objectCson["109"] = object._springType;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): TransitionStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const delayValue = objectCson["102"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectCson["103"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectCson["104"] != undefined) {
      for (const item of objectCson["104"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectCson["105"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectCson["106"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectCson["107"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectCson["108"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectCson["109"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new TransitionStyle({
      type: Number(objectCson["100"]),
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
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
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): TransitionStyle {
    return TransitionStyle.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): TransitionStyleProto {
    return TransitionStyle.__packProto__(this);
  }

  static __packProto__(object: TransitionStyle): TransitionStyleProto {
    const objectProto: Partial<TransitionStyleProto> = { metatype: 2200000 };
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
    objectProto.type = Number(object._type) as TransitionTypeProto;
    if (object._delay != null) {
      objectProto.delay = object._delay;
    }
    if (object._duration != null) {
      objectProto.duration = object._duration;
    }
    if (object._ease) {
      const packedEase: any[] = [];
      for (const item of object._ease) {
        packedEase.push(item);
      }
      objectProto.ease = packedEase;
    }
    if (object._stiffness != null) {
      objectProto.stiffness = object._stiffness;
    }
    if (object._damping != null) {
      objectProto.damping = object._damping;
    }
    if (object._mass != null) {
      objectProto.mass = object._mass;
    }
    if (object._bounce != null) {
      objectProto.bounce = object._bounce;
    }
    if (object._springType != null) {
      objectProto.springType = Number(object._springType) as SpringTypeProto;
    }
    return objectProto as TransitionStyleProto;
  }

  static __unpackProto__(
    objectProto: TransitionStyleProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): TransitionStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedEase: any[] = [];
    if (objectProto.ease) {
      for (const item of objectProto.ease) {
        unpackedEase.push(item);
      }
    }
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _graph, _graph, _connection),
        );
      }
    }
    return new TransitionStyle({
      type: Number(objectProto.type) as TransitionType,
      delay: objectProto.delay != undefined ? objectProto.delay : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      ease: unpackedEase,
      stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
      damping: objectProto.damping != undefined ? objectProto.damping : null,
      mass: objectProto.mass != undefined ? objectProto.mass : null,
      bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
      springType:
        objectProto.springType != undefined ? (Number(objectProto.springType) as SpringType) : null,
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(objectProto.parentPtr!, _session, _graph, _graph, _connection)
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _graph,
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
              _graph,
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
              _graph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(objectProto.ownedByPtr!, _session, _graph, _graph, _connection)
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(objectProto.scriptPtr!, _session, _graph, _graph, _connection)
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(objectProto.sourcePtr!, _session, _graph, _graph, _connection)
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: TransitionStyleProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): TransitionStyle {
    return TransitionStyle.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): TransitionStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TransitionStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TRANSITION_STYLE, TransitionStyle);
/* ==== DESTACK_GENERATED_END:NODE:2200000 ==== */
