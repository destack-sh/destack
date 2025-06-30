import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core/builtin";
import { Axis2 } from "@destack/language/core/common";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Color, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  GradientProto,
  GradientStopProto,
  GradientStyleProto,
  GradientTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:12015 ==== */
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
    objectValue["1"] = 12015;
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    objectValue["51"] = object.position;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStop {
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new GradientStop({
      color: unpackedColor,
      position: objectValue["51"],
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
    const objectProto: Partial<GradientStopProto> = { metatype: 12015 };
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
    return new GradientStop({
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
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
/* ==== DESTACK_GENERATED_END:STRUCT:12015 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12016 ==== */
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
   * style
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
      _type = GradientType.LINEAR;
    }
    if (_type === null) {
      throw new Error(`Gradient.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
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
    objectValue["1"] = 12016;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["40"] = object.stylePtr.toValue();
    }
    if (object.angle != null) {
      objectValue["50"] = object.angle;
    }
    if (object.stops.length > 0) {
      const packedStops: any[] = [];
      for (const item of object.stops) {
        packedStops.push(item.toValue());
      }
      objectValue["51"] = packedStops;
    }
    if (object.centerAnchor != null) {
      objectValue["52"] = object.centerAnchor.toValue();
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
    const stylePtrValue = objectValue["40"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const angleValue = objectValue["50"];
    const unpackedAngle = angleValue != undefined ? angleValue : null;
    const unpackedStops: any[] = [];
    if (objectValue["51"] != undefined) {
      for (const item of objectValue["51"]) {
        unpackedStops.push(
          GradientStop.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const centerAnchorValue = objectValue["52"];
    const unpackedCenterAnchor =
      centerAnchorValue != undefined
        ? Axis2.fromValue(centerAnchorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Gradient({
      type: Number(objectValue["30"]),
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
    const objectProto: Partial<GradientProto> = { metatype: 12016 };
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
    const unpackedStops: any[] = [];
    if (objectProto.stops) {
      for (const item of objectProto.stops) {
        unpackedStops.push(
          GradientStop.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Gradient({
      type: Number(objectProto.type) as GradientType,
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(
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
          ? Axis2.fromProto(objectProto.centerAnchor!, _session, _supergraph, _graph, _connection)
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
/* ==== DESTACK_GENERATED_END:STRUCT:12016 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12070 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12070 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12065 ==== */
/**
 * A gradient style.
 */
export class GradientStyle extends Style {
  static metatype: NodeType = NodeType.GRADIENT_STYLE;

  /**
   * Style.parent
   */
  get parent(): Scene | View | Theme | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | View | Theme | null;
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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * GradientStyle.type
   */
  type: GradientType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * GradientStyle.angle
   */
  angle: number | null;

  /**
   * GradientStyle.stops
   */
  stops: Array<GradientStop>;

  /**
   * GradientStyle.centerAnchor
   */
  centerAnchor: Axis2 | null;

  /**
   * GradientStyle.dark
   */
  dark: Gradient | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
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
      _type = GradientType.LINEAR;
    }
    if (_type === null) {
      throw new Error(`GradientStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`GradientStyle.name is required`);
    }
    this.name = _name;
    let _angle = options.angle ?? null;
    this.angle = _angle;
    let _stops = options.stops ?? null;
    if (_stops === null) {
      _stops = [];
    }
    this.stops = _stops;
    let _centerAnchor = options.centerAnchor ?? null;
    this.centerAnchor = _centerAnchor;
    let _dark = options.dark ?? null;
    this.dark = _dark;

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
    if (!(this.type === other.type)) {
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
    if (
      (this.dark == null) !== (other.dark == null) ||
      (this.dark != null && !this.dark.equals(other.dark))
    ) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
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
    if (this.dark !== null) {
      h = (h * 31 + this.dark.hash()) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
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
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.GRADIENT_STYLE,
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
    objectValue["1"] = 12065;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["22"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.angle != null) {
      objectValue["50"] = object.angle;
    }
    if (object.stops.length > 0) {
      const packedStops: any[] = [];
      for (const item of object.stops) {
        packedStops.push(item.toValue());
      }
      objectValue["51"] = packedStops;
    }
    if (object.centerAnchor != null) {
      objectValue["52"] = object.centerAnchor.toValue();
    }
    if (object.dark != null) {
      objectValue["60"] = object.dark.toValue();
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
    const angleValue = objectValue["50"];
    const unpackedAngle = angleValue != undefined ? angleValue : null;
    const unpackedStops: any[] = [];
    if (objectValue["51"] != undefined) {
      for (const item of objectValue["51"]) {
        unpackedStops.push(
          GradientStop.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const centerAnchorValue = objectValue["52"];
    const unpackedCenterAnchor =
      centerAnchorValue != undefined
        ? Axis2.fromValue(centerAnchorValue, _session, _supergraph, _graph, _connection)
        : null;
    const darkValue = objectValue["60"];
    const unpackedDark =
      darkValue != undefined
        ? Gradient.fromValue(darkValue, _session, _supergraph, _graph, _connection)
        : null;
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
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new GradientStyle({
      type: Number(objectValue["30"]),
      angle: unpackedAngle,
      stops: unpackedStops,
      centerAnchor: unpackedCenterAnchor,
      dark: unpackedDark,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
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
    const objectProto: Partial<GradientStyleProto> = { metatype: 12065 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
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
    objectProto.type = Number(object.type) as GradientTypeProto;
    objectProto.name = object.name;
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
    if (object.dark != null) {
      objectProto.dark = object.dark.toProto();
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
    const unpackedStops: any[] = [];
    if (objectProto.stops) {
      for (const item of objectProto.stops) {
        unpackedStops.push(
          GradientStop.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new GradientStyle({
      type: Number(objectProto.type) as GradientType,
      angle: objectProto.angle != undefined ? objectProto.angle : null,
      stops: unpackedStops,
      centerAnchor:
        objectProto.centerAnchor != undefined
          ? Axis2.fromProto(objectProto.centerAnchor!, _session, _supergraph, _graph, _connection)
          : null,
      dark:
        objectProto.dark != undefined
          ? Gradient.fromProto(objectProto.dark!, _session, _supergraph, _graph, _connection)
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
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
/* ==== DESTACK_GENERATED_END:NODE:12065 ==== */
