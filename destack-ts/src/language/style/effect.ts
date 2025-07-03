import {
  packProtoDuration,
  packProtoTimestamp,
  unpackProtoDuration,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  Axis3,
  Graph,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Supergraph,
  Vector2,
} from "@destack/language/core";
import {
  EnumType,
  Materialization,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Transition } from "@destack/language/style/transition";
import type { View } from "@destack/language/view";
import {
  EffectProto,
  EffectStyleProto,
  EffectTypeProto,
  MaterializationProto,
  OffscreenBehaviorProto,
  RepeatTypeProto,
  TextSplitTypeProto,
} from "@destack/proto";
import { base64Decode, timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:270212 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270212 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270222 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270222 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270223 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270223 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270224 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270224 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2701000 ==== */
/**
 * An effect style.
 */
export class EffectStyle extends Style {
  static metatype: NodeType = NodeType.EFFECT_STYLE;

  /**
   * Style.parent
   */
  get parent(): Scene | View | Theme | Palette | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | View | Theme | Palette | null;
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
  readonly materialization: Materialization;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * EffectStyle.type
   */
  type: EffectType;

  /**
   * Style.name
   */
  name: string;

  /**
   * EffectStyle.opacity
   */
  opacity: number | null;

  /**
   * EffectStyle.offset
   */
  offset: Vector2 | null;

  /**
   * EffectStyle.scale
   */
  scale: number | null;

  /**
   * EffectStyle.rotate
   */
  rotate: Axis3 | null;

  /**
   * EffectStyle.skew
   */
  skew: Vector2 | null;

  /**
   * EffectStyle.perspective
   */
  perspective: number | null;

  /**
   * EffectStyle.delay
   */
  delay: Temporal.Duration | null;

  /**
   * EffectStyle.duration
   */
  duration: number | null;

  /**
   * EffectStyle.threshold
   */
  threshold: number | null;

  /**
   * EffectStyle.once
   */
  once: boolean | null;

  /**
   * EffectStyle.repeat
   */
  repeat: RepeatType | null;

  /**
   * EffectStyle.split
   */
  split: TextSplitType | null;

  /**
   * EffectStyle.offscreen
   */
  offscreen: OffscreenBehavior | null;

  /**
   * EffectStyle.transition
   */
  transition: Transition | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL */;
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
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
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
    if (!(this.type === other.type)) {
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
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.opacity !== null) {
      h = (h * 31 + hashFloat(this.opacity)) & 0xffffffff;
    }
    if (this.offset !== null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.scale !== null) {
      h = (h * 31 + hashFloat(this.scale)) & 0xffffffff;
    }
    if (this.rotate !== null) {
      h = (h * 31 + this.rotate.hash()) & 0xffffffff;
    }
    if (this.skew !== null) {
      h = (h * 31 + this.skew.hash()) & 0xffffffff;
    }
    if (this.perspective !== null) {
      h = (h * 31 + hashFloat(this.perspective)) & 0xffffffff;
    }
    if (this.delay !== null) {
      h = (h * 31 + hashFloat(this.delay.total("seconds"))) & 0xffffffff;
    }
    if (this.duration !== null) {
      h = (h * 31 + hashFloat(this.duration)) & 0xffffffff;
    }
    if (this.threshold !== null) {
      h = (h * 31 + hashFloat(this.threshold)) & 0xffffffff;
    }
    if (this.once !== null) {
      h = (h * 31 + hashBool(this.once)) & 0xffffffff;
    }
    if (this.repeat !== null) {
      h = (h * 31 + this.repeat) & 0xffffffff;
    }
    if (this.split !== null) {
      h = (h * 31 + this.split) & 0xffffffff;
    }
    if (this.offscreen !== null) {
      h = (h * 31 + this.offscreen) & 0xffffffff;
    }
    if (this.transition !== null) {
      h = (h * 31 + this.transition.hash()) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
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
      type: NodeType.EFFECT_STYLE,
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${EffectType[this.type]}`);
    if (this.opacity !== null) {
      propertyReprs.push(`opacity=${this.opacity}`);
    }
    if (this.offset !== null) {
      propertyReprs.push(`offset=${this.offset.repr()}`);
    }
    if (this.scale !== null) {
      propertyReprs.push(`scale=${this.scale}`);
    }
    if (this.rotate !== null) {
      propertyReprs.push(`rotate=${this.rotate.repr()}`);
    }
    if (this.skew !== null) {
      propertyReprs.push(`skew=${this.skew.repr()}`);
    }
    if (this.perspective !== null) {
      propertyReprs.push(`perspective=${this.perspective}`);
    }
    if (this.delay !== null) {
      propertyReprs.push(`delay=${this.delay}`);
    }
    if (this.duration !== null) {
      propertyReprs.push(`duration=${this.duration}`);
    }
    if (this.threshold !== null) {
      propertyReprs.push(`threshold=${this.threshold}`);
    }
    if (this.once !== null) {
      propertyReprs.push(`once=${this.once}`);
    }
    if (this.repeat !== null) {
      propertyReprs.push(`repeat=${RepeatType[this.repeat]}`);
    }
    if (this.split !== null) {
      propertyReprs.push(`split=${TextSplitType[this.split]}`);
    }
    if (this.offscreen !== null) {
      propertyReprs.push(`offscreen=${OffscreenBehavior[this.offscreen]}`);
    }
    if (this.transition !== null) {
      propertyReprs.push(`transition=${this.transition.repr()}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<EffectStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return EffectStyle.__packValue__(this);
  }

  static __packValue__(object: EffectStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2701000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["27"] = object.orderKey;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.opacity != null) {
      objectValue["200"] = object.opacity;
    }
    if (object.offset != null) {
      objectValue["201"] = object.offset.toValue();
    }
    if (object.scale != null) {
      objectValue["202"] = object.scale;
    }
    if (object.rotate != null) {
      objectValue["203"] = object.rotate.toValue();
    }
    if (object.skew != null) {
      objectValue["204"] = object.skew.toValue();
    }
    if (object.perspective != null) {
      objectValue["205"] = object.perspective;
    }
    if (object.delay != null) {
      objectValue["206"] = timedeltaToISOFormat(object.delay);
    }
    if (object.duration != null) {
      objectValue["207"] = object.duration;
    }
    if (object.threshold != null) {
      objectValue["208"] = object.threshold;
    }
    if (object.once != null) {
      objectValue["209"] = object.once;
    }
    if (object.repeat != null) {
      objectValue["210"] = object.repeat;
    }
    if (object.split != null) {
      objectValue["211"] = object.split;
    }
    if (object.offscreen != null) {
      objectValue["212"] = object.offscreen;
    }
    if (object.transition != null) {
      objectValue["213"] = object.transition.toValue();
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Transition = STRUCT_CLASS_BY_TYPE[StructType.TRANSITION] as typeof Transition;
    const opacityValue = objectValue["200"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const offsetValue = objectValue["201"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Vector2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectValue["202"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const rotateValue = objectValue["203"];
    const unpackedRotate =
      rotateValue != undefined
        ? _Axis3.fromValue(rotateValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectValue["204"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const perspectiveValue = objectValue["205"];
    const unpackedPerspective = perspectiveValue != undefined ? perspectiveValue : null;
    const delayValue = objectValue["206"];
    const unpackedDelay = delayValue != undefined ? timedeltaFromISOFormat(delayValue) : null;
    const durationValue = objectValue["207"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const thresholdValue = objectValue["208"];
    const unpackedThreshold = thresholdValue != undefined ? thresholdValue : null;
    const onceValue = objectValue["209"];
    const unpackedOnce = onceValue != undefined ? onceValue : null;
    const repeatValue = objectValue["210"];
    const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : null;
    const splitValue = objectValue["211"];
    const unpackedSplit = splitValue != undefined ? Number(splitValue) : null;
    const offscreenValue = objectValue["212"];
    const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : null;
    const transitionValue = objectValue["213"];
    const unpackedTransition =
      transitionValue != undefined
        ? _Transition.fromValue(transitionValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new EffectStyle({
      type: Number(objectValue["100"]),
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
      parent: unpackedParentPtr,
      name: objectValue["101"],
      space: unpackedSpacePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      materialization: Number(objectValue["10"]),
      orderKey: objectValue["27"],
      deletedAt: unpackedDeletedAt,
      id: String(objectValue["2"]),
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
    const objectProto: Partial<EffectStyleProto> = { metatype: 2701000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Transition = STRUCT_CLASS_BY_TYPE[StructType.TRANSITION] as typeof Transition;
    return new EffectStyle({
      type: Number(objectProto.type) as EffectType,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      offset:
        objectProto.offset != undefined
          ? _Vector2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      rotate:
        objectProto.rotate != undefined
          ? _Axis3.fromProto(objectProto.rotate!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
      delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
      once: objectProto.once != undefined ? objectProto.once : null,
      repeat: objectProto.repeat != undefined ? (Number(objectProto.repeat) as RepeatType) : null,
      split: objectProto.split != undefined ? (Number(objectProto.split) as TextSplitType) : null,
      offscreen:
        objectProto.offscreen != undefined
          ? (Number(objectProto.offscreen) as OffscreenBehavior)
          : null,
      transition:
        objectProto.transition != undefined
          ? _Transition.fromProto(
              objectProto.transition!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      name: objectProto.name,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
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
      materialization: Number(objectProto.materialization) as Materialization,
      orderKey: objectProto.orderKey,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      id: String(objectProto.id),
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

  static fromProtoString(packedProtoString: string): EffectStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EffectStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EFFECT_STYLE, EffectStyle);
/* ==== DESTACK_GENERATED_END:NODE:2701000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2701000 ==== */
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
  readonly stylePtr: NodeReference | null;

  /**
   * Effect.opacity
   */
  readonly opacity: number | null;

  /**
   * Effect.offset
   */
  readonly offset: Vector2 | null;

  /**
   * Effect.scale
   */
  readonly scale: number | null;

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
  readonly perspective: number | null;

  /**
   * Effect.delay
   */
  readonly delay: Temporal.Duration | null;

  /**
   * Effect.duration
   */
  readonly duration: number | null;

  /**
   * Effect.threshold
   */
  readonly threshold: number | null;

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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Effect.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
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
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.opacity !== null) {
        propertyReprs.push(`opacity=${this.opacity}`);
      }
      if (this.offset !== null) {
        propertyReprs.push(`offset=${this.offset.repr()}`);
      }
      if (this.scale !== null) {
        propertyReprs.push(`scale=${this.scale}`);
      }
      if (this.rotate !== null) {
        propertyReprs.push(`rotate=${this.rotate.repr()}`);
      }
      if (this.skew !== null) {
        propertyReprs.push(`skew=${this.skew.repr()}`);
      }
      if (this.perspective !== null) {
        propertyReprs.push(`perspective=${this.perspective}`);
      }
      if (this.delay !== null) {
        propertyReprs.push(`delay=${this.delay}`);
      }
      if (this.duration !== null) {
        propertyReprs.push(`duration=${this.duration}`);
      }
      if (this.threshold !== null) {
        propertyReprs.push(`threshold=${this.threshold}`);
      }
      if (this.once !== null) {
        propertyReprs.push(`once=${this.once}`);
      }
      if (this.repeat !== null) {
        propertyReprs.push(`repeat=${RepeatType[this.repeat]}`);
      }
      if (this.split !== null) {
        propertyReprs.push(`split=${TextSplitType[this.split]}`);
      }
      if (this.offscreen !== null) {
        propertyReprs.push(`offscreen=${OffscreenBehavior[this.offscreen]}`);
      }
      if (this.transition !== null) {
        propertyReprs.push(`transition=${this.transition.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Effect ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.stylePtr !== null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.opacity !== null) {
      h = (h * 31 + hashFloat(this.opacity)) & 0xffffffff;
    }
    if (this.offset !== null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.scale !== null) {
      h = (h * 31 + hashFloat(this.scale)) & 0xffffffff;
    }
    if (this.rotate !== null) {
      h = (h * 31 + this.rotate.hash()) & 0xffffffff;
    }
    if (this.skew !== null) {
      h = (h * 31 + this.skew.hash()) & 0xffffffff;
    }
    if (this.perspective !== null) {
      h = (h * 31 + hashFloat(this.perspective)) & 0xffffffff;
    }
    if (this.delay !== null) {
      h = (h * 31 + hashFloat(this.delay.total("seconds"))) & 0xffffffff;
    }
    if (this.duration !== null) {
      h = (h * 31 + hashFloat(this.duration)) & 0xffffffff;
    }
    if (this.threshold !== null) {
      h = (h * 31 + hashFloat(this.threshold)) & 0xffffffff;
    }
    if (this.once !== null) {
      h = (h * 31 + hashBool(this.once)) & 0xffffffff;
    }
    if (this.repeat !== null) {
      h = (h * 31 + this.repeat) & 0xffffffff;
    }
    if (this.split !== null) {
      h = (h * 31 + this.split) & 0xffffffff;
    }
    if (this.offscreen !== null) {
      h = (h * 31 + this.offscreen) & 0xffffffff;
    }
    if (this.transition !== null) {
      h = (h * 31 + this.transition.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Effect.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Effect): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2701000;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.opacity != null) {
      objectValue["102"] = object.opacity;
    }
    if (object.offset != null) {
      objectValue["103"] = object.offset.toValue();
    }
    if (object.scale != null) {
      objectValue["104"] = object.scale;
    }
    if (object.rotate != null) {
      objectValue["105"] = object.rotate.toValue();
    }
    if (object.skew != null) {
      objectValue["106"] = object.skew.toValue();
    }
    if (object.perspective != null) {
      objectValue["107"] = object.perspective;
    }
    if (object.delay != null) {
      objectValue["108"] = timedeltaToISOFormat(object.delay);
    }
    if (object.duration != null) {
      objectValue["109"] = object.duration;
    }
    if (object.threshold != null) {
      objectValue["110"] = object.threshold;
    }
    if (object.once != null) {
      objectValue["111"] = object.once;
    }
    if (object.repeat != null) {
      objectValue["112"] = object.repeat;
    }
    if (object.split != null) {
      objectValue["113"] = object.split;
    }
    if (object.offscreen != null) {
      objectValue["114"] = object.offscreen;
    }
    if (object.transition != null) {
      objectValue["115"] = object.transition.toValue();
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Transition = STRUCT_CLASS_BY_TYPE[StructType.TRANSITION] as typeof Transition;
    const stylePtrValue = objectValue["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const opacityValue = objectValue["102"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const offsetValue = objectValue["103"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Vector2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectValue["104"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const rotateValue = objectValue["105"];
    const unpackedRotate =
      rotateValue != undefined
        ? _Axis3.fromValue(rotateValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectValue["106"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const perspectiveValue = objectValue["107"];
    const unpackedPerspective = perspectiveValue != undefined ? perspectiveValue : null;
    const delayValue = objectValue["108"];
    const unpackedDelay = delayValue != undefined ? timedeltaFromISOFormat(delayValue) : null;
    const durationValue = objectValue["109"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const thresholdValue = objectValue["110"];
    const unpackedThreshold = thresholdValue != undefined ? thresholdValue : null;
    const onceValue = objectValue["111"];
    const unpackedOnce = onceValue != undefined ? onceValue : null;
    const repeatValue = objectValue["112"];
    const unpackedRepeat = repeatValue != undefined ? Number(repeatValue) : null;
    const splitValue = objectValue["113"];
    const unpackedSplit = splitValue != undefined ? Number(splitValue) : null;
    const offscreenValue = objectValue["114"];
    const unpackedOffscreen = offscreenValue != undefined ? Number(offscreenValue) : null;
    const transitionValue = objectValue["115"];
    const unpackedTransition =
      transitionValue != undefined
        ? _Transition.fromValue(transitionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Effect({
      type: Number(objectValue["100"]),
      style: unpackedStylePtr,
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
      _value: objectValue,
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
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Effect.__packProto__(this);
    }
    return this._proto as EffectProto;
  }

  static __packProto__(object: Effect): EffectProto {
    const objectProto: Partial<EffectProto> = { metatype: 2701000 };
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Transition = STRUCT_CLASS_BY_TYPE[StructType.TRANSITION] as typeof Transition;
    return new Effect({
      type: Number(objectProto.type) as EffectType,
      style:
        objectProto.stylePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.stylePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      offset:
        objectProto.offset != undefined
          ? _Vector2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      rotate:
        objectProto.rotate != undefined
          ? _Axis3.fromProto(objectProto.rotate!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      perspective: objectProto.perspective != undefined ? objectProto.perspective : null,
      delay: objectProto.delay != undefined ? unpackProtoDuration(objectProto.delay!) : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      threshold: objectProto.threshold != undefined ? objectProto.threshold : null,
      once: objectProto.once != undefined ? objectProto.once : null,
      repeat: objectProto.repeat != undefined ? (Number(objectProto.repeat) as RepeatType) : null,
      split: objectProto.split != undefined ? (Number(objectProto.split) as TextSplitType) : null,
      offscreen:
        objectProto.offscreen != undefined
          ? (Number(objectProto.offscreen) as OffscreenBehavior)
          : null,
      transition:
        objectProto.transition != undefined
          ? _Transition.fromProto(
              objectProto.transition!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _proto: objectProto,
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

  static fromProtoString(packedProtoString: string): Effect {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EffectProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.EFFECT, Effect);
/* ==== DESTACK_GENERATED_END:STRUCT:2701000 ==== */
