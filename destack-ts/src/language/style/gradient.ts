import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Axis2,
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
import type { Color } from "@destack/language/style/color";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  GradientProto,
  GradientStopProto,
  GradientStyleProto,
  GradientTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:600209 ==== */
/**
 * GradientType
 */
export enum GradientType {
  LINEAR = 10,
  RADIAL = 11,
  CONIC = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.GRADIENT_TYPE, GradientType);
/* ==== DESTACK_GENERATED_END:ENUM:600209 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:600801 ==== */
/**
 * A gradient stop with color and position.
 */
export class GradientStop extends StructFrozen {
  static metatype: StructType = StructType.GRADIENT_STOP;
  static __isFrozen__: boolean = true;

  /**
   * GradientStop.color
   */
  readonly color: Color | null;

  /**
   * GradientStop.position
   */
  readonly position: number;

  constructor(options: {
    color?: Color | null;
    position: number;
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
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`GradientStop.position is required`);
    }
    this.position = _position;

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
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (!(this.position === other.position || Math.abs(this.position - other.position) < 1e-10)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      if (this.color !== null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      propertyReprs.push(`position=${this.position}`);
      // @ts-expect-error(readonly)
      this._repr = `<GradientStop ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashFloat(this.position)) & 0xffffffff;

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
      this._value = GradientStop.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: GradientStop): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600801;
    if (object.color != null) {
      objectValue["101"] = object.color.toValue();
    }
    objectValue["102"] = object.position;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStop {
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const colorValue = objectValue["101"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new GradientStop({
      color: unpackedColor,
      position: objectValue["102"],
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
  ): GradientStop {
    return GradientStop.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GradientStopProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = GradientStop.__packProto__(this);
    }
    return this._proto as GradientStopProto;
  }

  static __packProto__(object: GradientStop): GradientStopProto {
    const objectProto: Partial<GradientStopProto> = { metatype: 600801 };
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    objectProto.position = object.position;
    return objectProto as GradientStopProto;
  }

  static __unpackProto__(
    objectProto: GradientStopProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStop {
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    return new GradientStop({
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      position: objectProto.position,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GradientStopProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStop {
    return GradientStop.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): GradientStop {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = GradientStopProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRADIENT_STOP, GradientStop);
/* ==== DESTACK_GENERATED_END:STRUCT:600801 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:600800 ==== */
/**
 * A gradient value.
 */
export class Gradient extends StructFrozen {
  static metatype: StructType = StructType.GRADIENT;
  static __isFrozen__: boolean = true;

  /**
   * Gradient.type
   */
  readonly type: GradientType;

  /**
   * Gradient.style
   */
  get style(): GradientStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as GradientStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Gradient.angle
   */
  readonly angle: number | null;

  /**
   * Gradient.stops
   */
  readonly stops: Array<GradientStop>;

  /**
   * Gradient.centerAnchor
   */
  readonly centerAnchor: Axis2 | null;

  constructor(options: {
    type?: GradientType;
    style?: GradientStyle | NodeReference | null;
    angle?: number | null;
    stops?: Array<GradientStop>;
    centerAnchor?: Axis2 | null;
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
      _type = 10 /* GradientType.LINEAR */;
    }
    if (_type === null) {
      throw new Error(`Gradient.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style;
    let _angle = options.angle ?? null;
    this.angle = _angle;
    let _stops = options.stops ?? null;
    if (_stops === null) {
      _stops = [];
    }
    this.stops = _stops;
    let _centerAnchor = options.centerAnchor ?? null;
    this.centerAnchor = _centerAnchor;

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
      (this.angle == null) !== (other.angle == null) ||
      (this.angle != null &&
        !(this.angle === other.angle || Math.abs(this.angle - other.angle) < 1e-10))
    ) {
      return false;
    }
    if (this.stops.length !== other.stops.length) {
      return false;
    }
    for (let i = 0; i < this.stops.length; i++) {
      if (!this.stops[i].equals(other.stops[i])) {
        return false;
      }
    }
    if (
      (this.centerAnchor == null) !== (other.centerAnchor == null) ||
      (this.centerAnchor != null && !this.centerAnchor.equals(other.centerAnchor))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${GradientType[this.type]}`);
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.angle !== null) {
        propertyReprs.push(`angle=${this.angle}`);
      }
      if (this.stops.length > 0) {
        propertyReprs.push(`stops=${this.stops.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.centerAnchor !== null) {
        propertyReprs.push(`centerAnchor=${this.centerAnchor.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Gradient ${propertyReprs.join(" ")}>`;
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
    if (this.angle !== null) {
      h = (h * 31 + hashFloat(this.angle)) & 0xffffffff;
    }
    if (this.stops && this.stops.length > 0) {
      for (const _item of this.stops) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.centerAnchor !== null) {
      h = (h * 31 + this.centerAnchor.hash()) & 0xffffffff;
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
      this._value = Gradient.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Gradient): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600800;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.angle != null) {
      objectValue["102"] = object.angle;
    }
    if (object.stops.length > 0) {
      const packedStops: any[] = [];
      for (const item of object.stops) {
        packedStops.push(item.toValue());
      }
      objectValue["103"] = packedStops;
    }
    if (object.centerAnchor != null) {
      objectValue["104"] = object.centerAnchor.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Gradient {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _GradientStop = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT_STOP] as typeof GradientStop;
    const stylePtrValue = objectValue["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const angleValue = objectValue["102"];
    const unpackedAngle = angleValue != undefined ? angleValue : null;
    const unpackedStops: any[] = [];
    if (objectValue["103"] != undefined) {
      for (const item of objectValue["103"]) {
        unpackedStops.push(
          _GradientStop.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const centerAnchorValue = objectValue["104"];
    const unpackedCenterAnchor =
      centerAnchorValue != undefined
        ? _Axis2.fromValue(centerAnchorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Gradient({
      type: Number(objectValue["100"]),
      style: unpackedStylePtr,
      angle: unpackedAngle,
      stops: unpackedStops,
      centerAnchor: unpackedCenterAnchor,
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
  ): Gradient {
    return Gradient.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GradientProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Gradient.__packProto__(this);
    }
    return this._proto as GradientProto;
  }

  static __packProto__(object: Gradient): GradientProto {
    const objectProto: Partial<GradientProto> = { metatype: 600800 };
    objectProto.type = Number(object.type) as GradientTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.angle != null) {
      objectProto.angle = object.angle;
    }
    if (object.stops) {
      const packedStops: any[] = [];
      for (const item of object.stops) {
        packedStops.push(item.toProto());
      }
      objectProto.stops = packedStops;
    }
    if (object.centerAnchor != null) {
      objectProto.centerAnchor = object.centerAnchor.toProto();
    }
    return objectProto as GradientProto;
  }

  static __unpackProto__(
    objectProto: GradientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Gradient {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _GradientStop = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT_STOP] as typeof GradientStop;
    const unpackedStops: any[] = [];
    if (objectProto.stops) {
      for (const item of objectProto.stops) {
        unpackedStops.push(
          _GradientStop.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Gradient({
      type: Number(objectProto.type) as GradientType,
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
      angle: objectProto.angle != undefined ? objectProto.angle : null,
      stops: unpackedStops,
      centerAnchor:
        objectProto.centerAnchor != undefined
          ? _Axis2.fromProto(objectProto.centerAnchor!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: GradientProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Gradient {
    return Gradient.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Gradient {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = GradientProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.GRADIENT, Gradient);
/* ==== DESTACK_GENERATED_END:STRUCT:600800 ==== */

/* ==== DESTACK_GENERATED_START:NODE:600800 ==== */
/**
 * A gradient style.
 */
export class GradientStyle extends Style {
  static metatype: NodeType = NodeType.GRADIENT_STYLE;

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
  get predecessor(): GradientStyle | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as GradientStyle | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): GradientStyle | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as GradientStyle | null;
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
   * GradientStyle.type
   */
  get type(): GradientType {
    return this._type;
  }
  set type(value: GradientType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: GradientType;

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
   * GradientStyle.angle
   */
  get angle(): number | null {
    return this._angle;
  }
  set angle(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["angle"];
    this._session.updateSetProperty(this, prop, value);
    this._angle = value;
  }
  _angle: number | null;

  /**
   * GradientStyle.stops
   */
  get stops(): Array<GradientStop> {
    return this._stops;
  }
  set stops(value: Array<GradientStop>) {
    const prop = (this.constructor as NodeClass).__properties__["stops"];
    this._session.updateSetProperty(this, prop, value);
    this._stops = value;
  }
  _stops: Array<GradientStop>;

  /**
   * GradientStyle.centerAnchor
   */
  get centerAnchor(): Axis2 | null {
    return this._centerAnchor;
  }
  set centerAnchor(value: Axis2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["center_anchor"];
    this._session.updateSetProperty(this, prop, value);
    this._centerAnchor = value;
  }
  _centerAnchor: Axis2 | null;

  /**
   * GradientStyle.dark
   */
  get dark(): Gradient | null {
    return this._dark;
  }
  set dark(value: Gradient | null) {
    const prop = (this.constructor as NodeClass).__properties__["dark"];
    this._session.updateSetProperty(this, prop, value);
    this._dark = value;
  }
  _dark: Gradient | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: GradientStyle | NodeReference | null;
    template?: GradientStyle | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: GradientType;
    name: string;
    angle?: number | null;
    stops?: Array<GradientStop>;
    centerAnchor?: Axis2 | null;
    dark?: Gradient | null;
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
      throw new Error(`GradientStyle.materialization is required`);
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
      throw new Error(`GradientStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* GradientType.LINEAR */;
    }
    if (_type === null) {
      throw new Error(`GradientStyle.type is required`);
    }
    this._type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`GradientStyle.name is required`);
    }
    this._name = _name;
    let _angle = options.angle ?? null;
    this._angle = _angle;
    let _stops = options.stops ?? null;
    if (_stops === null) {
      _stops = [];
    }
    this._stops = _stops;
    let _centerAnchor = options.centerAnchor ?? null;
    this._centerAnchor = _centerAnchor;
    let _dark = options.dark ?? null;
    this._dark = _dark;

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
          `GradientStyle.createdAt and GradientStyle.updatedAt are required for existing Nodes`,
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
      (this._angle == null) !== (other._angle == null) ||
      (this._angle != null &&
        !(this._angle === other._angle || Math.abs(this._angle - other._angle) < 1e-10))
    ) {
      return false;
    }
    if (this._stops.length !== other._stops.length) {
      return false;
    }
    for (let i = 0; i < this._stops.length; i++) {
      if (!this._stops[i].equals(other._stops[i])) {
        return false;
      }
    }
    if (
      (this._centerAnchor == null) !== (other._centerAnchor == null) ||
      (this._centerAnchor != null && !this._centerAnchor.equals(other._centerAnchor))
    ) {
      return false;
    }
    if (
      (this._dark == null) !== (other._dark == null) ||
      (this._dark != null && !this._dark.equals(other._dark))
    ) {
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
    if (this._angle !== null) {
      h = (h * 31 + hashFloat(this._angle)) & 0xffffffff;
    }
    if (this._stops && this._stops.length > 0) {
      for (const _item of this._stops) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this._centerAnchor !== null) {
      h = (h * 31 + this._centerAnchor.hash()) & 0xffffffff;
    }
    if (this._dark !== null) {
      h = (h * 31 + this._dark.hash()) & 0xffffffff;
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
      type: NodeType.GRADIENT_STYLE,
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
    propertyReprs.push(`type=${GradientType[this.type]}`);
    if (this.angle !== null) {
      propertyReprs.push(`angle=${this.angle}`);
    }
    if (this.stops.length > 0) {
      propertyReprs.push(`stops=${this.stops.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.centerAnchor !== null) {
      propertyReprs.push(`centerAnchor=${this.centerAnchor.repr()}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<GradientStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return GradientStyle.__packValue__(this);
  }

  static __packValue__(object: GradientStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600800;
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
    if (object._angle != null) {
      objectValue["102"] = object._angle;
    }
    if (object._stops.length > 0) {
      const packedStops: any[] = [];
      for (const item of object._stops) {
        packedStops.push(item.toValue());
      }
      objectValue["103"] = packedStops;
    }
    if (object._centerAnchor != null) {
      objectValue["104"] = object._centerAnchor.toValue();
    }
    if (object._dark != null) {
      objectValue["105"] = object._dark.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    const _GradientStop = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT_STOP] as typeof GradientStop;
    const angleValue = objectValue["102"];
    const unpackedAngle = angleValue != undefined ? angleValue : null;
    const unpackedStops: any[] = [];
    if (objectValue["103"] != undefined) {
      for (const item of objectValue["103"]) {
        unpackedStops.push(
          _GradientStop.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const centerAnchorValue = objectValue["104"];
    const unpackedCenterAnchor =
      centerAnchorValue != undefined
        ? _Axis2.fromValue(centerAnchorValue, _session, _supergraph, _graph, _connection)
        : null;
    const darkValue = objectValue["105"];
    const unpackedDark =
      darkValue != undefined
        ? _Gradient.fromValue(darkValue, _session, _supergraph, _graph, _connection)
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
    return new GradientStyle({
      type: Number(objectValue["100"]),
      angle: unpackedAngle,
      stops: unpackedStops,
      centerAnchor: unpackedCenterAnchor,
      dark: unpackedDark,
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
  ): GradientStyle {
    return GradientStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): GradientStyleProto {
    return GradientStyle.__packProto__(this);
  }

  static __packProto__(object: GradientStyle): GradientStyleProto {
    const objectProto: Partial<GradientStyleProto> = { metatype: 600800 };
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
    objectProto.type = Number(object._type) as GradientTypeProto;
    objectProto.name = object._name;
    if (object._angle != null) {
      objectProto.angle = object._angle;
    }
    if (object._stops) {
      const packedStops: any[] = [];
      for (const item of object._stops) {
        packedStops.push(item.toProto());
      }
      objectProto.stops = packedStops;
    }
    if (object._centerAnchor != null) {
      objectProto.centerAnchor = object._centerAnchor.toProto();
    }
    if (object._dark != null) {
      objectProto.dark = object._dark.toProto();
    }
    return objectProto as GradientStyleProto;
  }

  static __unpackProto__(
    objectProto: GradientStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    const _GradientStop = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT_STOP] as typeof GradientStop;
    const unpackedStops: any[] = [];
    if (objectProto.stops) {
      for (const item of objectProto.stops) {
        unpackedStops.push(
          _GradientStop.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new GradientStyle({
      type: Number(objectProto.type) as GradientType,
      angle: objectProto.angle != undefined ? objectProto.angle : null,
      stops: unpackedStops,
      centerAnchor:
        objectProto.centerAnchor != undefined
          ? _Axis2.fromProto(objectProto.centerAnchor!, _session, _supergraph, _graph, _connection)
          : null,
      dark:
        objectProto.dark != undefined
          ? _Gradient.fromProto(objectProto.dark!, _session, _supergraph, _graph, _connection)
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
    objectProto: GradientStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStyle {
    return GradientStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): GradientStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = GradientStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.GRADIENT_STYLE, GradientStyle);
/* ==== DESTACK_GENERATED_END:NODE:600800 ==== */
