import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
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
import type { Color } from "@destack/language/style/color";
import { Style } from "@destack/language/style/style";
import type { Axis2 } from "@destack/language/view";
import {
  GradientProto,
  GradientStopProto,
  GradientStyleProto,
  GradientTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2100209 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100209 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2100801 ==== */
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
      if (this.color != null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      propertyReprs.push(`position=${this.position}`);
      // @ts-expect-error(readonly)
      this._repr = `<GradientStop ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.color != null) {
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = GradientStop.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: GradientStop): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100801;
    if (object.color != null) {
      objectValue["101"] = object.color.toValue();
    }
    objectValue["102"] = object.position;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
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
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<GradientStopProto> = { metatype: 2100801 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2100801 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2100800 ==== */
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
    if (nodePtr != null) {
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
  readonly stops: readonly GradientStop[];

  /**
   * Gradient.centerAnchor
   */
  readonly centerAnchor: Axis2 | null;

  constructor(options: {
    type?: GradientType;
    style?: GradientStyle | NodeReference | null;
    angle?: number | null;
    stops?: readonly GradientStop[];
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
    if (this.stops.length != other.stops.length) {
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
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.angle != null) {
        propertyReprs.push(`angle=${this.angle}`);
      }
      if (this.stops.length > 0) {
        propertyReprs.push(`stops=${this.stops.map((_item) => _item.repr()).join(", ")}`);
      }
      if (this.centerAnchor != null) {
        propertyReprs.push(`centerAnchor=${this.centerAnchor.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Gradient ${propertyReprs.join(" ")}>`;
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
    if (this.angle != null) {
      h = (h * 31 + hashFloat(this.angle)) & 0xffffffff;
    }
    if (this.stops && this.stops.length > 0) {
      for (const _item of this.stops) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.centerAnchor != null) {
      h = (h * 31 + this.centerAnchor.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Gradient.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Gradient): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100800;
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
    objectValue: { readonly [key: string]: any },
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
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<GradientProto> = { metatype: 2100800 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2100800 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2100800 ==== */
/**
 * A gradient style.
 */
export class GradientStyle extends Style {
  static metatype: NodeType = NodeType.GRADIENT_STYLE;

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
   * The definition this CustomEntity is an instance of.
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
   * The previous Entity this Entity is based on (from the base Snapshot).
   */
  get precededBy(): GradientStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as GradientStyle | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instantiationRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instantiationRootPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instantiationRootPtr: NodeReference | null;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * The main / root Script of this Node.
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
   * The main / root Script of this Node.
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
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * GradientStyle.type
   */
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
   * GradientStyle.angle
   */
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
  /**
   * GradientStyle.stops
   */
  get stops(): readonly GradientStop[] {
    return this._stops;
  }
  set stops(value: readonly GradientStop[]) {
    const prop = (this.constructor as NodeClass).__properties__["stops"];
    this._session.updateSetProperty(this, prop, value);
    this._stops = value;
  }
  _stops: readonly GradientStop[];

  /**
   * GradientStyle.centerAnchor
   */
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
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    snapshot?: Snapshot | NodeReference;
    precededBy?: GradientStyle | NodeReference | null;
    instantiationRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    orderKey?: string;
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type?: GradientType;
    angle?: number | null;
    stops?: readonly GradientStop[];
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
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for GradientStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`GradientStyle.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`GradientStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for GradientStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`GradientStyle.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instantiationRoot = options.instantiationRoot ?? null;
    if (_instantiationRoot != null && _instantiationRoot.metatype != StructType.NODE_REFERENCE) {
      _instantiationRoot = (_instantiationRoot as Node).toRef();
    }
    this.instantiationRootPtr = _instantiationRoot;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`GradientStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "GradientStyle";
    }
    if (_name === null) {
      throw new Error(`GradientStyle.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`GradientStyle.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* GradientType.LINEAR */;
    }
    if (_type === null) {
      throw new Error(`GradientStyle.type is required`);
    }
    this._type = _type;
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
          `GradientStyle.createdAt and GradientStyle.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (this._stops.length != other._stops.length) {
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
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
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
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._angle != null) {
      h = (h * 31 + hashFloat(this._angle)) & 0xffffffff;
    }
    if (this._stops && this._stops.length > 0) {
      for (const _item of this._stops) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this._centerAnchor != null) {
      h = (h * 31 + this._centerAnchor.hash()) & 0xffffffff;
    }
    if (this._dark != null) {
      h = (h * 31 + this._dark.hash()) & 0xffffffff;
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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
      definitionId: this.definitionPtr?.id ?? null,
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
    propertyReprs.push(`type=${GradientType[this.type]}`);
    if (this.angle != null) {
      propertyReprs.push(`angle=${this.angle}`);
    }
    if (this.stops.length > 0) {
      propertyReprs.push(`stops=${this.stops.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.centerAnchor != null) {
      propertyReprs.push(`centerAnchor=${this.centerAnchor.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<GradientStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return GradientStyle.__packValue__(this);
  }

  static __packValue__(object: GradientStyle): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100800;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectValue["11"] = object.definitionPtr.toValue();
    }
    objectValue["12"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["13"] = object.precededByPtr.toValue();
    }
    if (object.instantiationRootPtr != null) {
      objectValue["15"] = object.instantiationRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["31"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
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
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): GradientStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
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
    const definitionPtrValue = objectValue["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["13"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instantiationRootPtrValue = objectValue["15"];
    const unpackedInstantiationRootPtr =
      instantiationRootPtrValue != undefined
        ? _NodeReference.fromValue(
            instantiationRootPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new GradientStyle({
      type: Number(objectValue["100"]),
      angle: unpackedAngle,
      stops: unpackedStops,
      centerAnchor: unpackedCenterAnchor,
      dark: unpackedDark,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      definition: unpackedDefinitionPtr,
      snapshot: _NodeReference.fromValue(
        objectValue["12"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instantiationRoot: unpackedInstantiationRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      orderKey: objectValue["31"],
      isExtensible: objectValue["90"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      script: unpackedScriptPtr,
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<GradientStyleProto> = { metatype: 2100800 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instantiationRootPtr != null) {
      objectProto.instantiationRootPtr = object.instantiationRootPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as GradientTypeProto;
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
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
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
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
      instantiationRoot:
        objectProto.instantiationRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instantiationRootPtr!,
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
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      isExtensible: objectProto.isExtensible,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:2100800 ==== */
