import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Easing } from "@destack/language/animation";
import type {
  Branch,
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
  ACTIVE_BRANCH,
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
import type { Vector2 } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Color } from "@destack/language/style/color";
import { Style } from "@destack/language/style/style";
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

/* ==== DESTACK_GENERATED_START:ENUM:2100213 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100213 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2101100 ==== */
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
    _cson?: any | null;
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
    this._cson = options._cson ?? null;
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
    if (this._hash != null) {
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
    if (this.color != null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.start != null) {
      h = (h * 31 + this.start.hash()) & 0xffffffff;
    }
    if (this.end != null) {
      h = (h * 31 + this.end.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Stroke.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Stroke): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2101100;
    objectCson["100"] = object.type;
    objectCson["101"] = object.size;
    objectCson["102"] = object.thinning;
    objectCson["103"] = object.smoothing;
    objectCson["104"] = object.streamline;
    objectCson["105"] = object.easing;
    if (object.color != null) {
      objectCson["106"] = object.color.toCson();
    }
    if (object.start != null) {
      objectCson["110"] = object.start.toCson();
    }
    if (object.end != null) {
      objectCson["111"] = object.end.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    const colorValue = objectCson["106"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromCson(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const startValue = objectCson["110"];
    const unpackedStart =
      startValue != undefined
        ? _StrokeCap.fromCson(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectCson["111"];
    const unpackedEnd =
      endValue != undefined
        ? _StrokeCap.fromCson(endValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Stroke({
      type: Number(objectCson["100"]),
      size: Number(objectCson["101"]),
      thinning: objectCson["102"],
      smoothing: objectCson["103"],
      streamline: objectCson["104"],
      easing: Number(objectCson["105"]),
      color: unpackedColor,
      start: unpackedStart,
      end: unpackedEnd,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Stroke {
    return Stroke.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Stroke.__packProto__(this);
    }
    return this._proto as StrokeProto;
  }

  static __packProto__(object: Stroke): StrokeProto {
    const objectProto: Partial<StrokeProto> = { metatype: 2101100 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2101100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2101101 ==== */
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
    _cson?: any | null;
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
    this._cson = options._cson ?? null;
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
    if (this._hash != null) {
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

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = StrokeCap.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: StrokeCap): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2101101;
    objectCson["101"] = object.cap;
    objectCson["102"] = object.taper;
    objectCson["103"] = object.easing;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return new StrokeCap({
      cap: objectCson["101"],
      taper: objectCson["102"],
      easing: Number(objectCson["103"]),
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeCap {
    return StrokeCap.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeCapProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokeCap.__packProto__(this);
    }
    return this._proto as StrokeCapProto;
  }

  static __packProto__(object: StrokeCap): StrokeCapProto {
    const objectProto: Partial<StrokeCapProto> = { metatype: 2101101 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2101101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2101103 ==== */
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
    _cson?: any | null;
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
    this._cson = options._cson ?? null;
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
    if (this._hash != null) {
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

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = StrokePoint.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: StrokePoint): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2101103;
    objectCson["101"] = object.point.toCson();
    objectCson["102"] = object.originalPoint.toCson();
    objectCson["103"] = object.pressure;
    objectCson["104"] = object.direction.toCson();
    objectCson["105"] = object.distance;
    objectCson["106"] = object.runningLength;
    objectCson["107"] = object.radius;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new StrokePoint({
      point: _Vector2.fromCson(objectCson["101"], _session, _supergraph, _graph, _connection),
      originalPoint: _Vector2.fromCson(
        objectCson["102"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectCson["103"],
      direction: _Vector2.fromCson(objectCson["104"], _session, _supergraph, _graph, _connection),
      distance: objectCson["105"],
      runningLength: objectCson["106"],
      radius: objectCson["107"],
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePoint {
    return StrokePoint.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokePointProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokePoint.__packProto__(this);
    }
    return this._proto as StrokePointProto;
  }

  static __packProto__(object: StrokePoint): StrokePointProto {
    const objectProto: Partial<StrokePointProto> = { metatype: 2101103 };
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
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new StrokePoint({
      point: _Vector2.fromProto(objectProto.point!, _session, _supergraph, _graph, _connection),
      originalPoint: _Vector2.fromProto(
        objectProto.originalPoint!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      pressure: objectProto.pressure,
      direction: _Vector2.fromProto(
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
/* ==== DESTACK_GENERATED_END:STRUCT:2101103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2101102 ==== */
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
    _cson?: any | null;
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
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.points.length != other.points.length) {
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
    if (this._hash != null) {
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

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = StrokePath.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: StrokePath): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2101102;
    if (object.points.length > 0) {
      const packedPoints: any[] = [];
      for (const item of object.points) {
        packedPoints.push(item.toCson());
      }
      objectCson["101"] = packedPoints;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    const _StrokePoint = STRUCT_CLASS_BY_TYPE[StructType.STROKE_POINT] as typeof StrokePoint;
    const unpackedPoints: any[] = [];
    if (objectCson["101"] != undefined) {
      for (const item of objectCson["101"]) {
        unpackedPoints.push(
          _StrokePoint.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new StrokePath({
      points: unpackedPoints,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokePath {
    return StrokePath.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokePathProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StrokePath.__packProto__(this);
    }
    return this._proto as StrokePathProto;
  }

  static __packProto__(object: StrokePath): StrokePathProto {
    const objectProto: Partial<StrokePathProto> = { metatype: 2101102 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2101102 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2101100 ==== */
/**
 * A StrokeStyle.
 */
export class StrokeStyle extends Style {
  static metatype: NodeType = NodeType.STROKE_STYLE;

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
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
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
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): StrokeStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as StrokeStyle | null;
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
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

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
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: StrokeStyle | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type: StrokeType;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for StrokeStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`StrokeStyle.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`StrokeStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for StrokeStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`StrokeStyle.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for StrokeStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`StrokeStyle.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.metatype != StructType.NODE_REFERENCE) {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "StrokeStyle";
    }
    if (_name === null) {
      throw new Error(`StrokeStyle.name is required`);
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
      throw new Error(`StrokeStyle.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`StrokeStyle.type is required`);
    }
    this._type = _type;
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
          `StrokeStyle.createdAt and StrokeStyle.updatedAt are required for existing Nodes`,
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    h = (h * 31 + hashInt(this._size)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._thinning)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._smoothing)) & 0xffffffff;
    h = (h * 31 + hashFloat(this._streamline)) & 0xffffffff;
    h = (h * 31 + this._easing) & 0xffffffff;
    if (this._start != null) {
      h = (h * 31 + this._start.hash()) & 0xffffffff;
    }
    if (this._end != null) {
      h = (h * 31 + this._end.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
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
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
      type: NodeType.STROKE_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<StrokeStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return StrokeStyle.__packCson__(this);
  }

  static __packCson__(object: StrokeStyle): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2101100;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["30"] = packedCustomValues;
    }
    objectCson["50"] = object._name;
    if (object._scriptPtr != null) {
      objectCson["80"] = object._scriptPtr.toCson();
    }
    objectCson["90"] = object.isExtensible;
    objectCson["100"] = object._type;
    objectCson["200"] = object._size;
    objectCson["201"] = object._thinning;
    objectCson["202"] = object._smoothing;
    objectCson["203"] = object._streamline;
    objectCson["204"] = object._easing;
    if (object._start != null) {
      objectCson["205"] = object._start.toCson();
    }
    if (object._end != null) {
      objectCson["206"] = object._end.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    const startValue = objectCson["205"];
    const unpackedStart =
      startValue != undefined
        ? _StrokeCap.fromCson(startValue, _session, _supergraph, _graph, _connection)
        : null;
    const endValue = objectCson["206"];
    const unpackedEnd =
      endValue != undefined
        ? _StrokeCap.fromCson(endValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const scriptPtrValue = objectCson["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["30"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new StrokeStyle({
      type: Number(objectCson["100"]),
      size: Number(objectCson["200"]),
      thinning: objectCson["201"],
      smoothing: objectCson["202"],
      streamline: objectCson["203"],
      easing: Number(objectCson["204"]),
      start: unpackedStart,
      end: unpackedEnd,
      isExtensible: objectCson["90"],
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectCson["50"],
      script: unpackedScriptPtr,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StrokeStyle {
    return StrokeStyle.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StrokeStyleProto {
    return StrokeStyle.__packProto__(this);
  }

  static __packProto__(object: StrokeStyle): StrokeStyleProto {
    const objectProto: Partial<StrokeStyleProto> = { metatype: 2101100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
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
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as StrokeTypeProto;
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StrokeCap = STRUCT_CLASS_BY_TYPE[StructType.STROKE_CAP] as typeof StrokeCap;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
      isExtensible: objectProto.isExtensible,
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
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
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
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:2101100 ==== */
