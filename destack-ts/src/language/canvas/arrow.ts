import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Shape } from "@destack/language/canvas/shape";
import type {
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Graph,
  Grid,
  GridSpan,
  Insets,
  IsActor,
  IsExtensible,
  NodeClass,
  NodeReference,
  Position,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
  Vector2f,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Align,
  Direction,
  Distribute,
  Entity,
  EnumType,
  Event,
  Layout,
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
import type { Layer } from "@destack/language/scene";
import type { Border, Fill, Shadow, Stroke } from "@destack/language/style";
import type { Space } from "@destack/language/universe";
import type { ContainerView } from "@destack/language/view";
import {
  AlignProto,
  ArrowHeadTypeProto,
  ArrowProto,
  ArrowShapeProto,
  DirectionProto,
  DistributeProto,
  LayoutProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1900200 ==== */
/**
 * ArrowHeadType
 */
export enum ArrowHeadType {
  ARROW = 1,
  TRIANGLE = 2,
  DOT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ARROW_HEAD_TYPE, ArrowHeadType);
/* ==== DESTACK_GENERATED_END:ENUM:1900200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:1900200 ==== */
/**
 * An Arrow is a shape that represents an arrow.
 */
export class Arrow extends StructFrozen {
  static metatype: StructType = StructType.ARROW;
  static __isFrozen__: boolean = true;

  /**
   * Arrow.startType
   */
  readonly startType: ArrowHeadType;

  /**
   * Arrow.start
   */
  readonly start: Vector2f;

  /**
   * Arrow.endType
   */
  readonly endType: ArrowHeadType;

  /**
   * Arrow.end
   */
  readonly end: Vector2f;

  constructor(options: {
    startType: ArrowHeadType;
    start: Vector2f;
    endType: ArrowHeadType;
    end: Vector2f;
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
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`Arrow.startType is required`);
    }
    this.startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`Arrow.start is required`);
    }
    this.start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`Arrow.endType is required`);
    }
    this.endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`Arrow.end is required`);
    }
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
    if (!(this.startType === other.startType)) {
      return false;
    }
    if (!this.start.equals(other.start)) {
      return false;
    }
    if (!(this.endType === other.endType)) {
      return false;
    }
    if (!this.end.equals(other.end)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Arrow>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.startType) & 0xffffffff;
    h = (h * 31 + this.start.hash()) & 0xffffffff;
    h = (h * 31 + this.endType) & 0xffffffff;
    h = (h * 31 + this.end.hash()) & 0xffffffff;

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
      this._value = Arrow.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Arrow): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1900200;
    objectValue["200"] = object.startType;
    objectValue["201"] = object.start.toValue();
    objectValue["210"] = object.endType;
    objectValue["211"] = object.end.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow {
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new Arrow({
      startType: Number(objectValue["200"]),
      start: _Vector2f.fromValue(objectValue["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectValue["210"]),
      end: _Vector2f.fromValue(objectValue["211"], _session, _supergraph, _graph, _connection),
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
  ): Arrow {
    return Arrow.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ArrowProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Arrow.__packProto__(this);
    }
    return this._proto as ArrowProto;
  }

  static __packProto__(object: Arrow): ArrowProto {
    const objectProto: Partial<ArrowProto> = { metatype: 1900200 };
    objectProto.startType = Number(object.startType) as ArrowHeadTypeProto;
    objectProto.start = object.start.toProto();
    objectProto.endType = Number(object.endType) as ArrowHeadTypeProto;
    objectProto.end = object.end.toProto();
    return objectProto as ArrowProto;
  }

  static __unpackProto__(
    objectProto: ArrowProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow {
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    return new Arrow({
      startType: Number(objectProto.startType) as ArrowHeadType,
      start: _Vector2f.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2f.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ArrowProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow {
    return Arrow.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Arrow {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ArrowProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ARROW, Arrow);
/* ==== DESTACK_GENERATED_END:STRUCT:1900200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1900200 ==== */
/**
 * An ArrowShape is a shape that represents an arrow.
 */
export class ArrowShape extends Shape {
  static metatype: NodeType = NodeType.ARROW_SHAPE;

  /**
   * View.parent
   */
  get parent(): Layer | ContainerView | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Layer | ContainerView | null;
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
   * The definition this CustomEntity is an instance of.
   */
  get definition(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

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
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): ArrowShape | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as ArrowShape | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

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
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

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
   * IsDeletable.deletedAt
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
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

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
   * View.name
   */
  /**
   * View.name
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
   * View.position
   */
  /**
   * View.position
   */
  get position(): Position | null {
    return this._position;
  }
  set position(value: Position | null) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: Position | null;

  /**
   * View.width
   */
  /**
   * View.width
   */
  get width(): Dimension | null {
    return this._width;
  }
  set width(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: Dimension | null;

  /**
   * View.height
   */
  /**
   * View.height
   */
  get height(): Dimension | null {
    return this._height;
  }
  set height(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: Dimension | null;

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  get minWidth(): Dimension | null {
    return this._minWidth;
  }
  set minWidth(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_width"];
    this._session.updateSetProperty(this, prop, value);
    this._minWidth = value;
  }
  _minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  get minHeight(): Dimension | null {
    return this._minHeight;
  }
  set minHeight(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_height"];
    this._session.updateSetProperty(this, prop, value);
    this._minHeight = value;
  }
  _minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  get maxWidth(): Dimension | null {
    return this._maxWidth;
  }
  set maxWidth(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_width"];
    this._session.updateSetProperty(this, prop, value);
    this._maxWidth = value;
  }
  _maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  get maxHeight(): Dimension | null {
    return this._maxHeight;
  }
  set maxHeight(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_height"];
    this._session.updateSetProperty(this, prop, value);
    this._maxHeight = value;
  }
  _maxHeight: Dimension | null;

  /**
   * ContainerView.layout
   */
  /**
   * ContainerView.layout
   */
  get layout(): Layout | null {
    return this._layout;
  }
  set layout(value: Layout | null) {
    const prop = (this.constructor as NodeClass).__properties__["layout"];
    this._session.updateSetProperty(this, prop, value);
    this._layout = value;
  }
  _layout: Layout | null;

  /**
   * ContainerView.direction
   */
  /**
   * ContainerView.direction
   */
  get direction(): Direction | null {
    return this._direction;
  }
  set direction(value: Direction | null) {
    const prop = (this.constructor as NodeClass).__properties__["direction"];
    this._session.updateSetProperty(this, prop, value);
    this._direction = value;
  }
  _direction: Direction | null;

  /**
   * ContainerView.distribute
   */
  /**
   * ContainerView.distribute
   */
  get distribute(): Distribute | null {
    return this._distribute;
  }
  set distribute(value: Distribute | null) {
    const prop = (this.constructor as NodeClass).__properties__["distribute"];
    this._session.updateSetProperty(this, prop, value);
    this._distribute = value;
  }
  _distribute: Distribute | null;

  /**
   * ContainerView.align
   */
  /**
   * ContainerView.align
   */
  get align(): Align | null {
    return this._align;
  }
  set align(value: Align | null) {
    const prop = (this.constructor as NodeClass).__properties__["align"];
    this._session.updateSetProperty(this, prop, value);
    this._align = value;
  }
  _align: Align | null;

  /**
   * ContainerView.gap
   */
  /**
   * ContainerView.gap
   */
  get gap(): Axis2 | null {
    return this._gap;
  }
  set gap(value: Axis2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["gap"];
    this._session.updateSetProperty(this, prop, value);
    this._gap = value;
  }
  _gap: Axis2 | null;

  /**
   * ContainerView.padding
   */
  /**
   * ContainerView.padding
   */
  get padding(): Insets | null {
    return this._padding;
  }
  set padding(value: Insets | null) {
    const prop = (this.constructor as NodeClass).__properties__["padding"];
    this._session.updateSetProperty(this, prop, value);
    this._padding = value;
  }
  _padding: Insets | null;

  /**
   * ContainerView.grid
   */
  /**
   * ContainerView.grid
   */
  get grid(): Grid | null {
    return this._grid;
  }
  set grid(value: Grid | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid"];
    this._session.updateSetProperty(this, prop, value);
    this._grid = value;
  }
  _grid: Grid | null;

  /**
   * ContainerView.gridSpan
   */
  /**
   * ContainerView.gridSpan
   */
  get gridSpan(): GridSpan | null {
    return this._gridSpan;
  }
  set gridSpan(value: GridSpan | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid_span"];
    this._session.updateSetProperty(this, prop, value);
    this._gridSpan = value;
  }
  _gridSpan: GridSpan | null;

  /**
   * ContainerView.aspectRatio
   */
  /**
   * ContainerView.aspectRatio
   */
  get aspectRatio(): number | null {
    return this._aspectRatio;
  }
  set aspectRatio(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["aspect_ratio"];
    this._session.updateSetProperty(this, prop, value);
    this._aspectRatio = value;
  }
  _aspectRatio: number | null;

  /**
   * ContainerView.isWrap
   */
  /**
   * ContainerView.isWrap
   */
  get isWrap(): boolean | null {
    return this._isWrap;
  }
  set isWrap(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_wrap"];
    this._session.updateSetProperty(this, prop, value);
    this._isWrap = value;
  }
  _isWrap: boolean | null;

  /**
   * ContainerView.isVisible
   */
  /**
   * ContainerView.isVisible
   */
  get isVisible(): boolean | null {
    return this._isVisible;
  }
  set isVisible(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_visible"];
    this._session.updateSetProperty(this, prop, value);
    this._isVisible = value;
  }
  _isVisible: boolean | null;

  /**
   * ContainerView.opacity
   */
  /**
   * ContainerView.opacity
   */
  get opacity(): number | null {
    return this._opacity;
  }
  set opacity(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["opacity"];
    this._session.updateSetProperty(this, prop, value);
    this._opacity = value;
  }
  _opacity: number | null;

  /**
   * ContainerView.fill
   */
  /**
   * ContainerView.fill
   */
  get fill(): Fill | null {
    return this._fill;
  }
  set fill(value: Fill | null) {
    const prop = (this.constructor as NodeClass).__properties__["fill"];
    this._session.updateSetProperty(this, prop, value);
    this._fill = value;
  }
  _fill: Fill | null;

  /**
   * ContainerView.rotation
   */
  /**
   * ContainerView.rotation
   */
  get rotation(): Axis3 | null {
    return this._rotation;
  }
  set rotation(value: Axis3 | null) {
    const prop = (this.constructor as NodeClass).__properties__["rotation"];
    this._session.updateSetProperty(this, prop, value);
    this._rotation = value;
  }
  _rotation: Axis3 | null;

  /**
   * ContainerView.skew
   */
  /**
   * ContainerView.skew
   */
  get skew(): Vector2f | null {
    return this._skew;
  }
  set skew(value: Vector2f | null) {
    const prop = (this.constructor as NodeClass).__properties__["skew"];
    this._session.updateSetProperty(this, prop, value);
    this._skew = value;
  }
  _skew: Vector2f | null;

  /**
   * ContainerView.scale
   */
  /**
   * ContainerView.scale
   */
  get scale(): number | null {
    return this._scale;
  }
  set scale(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["scale"];
    this._session.updateSetProperty(this, prop, value);
    this._scale = value;
  }
  _scale: number | null;

  /**
   * ContainerView.shadow
   */
  /**
   * ContainerView.shadow
   */
  get shadow(): Shadow | null {
    return this._shadow;
  }
  set shadow(value: Shadow | null) {
    const prop = (this.constructor as NodeClass).__properties__["shadow"];
    this._session.updateSetProperty(this, prop, value);
    this._shadow = value;
  }
  _shadow: Shadow | null;

  /**
   * ContainerView.border
   */
  /**
   * ContainerView.border
   */
  get border(): Border | null {
    return this._border;
  }
  set border(value: Border | null) {
    const prop = (this.constructor as NodeClass).__properties__["border"];
    this._session.updateSetProperty(this, prop, value);
    this._border = value;
  }
  _border: Border | null;

  /**
   * ContainerView.radius
   */
  /**
   * ContainerView.radius
   */
  get radius(): Corners | null {
    return this._radius;
  }
  set radius(value: Corners | null) {
    const prop = (this.constructor as NodeClass).__properties__["radius"];
    this._session.updateSetProperty(this, prop, value);
    this._radius = value;
  }
  _radius: Corners | null;

  /**
   * Shape.stroke
   */
  /**
   * Shape.stroke
   */
  get stroke(): Stroke | null {
    return this._stroke;
  }
  set stroke(value: Stroke | null) {
    const prop = (this.constructor as NodeClass).__properties__["stroke"];
    this._session.updateSetProperty(this, prop, value);
    this._stroke = value;
  }
  _stroke: Stroke | null;

  /**
   * ArrowShape.startType
   */
  /**
   * ArrowShape.startType
   */
  get startType(): ArrowHeadType {
    return this._startType;
  }
  set startType(value: ArrowHeadType) {
    const prop = (this.constructor as NodeClass).__properties__["start_type"];
    this._session.updateSetProperty(this, prop, value);
    this._startType = value;
  }
  _startType: ArrowHeadType;

  /**
   * ArrowShape.start
   */
  /**
   * ArrowShape.start
   */
  get start(): Vector2f {
    return this._start;
  }
  set start(value: Vector2f) {
    const prop = (this.constructor as NodeClass).__properties__["start"];
    this._session.updateSetProperty(this, prop, value);
    this._start = value;
  }
  _start: Vector2f;

  /**
   * ArrowShape.endType
   */
  /**
   * ArrowShape.endType
   */
  get endType(): ArrowHeadType {
    return this._endType;
  }
  set endType(value: ArrowHeadType) {
    const prop = (this.constructor as NodeClass).__properties__["end_type"];
    this._session.updateSetProperty(this, prop, value);
    this._endType = value;
  }
  _endType: ArrowHeadType;

  /**
   * ArrowShape.end
   */
  /**
   * ArrowShape.end
   */
  get end(): Vector2f {
    return this._end;
  }
  set end(value: Vector2f) {
    const prop = (this.constructor as NodeClass).__properties__["end"];
    this._session.updateSetProperty(this, prop, value);
    this._end = value;
  }
  _end: Vector2f;

  constructor(options: {
    id?: string;
    parent?: Layer | ContainerView | NodeReference | null;
    space?: Space | NodeReference;
    definition?: (Entity & IsExtensible) | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: ArrowShape | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    orderKey?: string;
    source?: Script | NodeReference | null;
    key?: string | null;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    name: string;
    position?: Position | null;
    width?: Dimension | null;
    height?: Dimension | null;
    minWidth?: Dimension | null;
    minHeight?: Dimension | null;
    maxWidth?: Dimension | null;
    maxHeight?: Dimension | null;
    layout?: Layout | null;
    direction?: Direction | null;
    distribute?: Distribute | null;
    align?: Align | null;
    gap?: Axis2 | null;
    padding?: Insets | null;
    grid?: Grid | null;
    gridSpan?: GridSpan | null;
    aspectRatio?: number | null;
    isWrap?: boolean | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    fill?: Fill | null;
    rotation?: Axis3 | null;
    skew?: Vector2f | null;
    scale?: number | null;
    shadow?: Shadow | null;
    border?: Border | null;
    radius?: Corners | null;
    stroke?: Stroke | null;
    startType: ArrowHeadType;
    start: Vector2f;
    endType: ArrowHeadType;
    end: Vector2f;
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
        throw new Error(`ArrowShape has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`ArrowShape has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ArrowShape.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`ArrowShape.materialization is required`);
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
      throw new Error(`ArrowShape.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _key = options.key ?? null;
    this._key = _key;
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
      throw new Error(`ArrowShape.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ArrowShape.name is required`);
    }
    this._name = _name;
    let _position = options.position ?? null;
    this._position = _position;
    let _width = options.width ?? null;
    this._width = _width;
    let _height = options.height ?? null;
    this._height = _height;
    let _minWidth = options.minWidth ?? null;
    this._minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this._minHeight = _minHeight;
    let _maxWidth = options.maxWidth ?? null;
    this._maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this._maxHeight = _maxHeight;
    let _layout = options.layout ?? null;
    this._layout = _layout;
    let _direction = options.direction ?? null;
    this._direction = _direction;
    let _distribute = options.distribute ?? null;
    this._distribute = _distribute;
    let _align = options.align ?? null;
    this._align = _align;
    let _gap = options.gap ?? null;
    this._gap = _gap;
    let _padding = options.padding ?? null;
    this._padding = _padding;
    let _grid = options.grid ?? null;
    this._grid = _grid;
    let _gridSpan = options.gridSpan ?? null;
    this._gridSpan = _gridSpan;
    let _aspectRatio = options.aspectRatio ?? null;
    this._aspectRatio = _aspectRatio;
    let _isWrap = options.isWrap ?? null;
    this._isWrap = _isWrap;
    let _isVisible = options.isVisible ?? null;
    this._isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this._opacity = _opacity;
    let _fill = options.fill ?? null;
    this._fill = _fill;
    let _rotation = options.rotation ?? null;
    this._rotation = _rotation;
    let _skew = options.skew ?? null;
    this._skew = _skew;
    let _scale = options.scale ?? null;
    this._scale = _scale;
    let _shadow = options.shadow ?? null;
    this._shadow = _shadow;
    let _border = options.border ?? null;
    this._border = _border;
    let _radius = options.radius ?? null;
    this._radius = _radius;
    let _stroke = options.stroke ?? null;
    this._stroke = _stroke;
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`ArrowShape.startType is required`);
    }
    this._startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`ArrowShape.start is required`);
    }
    this._start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`ArrowShape.endType is required`);
    }
    this._endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`ArrowShape.end is required`);
    }
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
          `ArrowShape.createdAt and ArrowShape.updatedAt are required for existing Nodes`,
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
    if (!(this._startType === other._startType)) {
      return false;
    }
    if (!this._start.equals(other._start)) {
      return false;
    }
    if (!(this._endType === other._endType)) {
      return false;
    }
    if (!this._end.equals(other._end)) {
      return false;
    }
    if (
      (this._stroke == null) !== (other._stroke == null) ||
      (this._stroke != null && !this._stroke.equals(other._stroke))
    ) {
      return false;
    }
    if (!(this._layout === other._layout)) {
      return false;
    }
    if (!(this._direction === other._direction)) {
      return false;
    }
    if (!(this._distribute === other._distribute)) {
      return false;
    }
    if (!(this._align === other._align)) {
      return false;
    }
    if (
      (this._gap == null) !== (other._gap == null) ||
      (this._gap != null && !this._gap.equals(other._gap))
    ) {
      return false;
    }
    if (
      (this._padding == null) !== (other._padding == null) ||
      (this._padding != null && !this._padding.equals(other._padding))
    ) {
      return false;
    }
    if (
      (this._grid == null) !== (other._grid == null) ||
      (this._grid != null && !this._grid.equals(other._grid))
    ) {
      return false;
    }
    if (
      (this._gridSpan == null) !== (other._gridSpan == null) ||
      (this._gridSpan != null && !this._gridSpan.equals(other._gridSpan))
    ) {
      return false;
    }
    if (
      (this._aspectRatio == null) !== (other._aspectRatio == null) ||
      (this._aspectRatio != null &&
        !(
          this._aspectRatio === other._aspectRatio ||
          Math.abs(this._aspectRatio - other._aspectRatio) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this._isWrap === other._isWrap)) {
      return false;
    }
    if (!(this._isVisible === other._isVisible)) {
      return false;
    }
    if (
      (this._opacity == null) !== (other._opacity == null) ||
      (this._opacity != null &&
        !(this._opacity === other._opacity || Math.abs(this._opacity - other._opacity) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._fill == null) !== (other._fill == null) ||
      (this._fill != null && !this._fill.equals(other._fill))
    ) {
      return false;
    }
    if (
      (this._rotation == null) !== (other._rotation == null) ||
      (this._rotation != null && !this._rotation.equals(other._rotation))
    ) {
      return false;
    }
    if (
      (this._skew == null) !== (other._skew == null) ||
      (this._skew != null && !this._skew.equals(other._skew))
    ) {
      return false;
    }
    if (
      (this._scale == null) !== (other._scale == null) ||
      (this._scale != null &&
        !(this._scale === other._scale || Math.abs(this._scale - other._scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._shadow == null) !== (other._shadow == null) ||
      (this._shadow != null && !this._shadow.equals(other._shadow))
    ) {
      return false;
    }
    if (
      (this._border == null) !== (other._border == null) ||
      (this._border != null && !this._border.equals(other._border))
    ) {
      return false;
    }
    if (
      (this._radius == null) !== (other._radius == null) ||
      (this._radius != null && !this._radius.equals(other._radius))
    ) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (
      (this._position == null) !== (other._position == null) ||
      (this._position != null && !this._position.equals(other._position))
    ) {
      return false;
    }
    if (
      (this._width == null) !== (other._width == null) ||
      (this._width != null && !this._width.equals(other._width))
    ) {
      return false;
    }
    if (
      (this._height == null) !== (other._height == null) ||
      (this._height != null && !this._height.equals(other._height))
    ) {
      return false;
    }
    if (
      (this._minWidth == null) !== (other._minWidth == null) ||
      (this._minWidth != null && !this._minWidth.equals(other._minWidth))
    ) {
      return false;
    }
    if (
      (this._minHeight == null) !== (other._minHeight == null) ||
      (this._minHeight != null && !this._minHeight.equals(other._minHeight))
    ) {
      return false;
    }
    if (
      (this._maxWidth == null) !== (other._maxWidth == null) ||
      (this._maxWidth != null && !this._maxWidth.equals(other._maxWidth))
    ) {
      return false;
    }
    if (
      (this._maxHeight == null) !== (other._maxHeight == null) ||
      (this._maxHeight != null && !this._maxHeight.equals(other._maxHeight))
    ) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
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
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
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
    h = (h * 31 + this._startType) & 0xffffffff;
    h = (h * 31 + this._start.hash()) & 0xffffffff;
    h = (h * 31 + this._endType) & 0xffffffff;
    h = (h * 31 + this._end.hash()) & 0xffffffff;
    if (this._stroke != null) {
      h = (h * 31 + this._stroke.hash()) & 0xffffffff;
    }
    if (this._layout != null) {
      h = (h * 31 + this._layout) & 0xffffffff;
    }
    if (this._direction != null) {
      h = (h * 31 + this._direction) & 0xffffffff;
    }
    if (this._distribute != null) {
      h = (h * 31 + this._distribute) & 0xffffffff;
    }
    if (this._align != null) {
      h = (h * 31 + this._align) & 0xffffffff;
    }
    if (this._gap != null) {
      h = (h * 31 + this._gap.hash()) & 0xffffffff;
    }
    if (this._padding != null) {
      h = (h * 31 + this._padding.hash()) & 0xffffffff;
    }
    if (this._grid != null) {
      h = (h * 31 + this._grid.hash()) & 0xffffffff;
    }
    if (this._gridSpan != null) {
      h = (h * 31 + this._gridSpan.hash()) & 0xffffffff;
    }
    if (this._aspectRatio != null) {
      h = (h * 31 + hashFloat(this._aspectRatio)) & 0xffffffff;
    }
    if (this._isWrap != null) {
      h = (h * 31 + hashBool(this._isWrap)) & 0xffffffff;
    }
    if (this._isVisible != null) {
      h = (h * 31 + hashBool(this._isVisible)) & 0xffffffff;
    }
    if (this._opacity != null) {
      h = (h * 31 + hashFloat(this._opacity)) & 0xffffffff;
    }
    if (this._fill != null) {
      h = (h * 31 + this._fill.hash()) & 0xffffffff;
    }
    if (this._rotation != null) {
      h = (h * 31 + this._rotation.hash()) & 0xffffffff;
    }
    if (this._skew != null) {
      h = (h * 31 + this._skew.hash()) & 0xffffffff;
    }
    if (this._scale != null) {
      h = (h * 31 + hashFloat(this._scale)) & 0xffffffff;
    }
    if (this._shadow != null) {
      h = (h * 31 + this._shadow.hash()) & 0xffffffff;
    }
    if (this._border != null) {
      h = (h * 31 + this._border.hash()) & 0xffffffff;
    }
    if (this._radius != null) {
      h = (h * 31 + this._radius.hash()) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._position != null) {
      h = (h * 31 + this._position.hash()) & 0xffffffff;
    }
    if (this._width != null) {
      h = (h * 31 + this._width.hash()) & 0xffffffff;
    }
    if (this._height != null) {
      h = (h * 31 + this._height.hash()) & 0xffffffff;
    }
    if (this._minWidth != null) {
      h = (h * 31 + this._minWidth.hash()) & 0xffffffff;
    }
    if (this._minHeight != null) {
      h = (h * 31 + this._minHeight.hash()) & 0xffffffff;
    }
    if (this._maxWidth != null) {
      h = (h * 31 + this._maxWidth.hash()) & 0xffffffff;
    }
    if (this._maxHeight != null) {
      h = (h * 31 + this._maxHeight.hash()) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr != null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.ARROW_SHAPE,
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
    if (this.stroke != null) {
      propertyReprs.push(`stroke=${this.stroke.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<ArrowShape "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return ArrowShape.__packValue__(this);
  }

  static __packValue__(object: ArrowShape): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1900200;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
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
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["27"] = object.orderKey;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    if (object._key != null) {
      objectValue["70"] = object._key;
    }
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["101"] = object._name;
    if (object._position != null) {
      objectValue["110"] = object._position.toValue();
    }
    if (object._width != null) {
      objectValue["111"] = object._width.toValue();
    }
    if (object._height != null) {
      objectValue["112"] = object._height.toValue();
    }
    if (object._minWidth != null) {
      objectValue["113"] = object._minWidth.toValue();
    }
    if (object._minHeight != null) {
      objectValue["114"] = object._minHeight.toValue();
    }
    if (object._maxWidth != null) {
      objectValue["115"] = object._maxWidth.toValue();
    }
    if (object._maxHeight != null) {
      objectValue["116"] = object._maxHeight.toValue();
    }
    if (object._layout != null) {
      objectValue["120"] = object._layout;
    }
    if (object._direction != null) {
      objectValue["121"] = object._direction;
    }
    if (object._distribute != null) {
      objectValue["122"] = object._distribute;
    }
    if (object._align != null) {
      objectValue["123"] = object._align;
    }
    if (object._gap != null) {
      objectValue["124"] = object._gap.toValue();
    }
    if (object._padding != null) {
      objectValue["125"] = object._padding.toValue();
    }
    if (object._grid != null) {
      objectValue["126"] = object._grid.toValue();
    }
    if (object._gridSpan != null) {
      objectValue["127"] = object._gridSpan.toValue();
    }
    if (object._aspectRatio != null) {
      objectValue["128"] = object._aspectRatio;
    }
    if (object._isWrap != null) {
      objectValue["129"] = object._isWrap;
    }
    if (object._isVisible != null) {
      objectValue["140"] = object._isVisible;
    }
    if (object._opacity != null) {
      objectValue["141"] = object._opacity;
    }
    if (object._fill != null) {
      objectValue["142"] = object._fill.toValue();
    }
    if (object._rotation != null) {
      objectValue["143"] = object._rotation.toValue();
    }
    if (object._skew != null) {
      objectValue["144"] = object._skew.toValue();
    }
    if (object._scale != null) {
      objectValue["145"] = object._scale;
    }
    if (object._shadow != null) {
      objectValue["146"] = object._shadow.toValue();
    }
    if (object._border != null) {
      objectValue["147"] = object._border.toValue();
    }
    if (object._radius != null) {
      objectValue["148"] = object._radius.toValue();
    }
    if (object._stroke != null) {
      objectValue["180"] = object._stroke.toValue();
    }
    objectValue["200"] = object._startType;
    objectValue["201"] = object._start.toValue();
    objectValue["210"] = object._endType;
    objectValue["211"] = object._end.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const _Grid = STRUCT_CLASS_BY_TYPE[StructType.GRID] as typeof Grid;
    const _GridSpan = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN] as typeof GridSpan;
    const _Insets = STRUCT_CLASS_BY_TYPE[StructType.INSETS] as typeof Insets;
    const _Corners = STRUCT_CLASS_BY_TYPE[StructType.CORNERS] as typeof Corners;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Stroke = STRUCT_CLASS_BY_TYPE[StructType.STROKE] as typeof Stroke;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    const strokeValue = objectValue["180"];
    const unpackedStroke =
      strokeValue != undefined
        ? _Stroke.fromValue(strokeValue, _session, _supergraph, _graph, _connection)
        : null;
    const layoutValue = objectValue["120"];
    const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : null;
    const directionValue = objectValue["121"];
    const unpackedDirection = directionValue != undefined ? Number(directionValue) : null;
    const distributeValue = objectValue["122"];
    const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : null;
    const alignValue = objectValue["123"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const gapValue = objectValue["124"];
    const unpackedGap =
      gapValue != undefined
        ? _Axis2.fromValue(gapValue, _session, _supergraph, _graph, _connection)
        : null;
    const paddingValue = objectValue["125"];
    const unpackedPadding =
      paddingValue != undefined
        ? _Insets.fromValue(paddingValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridValue = objectValue["126"];
    const unpackedGrid =
      gridValue != undefined
        ? _Grid.fromValue(gridValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridSpanValue = objectValue["127"];
    const unpackedGridSpan =
      gridSpanValue != undefined
        ? _GridSpan.fromValue(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectValue["128"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const isWrapValue = objectValue["129"];
    const unpackedIsWrap = isWrapValue != undefined ? isWrapValue : null;
    const isVisibleValue = objectValue["140"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectValue["141"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const fillValue = objectValue["142"];
    const unpackedFill =
      fillValue != undefined
        ? _Fill.fromValue(fillValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectValue["143"];
    const unpackedRotation =
      rotationValue != undefined
        ? _Axis3.fromValue(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectValue["144"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2f.fromValue(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectValue["145"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const shadowValue = objectValue["146"];
    const unpackedShadow =
      shadowValue != undefined
        ? _Shadow.fromValue(shadowValue, _session, _supergraph, _graph, _connection)
        : null;
    const borderValue = objectValue["147"];
    const unpackedBorder =
      borderValue != undefined
        ? _Border.fromValue(borderValue, _session, _supergraph, _graph, _connection)
        : null;
    const radiusValue = objectValue["148"];
    const unpackedRadius =
      radiusValue != undefined
        ? _Corners.fromValue(radiusValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["110"];
    const unpackedPosition =
      positionValue != undefined
        ? _Position.fromValue(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["111"];
    const unpackedWidth =
      widthValue != undefined
        ? _Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectValue["112"];
    const unpackedHeight =
      heightValue != undefined
        ? _Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectValue["113"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? _Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["114"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? _Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["115"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? _Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["116"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? _Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["70"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
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
    const unpackedCustomValues = {} as any;
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ArrowShape({
      startType: Number(objectValue["200"]),
      start: _Vector2f.fromValue(objectValue["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectValue["210"]),
      end: _Vector2f.fromValue(objectValue["211"], _session, _supergraph, _graph, _connection),
      stroke: unpackedStroke,
      layout: unpackedLayout,
      direction: unpackedDirection,
      distribute: unpackedDistribute,
      align: unpackedAlign,
      gap: unpackedGap,
      padding: unpackedPadding,
      grid: unpackedGrid,
      gridSpan: unpackedGridSpan,
      aspectRatio: unpackedAspectRatio,
      isWrap: unpackedIsWrap,
      isVisible: unpackedIsVisible,
      opacity: unpackedOpacity,
      fill: unpackedFill,
      rotation: unpackedRotation,
      skew: unpackedSkew,
      scale: unpackedScale,
      shadow: unpackedShadow,
      border: unpackedBorder,
      radius: unpackedRadius,
      parent: unpackedParentPtr,
      name: objectValue["101"],
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
      key: unpackedKey,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      orderKey: objectValue["27"],
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
  ): ArrowShape {
    return ArrowShape.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ArrowShapeProto {
    return ArrowShape.__packProto__(this);
  }

  static __packProto__(object: ArrowShape): ArrowShapeProto {
    const objectProto: Partial<ArrowShapeProto> = { metatype: 1900200 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.name = object._name;
    if (object._position != null) {
      objectProto.position = object._position.toProto();
    }
    if (object._width != null) {
      objectProto.width = object._width.toProto();
    }
    if (object._height != null) {
      objectProto.height = object._height.toProto();
    }
    if (object._minWidth != null) {
      objectProto.minWidth = object._minWidth.toProto();
    }
    if (object._minHeight != null) {
      objectProto.minHeight = object._minHeight.toProto();
    }
    if (object._maxWidth != null) {
      objectProto.maxWidth = object._maxWidth.toProto();
    }
    if (object._maxHeight != null) {
      objectProto.maxHeight = object._maxHeight.toProto();
    }
    if (object._layout != null) {
      objectProto.layout = Number(object._layout) as LayoutProto;
    }
    if (object._direction != null) {
      objectProto.direction = Number(object._direction) as DirectionProto;
    }
    if (object._distribute != null) {
      objectProto.distribute = Number(object._distribute) as DistributeProto;
    }
    if (object._align != null) {
      objectProto.align = Number(object._align) as AlignProto;
    }
    if (object._gap != null) {
      objectProto.gap = object._gap.toProto();
    }
    if (object._padding != null) {
      objectProto.padding = object._padding.toProto();
    }
    if (object._grid != null) {
      objectProto.grid = object._grid.toProto();
    }
    if (object._gridSpan != null) {
      objectProto.gridSpan = object._gridSpan.toProto();
    }
    if (object._aspectRatio != null) {
      objectProto.aspectRatio = object._aspectRatio;
    }
    if (object._isWrap != null) {
      objectProto.isWrap = object._isWrap;
    }
    if (object._isVisible != null) {
      objectProto.isVisible = object._isVisible;
    }
    if (object._opacity != null) {
      objectProto.opacity = object._opacity;
    }
    if (object._fill != null) {
      objectProto.fill = object._fill.toProto();
    }
    if (object._rotation != null) {
      objectProto.rotation = object._rotation.toProto();
    }
    if (object._skew != null) {
      objectProto.skew = object._skew.toProto();
    }
    if (object._scale != null) {
      objectProto.scale = object._scale;
    }
    if (object._shadow != null) {
      objectProto.shadow = object._shadow.toProto();
    }
    if (object._border != null) {
      objectProto.border = object._border.toProto();
    }
    if (object._radius != null) {
      objectProto.radius = object._radius.toProto();
    }
    if (object._stroke != null) {
      objectProto.stroke = object._stroke.toProto();
    }
    objectProto.startType = Number(object._startType) as ArrowHeadTypeProto;
    objectProto.start = object._start.toProto();
    objectProto.endType = Number(object._endType) as ArrowHeadTypeProto;
    objectProto.end = object._end.toProto();
    return objectProto as ArrowShapeProto;
  }

  static __unpackProto__(
    objectProto: ArrowShapeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const _Grid = STRUCT_CLASS_BY_TYPE[StructType.GRID] as typeof Grid;
    const _GridSpan = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN] as typeof GridSpan;
    const _Insets = STRUCT_CLASS_BY_TYPE[StructType.INSETS] as typeof Insets;
    const _Corners = STRUCT_CLASS_BY_TYPE[StructType.CORNERS] as typeof Corners;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Stroke = STRUCT_CLASS_BY_TYPE[StructType.STROKE] as typeof Stroke;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ArrowShape({
      startType: Number(objectProto.startType) as ArrowHeadType,
      start: _Vector2f.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2f.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
      stroke:
        objectProto.stroke != undefined
          ? _Stroke.fromProto(objectProto.stroke!, _session, _supergraph, _graph, _connection)
          : null,
      layout: objectProto.layout != undefined ? (Number(objectProto.layout) as Layout) : null,
      direction:
        objectProto.direction != undefined ? (Number(objectProto.direction) as Direction) : null,
      distribute:
        objectProto.distribute != undefined ? (Number(objectProto.distribute) as Distribute) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as Align) : null,
      gap:
        objectProto.gap != undefined
          ? _Axis2.fromProto(objectProto.gap!, _session, _supergraph, _graph, _connection)
          : null,
      padding:
        objectProto.padding != undefined
          ? _Insets.fromProto(objectProto.padding!, _session, _supergraph, _graph, _connection)
          : null,
      grid:
        objectProto.grid != undefined
          ? _Grid.fromProto(objectProto.grid!, _session, _supergraph, _graph, _connection)
          : null,
      gridSpan:
        objectProto.gridSpan != undefined
          ? _GridSpan.fromProto(objectProto.gridSpan!, _session, _supergraph, _graph, _connection)
          : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      fill:
        objectProto.fill != undefined
          ? _Fill.fromProto(objectProto.fill!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? _Axis3.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2f.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      shadow:
        objectProto.shadow != undefined
          ? _Shadow.fromProto(objectProto.shadow!, _session, _supergraph, _graph, _connection)
          : null,
      border:
        objectProto.border != undefined
          ? _Border.fromProto(objectProto.border!, _session, _supergraph, _graph, _connection)
          : null,
      radius:
        objectProto.radius != undefined
          ? _Corners.fromProto(objectProto.radius!, _session, _supergraph, _graph, _connection)
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
      position:
        objectProto.position != undefined
          ? _Position.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? _Dimension.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? _Dimension.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? _Dimension.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? _Dimension.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? _Dimension.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? _Dimension.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
          : null,
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
      isExtensible: objectProto.isExtensible,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
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
      id: String(objectProto.id),
      customValues: unpackedCustomValues,
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
      orderKey: objectProto.orderKey,
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
    objectProto: ArrowShapeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape {
    return ArrowShape.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ArrowShape {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ArrowShapeProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ARROW_SHAPE, ArrowShape);
/* ==== DESTACK_GENERATED_END:NODE:1900200 ==== */
