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
  Vector2f,
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
import { Easing } from "@destack/language/style/easing";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  EasingProto,
  MaterializationProto,
  StrokeCapProto,
  StrokePathProto,
  StrokePointProto,
  StrokeProto,
  StrokeStyleProto,
  StrokeTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:600213 ==== */
/**
 * StrokeType
 */
export enum StrokeType {
  SOLID = 1,
  DASHED = 2,
  DOTTED = 3,
  FREEHAND = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.STROKE_TYPE, StrokeType);
/* ==== DESTACK_GENERATED_END:ENUM:600213 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:6001100 ==== */
/**
 * A Stroke.
 */
export class Stroke extends StructFrozen {
  static metatype: StructType = StructType.STROKE;
  static __isFrozen__: boolean = true;

  /**
   * Stroke.type
   */
  readonly type: StrokeType;

  /**
   * The stroke size/width.
   */
  readonly size: number;

  /**
   * The amount of pressure-based thinning (0-1).
   */
  readonly thinning: number;

  /**
   * The amount of path smoothing (0-1).
   */
  readonly smoothing: number;

  /**
   * The amount of streamlining applied to path (0-1).
   */
  readonly streamline: number;

  /**
   * The easing function for pressure mapping.
   */
  readonly easing: Easing;

  /**
   * The stroke color.
   */
  readonly color: Color | null;

  /**
   * The start cap configuration.
   */
  readonly start: StrokeCap | null;

  /**
   * The end cap configuration.
   */
  readonly end: StrokeCap | null;

  constructor(options: {
    type: StrokeType;
    size: number;
    thinning: number;
    smoothing: number;
    streamline: number;
    easing: Easing;
    color?: Color | null;
    start?: StrokeCap | null;
    end?: StrokeCap | null;
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
      throw new Error(`Stroke.type is required`);
    }
    this.type = _type;
    let _size = options.size;
    if (_size === null) {
      throw new Error(`Stroke.size is required`);
    }
    this.size = _size;
    let _thinning = options.thinning;
    if (_thinning === null) {
      throw new Error(`Stroke.thinning is required`);
    }
    this.thinning = _thinning;
    let _smoothing = options.smoothing;
    if (_smoothing === null) {
      throw new Error(`Stroke.smoothing is required`);
    }
    this.smoothing = _smoothing;
    let _streamline = options.streamline;
    if (_streamline === null) {
      throw new Error(`Stroke.streamline is required`);
    }
    this.streamline = _streamline;
    let _easing = options.easing;
    if (_easing === null) {
      throw new Error(`Stroke.easing is required`);
    }
    this.easing = _easing;
    let _color = options.color ?? null;
    this.color = _color;
    let _start = options.start ?? null;
    this.start = _start;
    let _end = options.end ?? null;
    this.end = _end;

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
    if (!(this.size === other.size)) {
      return false;
    }
    if (!(this.thinning === other.thinning || Math.abs(this.thinning - other.thinning) < 1e-10)) {
      return false;
    }
    if (
      !(this.smoothing === other.smoothing || Math.abs(this.smoothing - other.smoothing) < 1e-10)
    ) {
      return false;
    }
    if (
      !(
        this.streamline === other.streamline || Math.abs(this.streamline - other.streamline) < 1e-10
      )
    ) {
      return false;
    }
    if (!(this.easing === other.easing)) {
      return false;
    }
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (
      (this.start == null) !== (other.start == null) ||
      (this.start != null && !this.start.equals(other.start))
    ) {
      return false;
    }
    if (
      (this.end == null) !== (other.end == null) ||
      (this.end != null && !this.end.equals(other.end))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Stroke>`;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashInt(this.size)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.thinning)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.smoothing)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.streamline)) & 0xffffffff;
    h = (h * 31 + this.easing) & 0xffffffff;
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.start !== null) {
      h = (h * 31 + this.start.hash()) & 0xffffffff;
    }
    if (this.end !== null) {
      h = (h * 31 + this.end.hash()) & 0xffffffff;
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
      this._value = Stroke.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Stroke): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 6001100;
    objectValue["100"] = object.type;
    objectValue["101"] = object.size;
    objectValue["102"] = object.thinning;
    objectValue["103"] = object.smoothing;
    objectValue["104"] = object.streamline;
    objectValue["105"] = object.easing;
    if (object.color != null) {
      objectValue["106"] = object.color.toValue();
    }
    if (object.start != null) {
      objectValue["110"] = object.start.toValue();
    }
    if (object.end != null) {
      objectValue["111"] = object.end.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    const colorValue = objectValue["106"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const startValue = objectValue["110"];
    const unpackedStart =
      startValue != undefined
        ? _StrokeCap.fromValue(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectValue["111"];
    const unpackedEnd =
      endValue != undefined
        ? _StrokeCap.fromValue(endValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Stroke({
      type: Number(objectValue["100"]),
      size: Number(objectValue["101"]),
      thinning: objectValue["102"],
      smoothing: objectValue["103"],
      streamline: objectValue["104"],
      easing: Number(objectValue["105"]),
      color: unpackedColor,
      start: unpackedStart,
      end: unpackedEnd,
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
  ): Stroke {
    return Stroke.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Stroke.__packProto__(this);
    }
    return this._proto as StrokeProto;
  }

  static __packProto__(object: Stroke): StrokeProto {
    const objectProto: Partial<StrokeProto> = { metatype: 6001100 };
    objectProto.type = Number(object.type) as StrokeTypeProto;
    objectProto.size = object.size;
    objectProto.thinning = object.thinning;
    objectProto.smoothing = object.smoothing;
    objectProto.streamline = object.streamline;
    objectProto.easing = Number(object.easing) as EasingProto;
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.start != null) {
      objectProto.start = object.start.toProto();
    }
    if (object.end != null) {
      objectProto.end = object.end.toProto();
    }
    return objectProto as StrokeProto;
  }

  static __unpackProto__(
    objectProto: StrokeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    return new Stroke({
      type: Number(objectProto.type) as StrokeType,
      size: Number(objectProto.size),
      thinning: objectProto.thinning,
      smoothing: objectProto.smoothing,
      streamline: objectProto.streamline,
      easing: Number(objectProto.easing) as Easing,
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      start:
        objectProto.start != undefined
          ? _StrokeCap.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection)
          : null,
      end:
        objectProto.end != undefined
          ? _StrokeCap.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StrokeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    return Stroke.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Stroke {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StrokeProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STROKE, Stroke);
/* ==== DESTACK_GENERATED_END:STRUCT:6001100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:6001101 ==== */
/**
 * A stroke cap.
 */
export class StrokeCap extends StructFrozen {
  static metatype: StructType = StructType.STROKE_CAP;
  static __isFrozen__: boolean = true;

  /**
   * Whether to cap the stroke.
   */
  readonly cap: boolean;

  /**
   * Whether to taper the stroke.
   */
  readonly taper: boolean;

  /**
   * The easing function for taper.
   */
  readonly easing: Easing;

  constructor(options: {
    cap: boolean;
    taper: boolean;
    easing: Easing;
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
    let _cap = options.cap;
    if (_cap === null) {
      throw new Error(`StrokeCap.cap is required`);
    }
    this.cap = _cap;
    let _taper = options.taper;
    if (_taper === null) {
      throw new Error(`StrokeCap.taper is required`);
    }
    this.taper = _taper;
    let _easing = options.easing;
    if (_easing === null) {
      throw new Error(`StrokeCap.easing is required`);
    }
    this.easing = _easing;

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
    if (!(this.cap === other.cap)) {
      return false;
    }
    if (!(this.taper === other.taper)) {
      return false;
    }
    if (!(this.easing === other.easing)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<StrokeCap>`;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashBool(this.cap)) & 0xffffffff;
    h = (h * 31 + hashBool(this.taper)) & 0xffffffff;
    h = (h * 31 + this.easing) & 0xffffffff;

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
      this._value = StrokeCap.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokeCap): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 6001101;
    objectValue["101"] = object.cap;
    objectValue["102"] = object.taper;
    objectValue["103"] = object.easing;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return new StrokeCap({
      cap: objectValue["101"],
      taper: objectValue["102"],
      easing: Number(objectValue["103"]),
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
  ): StrokeCap {
    return StrokeCap.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeCapProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokeCap.__packProto__(this);
    }
    return this._proto as StrokeCapProto;
  }

  static __packProto__(object: StrokeCap): StrokeCapProto {
    const objectProto: Partial<StrokeCapProto> = { metatype: 6001101 };
    objectProto.cap = object.cap;
    objectProto.taper = object.taper;
    objectProto.easing = Number(object.easing) as EasingProto;
    return objectProto as StrokeCapProto;
  }

  static __unpackProto__(
    objectProto: StrokeCapProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return new StrokeCap({
      cap: objectProto.cap,
      taper: objectProto.taper,
      easing: Number(objectProto.easing) as Easing,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StrokeCapProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return StrokeCap.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): StrokeCap {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StrokeCapProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STROKE_CAP, StrokeCap);
/* ==== DESTACK_GENERATED_END:STRUCT:6001101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:6001103 ==== */
/**
 * A computed point in a stroke.
 */
export class StrokePoint extends StructFrozen {
  static metatype: StructType = StructType.STROKE_POINT;
  static __isFrozen__: boolean = true;

  /**
   * The adjusted point position.
   */
  readonly point: Vector2f;

  /**
   * The original input point.
   */
  readonly originalPoint: Vector2f;

  /**
   * The pressure value at this point (0-1).
   */
  readonly pressure: number;

  /**
   * The normalized direction vector from previous point.
   */
  readonly direction: Vector2f;

  /**
   * Distance from the previous point.
   */
  readonly distance: number;

  /**
   * Total distance from stroke start.
   */
  readonly runningLength: number;

  /**
   * The computed radius at this point.
   */
  readonly radius: number;

  constructor(options: {
    point: Vector2f;
    originalPoint: Vector2f;
    pressure: number;
    direction: Vector2f;
    distance: number;
    runningLength: number;
    radius: number;
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
    let _point = options.point;
    if (_point === null) {
      throw new Error(`StrokePoint.point is required`);
    }
    this.point = _point;
    let _originalPoint = options.originalPoint;
    if (_originalPoint === null) {
      throw new Error(`StrokePoint.originalPoint is required`);
    }
    this.originalPoint = _originalPoint;
    let _pressure = options.pressure;
    if (_pressure === null) {
      throw new Error(`StrokePoint.pressure is required`);
    }
    this.pressure = _pressure;
    let _direction = options.direction;
    if (_direction === null) {
      throw new Error(`StrokePoint.direction is required`);
    }
    this.direction = _direction;
    let _distance = options.distance;
    if (_distance === null) {
      throw new Error(`StrokePoint.distance is required`);
    }
    this.distance = _distance;
    let _runningLength = options.runningLength;
    if (_runningLength === null) {
      throw new Error(`StrokePoint.runningLength is required`);
    }
    this.runningLength = _runningLength;
    let _radius = options.radius;
    if (_radius === null) {
      throw new Error(`StrokePoint.radius is required`);
    }
    this.radius = _radius;

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
    if (!this.point.equals(other.point)) {
      return false;
    }
    if (!this.originalPoint.equals(other.originalPoint)) {
      return false;
    }
    if (!(this.pressure === other.pressure || Math.abs(this.pressure - other.pressure) < 1e-10)) {
      return false;
    }
    if (!this.direction.equals(other.direction)) {
      return false;
    }
    if (!(this.distance === other.distance || Math.abs(this.distance - other.distance) < 1e-10)) {
      return false;
    }
    if (
      !(
        this.runningLength === other.runningLength ||
        Math.abs(this.runningLength - other.runningLength) < 1e-10
      )
    ) {
      return false;
    }
    if (!(this.radius === other.radius || Math.abs(this.radius - other.radius) < 1e-10)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`point=${this.point.repr()}`);
      propertyReprs.push(`originalPoint=${this.originalPoint.repr()}`);
      // @ts-expect-error(readonly)
      this._repr = `<StrokePoint ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.point.hash()) & 0xffffffff;
    h = (h * 31 + this.originalPoint.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.pressure)) & 0xffffffff;
    h = (h * 31 + this.direction.hash()) & 0xffffffff;
    h = (h * 31 + hashFloat(this.distance)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.runningLength)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.radius)) & 0xffffffff;

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
      this._value = StrokePoint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokePoint): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 6001103;
    objectValue["101"] = object.point.toValue();
    objectValue["102"] = object.originalPoint.toValue();
    objectValue["103"] = object.pressure;
    objectValue["104"] = object.direction.toValue();
    objectValue["105"] = object.distance;
    objectValue["106"] = object.runningLength;
    objectValue["107"] = object.radius;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new StrokePoint({
      point: _Vector2f.fromValue(objectValue["101"], _session, _supergraph, _graph, _connection),
      originalPoint: _Vector2f.fromValue(
        objectValue["102"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectValue["103"],
      direction: _Vector2f.fromValue(
        objectValue["104"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      distance: objectValue["105"],
      runningLength: objectValue["106"],
      radius: objectValue["107"],
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
  ): StrokePoint {
    return StrokePoint.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokePointProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokePoint.__packProto__(this);
    }
    return this._proto as StrokePointProto;
  }

  static __packProto__(object: StrokePoint): StrokePointProto {
    const objectProto: Partial<StrokePointProto> = { metatype: 6001103 };
    objectProto.point = object.point.toProto();
    objectProto.originalPoint = object.originalPoint.toProto();
    objectProto.pressure = object.pressure;
    objectProto.direction = object.direction.toProto();
    objectProto.distance = object.distance;
    objectProto.runningLength = object.runningLength;
    objectProto.radius = object.radius;
    return objectProto as StrokePointProto;
  }

  static __unpackProto__(
    objectProto: StrokePointProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new StrokePoint({
      point: _Vector2f.fromProto(objectProto.point!, _session, _supergraph, _graph, _connection),
      originalPoint: _Vector2f.fromProto(
        objectProto.originalPoint!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      direction: _Vector2f.fromProto(
        objectProto.direction!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      distance: objectProto.distance,
      runningLength: objectProto.runningLength,
      radius: objectProto.radius,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StrokePointProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    return StrokePoint.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): StrokePoint {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StrokePointProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STROKE_POINT, StrokePoint);
/* ==== DESTACK_GENERATED_END:STRUCT:6001103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:6001102 ==== */
/**
 * A stroke path.
 */
export class StrokePath extends StructFrozen {
  static metatype: StructType = StructType.STROKE_PATH;
  static __isFrozen__: boolean = true;

  /**
   * StrokePath.points
   */
  readonly points: readonly StrokePoint[];

  constructor(options: {
    points?: readonly StrokePoint[];
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
    let _points = options.points ?? null;
    if (_points === null) {
      _points = [];
    }
    this.points = _points;

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
    if (this.points.length !== other.points.length) {
      return false;
    }
    for (let i = 0; i < this.points.length; i++) {
      if (!this.points[i].equals(other.points[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      if (this.points.length > 0) {
        propertyReprs.push(`points=${this.points.map((_item) => _item.repr()).join(", ")}`);
      }
      if (propertyReprs.length > 0) {
        // @ts-expect-error(readonly)
        this._repr = `<StrokePath ${propertyReprs.join(" ")}>`;
      } else {
        // @ts-expect-error(readonly)
        this._repr = `<StrokePath>`;
      }
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.points && this.points.length > 0) {
      for (const _item of this.points) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
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
      this._value = StrokePath.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokePath): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 6001102;
    if (object.points.length > 0) {
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.toValue());
      }
      objectValue["101"] = packedPoints;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    const _StrokePoint = STRUCT_CLASS_BY_TYPE[StructType.STROKE_POINT] as typeof StrokePoint;
    const unpackedPoints: any[] = [];
    if (objectValue["101"] != undefined) {
      for (const item of objectValue["101"]) {
        unpackedPoints.push(
          _StrokePoint.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new StrokePath({
      points: unpackedPoints,
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
  ): StrokePath {
    return StrokePath.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokePathProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokePath.__packProto__(this);
    }
    return this._proto as StrokePathProto;
  }

  static __packProto__(object: StrokePath): StrokePathProto {
    const objectProto: Partial<StrokePathProto> = { metatype: 6001102 };
    if (object.points) {
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.toProto());
      }
      objectProto.points = packedPoints;
    }
    return objectProto as StrokePathProto;
  }

  static __unpackProto__(
    objectProto: StrokePathProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    const _StrokePoint = STRUCT_CLASS_BY_TYPE[StructType.STROKE_POINT] as typeof StrokePoint;
    const unpackedPoints: any[] = [];
    if (objectProto.points) {
      for (const item of objectProto.points) {
        unpackedPoints.push(
          _StrokePoint.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new StrokePath({
      points: unpackedPoints,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StrokePathProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    return StrokePath.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): StrokePath {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StrokePathProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STROKE_PATH, StrokePath);
/* ==== DESTACK_GENERATED_END:STRUCT:6001102 ==== */

/* ==== DESTACK_GENERATED_START:NODE:6001100 ==== */
/**
 * A StrokeStyle.
 */
export class StrokeStyle extends Style {
  static metatype: NodeType = NodeType.STROKE_STYLE;

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
  readonly spacePtr: NodeReference;

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
  get predecessor(): StrokeStyle | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as StrokeStyle | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): StrokeStyle | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as StrokeStyle | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Subject that created this Entity.
   */
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Subject that last updated this Entity.
   */
  get updatedBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
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
   * StrokeStyle.type
   */
  /**
   * StrokeStyle.type
   */
  get type(): StrokeType {
    return this._type;
  }
  set type(value: StrokeType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: StrokeType;

  /**
   * Style.name
   */
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
   * The stroke size/width.
   */
  /**
   * The stroke size/width.
   */
  get size(): number {
    return this._size;
  }
  set size(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["size"];
    this._session.updateSetProperty(this, prop, value);
    this._size = value;
  }
  _size: number;

  /**
   * The amount of pressure-based thinning (0-1).
   */
  /**
   * The amount of pressure-based thinning (0-1).
   */
  get thinning(): number {
    return this._thinning;
  }
  set thinning(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["thinning"];
    this._session.updateSetProperty(this, prop, value);
    this._thinning = value;
  }
  _thinning: number;

  /**
   * The amount of path smoothing (0-1).
   */
  /**
   * The amount of path smoothing (0-1).
   */
  get smoothing(): number {
    return this._smoothing;
  }
  set smoothing(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["smoothing"];
    this._session.updateSetProperty(this, prop, value);
    this._smoothing = value;
  }
  _smoothing: number;

  /**
   * The amount of streamlining applied to path (0-1).
   */
  /**
   * The amount of streamlining applied to path (0-1).
   */
  get streamline(): number {
    return this._streamline;
  }
  set streamline(value: number) {
    const prop = (this.constructor as NodeClass).__properties__["streamline"];
    this._session.updateSetProperty(this, prop, value);
    this._streamline = value;
  }
  _streamline: number;

  /**
   * The easing function for pressure mapping.
   */
  /**
   * The easing function for pressure mapping.
   */
  get easing(): Easing {
    return this._easing;
  }
  set easing(value: Easing) {
    const prop = (this.constructor as NodeClass).__properties__["easing"];
    this._session.updateSetProperty(this, prop, value);
    this._easing = value;
  }
  _easing: Easing;

  /**
   * The start cap configuration.
   */
  /**
   * The start cap configuration.
   */
  get start(): StrokeCap | null {
    return this._start;
  }
  set start(value: StrokeCap | null) {
    const prop = (this.constructor as NodeClass).__properties__["start"];
    this._session.updateSetProperty(this, prop, value);
    this._start = value;
  }
  _start: StrokeCap | null;

  /**
   * The end cap configuration.
   */
  /**
   * The end cap configuration.
   */
  get end(): StrokeCap | null {
    return this._end;
  }
  set end(value: StrokeCap | null) {
    const prop = (this.constructor as NodeClass).__properties__["end"];
    this._session.updateSetProperty(this, prop, value);
    this._end = value;
  }
  _end: StrokeCap | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: StrokeStyle | NodeReference | null;
    template?: StrokeStyle | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type: StrokeType;
    name: string;
    size: number;
    thinning: number;
    smoothing: number;
    streamline: number;
    easing: Easing;
    start?: StrokeCap | null;
    end?: StrokeCap | null;
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
      if (this._session === null) {
        throw new Error(`StrokeStyle has no session`);
      }
      if (this._session.spacePtr === null) {
        throw new Error(`StrokeStyle has no space`);
      }
      _space = this._session.spacePtr;
    }
    if (_space === null) {
      throw new Error(`StrokeStyle.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 32 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`StrokeStyle.materialization is required`);
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`StrokeStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`StrokeStyle.type is required`);
    }
    this._type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`StrokeStyle.name is required`);
    }
    this._name = _name;
    let _size = options.size;
    if (_size === null) {
      throw new Error(`StrokeStyle.size is required`);
    }
    this._size = _size;
    let _thinning = options.thinning;
    if (_thinning === null) {
      throw new Error(`StrokeStyle.thinning is required`);
    }
    this._thinning = _thinning;
    let _smoothing = options.smoothing;
    if (_smoothing === null) {
      throw new Error(`StrokeStyle.smoothing is required`);
    }
    this._smoothing = _smoothing;
    let _streamline = options.streamline;
    if (_streamline === null) {
      throw new Error(`StrokeStyle.streamline is required`);
    }
    this._streamline = _streamline;
    let _easing = options.easing;
    if (_easing === null) {
      throw new Error(`StrokeStyle.easing is required`);
    }
    this._easing = _easing;
    let _start = options.start ?? null;
    this._start = _start;
    let _end = options.end ?? null;
    this._end = _end;

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
          `StrokeStyle.createdAt and StrokeStyle.updatedAt are required for existing Nodes`,
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
    if (!(this._size === other._size)) {
      return false;
    }
    if (
      !(this._thinning === other._thinning || Math.abs(this._thinning - other._thinning) < 1e-10)
    ) {
      return false;
    }
    if (
      !(
        this._smoothing === other._smoothing || Math.abs(this._smoothing - other._smoothing) < 1e-10
      )
    ) {
      return false;
    }
    if (
      !(
        this._streamline === other._streamline ||
        Math.abs(this._streamline - other._streamline) < 1e-10
      )
    ) {
      return false;
    }
    if (!(this._easing === other._easing)) {
      return false;
    }
    if (
      (this._start == null) !== (other._start == null) ||
      (this._start != null && !this._start.equals(other._start))
    ) {
      return false;
    }
    if (
      (this._end == null) !== (other._end == null) ||
      (this._end != null && !this._end.equals(other._end))
    ) {
      return false;
    }
    if (!(this._name === other._name)) {
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
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + hashInt(this._size)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._thinning)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._smoothing)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._streamline)) & 0xffffffff;
    h = (h * 31 + this._easing) & 0xffffffff;
    if (this._start !== null) {
      h = (h * 31 + this._start.hash()) & 0xffffffff;
    }
    if (this._end !== null) {
      h = (h * 31 + this._end.hash()) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.STROKE_STYLE,
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
    let lastNode: Node | null = this;
    while (node !== null) {
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<StrokeStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return StrokeStyle.__packValue__(this);
  }

  static __packValue__(object: StrokeStyle): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 6001100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
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
    objectValue["200"] = object._size;
    objectValue["201"] = object._thinning;
    objectValue["202"] = object._smoothing;
    objectValue["203"] = object._streamline;
    objectValue["204"] = object._easing;
    if (object._start != null) {
      objectValue["205"] = object._start.toValue();
    }
    if (object._end != null) {
      objectValue["206"] = object._end.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    const startValue = objectValue["205"];
    const unpackedStart =
      startValue != undefined
        ? _StrokeCap.fromValue(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectValue["206"];
    const unpackedEnd =
      endValue != undefined
        ? _StrokeCap.fromValue(endValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
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
    return new StrokeStyle({
      type: Number(objectValue["100"]),
      size: Number(objectValue["200"]),
      thinning: objectValue["201"],
      smoothing: objectValue["202"],
      streamline: objectValue["203"],
      easing: Number(objectValue["204"]),
      start: unpackedStart,
      end: unpackedEnd,
      parent: unpackedParentPtr,
      name: objectValue["101"],
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      orderKey: objectValue["27"],
      deletedAt: unpackedDeletedAt,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
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
  ): StrokeStyle {
    return StrokeStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeStyleProto {
    return StrokeStyle.__packProto__(this);
  }

  static __packProto__(object: StrokeStyle): StrokeStyleProto {
    const objectProto: Partial<StrokeStyleProto> = { metatype: 6001100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
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
    objectProto.type = Number(object._type) as StrokeTypeProto;
    objectProto.name = object._name;
    objectProto.size = object._size;
    objectProto.thinning = object._thinning;
    objectProto.smoothing = object._smoothing;
    objectProto.streamline = object._streamline;
    objectProto.easing = Number(object._easing) as EasingProto;
    if (object._start != null) {
      objectProto.start = object._start.toProto();
    }
    if (object._end != null) {
      objectProto.end = object._end.toProto();
    }
    return objectProto as StrokeStyleProto;
  }

  static __unpackProto__(
    objectProto: StrokeStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    return new StrokeStyle({
      type: Number(objectProto.type) as StrokeType,
      size: Number(objectProto.size),
      thinning: objectProto.thinning,
      smoothing: objectProto.smoothing,
      streamline: objectProto.streamline,
      easing: Number(objectProto.easing) as Easing,
      start:
        objectProto.start != undefined
          ? _StrokeCap.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection)
          : null,
      end:
        objectProto.end != undefined
          ? _StrokeCap.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection)
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
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: StrokeStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    return StrokeStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): StrokeStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StrokeStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.STROKE_STYLE, StrokeStyle);
/* ==== DESTACK_GENERATED_END:NODE:6001100 ==== */
