import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type { IsSubject, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  Graph,
  Node,
  NodeReference,
  NodeType,
  StructFrozen,
  StructType,
  Vector2,
} from "@destack/language/core";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { Theme } from "@destack/language/style";
import { Color, Easing, Style } from "@destack/language/style";
import type { View } from "@destack/language/view";
import {
  EasingProto,
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

/* ==== DESTACK_GENERATED_START:ENUM:270213 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270213 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:270100 ==== */
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
   * Whether to simulate pressure if not provided.
   */
  readonly simulatePressure: boolean;

  /**
   * The easing function for pressure mapping.
   */
  readonly easing: Easing;

  /**
   * The start cap configuration.
   */
  readonly start: StrokeCap | null;

  /**
   * The end cap configuration.
   */
  readonly end: StrokeCap | null;

  /**
   * The stroke color.
   */
  readonly color: Color | null;

  constructor(options: {
    type: StrokeType;
    size: number;
    thinning: number;
    smoothing: number;
    streamline: number;
    simulatePressure: boolean;
    easing: Easing;
    start?: StrokeCap | null;
    end?: StrokeCap | null;
    color?: Color | null;
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
    let _simulatePressure = options.simulatePressure;
    if (_simulatePressure === null) {
      throw new Error(`Stroke.simulatePressure is required`);
    }
    this.simulatePressure = _simulatePressure;
    let _easing = options.easing;
    if (_easing === null) {
      throw new Error(`Stroke.easing is required`);
    }
    this.easing = _easing;
    let _start = options.start ?? null;
    this.start = _start;
    let _end = options.end ?? null;
    this.end = _end;
    let _color = options.color ?? null;
    this.color = _color;

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
    if (!(this.simulatePressure === other.simulatePressure)) {
      return false;
    }
    if (!(this.easing === other.easing)) {
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
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
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
    h = (h * 31 + hashBool(this.simulatePressure)) & 0xffffffff;
    h = (h * 31 + this.easing) & 0xffffffff;
    if (this.start !== null) {
      h = (h * 31 + this.start.hash()) & 0xffffffff;
    }
    if (this.end !== null) {
      h = (h * 31 + this.end.hash()) & 0xffffffff;
    }
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
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
      this._value = Stroke.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Stroke): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270100;
    objectValue["30"] = object.type;
    objectValue["50"] = object.size;
    objectValue["51"] = object.thinning;
    objectValue["52"] = object.smoothing;
    objectValue["53"] = object.streamline;
    objectValue["54"] = object.simulatePressure;
    objectValue["55"] = object.easing;
    if (object.start != null) {
      objectValue["60"] = object.start.toValue();
    }
    if (object.end != null) {
      objectValue["61"] = object.end.toValue();
    }
    if (object.color != null) {
      objectValue["70"] = object.color.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    const startValue = objectValue["60"];
    const unpackedStart =
      startValue != undefined
        ? StrokeCap.fromValue(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectValue["61"];
    const unpackedEnd =
      endValue != undefined
        ? StrokeCap.fromValue(endValue, _session, _supergraph, _graph, _connection)
        : null;
    const colorValue = objectValue["70"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Stroke({
      type: Number(objectValue["30"]),
      size: Number(objectValue["50"]),
      thinning: objectValue["51"],
      smoothing: objectValue["52"],
      streamline: objectValue["53"],
      simulatePressure: objectValue["54"],
      easing: Number(objectValue["55"]),
      start: unpackedStart,
      end: unpackedEnd,
      color: unpackedColor,
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
    const objectProto: Partial<StrokeProto> = { metatype: 270100 };
    objectProto.type = Number(object.type) as StrokeTypeProto;
    objectProto.size = object.size;
    objectProto.thinning = object.thinning;
    objectProto.smoothing = object.smoothing;
    objectProto.streamline = object.streamline;
    objectProto.simulatePressure = object.simulatePressure;
    objectProto.easing = Number(object.easing) as EasingProto;
    if (object.start != null) {
      objectProto.start = object.start.toProto();
    }
    if (object.end != null) {
      objectProto.end = object.end.toProto();
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
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
    return new Stroke({
      type: Number(objectProto.type) as StrokeType,
      size: Number(objectProto.size),
      thinning: objectProto.thinning,
      smoothing: objectProto.smoothing,
      streamline: objectProto.streamline,
      simulatePressure: objectProto.simulatePressure,
      easing: Number(objectProto.easing) as Easing,
      start:
        objectProto.start != undefined
          ? StrokeCap.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection)
          : null,
      end:
        objectProto.end != undefined
          ? StrokeCap.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection)
          : null,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
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
/* ==== DESTACK_GENERATED_END:STRUCT:270100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:270101 ==== */
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = StrokeCap.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokeCap): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270101;
    objectValue["50"] = object.cap;
    objectValue["51"] = object.taper;
    objectValue["52"] = object.easing;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return new StrokeCap({
      cap: objectValue["50"],
      taper: objectValue["51"],
      easing: Number(objectValue["52"]),
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
    const objectProto: Partial<StrokeCapProto> = { metatype: 270101 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:270101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:270103 ==== */
/**
 * A computed point in a stroke.
 */
export class StrokePoint extends StructFrozen {
  static metatype: StructType = StructType.STROKE_POINT;
  static __isFrozen__: boolean = true;

  /**
   * The adjusted point position.
   */
  readonly point: Vector2;

  /**
   * The original input point.
   */
  readonly originalPoint: Vector2;

  /**
   * The pressure value at this point (0-1).
   */
  readonly pressure: number;

  /**
   * The normalized direction vector from previous point.
   */
  readonly direction: Vector2;

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
    point: Vector2;
    originalPoint: Vector2;
    pressure: number;
    direction: Vector2;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = StrokePoint.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokePoint): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270103;
    objectValue["50"] = object.point.toValue();
    objectValue["51"] = object.originalPoint.toValue();
    objectValue["52"] = object.pressure;
    objectValue["53"] = object.direction.toValue();
    objectValue["54"] = object.distance;
    objectValue["55"] = object.runningLength;
    objectValue["56"] = object.radius;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    return new StrokePoint({
      point: Vector2.fromValue(objectValue["50"], _session, _supergraph, _graph, _connection),
      originalPoint: Vector2.fromValue(
        objectValue["51"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectValue["52"],
      direction: Vector2.fromValue(objectValue["53"], _session, _supergraph, _graph, _connection),
      distance: objectValue["54"],
      runningLength: objectValue["55"],
      radius: objectValue["56"],
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
    const objectProto: Partial<StrokePointProto> = { metatype: 270103 };
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
    return new StrokePoint({
      point: Vector2.fromProto(objectProto.point!, _session, _supergraph, _graph, _connection),
      originalPoint: Vector2.fromProto(
        objectProto.originalPoint!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      direction: Vector2.fromProto(
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
/* ==== DESTACK_GENERATED_END:STRUCT:270103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:270102 ==== */
/**
 * A stroke path.
 */
export class StrokePath extends StructFrozen {
  static metatype: StructType = StructType.STROKE_PATH;
  static __isFrozen__: boolean = true;

  /**
   * StrokePath.points
   */
  readonly points: Array<StrokePoint>;

  constructor(options: {
    points?: Array<StrokePoint>;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = StrokePath.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StrokePath): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270102;
    if (object.points.length > 0) {
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.toValue());
      }
      objectValue["100"] = packedPoints;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    const unpackedPoints: any[] = [];
    if (objectValue["100"] != undefined) {
      for (const item of objectValue["100"]) {
        unpackedPoints.push(
          StrokePoint.fromValue(item, _session, _supergraph, _graph, _connection),
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
    objectValue: { [key: string]: any },
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
    const objectProto: Partial<StrokePathProto> = { metatype: 270102 };
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
    const unpackedPoints: any[] = [];
    if (objectProto.points) {
      for (const item of objectProto.points) {
        unpackedPoints.push(
          StrokePoint.fromProto(item!, _session, _supergraph, _graph, _connection),
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
/* ==== DESTACK_GENERATED_END:STRUCT:270102 ==== */

/* ==== DESTACK_GENERATED_START:NODE:270208 ==== */
/**
 * A StrokeStyle.
 */
export class StrokeStyle extends Style {
  static metatype: NodeType = NodeType.STROKE_STYLE;

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
   * StrokeStyle.type
   */
  type: StrokeType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * The stroke size/width.
   */
  size: number;

  /**
   * The amount of pressure-based thinning (0-1).
   */
  thinning: number;

  /**
   * The amount of path smoothing (0-1).
   */
  smoothing: number;

  /**
   * The amount of streamlining applied to path (0-1).
   */
  streamline: number;

  /**
   * The easing function for pressure mapping.
   */
  easing: Easing;

  /**
   * The start cap configuration.
   */
  start: StrokeCap | null;

  /**
   * The end cap configuration.
   */
  end: StrokeCap | null;

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
      throw new Error(`StrokeStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`StrokeStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`StrokeStyle.name is required`);
    }
    this.name = _name;
    let _size = options.size;
    if (_size === null) {
      throw new Error(`StrokeStyle.size is required`);
    }
    this.size = _size;
    let _thinning = options.thinning;
    if (_thinning === null) {
      throw new Error(`StrokeStyle.thinning is required`);
    }
    this.thinning = _thinning;
    let _smoothing = options.smoothing;
    if (_smoothing === null) {
      throw new Error(`StrokeStyle.smoothing is required`);
    }
    this.smoothing = _smoothing;
    let _streamline = options.streamline;
    if (_streamline === null) {
      throw new Error(`StrokeStyle.streamline is required`);
    }
    this.streamline = _streamline;
    let _easing = options.easing;
    if (_easing === null) {
      throw new Error(`StrokeStyle.easing is required`);
    }
    this.easing = _easing;
    let _start = options.start ?? null;
    this.start = _start;
    let _end = options.end ?? null;
    this.end = _end;

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
    h = (h * 31 + hashInt(this.size)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.thinning)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.smoothing)) & 0xffffffff;
    h = (h * 31 + hashFloat(this.streamline)) & 0xffffffff;
    h = (h * 31 + this.easing) & 0xffffffff;
    if (this.start !== null) {
      h = (h * 31 + this.start.hash()) & 0xffffffff;
    }
    if (this.end !== null) {
      h = (h * 31 + this.end.hash()) & 0xffffffff;
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
      nodeType: NodeType.STROKE_STYLE,
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
    propertyReprs.push(`name=${this.name}`);
    return `<StrokeStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return StrokeStyle.__packValue__(this);
  }

  static __packValue__(object: StrokeStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270208;
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
    objectValue["50"] = object.size;
    objectValue["51"] = object.thinning;
    objectValue["52"] = object.smoothing;
    objectValue["53"] = object.streamline;
    objectValue["55"] = object.easing;
    if (object.start != null) {
      objectValue["60"] = object.start.toValue();
    }
    if (object.end != null) {
      objectValue["61"] = object.end.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    const startValue = objectValue["60"];
    const unpackedStart =
      startValue != undefined
        ? StrokeCap.fromValue(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectValue["61"];
    const unpackedEnd =
      endValue != undefined
        ? StrokeCap.fromValue(endValue, _session, _supergraph, _graph, _connection)
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
    return new StrokeStyle({
      type: Number(objectValue["30"]),
      size: Number(objectValue["50"]),
      thinning: objectValue["51"],
      smoothing: objectValue["52"],
      streamline: objectValue["53"],
      easing: Number(objectValue["55"]),
      start: unpackedStart,
      end: unpackedEnd,
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
  ): StrokeStyle {
    return StrokeStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeStyleProto {
    return StrokeStyle.__packProto__(this);
  }

  static __packProto__(object: StrokeStyle): StrokeStyleProto {
    const objectProto: Partial<StrokeStyleProto> = { metatype: 270208 };
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
    objectProto.type = Number(object.type) as StrokeTypeProto;
    objectProto.name = object.name;
    objectProto.size = object.size;
    objectProto.thinning = object.thinning;
    objectProto.smoothing = object.smoothing;
    objectProto.streamline = object.streamline;
    objectProto.easing = Number(object.easing) as EasingProto;
    if (object.start != null) {
      objectProto.start = object.start.toProto();
    }
    if (object.end != null) {
      objectProto.end = object.end.toProto();
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
    return new StrokeStyle({
      type: Number(objectProto.type) as StrokeType,
      size: Number(objectProto.size),
      thinning: objectProto.thinning,
      smoothing: objectProto.smoothing,
      streamline: objectProto.streamline,
      easing: Number(objectProto.easing) as Easing,
      start:
        objectProto.start != undefined
          ? StrokeCap.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection)
          : null,
      end:
        objectProto.end != undefined
          ? StrokeCap.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection)
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
/* ==== DESTACK_GENERATED_END:NODE:270208 ==== */
