import { packProtoDuration, packProtoTimestamp, unpackProtoDuration, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  MaterializationType,
  Node,
  NodeType,
  Struct,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Axis3, Vector2 } from "@destack/language/core/common";
import { registerEnumClass, registerNodeClass, registerStructClass } from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Style, Theme, Transition } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  EffectProto,
  EffectStyleProto,
  EffectTypeProto,
  MaterializationTypeProto,
  OffscreenBehaviorProto,
  RepeatTypeProto,
  TextSplitTypeProto,
} from "@destack/proto";
import { timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import type { IMessageType } from "@protobuf-ts/runtime";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12046 ==== */
/**
 * EffectType
 */
export enum EffectType {
  STYLE = 2,
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
/* ==== DESTACK_GENERATED_END:ENUM:12046 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12047 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12047 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12048 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12048 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12049 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12049 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12025 ==== */
/**
 * An effect value.
 */
export class Effect extends Struct {
  static metatype: StructType = StructType.EFFECT;
  static __protoClass__ = EffectProto as IMessageType<any>;
  static __isFrozen__: boolean = false;

  /**
   * EffectBase.type
   */
  type: EffectType;

  /**
   * style
   */
  get style(): EffectStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as EffectStyle | null;
    }
    return null;
  }
  set style(value: EffectStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * EffectBase.opacity
   */
  opacity: number | null;

  /**
   * EffectBase.offset
   */
  offset: Vector2 | null;

  /**
   * EffectBase.scale
   */
  scale: number | null;

  /**
   * EffectBase.rotate
   */
  rotate: Axis3 | null;

  /**
   * EffectBase.skew
   */
  skew: Vector2 | null;

  /**
   * EffectBase.perspective
   */
  perspective: number | null;

  /**
   * EffectBase.delay
   */
  delay: Temporal.Duration | null;

  /**
   * EffectBase.duration
   */
  duration: number | null;

  /**
   * EffectBase.threshold
   */
  threshold: number | null;

  /**
   * EffectBase.once
   */
  once: boolean | null;

  /**
   * EffectBase.repeat
   */
  repeat: RepeatType | null;

  /**
   * EffectBase.split
   */
  split: TextSplitType | null;

  /**
   * EffectBase.offscreen
   */
  offscreen: OffscreenBehavior | null;

  /**
   * EffectBase.transition
   */
  transition: Transition | null;

  constructor(options: {
    type: EffectType;
    style?: EffectStyle | NodeReference | null;
    opacity?: number | null;
    offset?: Vector2 | null;
    scale?: number | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: number | null;
    delay?: Temporal.Duration | null;
    duration?: number | null;
    threshold?: number | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Effect.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
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

    // identity
    // ...
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.opacity == null) !== (other.opacity == null) ||
      (this.opacity != null && !(this.opacity === other.opacity || Math.abs(this.opacity - other.opacity) < 1e-10))
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
      (this.scale != null && !(this.scale === other.scale || Math.abs(this.scale - other.scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.rotate == null) !== (other.rotate == null) ||
      (this.rotate != null && !this.rotate.equals(other.rotate))
    ) {
      return false;
    }
    if ((this.skew == null) !== (other.skew == null) || (this.skew != null && !this.skew.equals(other.skew))) {
      return false;
    }
    if (
      (this.perspective == null) !== (other.perspective == null) ||
      (this.perspective != null &&
        !(this.perspective === other.perspective || Math.abs(this.perspective - other.perspective) < 1e-10))
    ) {
      return false;
    }
    if (!(this.delay === other.delay)) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null && !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
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

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return Effect.__packValue__(this);
  }

  static __packValue__(object: Effect): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12025;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.opacity != null) {
      objectValue["50"] = object.opacity;
    }
    if (object.offset != null) {
      objectValue["51"] = object.offset.toValue();
    }
    if (object.scale != null) {
      objectValue["52"] = object.scale;
    }
    if (object.rotate != null) {
      objectValue["53"] = object.rotate.toValue();
    }
    if (object.skew != null) {
      objectValue["54"] = object.skew.toValue();
    }
    if (object.perspective != null) {
      objectValue["55"] = object.perspective;
    }
    if (object.delay != null) {
      objectValue["56"] = timedeltaToISOFormat(object.delay);
    }
    if (object.duration != null) {
      objectValue["57"] = object.duration;
    }
    if (object.threshold != null) {
      objectValue["58"] = object.threshold;
    }
    if (object.once != null) {
      objectValue["59"] = object.once;
    }
    if (object.repeat != null) {
      objectValue["60"] = object.repeat;
    }
    if (object.split != null) {
      objectValue["61"] = object.split;
    }
    if (object.offscreen != null) {
      objectValue["62"] = object.offscreen;
    }
    if (object.transition != null) {
      objectValue["70"] = object.transition.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Effect {
    const stylePtrValue = objectValue["41"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const opacityValue = objectValue["50"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const offsetValue = objectValue["51"];
    const unpackedOffset =
      offsetValue != undefined ? Vector2.fromValue(offsetValue, _session, _supergraph, _graph, _connection) : null;
    const scaleValue = objectValue["52"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const rotateValue = objectValue["53"];
    const unpackedRotate =
      rotateValue != undefined ? Axis3.fromValue(rotateValue, _session, _supergraph, _graph, _connection) : null;
    const skewValue = objectValue["54"];
    const unpackedSkew =
      skewValue != undefined ? Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection) : null;
    const perspectiveValue = objectValue["55"];
    const unpackedPerspective = perspectiveValue != undefined ? perspectiveValue : null;
    const delayValue = objectValue["56"];
    const unpackedDelay = delayValue != undefined ? timedeltaFromISOFormat(delayValue) : null;
    const durationValue = objectValue["57"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const thresholdValue = objectValue["58"];
    const unpackedThreshold = thresholdValue != undefined ? thresholdValue : null;
    const onceValue = objectValue["59"];
    const unpackedOnce = onceValue != undefined ? onceValue : null;
    const repeatValue = objectValue["60"];
    const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : null;
    const splitValue = objectValue["61"];
    const unpackedSplit = splitValue != undefined ? Number(splitValue) : null;
    const offscreenValue = objectValue["62"];
    const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : null;
    const transitionValue = objectValue["70"];
    const unpackedTransition =
      transitionValue != undefined
        ? Transition.fromValue(transitionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Effect({
      style: unpackedStylePtr,
      type: Number(objectValue["30"]),
      opacity: unpackedOpacity,
      offset: unpackedOffset,
      scale: unpackedScale,
      rotate: unpackedRotate,
      skew: unpackedSkew,
      perspective: unpackedPerspective,
      delay: unpackedDelay,
      duration: unpackedDuration,
      threshold: unpackedThreshold,
      once: unpackedOnce,
      repeat: unpackedRepeat,
      split: unpackedSplit,
      offscreen: unpackedOffscreen,
      transition: unpackedTransition,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Effect {
    return Effect.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EffectProto {
    return Effect.__packProto__(this);
  }

  static __packProto__(object: Effect): EffectProto {
    const objectProto: Partial<EffectProto> = { metatype: 12025 };
    objectProto.type = Number(object.type) as EffectTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.opacity != null) {
      objectProto.opacity = object.opacity;
    }
    if (object.offset != null) {
      objectProto.offset = object.offset.toProto();
    }
    if (object.scale != null) {
      objectProto.scale = object.scale;
    }
    if (object.rotate != null) {
      objectProto.rotate = object.rotate.toProto();
    }
    if (object.skew != null) {
      objectProto.skew = object.skew.toProto();
    }
    if (object.perspective != null) {
      objectProto.perspective = object.perspective;
    }
    if (object.delay != null) {
      objectProto.delay = packProtoDuration(object.delay);
    }
    if (object.duration != null) {
      objectProto.duration = object.duration;
    }
    if (object.threshold != null) {
      objectProto.threshold = object.threshold;
    }
    if (object.once != null) {
      objectProto.once = object.once;
    }
    if (object.repeat != null) {
      objectProto.repeat = Number(object.repeat) as RepeatTypeProto;
    }
    if (object.split != null) {
      objectProto.split = Number(object.split) as TextSplitTypeProto;
    }
    if (object.offscreen != null) {
      objectProto.offscreen = Number(object.offscreen) as OffscreenBehaviorProto;
    }
    if (object.transition != null) {
      objectProto.transition = object.transition.toProto();
    }
    return objectProto as EffectProto;
  }

  static __unpackProto__(
    objectProto: EffectProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Effect {
    return new Effect({
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(objectProto.stylePtr!, _session, _supergraph, _graph, _connection)
          : null,
      type: Number(objectProto.type) as EffectType,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      offset:
        objectProto.offset != undefined
          ? Vector2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      rotate:
        objectProto.rotate != undefined
          ? Axis3.fromProto(objectProto.rotate!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
      delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
      once: objectProto.once != undefined ? objectProto.once : null,
      repeat: objectProto.repeat != undefined ? (Number(objectProto.repeat) as RepeatType) : null,
      split: objectProto.split != undefined ? (Number(objectProto.split) as TextSplitType) : null,
      offscreen: objectProto.offscreen != undefined ? (Number(objectProto.offscreen) as OffscreenBehavior) : null,
      transition:
        objectProto.transition != undefined
          ? Transition.fromProto(objectProto.transition!, _session, _supergraph, _graph, _connection)
          : null,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: EffectProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Effect {
    return Effect.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.EFFECT, Effect);
/* ==== DESTACK_GENERATED_END:STRUCT:12025 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12027 ==== */
/**
 * An effect style.
 */
export class EffectStyle extends Node implements Style {
  static metatype: NodeType = NodeType.EFFECT_STYLE;
  static __protoClass__ = EffectStyleProto as IMessageType<any>;
  static __traits__: TraitType[] = [
    TraitType.STYLE,
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.ORDERED,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.THEME,
    NodeType.THREAD_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.CANVAS,
    NodeType.TEXT_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.LAYER,
  ];
  static __childTypes__: NodeType[] = [NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.TEXT_VIEW,
    NodeType.THEME,
    NodeType.FOLDER,
    NodeType.THREAD_VIEW,
    NodeType.CANVAS,
  ];
  static __descendantTypes__: NodeType[] = [NodeType.TAGGING];

  /**
   * Style.parent
   */
  get parent(): Scene | (Node & View) | Theme | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | (Node & View) | Theme | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * EffectBase.type
   */
  type: EffectType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * EffectBase.opacity
   */
  opacity: number | null;

  /**
   * EffectBase.offset
   */
  offset: Vector2 | null;

  /**
   * EffectBase.scale
   */
  scale: number | null;

  /**
   * EffectBase.rotate
   */
  rotate: Axis3 | null;

  /**
   * EffectBase.skew
   */
  skew: Vector2 | null;

  /**
   * EffectBase.perspective
   */
  perspective: number | null;

  /**
   * EffectBase.delay
   */
  delay: Temporal.Duration | null;

  /**
   * EffectBase.duration
   */
  duration: number | null;

  /**
   * EffectBase.threshold
   */
  threshold: number | null;

  /**
   * EffectBase.once
   */
  once: boolean | null;

  /**
   * EffectBase.repeat
   */
  repeat: RepeatType | null;

  /**
   * EffectBase.split
   */
  split: TextSplitType | null;

  /**
   * EffectBase.offscreen
   */
  offscreen: OffscreenBehavior | null;

  /**
   * EffectBase.transition
   */
  transition: Transition | null;

  constructor(options: {
    id?: string;
    parent?: Scene | (Node & View) | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type: EffectType;
    name: string;
    opacity?: number | null;
    offset?: Vector2 | null;
    scale?: number | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: number | null;
    delay?: Temporal.Duration | null;
    duration?: number | null;
    threshold?: number | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
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
      // is_attached
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`EffectStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`EffectStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EffectStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`EffectStyle.name is required`);
    }
    this.name = _name;
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

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.opacity == null) !== (other.opacity == null) ||
      (this.opacity != null && !(this.opacity === other.opacity || Math.abs(this.opacity - other.opacity) < 1e-10))
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
      (this.scale != null && !(this.scale === other.scale || Math.abs(this.scale - other.scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.rotate == null) !== (other.rotate == null) ||
      (this.rotate != null && !this.rotate.equals(other.rotate))
    ) {
      return false;
    }
    if ((this.skew == null) !== (other.skew == null) || (this.skew != null && !this.skew.equals(other.skew))) {
      return false;
    }
    if (
      (this.perspective == null) !== (other.perspective == null) ||
      (this.perspective != null &&
        !(this.perspective === other.perspective || Math.abs(this.perspective - other.perspective) < 1e-10))
    ) {
      return false;
    }
    if (!(this.delay === other.delay)) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null && !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
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

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.EFFECT_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      node = node.parent;
    }
    if (!this._isAttached) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  toValue(): { [key: string]: any } {
    return EffectStyle.__packValue__(this);
  }

  static __packValue__(object: EffectStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12027;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString();
    }
    objectValue["22"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.opacity != null) {
      objectValue["50"] = object.opacity;
    }
    if (object.offset != null) {
      objectValue["51"] = object.offset.toValue();
    }
    if (object.scale != null) {
      objectValue["52"] = object.scale;
    }
    if (object.rotate != null) {
      objectValue["53"] = object.rotate.toValue();
    }
    if (object.skew != null) {
      objectValue["54"] = object.skew.toValue();
    }
    if (object.perspective != null) {
      objectValue["55"] = object.perspective;
    }
    if (object.delay != null) {
      objectValue["56"] = timedeltaToISOFormat(object.delay);
    }
    if (object.duration != null) {
      objectValue["57"] = object.duration;
    }
    if (object.threshold != null) {
      objectValue["58"] = object.threshold;
    }
    if (object.once != null) {
      objectValue["59"] = object.once;
    }
    if (object.repeat != null) {
      objectValue["60"] = object.repeat;
    }
    if (object.split != null) {
      objectValue["61"] = object.split;
    }
    if (object.offscreen != null) {
      objectValue["62"] = object.offscreen;
    }
    if (object.transition != null) {
      objectValue["70"] = object.transition.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EffectStyle {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const opacityValue = objectValue["50"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const offsetValue = objectValue["51"];
    const unpackedOffset =
      offsetValue != undefined ? Vector2.fromValue(offsetValue, _session, _supergraph, _graph, _connection) : null;
    const scaleValue = objectValue["52"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const rotateValue = objectValue["53"];
    const unpackedRotate =
      rotateValue != undefined ? Axis3.fromValue(rotateValue, _session, _supergraph, _graph, _connection) : null;
    const skewValue = objectValue["54"];
    const unpackedSkew =
      skewValue != undefined ? Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection) : null;
    const perspectiveValue = objectValue["55"];
    const unpackedPerspective = perspectiveValue != undefined ? perspectiveValue : null;
    const delayValue = objectValue["56"];
    const unpackedDelay = delayValue != undefined ? timedeltaFromISOFormat(delayValue) : null;
    const durationValue = objectValue["57"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const thresholdValue = objectValue["58"];
    const unpackedThreshold = thresholdValue != undefined ? thresholdValue : null;
    const onceValue = objectValue["59"];
    const unpackedOnce = onceValue != undefined ? onceValue : null;
    const repeatValue = objectValue["60"];
    const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : null;
    const splitValue = objectValue["61"];
    const unpackedSplit = splitValue != undefined ? Number(splitValue) : null;
    const offscreenValue = objectValue["62"];
    const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : null;
    const transitionValue = objectValue["70"];
    const unpackedTransition =
      transitionValue != undefined
        ? Transition.fromValue(transitionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new EffectStyle({
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      opacity: unpackedOpacity,
      offset: unpackedOffset,
      scale: unpackedScale,
      rotate: unpackedRotate,
      skew: unpackedSkew,
      perspective: unpackedPerspective,
      delay: unpackedDelay,
      duration: unpackedDuration,
      threshold: unpackedThreshold,
      once: unpackedOnce,
      repeat: unpackedRepeat,
      split: unpackedSplit,
      offscreen: unpackedOffscreen,
      transition: unpackedTransition,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EffectStyle {
    return EffectStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EffectStyleProto {
    return EffectStyle.__packProto__(this);
  }

  static __packProto__(object: EffectStyle): EffectStyleProto {
    const objectProto: Partial<EffectStyleProto> = { metatype: 12027 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationTypeProto;
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
    objectProto.type = Number(object.type) as EffectTypeProto;
    objectProto.name = object.name;
    if (object.opacity != null) {
      objectProto.opacity = object.opacity;
    }
    if (object.offset != null) {
      objectProto.offset = object.offset.toProto();
    }
    if (object.scale != null) {
      objectProto.scale = object.scale;
    }
    if (object.rotate != null) {
      objectProto.rotate = object.rotate.toProto();
    }
    if (object.skew != null) {
      objectProto.skew = object.skew.toProto();
    }
    if (object.perspective != null) {
      objectProto.perspective = object.perspective;
    }
    if (object.delay != null) {
      objectProto.delay = packProtoDuration(object.delay);
    }
    if (object.duration != null) {
      objectProto.duration = object.duration;
    }
    if (object.threshold != null) {
      objectProto.threshold = object.threshold;
    }
    if (object.once != null) {
      objectProto.once = object.once;
    }
    if (object.repeat != null) {
      objectProto.repeat = Number(object.repeat) as RepeatTypeProto;
    }
    if (object.split != null) {
      objectProto.split = Number(object.split) as TextSplitTypeProto;
    }
    if (object.offscreen != null) {
      objectProto.offscreen = Number(object.offscreen) as OffscreenBehaviorProto;
    }
    if (object.transition != null) {
      objectProto.transition = object.transition.toProto();
    }
    return objectProto as EffectStyleProto;
  }

  static __unpackProto__(
    objectProto: EffectStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EffectStyle {
    return new EffectStyle({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      type: Number(objectProto.type) as EffectType,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      offset:
        objectProto.offset != undefined
          ? Vector2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      rotate:
        objectProto.rotate != undefined
          ? Axis3.fromProto(objectProto.rotate!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
      delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
      once: objectProto.once != undefined ? objectProto.once : null,
      repeat: objectProto.repeat != undefined ? (Number(objectProto.repeat) as RepeatType) : null,
      split: objectProto.split != undefined ? (Number(objectProto.split) as TextSplitType) : null,
      offscreen: objectProto.offscreen != undefined ? (Number(objectProto.offscreen) as OffscreenBehavior) : null,
      transition:
        objectProto.transition != undefined
          ? Transition.fromProto(objectProto.transition!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: EffectStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EffectStyle {
    return EffectStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EFFECT_STYLE, EffectStyle);
/* ==== DESTACK_GENERATED_END:NODE:12027 ==== */
