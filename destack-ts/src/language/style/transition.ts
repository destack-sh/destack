import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsSubject,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
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
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  MaterializationProto,
  SpringTypeProto,
  TransitionProto,
  TransitionStyleProto,
  TransitionTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:600210 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:600210 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:600211 ==== */
/**
 * SpringType
 */
export enum SpringType {
  TIME = 1,
  PHYSICS = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SPRING_TYPE, SpringType);
/* ==== DESTACK_GENERATED_END:ENUM:600211 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:600900 ==== */
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
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as TransitionStyle | null;
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
  readonly ease: Array<number>;

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
    ease?: Array<number>;
    stiffness?: number | null;
    damping?: number | null;
    mass?: number | null;
    bounce?: number | null;
    springType?: SpringType | null;
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* TransitionType.TWEEN */;
    }
    if (_type === null) {
      throw new Error(`Transition.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
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
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.delay !== null) {
        propertyReprs.push(`delay=${this.delay}`);
      }
      if (this.duration !== null) {
        propertyReprs.push(`duration=${this.duration}`);
      }
      if (this.ease.length > 0) {
        propertyReprs.push(`ease=${this.ease.map((_item) => _item).join(", ")}`);
      }
      if (this.stiffness !== null) {
        propertyReprs.push(`stiffness=${this.stiffness}`);
      }
      if (this.damping !== null) {
        propertyReprs.push(`damping=${this.damping}`);
      }
      if (this.mass !== null) {
        propertyReprs.push(`mass=${this.mass}`);
      }
      if (this.bounce !== null) {
        propertyReprs.push(`bounce=${this.bounce}`);
      }
      if (this.springType !== null) {
        propertyReprs.push(`springType=${SpringType[this.springType]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Transition ${propertyReprs.join(" ")}>`;
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
    if (this.delay !== null) {
      h = (h * 31 + hashFloat(this.delay)) & 0xffffffff;
    }
    if (this.duration !== null) {
      h = (h * 31 + hashFloat(this.duration)) & 0xffffffff;
    }
    if (this.ease && this.ease.length > 0) {
      for (const _item of this.ease) {
        h = (h * 31 + hashFloat(_item)) & 0xffffffff;
      }
    }
    if (this.stiffness !== null) {
      h = (h * 31 + hashFloat(this.stiffness)) & 0xffffffff;
    }
    if (this.damping !== null) {
      h = (h * 31 + hashFloat(this.damping)) & 0xffffffff;
    }
    if (this.mass !== null) {
      h = (h * 31 + hashFloat(this.mass)) & 0xffffffff;
    }
    if (this.bounce !== null) {
      h = (h * 31 + hashFloat(this.bounce)) & 0xffffffff;
    }
    if (this.springType !== null) {
      h = (h * 31 + this.springType) & 0xffffffff;
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
      this._value = Transition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Transition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600900;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.delay != null) {
      objectValue["102"] = object.delay;
    }
    if (object.duration != null) {
      objectValue["103"] = object.duration;
    }
    if (object.ease.length > 0) {
      const packedEase: any[] = [];
      for (const item of object.ease) {
        packedEase.push(item);
      }
      objectValue["104"] = packedEase;
    }
    if (object.stiffness != null) {
      objectValue["105"] = object.stiffness;
    }
    if (object.damping != null) {
      objectValue["106"] = object.damping;
    }
    if (object.mass != null) {
      objectValue["107"] = object.mass;
    }
    if (object.bounce != null) {
      objectValue["108"] = object.bounce;
    }
    if (object.springType != null) {
      objectValue["109"] = object.springType;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const stylePtrValue = objectValue["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const delayValue = objectValue["102"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectValue["103"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectValue["105"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectValue["106"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectValue["107"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectValue["108"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectValue["109"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
    return new Transition({
      type: Number(objectValue["100"]),
      style: unpackedStylePtr,
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
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
  ): Transition {
    return Transition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TransitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Transition.__packProto__(this);
    }
    return this._proto as TransitionProto;
  }

  static __packProto__(object: Transition): TransitionProto {
    const objectProto: Partial<TransitionProto> = { metatype: 600900 };
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
          ? _NodeReference.fromProto(
              objectProto.stylePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
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
/* ==== DESTACK_GENERATED_END:STRUCT:600900 ==== */

/* ==== DESTACK_GENERATED_START:NODE:600900 ==== */
/**
 * A transition style.
 */
export class TransitionStyle extends Style {
  static metatype: NodeType = NodeType.TRANSITION_STYLE;

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
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as TransitionStyle | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): TransitionStyle | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as TransitionStyle | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree (not the template tree).
   */
  get instanceRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instanceRootPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instanceRootPtr: NodeReference | null;

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
   * Style.name
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
  get ease(): Array<number> {
    return this._ease;
  }
  set ease(value: Array<number>) {
    const prop = (this.constructor as NodeClass).__properties__["ease"];
    this._session.updateSetProperty(this, prop, value);
    this._ease = value;
  }
  _ease: Array<number>;

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
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: TransitionStyle | NodeReference | null;
    template?: TransitionStyle | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
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
      _materialization = 32 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`TransitionStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _template = options.template ?? null;
    if (_template != null && _template.metatype != StructType.NODE_REFERENCE) {
      _template = (_template as Node).toRef();
    }
    this.templatePtr = _template;
    let _instanceRoot = options.instanceRoot ?? null;
    if (_instanceRoot != null && _instanceRoot.metatype != StructType.NODE_REFERENCE) {
      _instanceRoot = (_instanceRoot as Node).toRef();
    }
    this.instanceRootPtr = _instanceRoot;
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
      _type = 10 /* TransitionType.TWEEN */;
    }
    if (_type === null) {
      throw new Error(`TransitionStyle.type is required`);
    }
    this._type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`TransitionStyle.name is required`);
    }
    this._name = _name;
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
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `TransitionStyle.createdAt and TransitionStyle.updatedAt are required for existing Nodes`,
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
    if (this._ease.length !== other._ease.length) {
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
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this.templatePtr?.id === other.templatePtr?.id)) {
      return false;
    }
    if (!(this.instanceRootPtr?.id === other.instanceRootPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._delay !== null) {
      h = (h * 31 + hashFloat(this._delay)) & 0xffffffff;
    }
    if (this._duration !== null) {
      h = (h * 31 + hashFloat(this._duration)) & 0xffffffff;
    }
    if (this._ease && this._ease.length > 0) {
      for (const _item of this._ease) {
        h = (h * 31 + hashFloat(_item)) & 0xffffffff;
      }
    }
    if (this._stiffness !== null) {
      h = (h * 31 + hashFloat(this._stiffness)) & 0xffffffff;
    }
    if (this._damping !== null) {
      h = (h * 31 + hashFloat(this._damping)) & 0xffffffff;
    }
    if (this._mass !== null) {
      h = (h * 31 + hashFloat(this._mass)) & 0xffffffff;
    }
    if (this._bounce !== null) {
      h = (h * 31 + hashFloat(this._bounce)) & 0xffffffff;
    }
    if (this._springType !== null) {
      h = (h * 31 + this._springType) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
    }
    if (this.instanceRootPtr !== null) {
      h = (h * 31 + hashString(this.instanceRootPtr.id)) & 0xffffffff;
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
      type: NodeType.TRANSITION_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
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
    propertyReprs.push(`type=${TransitionType[this.type]}`);
    if (this.delay !== null) {
      propertyReprs.push(`delay=${this.delay}`);
    }
    if (this.duration !== null) {
      propertyReprs.push(`duration=${this.duration}`);
    }
    if (this.ease.length > 0) {
      propertyReprs.push(`ease=${this.ease.map((_item) => _item).join(", ")}`);
    }
    if (this.stiffness !== null) {
      propertyReprs.push(`stiffness=${this.stiffness}`);
    }
    if (this.damping !== null) {
      propertyReprs.push(`damping=${this.damping}`);
    }
    if (this.mass !== null) {
      propertyReprs.push(`mass=${this.mass}`);
    }
    if (this.bounce !== null) {
      propertyReprs.push(`bounce=${this.bounce}`);
    }
    if (this.springType !== null) {
      propertyReprs.push(`springType=${SpringType[this.springType]}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<TransitionStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return TransitionStyle.__packValue__(this);
  }

  static __packValue__(object: TransitionStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600900;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    if (object.templatePtr != null) {
      objectValue["13"] = object.templatePtr.toValue();
    }
    if (object.instanceRootPtr != null) {
      objectValue["14"] = object.instanceRootPtr.toValue();
    }
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
    objectValue["100"] = object._type;
    objectValue["101"] = object._name;
    if (object._delay != null) {
      objectValue["102"] = object._delay;
    }
    if (object._duration != null) {
      objectValue["103"] = object._duration;
    }
    if (object._ease.length > 0) {
      const packedEase: any[] = [];
      for (const item of object._ease) {
        packedEase.push(item);
      }
      objectValue["104"] = packedEase;
    }
    if (object._stiffness != null) {
      objectValue["105"] = object._stiffness;
    }
    if (object._damping != null) {
      objectValue["106"] = object._damping;
    }
    if (object._mass != null) {
      objectValue["107"] = object._mass;
    }
    if (object._bounce != null) {
      objectValue["108"] = object._bounce;
    }
    if (object._springType != null) {
      objectValue["109"] = object._springType;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const delayValue = objectValue["102"];
    const unpackedDelay = delayValue != undefined ? delayValue : null;
    const durationValue = objectValue["103"];
    const unpackedDuration = durationValue != undefined ? durationValue : null;
    const unpackedEase: any[] = [];
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedEase.push(item);
      }
    }
    const stiffnessValue = objectValue["105"];
    const unpackedStiffness = stiffnessValue != undefined ? stiffnessValue : null;
    const dampingValue = objectValue["106"];
    const unpackedDamping = dampingValue != undefined ? dampingValue : null;
    const massValue = objectValue["107"];
    const unpackedMass = massValue != undefined ? massValue : null;
    const bounceValue = objectValue["108"];
    const unpackedBounce = bounceValue != undefined ? bounceValue : null;
    const springTypeValue = objectValue["109"];
    const unpackedSpringType = springTypeValue != undefined ? Number(springTypeValue) : null;
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
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const templatePtrValue = objectValue["13"];
    const unpackedTemplatePtr =
      templatePtrValue != undefined
        ? _NodeReference.fromValue(templatePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instanceRootPtrValue = objectValue["14"];
    const unpackedInstanceRootPtr =
      instanceRootPtrValue != undefined
        ? _NodeReference.fromValue(instanceRootPtrValue, _session, _supergraph, _graph, _connection)
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
    return new TransitionStyle({
      type: Number(objectValue["100"]),
      delay: unpackedDelay,
      duration: unpackedDuration,
      ease: unpackedEase,
      stiffness: unpackedStiffness,
      damping: unpackedDamping,
      mass: unpackedMass,
      bounce: unpackedBounce,
      springType: unpackedSpringType,
      parent: unpackedParentPtr,
      name: objectValue["101"],
      space: unpackedSpacePtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
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
  ): TransitionStyle {
    return TransitionStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TransitionStyleProto {
    return TransitionStyle.__packProto__(this);
  }

  static __packProto__(object: TransitionStyle): TransitionStyleProto {
    const objectProto: Partial<TransitionStyleProto> = { metatype: 600900 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    if (object.templatePtr != null) {
      objectProto.templatePtr = object.templatePtr.toProto();
    }
    if (object.instanceRootPtr != null) {
      objectProto.instanceRootPtr = object.instanceRootPtr.toProto();
    }
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
    objectProto.type = Number(object._type) as TransitionTypeProto;
    objectProto.name = object._name;
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
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TransitionStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedEase: any[] = [];
    if (objectProto.ease) {
      for (const item of objectProto.ease) {
        unpackedEase.push(item);
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
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      template:
        objectProto.templatePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.templatePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instanceRoot:
        objectProto.instanceRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instanceRootPtr!,
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
    objectProto: TransitionStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TransitionStyle {
    return TransitionStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:NODE:600900 ==== */
