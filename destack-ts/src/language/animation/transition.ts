import type {
  Branch,
  NodeClass,
  NodeReference,
  PackedCache,
  Session,
  Snapshot,
  Space,
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
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as TransitionStyle | null;
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
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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

    /* identity */
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
      // @ts-expect-error(readonly) */
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
  get precededBy(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as TransitionStyle | null;
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
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null
        ? options.parent.constructor.name == "NodeReference"
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

    /* identity */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TRANSITION_STYLE, TransitionStyle);
/* ==== DESTACK_GENERATED_END:NODE:2200000 ==== */
