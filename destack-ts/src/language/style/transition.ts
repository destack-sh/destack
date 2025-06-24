import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  Graph,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Struct,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  MaterializationTypeProto,
  SpringTypeProto,
  TransitionProto,
  TransitionStyleProto,
  TransitionTypeProto,
} from "@destack/proto";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12043 ==== */
/**
 * TransitionType
 */
export enum TransitionType {
  STYLE = 2,
  TWEEN = 10,
  SPRING = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:12043 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12044 ==== */
/**
 * SpringType
 */
export enum SpringType {
  TIME = 1,
  PHYSICS = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12044 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12024 ==== */
/**
 * A transition value.
 */
export class Transition extends Struct {
  static metatype: StructType = StructType.TRANSITION;
  static __isFrozen__: boolean = false;

  /**
   * TransitionBase.type
   */
  type: TransitionType;

  /**
   * style
   */
  get style(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as TransitionStyle | null;
    }
    return null;
  }
  set style(value: TransitionStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * TransitionBase.delay
   */
  delay: number | null;

  /**
   * TransitionBase.duration
   */
  duration: number | null;

  /**
   * TransitionBase.ease
   */
  ease: Array<number>;

  /**
   * TransitionBase.stiffness
   */
  stiffness: number | null;

  /**
   * TransitionBase.damping
   */
  damping: number | null;

  /**
   * TransitionBase.mass
   */
  mass: number | null;

  /**
   * TransitionBase.bounce
   */
  bounce: number | null;

  /**
   * TransitionBase.springType
   */
  springType: SpringType | null;

  constructor(options: {
    type?: TransitionType;
    style?: TransitionStyle | NodeReference | null;
    delay?: number | null;
    duration?: number | null;
    ease?: Array<number>;
    stiffness?: number | null;
    damping?: number | null;
    mass?: number | null;
    bounce?: number | null;
    springType?: SpringType | null;
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = TransitionType.TWEEN;
    }
    if (_type === null) {
      throw new Error(`Transition.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
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
    // ...
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.delay == null) !== (other.delay == null) ||
      (this.delay != null && !(this.delay === other.delay || Math.abs(this.delay - other.delay) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null && !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
    ) {
      return false;
    }
    if (this.ease.length !== other.ease.length) {
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
      (this.damping != null && !(this.damping === other.damping || Math.abs(this.damping - other.damping) < 1e-10))
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
      (this.bounce != null && !(this.bounce === other.bounce || Math.abs(this.bounce - other.bounce) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.springType == null) !== (other.springType == null) ||
      (this.springType != null && !(this.springType === other.springType))
    ) {
      return false;
    }
    if (
      (this.stylePtr == null) !== (other.stylePtr == null) ||
      (this.stylePtr != null && !(this.stylePtr.id === other.stylePtr.id))
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
    return Transition.__packValue__(this);
  }

  static __packValue__(object: Transition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12024;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.delay != null) {
      objectValue["50"] = object.delay;
    }
    if (object.duration != null) {
      objectValue["51"] = object.duration;
    }
    if (object.ease) {
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectValue["52"] = packedEase;
    }
    if (object.stiffness != null) {
      objectValue["53"] = object.stiffness;
    }
    if (object.damping != null) {
      objectValue["54"] = object.damping;
    }
    if (object.mass != null) {
      objectValue["55"] = object.mass;
    }
    if (object.bounce != null) {
      objectValue["56"] = object.bounce;
    }
    if (object.springType != null) {
      objectValue["57"] = object.springType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Transition {
    const delayValue = objectValue["50"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectValue["51"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectValue["52"] != undefined) {
      for (const item of objectValue["52"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectValue["53"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectValue["54"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectValue["55"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectValue["56"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectValue["57"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
    const styleValue = objectValue["41"];
    const unpackedStyle =
      styleValue != undefined ? NodeReference.fromValue(styleValue, _session, _supergraph, _graph, _connection) : null;
    return new Transition({
      type: Number(objectValue["30"]),
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
      style: unpackedStyle,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Transition {
    return Transition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TransitionProto {
    return Transition.__packProto__(this);
  }

  static __packProto__(object: Transition): TransitionProto {
    const objectProto: Partial<TransitionProto> = { metatype: 12024 };
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
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Transition {
    const unpackedEase: any[] = [];
    if (objectProto.ease) {
      for (const item of objectProto.ease) {
        unpackedEase.push(item);
      }
    }
    return new Transition({
      type: Number(objectProto.type) as TransitionType,
      delay: objectProto.delay != undefined ? objectProto.delay : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      ease: unpackedEase,
      stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
      damping: objectProto.damping != undefined ? objectProto.damping : null,
      mass: objectProto.mass != undefined ? objectProto.mass : null,
      bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
      springType: objectProto.springType != undefined ? (Number(objectProto.springType) as SpringType) : null,
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(objectProto.stylePtr!, _session, _supergraph, _graph, _connection)
          : null,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: TransitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Transition {
    return Transition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:12024 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12026 ==== */
/**
 * A transition style.
 */
export class TransitionStyle extends Node implements Style {
  static metatype: NodeType = NodeType.TRANSITION_STYLE;
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
   * TransitionBase.type
   */
  type: TransitionType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * TransitionBase.delay
   */
  delay: number | null;

  /**
   * TransitionBase.duration
   */
  duration: number | null;

  /**
   * TransitionBase.ease
   */
  ease: Array<number>;

  /**
   * TransitionBase.stiffness
   */
  stiffness: number | null;

  /**
   * TransitionBase.damping
   */
  damping: number | null;

  /**
   * TransitionBase.mass
   */
  mass: number | null;

  /**
   * TransitionBase.bounce
   */
  bounce: number | null;

  /**
   * TransitionBase.springType
   */
  springType: SpringType | null;

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
    type?: TransitionType;
    name: string;
    delay?: number | null;
    duration?: number | null;
    ease?: Array<number>;
    stiffness?: number | null;
    damping?: number | null;
    mass?: number | null;
    bounce?: number | null;
    springType?: SpringType | null;
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
      throw new Error(`TransitionStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`TransitionStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = TransitionType.TWEEN;
    }
    if (_type === null) {
      throw new Error(`TransitionStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`TransitionStyle.name is required`);
    }
    this.name = _name;
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
    if (!(this.materialization === other.materialization)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (
      (this.delay == null) !== (other.delay == null) ||
      (this.delay != null && !(this.delay === other.delay || Math.abs(this.delay - other.delay) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.duration == null) !== (other.duration == null) ||
      (this.duration != null && !(this.duration === other.duration || Math.abs(this.duration - other.duration) < 1e-10))
    ) {
      return false;
    }
    if (this.ease.length !== other.ease.length) {
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
      (this.damping != null && !(this.damping === other.damping || Math.abs(this.damping - other.damping) < 1e-10))
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
      (this.bounce != null && !(this.bounce === other.bounce || Math.abs(this.bounce - other.bounce) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.springType == null) !== (other.springType == null) ||
      (this.springType != null && !(this.springType === other.springType))
    ) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
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
      nodeType: NodeType.TRANSITION_STYLE,
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
    return TransitionStyle.__packValue__(this);
  }

  static __packValue__(object: TransitionStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12026;
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
    if (object.delay != null) {
      objectValue["50"] = object.delay;
    }
    if (object.duration != null) {
      objectValue["51"] = object.duration;
    }
    if (object.ease) {
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectValue["52"] = packedEase;
    }
    if (object.stiffness != null) {
      objectValue["53"] = object.stiffness;
    }
    if (object.damping != null) {
      objectValue["54"] = object.damping;
    }
    if (object.mass != null) {
      objectValue["55"] = object.mass;
    }
    if (object.bounce != null) {
      objectValue["56"] = object.bounce;
    }
    if (object.springType != null) {
      objectValue["57"] = object.springType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TransitionStyle {
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const delayValue = objectValue["50"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectValue["51"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectValue["52"] != undefined) {
      for (const item of objectValue["52"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectValue["53"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectValue["54"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectValue["55"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectValue["56"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectValue["57"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new TransitionStyle({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
  ): TransitionStyle {
    return TransitionStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TransitionStyleProto {
    return TransitionStyle.__packProto__(this);
  }

  static __packProto__(object: TransitionStyle): TransitionStyleProto {
    const objectProto: Partial<TransitionStyleProto> = { metatype: 12026 };
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
    objectProto.type = Number(object.type) as TransitionTypeProto;
    objectProto.name = object.name;
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
    return objectProto as TransitionStyleProto;
  }

  static __unpackProto__(
    objectProto: TransitionStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TransitionStyle {
    const unpackedEase: any[] = [];
    if (objectProto.ease) {
      for (const item of objectProto.ease) {
        unpackedEase.push(item);
      }
    }
    return new TransitionStyle({
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      type: Number(objectProto.type) as TransitionType,
      delay: objectProto.delay != undefined ? objectProto.delay : null,
      duration: objectProto.duration != undefined ? objectProto.duration : null,
      ease: unpackedEase,
      stiffness: objectProto.stiffness != undefined ? objectProto.stiffness : null,
      damping: objectProto.damping != undefined ? objectProto.damping : null,
      mass: objectProto.mass != undefined ? objectProto.mass : null,
      bounce: objectProto.bounce != undefined ? objectProto.bounce : null,
      springType: objectProto.springType != undefined ? (Number(objectProto.springType) as SpringType) : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: TransitionStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TransitionStyle {
    return TransitionStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:12026 ==== */
