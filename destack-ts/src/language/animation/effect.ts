import type { Transition } from "@destack/language/animation/transition";
import type {
  Branch,
  Datetime,
  Duration,
  Float32,
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
  EnumType,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import type { Axis3, Vector2 } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { Style } from "@destack/language/style";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2200002 ==== */
/**
 * EffectType
 */
export enum EffectType {
  APPEAR = 10,
  ENTER = 11,
  EXIT = 12,
  HOVER = 20,
  PRESS = 21,
  DRAG = 22,
  FOCUS = 23,
  LOOP = 30,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.EFFECT_TYPE, EffectType);
/* ==== DESTACK_GENERATED_END:ENUM:2200002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2200003 ==== */
/**
 * RepeatType
 */
export enum RepeatType {
  LOOP = 1,
  REVERSE = 2,
  MIRROR = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.REPEAT_TYPE, RepeatType);
/* ==== DESTACK_GENERATED_END:ENUM:2200003 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100223 ==== */
/**
 * TextSplitType
 */
export enum TextSplitType {
  CHAR = 1,
  WORD = 2,
  LINE = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TEXT_SPLIT_TYPE, TextSplitType);
/* ==== DESTACK_GENERATED_END:ENUM:2100223 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100224 ==== */
/**
 * OffscreenBehavior
 */
export enum OffscreenBehavior {
  PLAY = 1,
  PAUSE = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.OFFSCREEN_BEHAVIOR, OffscreenBehavior);
/* ==== DESTACK_GENERATED_END:ENUM:2100224 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2200100 ==== */
/**
 * An effect value.
 */
export class Effect extends StructFrozen {
  static metatype: StructType = StructType.EFFECT;
  static __isFrozen__: boolean = true;

  /**
   * Effect.type
   */
  readonly type: EffectType;

  /**
   * Effect.style
   */
  get style(): EffectStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as EffectStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Effect.opacity
   */
  readonly opacity: Float32 | null;

  /**
   * Effect.offset
   */
  readonly offset: Vector2 | null;

  /**
   * Effect.scale
   */
  readonly scale: Float32 | null;

  /**
   * Effect.rotate
   */
  readonly rotate: Axis3 | null;

  /**
   * Effect.skew
   */
  readonly skew: Vector2 | null;

  /**
   * Effect.perspective
   */
  readonly perspective: Float32 | null;

  /**
   * Effect.delay
   */
  readonly delay: Duration | null;

  /**
   * Effect.duration
   */
  readonly duration: Float32 | null;

  /**
   * Effect.threshold
   */
  readonly threshold: Float32 | null;

  /**
   * Effect.once
   */
  readonly once: boolean | null;

  /**
   * Effect.repeat
   */
  readonly repeat: RepeatType | null;

  /**
   * Effect.split
   */
  readonly split: TextSplitType | null;

  /**
   * Effect.offscreen
   */
  readonly offscreen: OffscreenBehavior | null;

  /**
   * Effect.transition
   */
  readonly transition: Transition | null;

  constructor(options: {
    type: EffectType;
    style?: EffectStyle | NodeReference | null;
    opacity?: Float32 | null;
    offset?: Vector2 | null;
    scale?: Float32 | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: Float32 | null;
    delay?: Duration | null;
    duration?: Float32 | null;
    threshold?: Float32 | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
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
    let _type = options.type;
    if (_type == null) {
      throw new Error(`Effect.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name !== "NodeReference") {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style as NodeReference | null;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _scale = options.scale ?? null;
    this.scale = _scale;
    let _rotate = options.rotate ?? null;
    this.rotate = _rotate;
    let _skew = options.skew ?? null;
    this.skew = _skew;
    let _perspective = options.perspective ?? null;
    this.perspective = _perspective;
    let _delay = options.delay ?? null;
    this.delay = _delay;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _threshold = options.threshold ?? null;
    this.threshold = _threshold;
    let _once = options.once ?? null;
    this.once = _once;
    let _repeat = options.repeat ?? null;
    this.repeat = _repeat;
    let _split = options.split ?? null;
    this.split = _split;
    let _offscreen = options.offscreen ?? null;
    this.offscreen = _offscreen;
    let _transition = options.transition ?? null;
    this.transition = _transition;

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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    if (
      (this.opacity == null) !== (other.opacity == null) ||
      (this.opacity != null &&
        !(this.opacity === other.opacity || Math.abs(this.opacity - other.opacity) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.offset == null) !== (other.offset == null) ||
      (this.offset != null && !this.offset.equals(other.offset))
    ) {
      return false;
    }
    if (
      (this.scale == null) !== (other.scale == null) ||
      (this.scale != null &&
        !(this.scale === other.scale || Math.abs(this.scale - other.scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.rotate == null) !== (other.rotate == null) ||
      (this.rotate != null && !this.rotate.equals(other.rotate))
    ) {
      return false;
    }
    if (
      (this.skew == null) !== (other.skew == null) ||
      (this.skew != null && !this.skew.equals(other.skew))
    ) {
      return false;
    }
    if (
      (this.perspective == null) !== (other.perspective == null) ||
      (this.perspective != null &&
        !(
          this.perspective === other.perspective ||
          Math.abs(this.perspective - other.perspective) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this.delay === other.delay)) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null &&
        !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.threshold == null) !== (other.threshold == null) ||
      (this.threshold != null &&
        !(this.threshold === other.threshold || Math.abs(this.threshold - other.threshold) < 1e-10))
    ) {
      return false;
    }
    if (!(this.once === other.once)) {
      return false;
    }
    if (!(this.repeat === other.repeat)) {
      return false;
    }
    if (!(this.split === other.split)) {
      return false;
    }
    if (!(this.offscreen === other.offscreen)) {
      return false;
    }
    if (
      (this.transition == null) !== (other.transition == null) ||
      (this.transition != null && !this.transition.equals(other.transition))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EffectType[this.type]}`);
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.opacity != null) {
        propertyReprs.push(`opacity=${this.opacity}`);
      }
      if (this.offset != null) {
        propertyReprs.push(`offset=${this.offset.repr()}`);
      }
      if (this.scale != null) {
        propertyReprs.push(`scale=${this.scale}`);
      }
      if (this.rotate != null) {
        propertyReprs.push(`rotate=${this.rotate.repr()}`);
      }
      if (this.skew != null) {
        propertyReprs.push(`skew=${this.skew.repr()}`);
      }
      if (this.perspective != null) {
        propertyReprs.push(`perspective=${this.perspective}`);
      }
      if (this.delay != null) {
        propertyReprs.push(`delay=${this.delay}`);
      }
      if (this.duration != null) {
        propertyReprs.push(`duration=${this.duration}`);
      }
      if (this.threshold != null) {
        propertyReprs.push(`threshold=${this.threshold}`);
      }
      if (this.once != null) {
        propertyReprs.push(`once=${this.once}`);
      }
      if (this.repeat != null) {
        propertyReprs.push(`repeat=${RepeatType[this.repeat]}`);
      }
      if (this.split != null) {
        propertyReprs.push(`split=${TextSplitType[this.split]}`);
      }
      if (this.offscreen != null) {
        propertyReprs.push(`offscreen=${OffscreenBehavior[this.offscreen]}`);
      }
      if (this.transition != null) {
        propertyReprs.push(`transition=${this.transition.repr()}`);
      }
      // @ts-expect-error(readonly) */
      this._repr = `<Effect ${propertyReprs.join(" ")}>`;
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
    if (this.opacity != null) {
      h = (h * 31 + hashFloat(this.opacity)) & 0xffffffff;
    }
    if (this.offset != null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.scale != null) {
      h = (h * 31 + hashFloat(this.scale)) & 0xffffffff;
    }
    if (this.rotate != null) {
      h = (h * 31 + this.rotate.hash()) & 0xffffffff;
    }
    if (this.skew != null) {
      h = (h * 31 + this.skew.hash()) & 0xffffffff;
    }
    if (this.perspective != null) {
      h = (h * 31 + hashFloat(this.perspective)) & 0xffffffff;
    }
    if (this.delay != null) {
      h = (h * 31 + hashFloat(this.delay.total("seconds"))) & 0xffffffff;
    }
    if (this.duration != null) {
      h = (h * 31 + hashFloat(this.duration)) & 0xffffffff;
    }
    if (this.threshold != null) {
      h = (h * 31 + hashFloat(this.threshold)) & 0xffffffff;
    }
    if (this.once != null) {
      h = (h * 31 + hashBool(this.once)) & 0xffffffff;
    }
    if (this.repeat != null) {
      h = (h * 31 + this.repeat) & 0xffffffff;
    }
    if (this.split != null) {
      h = (h * 31 + this.split) & 0xffffffff;
    }
    if (this.offscreen != null) {
      h = (h * 31 + this.offscreen) & 0xffffffff;
    }
    if (this.transition != null) {
      h = (h * 31 + this.transition.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.EFFECT, Effect);
/* ==== DESTACK_GENERATED_END:STRUCT:2200100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2200100 ==== */
/**
 * An effect style.
 */
export class EffectStyle extends Style {
  static metatype: NodeType = NodeType.EFFECT_STYLE;

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
  get precededBy(): EffectStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as EffectStyle | null;
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
  readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: UInt128;

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
  readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: UInt128;

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
  readonly deletedAt: Datetime | null;

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
   * EffectStyle.type
   */
  /**
   * EffectStyle.type
   */
  get type(): EffectType {
    return this._type;
  }
  set type(value: EffectType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: EffectType;

  /**
   * EffectStyle.opacity
   */
  /**
   * EffectStyle.opacity
   */
  get opacity(): Float32 | null {
    return this._opacity;
  }
  set opacity(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["opacity"];
    this._session.updateSetProperty(this, prop, value);
    this._opacity = value;
  }
  _opacity: Float32 | null;

  /**
   * EffectStyle.offset
   */
  /**
   * EffectStyle.offset
   */
  get offset(): Vector2 | null {
    return this._offset;
  }
  set offset(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["offset"];
    this._session.updateSetProperty(this, prop, value);
    this._offset = value;
  }
  _offset: Vector2 | null;

  /**
   * EffectStyle.scale
   */
  /**
   * EffectStyle.scale
   */
  get scale(): Float32 | null {
    return this._scale;
  }
  set scale(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["scale"];
    this._session.updateSetProperty(this, prop, value);
    this._scale = value;
  }
  _scale: Float32 | null;

  /**
   * EffectStyle.rotate
   */
  /**
   * EffectStyle.rotate
   */
  get rotate(): Axis3 | null {
    return this._rotate;
  }
  set rotate(value: Axis3 | null) {
    const prop = (this.constructor as NodeClass).__properties__["rotate"];
    this._session.updateSetProperty(this, prop, value);
    this._rotate = value;
  }
  _rotate: Axis3 | null;

  /**
   * EffectStyle.skew
   */
  /**
   * EffectStyle.skew
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
   * EffectStyle.perspective
   */
  /**
   * EffectStyle.perspective
   */
  get perspective(): Float32 | null {
    return this._perspective;
  }
  set perspective(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["perspective"];
    this._session.updateSetProperty(this, prop, value);
    this._perspective = value;
  }
  _perspective: Float32 | null;

  /**
   * EffectStyle.delay
   */
  /**
   * EffectStyle.delay
   */
  get delay(): Duration | null {
    return this._delay;
  }
  set delay(value: Duration | null) {
    const prop = (this.constructor as NodeClass).__properties__["delay"];
    this._session.updateSetProperty(this, prop, value);
    this._delay = value;
  }
  _delay: Duration | null;

  /**
   * EffectStyle.duration
   */
  /**
   * EffectStyle.duration
   */
  get duration(): Float32 | null {
    return this._duration;
  }
  set duration(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["duration"];
    this._session.updateSetProperty(this, prop, value);
    this._duration = value;
  }
  _duration: Float32 | null;

  /**
   * EffectStyle.threshold
   */
  /**
   * EffectStyle.threshold
   */
  get threshold(): Float32 | null {
    return this._threshold;
  }
  set threshold(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["threshold"];
    this._session.updateSetProperty(this, prop, value);
    this._threshold = value;
  }
  _threshold: Float32 | null;

  /**
   * EffectStyle.once
   */
  /**
   * EffectStyle.once
   */
  get once(): boolean | null {
    return this._once;
  }
  set once(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["once"];
    this._session.updateSetProperty(this, prop, value);
    this._once = value;
  }
  _once: boolean | null;

  /**
   * EffectStyle.repeat
   */
  /**
   * EffectStyle.repeat
   */
  get repeat(): RepeatType | null {
    return this._repeat;
  }
  set repeat(value: RepeatType | null) {
    const prop = (this.constructor as NodeClass).__properties__["repeat"];
    this._session.updateSetProperty(this, prop, value);
    this._repeat = value;
  }
  _repeat: RepeatType | null;

  /**
   * EffectStyle.split
   */
  /**
   * EffectStyle.split
   */
  get split(): TextSplitType | null {
    return this._split;
  }
  set split(value: TextSplitType | null) {
    const prop = (this.constructor as NodeClass).__properties__["split"];
    this._session.updateSetProperty(this, prop, value);
    this._split = value;
  }
  _split: TextSplitType | null;

  /**
   * EffectStyle.offscreen
   */
  /**
   * EffectStyle.offscreen
   */
  get offscreen(): OffscreenBehavior | null {
    return this._offscreen;
  }
  set offscreen(value: OffscreenBehavior | null) {
    const prop = (this.constructor as NodeClass).__properties__["offscreen"];
    this._session.updateSetProperty(this, prop, value);
    this._offscreen = value;
  }
  _offscreen: OffscreenBehavior | null;

  /**
   * EffectStyle.transition
   */
  /**
   * EffectStyle.transition
   */
  get transition(): Transition | null {
    return this._transition;
  }
  set transition(value: Transition | null) {
    const prop = (this.constructor as NodeClass).__properties__["transition"];
    this._session.updateSetProperty(this, prop, value);
    this._transition = value;
  }
  _transition: Transition | null;

  constructor(options: {
    id?: UUID;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: EffectStyle | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: Datetime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Datetime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: UUID]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    type: EffectType;
    opacity?: Float32 | null;
    offset?: Vector2 | null;
    scale?: Float32 | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: Float32 | null;
    delay?: Duration | null;
    duration?: Float32 | null;
    threshold?: Float32 | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
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
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space == null) {
      _space = ACTIVE_SPACE.get();
      if (_space == null) {
        throw new Error(`no active Space for EffectStyle`);
      }
      _space = _space.toRef();
    }
    if (_space == null) {
      throw new Error(`EffectStyle.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization == null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization == null) {
      throw new Error(`EffectStyle.materialization is required`);
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
    if (_branch == null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch == null) {
        throw new Error(`no active Branch for EffectStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch == null) {
      throw new Error(`EffectStyle.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot == null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot == null) {
        throw new Error(`no active Snapshot for EffectStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot == null) {
      throw new Error(`EffectStyle.snapshot is required`);
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
    if (_name == null) {
      _name = "EffectStyle";
    }
    if (_name == null) {
      throw new Error(`EffectStyle.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey == null) {
      _orderKey = "a0";
    }
    if (_orderKey == null) {
      throw new Error(`EffectStyle.orderKey is required`);
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
    let _type = options.type;
    if (_type == null) {
      throw new Error(`EffectStyle.type is required`);
    }
    this._type = _type;
    let _opacity = options.opacity ?? null;
    this._opacity = _opacity;
    let _offset = options.offset ?? null;
    this._offset = _offset;
    let _scale = options.scale ?? null;
    this._scale = _scale;
    let _rotate = options.rotate ?? null;
    this._rotate = _rotate;
    let _skew = options.skew ?? null;
    this._skew = _skew;
    let _perspective = options.perspective ?? null;
    this._perspective = _perspective;
    let _delay = options.delay ?? null;
    this._delay = _delay;
    let _duration = options.duration ?? null;
    this._duration = _duration;
    let _threshold = options.threshold ?? null;
    this._threshold = _threshold;
    let _once = options.once ?? null;
    this._once = _once;
    let _repeat = options.repeat ?? null;
    this._repeat = _repeat;
    let _split = options.split ?? null;
    this._split = _split;
    let _offscreen = options.offscreen ?? null;
    this._offscreen = _offscreen;
    let _transition = options.transition ?? null;
    this._transition = _transition;

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.remoteEpoch;
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
        throw new Error(
          `EffectStyle.createdAt and EffectStyle.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorPtr;
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
      (this._opacity == null) !== (other._opacity == null) ||
      (this._opacity != null &&
        !(this._opacity === other._opacity || Math.abs(this._opacity - other._opacity) < 1e-10))
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
      (this._scale != null &&
        !(this._scale === other._scale || Math.abs(this._scale - other._scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._rotate == null) !== (other._rotate == null) ||
      (this._rotate != null && !this._rotate.equals(other._rotate))
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
      (this._perspective == null) !== (other._perspective == null) ||
      (this._perspective != null &&
        !(
          this._perspective === other._perspective ||
          Math.abs(this._perspective - other._perspective) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this._delay === other._delay)) {
      return false;
    }
    if (
      (this._duration == null) !== (other._duration == null) ||
      (this._duration != null &&
        !(this._duration === other._duration || Math.abs(this._duration - other._duration) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._threshold == null) !== (other._threshold == null) ||
      (this._threshold != null &&
        !(
          this._threshold === other._threshold ||
          Math.abs(this._threshold - other._threshold) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this._once === other._once)) {
      return false;
    }
    if (!(this._repeat === other._repeat)) {
      return false;
    }
    if (!(this._split === other._split)) {
      return false;
    }
    if (!(this._offscreen === other._offscreen)) {
      return false;
    }
    if (
      (this._transition == null) !== (other._transition == null) ||
      (this._transition != null && !this._transition.equals(other._transition))
    ) {
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
    if (JSON.stringify(this._customValues) !== JSON.stringify(other._customValues)) {
      return false;
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
    if (this._opacity != null) {
      h = (h * 31 + hashFloat(this._opacity)) & 0xffffffff;
    }
    if (this._offset != null) {
      h = (h * 31 + this._offset.hash()) & 0xffffffff;
    }
    if (this._scale != null) {
      h = (h * 31 + hashFloat(this._scale)) & 0xffffffff;
    }
    if (this._rotate != null) {
      h = (h * 31 + this._rotate.hash()) & 0xffffffff;
    }
    if (this._skew != null) {
      h = (h * 31 + this._skew.hash()) & 0xffffffff;
    }
    if (this._perspective != null) {
      h = (h * 31 + hashFloat(this._perspective)) & 0xffffffff;
    }
    if (this._delay != null) {
      h = (h * 31 + hashFloat(this._delay.total("seconds"))) & 0xffffffff;
    }
    if (this._duration != null) {
      h = (h * 31 + hashFloat(this._duration)) & 0xffffffff;
    }
    if (this._threshold != null) {
      h = (h * 31 + hashFloat(this._threshold)) & 0xffffffff;
    }
    if (this._once != null) {
      h = (h * 31 + hashBool(this._once)) & 0xffffffff;
    }
    if (this._repeat != null) {
      h = (h * 31 + this._repeat) & 0xffffffff;
    }
    if (this._split != null) {
      h = (h * 31 + this._split) & 0xffffffff;
    }
    if (this._offscreen != null) {
      h = (h * 31 + this._offscreen) & 0xffffffff;
    }
    if (this._transition != null) {
      h = (h * 31 + this._transition.hash()) & 0xffffffff;
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

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.EFFECT_STYLE,
      id: this.id,
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
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
    propertyReprs.push(`type=${EffectType[this.type]}`);
    if (this.opacity != null) {
      propertyReprs.push(`opacity=${this.opacity}`);
    }
    if (this.offset != null) {
      propertyReprs.push(`offset=${this.offset.repr()}`);
    }
    if (this.scale != null) {
      propertyReprs.push(`scale=${this.scale}`);
    }
    if (this.rotate != null) {
      propertyReprs.push(`rotate=${this.rotate.repr()}`);
    }
    if (this.skew != null) {
      propertyReprs.push(`skew=${this.skew.repr()}`);
    }
    if (this.perspective != null) {
      propertyReprs.push(`perspective=${this.perspective}`);
    }
    if (this.delay != null) {
      propertyReprs.push(`delay=${this.delay}`);
    }
    if (this.duration != null) {
      propertyReprs.push(`duration=${this.duration}`);
    }
    if (this.threshold != null) {
      propertyReprs.push(`threshold=${this.threshold}`);
    }
    if (this.once != null) {
      propertyReprs.push(`once=${this.once}`);
    }
    if (this.repeat != null) {
      propertyReprs.push(`repeat=${RepeatType[this.repeat]}`);
    }
    if (this.split != null) {
      propertyReprs.push(`split=${TextSplitType[this.split]}`);
    }
    if (this.offscreen != null) {
      propertyReprs.push(`offscreen=${OffscreenBehavior[this.offscreen]}`);
    }
    if (this.transition != null) {
      propertyReprs.push(`transition=${this.transition.repr()}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<EffectStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EFFECT_STYLE, EffectStyle);
/* ==== DESTACK_GENERATED_END:NODE:2200100 ==== */
