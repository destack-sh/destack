import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Shape } from "@destack/language/canvas/shape";
import type {
  Axis2,
  Axis3,
  Corners,
  CustomEntityDefinition,
  CustomEventDefinition,
  Dimension,
  Graph,
  Grid,
  GridSpan,
  Insets,
  IsSubject,
  NodeDefinitionReference,
  NodeReference,
  Position,
  QueryConnection,
  Session,
  Supergraph,
  Value,
  Vector2,
} from "@destack/language/core";
import {
  Align,
  Direction,
  Distribute,
  EnumType,
  Layout,
  Materialization,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import type { Folder } from "@destack/language/folder";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Layer, Scene, Window } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { Border, Fill, Shadow, Stroke } from "@destack/language/style";
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

/* ==== DESTACK_GENERATED_START:STRUCT:250300 ==== */
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
  readonly start: Vector2;

  /**
   * Arrow.endType
   */
  readonly endType: ArrowHeadType;

  /**
   * Arrow.end
   */
  readonly end: Vector2;

  constructor(options: {
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
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
    if (this._hash !== null) {
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Arrow.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Arrow): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 250300;
    objectValue["200"] = object.startType;
    objectValue["201"] = object.start.toValue();
    objectValue["210"] = object.endType;
    objectValue["211"] = object.end.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Arrow {
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new Arrow({
      startType: Number(objectValue["200"]),
      start: _Vector2.fromValue(objectValue["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectValue["210"]),
      end: _Vector2.fromValue(objectValue["211"], _session, _supergraph, _graph, _connection),
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
    const objectProto: Partial<ArrowProto> = { metatype: 250300 };
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
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    return new Arrow({
      startType: Number(objectProto.startType) as ArrowHeadType,
      start: _Vector2.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
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
/* ==== DESTACK_GENERATED_END:STRUCT:250300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:250300 ==== */
/**
 * An ArrowShape is a shape that represents an arrow.
 */
export class ArrowShape extends Shape {
  static metatype: NodeType = NodeType.ARROW_SHAPE;

  /**
   * View.parent
   */
  get parent(): Window | Scene | Layer | ContainerView | Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Window
        | Scene
        | Layer
        | ContainerView
        | Folder
        | null;
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
   * The definitionthis CustomEntity is an instance of.
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  readonly baseType: NodeDefinitionReference | null;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
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
  scriptPtr: NodeReference | null;

  /**
   * View.name
   */
  name: string;

  /**
   * View.position
   */
  position: Position | null;

  /**
   * View.width
   */
  width: Dimension | null;

  /**
   * View.height
   */
  height: Dimension | null;

  /**
   * View.minWidth
   */
  minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  maxHeight: Dimension | null;

  /**
   * ContainerView.layout
   */
  layout: Layout | null;

  /**
   * ContainerView.direction
   */
  direction: Direction | null;

  /**
   * ContainerView.distribute
   */
  distribute: Distribute | null;

  /**
   * ContainerView.align
   */
  align: Align | null;

  /**
   * ContainerView.gap
   */
  gap: Axis2 | null;

  /**
   * ContainerView.padding
   */
  padding: Insets | null;

  /**
   * ContainerView.grid
   */
  grid: Grid | null;

  /**
   * ContainerView.gridSpan
   */
  gridSpan: GridSpan | null;

  /**
   * ContainerView.aspectRatio
   */
  aspectRatio: number | null;

  /**
   * ContainerView.isWrap
   */
  isWrap: boolean | null;

  /**
   * ContainerView.isVisible
   */
  isVisible: boolean | null;

  /**
   * ContainerView.opacity
   */
  opacity: number | null;

  /**
   * ContainerView.fill
   */
  fill: Fill | null;

  /**
   * ContainerView.rotation
   */
  rotation: Axis3 | null;

  /**
   * ContainerView.skew
   */
  skew: Vector2 | null;

  /**
   * ContainerView.scale
   */
  scale: number | null;

  /**
   * ContainerView.shadow
   */
  shadow: Shadow | null;

  /**
   * ContainerView.border
   */
  border: Border | null;

  /**
   * ContainerView.radius
   */
  radius: Corners | null;

  /**
   * Shape.stroke
   */
  stroke: Stroke | null;

  /**
   * ArrowShape.startType
   */
  startType: ArrowHeadType;

  /**
   * ArrowShape.start
   */
  start: Vector2;

  /**
   * ArrowShape.endType
   */
  endType: ArrowHeadType;

  /**
   * ArrowShape.end
   */
  end: Vector2;

  constructor(options: {
    id?: string;
    parent?: Window | Scene | Layer | ContainerView | Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    definition?: CustomEntityDefinition | CustomEventDefinition | NodeReference | null;
    baseType?: NodeDefinitionReference | null;
    materialization?: Materialization;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: Map<string, Value>;
    orderKey?: string;
    script?: Script | NodeReference | null;
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
    skew?: Vector2 | null;
    scale?: number | null;
    shadow?: Shadow | null;
    border?: Border | null;
    radius?: Corners | null;
    stroke?: Stroke | null;
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
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
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`ArrowShape.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ArrowShape.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this.scriptPtr = _script;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ArrowShape.name is required`);
    }
    this.name = _name;
    let _position = options.position ?? null;
    this.position = _position;
    let _width = options.width ?? null;
    this.width = _width;
    let _height = options.height ?? null;
    this.height = _height;
    let _minWidth = options.minWidth ?? null;
    this.minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this.minHeight = _minHeight;
    let _maxWidth = options.maxWidth ?? null;
    this.maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this.maxHeight = _maxHeight;
    let _layout = options.layout ?? null;
    this.layout = _layout;
    let _direction = options.direction ?? null;
    this.direction = _direction;
    let _distribute = options.distribute ?? null;
    this.distribute = _distribute;
    let _align = options.align ?? null;
    this.align = _align;
    let _gap = options.gap ?? null;
    this.gap = _gap;
    let _padding = options.padding ?? null;
    this.padding = _padding;
    let _grid = options.grid ?? null;
    this.grid = _grid;
    let _gridSpan = options.gridSpan ?? null;
    this.gridSpan = _gridSpan;
    let _aspectRatio = options.aspectRatio ?? null;
    this.aspectRatio = _aspectRatio;
    let _isWrap = options.isWrap ?? null;
    this.isWrap = _isWrap;
    let _isVisible = options.isVisible ?? null;
    this.isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _fill = options.fill ?? null;
    this.fill = _fill;
    let _rotation = options.rotation ?? null;
    this.rotation = _rotation;
    let _skew = options.skew ?? null;
    this.skew = _skew;
    let _scale = options.scale ?? null;
    this.scale = _scale;
    let _shadow = options.shadow ?? null;
    this.shadow = _shadow;
    let _border = options.border ?? null;
    this.border = _border;
    let _radius = options.radius ?? null;
    this.radius = _radius;
    let _stroke = options.stroke ?? null;
    this.stroke = _stroke;
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`ArrowShape.startType is required`);
    }
    this.startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`ArrowShape.start is required`);
    }
    this.start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`ArrowShape.endType is required`);
    }
    this.endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`ArrowShape.end is required`);
    }
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
    if (
      (this.stroke == null) !== (other.stroke == null) ||
      (this.stroke != null && !this.stroke.equals(other.stroke))
    ) {
      return false;
    }
    if (!(this.layout === other.layout)) {
      return false;
    }
    if (!(this.direction === other.direction)) {
      return false;
    }
    if (!(this.distribute === other.distribute)) {
      return false;
    }
    if (!(this.align === other.align)) {
      return false;
    }
    if (
      (this.gap == null) !== (other.gap == null) ||
      (this.gap != null && !this.gap.equals(other.gap))
    ) {
      return false;
    }
    if (
      (this.padding == null) !== (other.padding == null) ||
      (this.padding != null && !this.padding.equals(other.padding))
    ) {
      return false;
    }
    if (
      (this.grid == null) !== (other.grid == null) ||
      (this.grid != null && !this.grid.equals(other.grid))
    ) {
      return false;
    }
    if (
      (this.gridSpan == null) !== (other.gridSpan == null) ||
      (this.gridSpan != null && !this.gridSpan.equals(other.gridSpan))
    ) {
      return false;
    }
    if (
      (this.aspectRatio == null) !== (other.aspectRatio == null) ||
      (this.aspectRatio != null &&
        !(
          this.aspectRatio === other.aspectRatio ||
          Math.abs(this.aspectRatio - other.aspectRatio) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this.isWrap === other.isWrap)) {
      return false;
    }
    if (!(this.isVisible === other.isVisible)) {
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
      (this.fill == null) !== (other.fill == null) ||
      (this.fill != null && !this.fill.equals(other.fill))
    ) {
      return false;
    }
    if (
      (this.rotation == null) !== (other.rotation == null) ||
      (this.rotation != null && !this.rotation.equals(other.rotation))
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
      (this.scale == null) !== (other.scale == null) ||
      (this.scale != null &&
        !(this.scale === other.scale || Math.abs(this.scale - other.scale) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.shadow == null) !== (other.shadow == null) ||
      (this.shadow != null && !this.shadow.equals(other.shadow))
    ) {
      return false;
    }
    if (
      (this.border == null) !== (other.border == null) ||
      (this.border != null && !this.border.equals(other.border))
    ) {
      return false;
    }
    if (
      (this.radius == null) !== (other.radius == null) ||
      (this.radius != null && !this.radius.equals(other.radius))
    ) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.position == null) !== (other.position == null) ||
      (this.position != null && !this.position.equals(other.position))
    ) {
      return false;
    }
    if (
      (this.width == null) !== (other.width == null) ||
      (this.width != null && !this.width.equals(other.width))
    ) {
      return false;
    }
    if (
      (this.height == null) !== (other.height == null) ||
      (this.height != null && !this.height.equals(other.height))
    ) {
      return false;
    }
    if (
      (this.minWidth == null) !== (other.minWidth == null) ||
      (this.minWidth != null && !this.minWidth.equals(other.minWidth))
    ) {
      return false;
    }
    if (
      (this.minHeight == null) !== (other.minHeight == null) ||
      (this.minHeight != null && !this.minHeight.equals(other.minHeight))
    ) {
      return false;
    }
    if (
      (this.maxWidth == null) !== (other.maxWidth == null) ||
      (this.maxWidth != null && !this.maxWidth.equals(other.maxWidth))
    ) {
      return false;
    }
    if (
      (this.maxHeight == null) !== (other.maxHeight == null) ||
      (this.maxHeight != null && !this.maxHeight.equals(other.maxHeight))
    ) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues.get(key)!.equals(other.customValues.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.startType) & 0xffffffff;
    h = (h * 31 + this.start.hash()) & 0xffffffff;
    h = (h * 31 + this.endType) & 0xffffffff;
    h = (h * 31 + this.end.hash()) & 0xffffffff;
    if (this.stroke !== null) {
      h = (h * 31 + this.stroke.hash()) & 0xffffffff;
    }
    if (this.layout !== null) {
      h = (h * 31 + this.layout) & 0xffffffff;
    }
    if (this.direction !== null) {
      h = (h * 31 + this.direction) & 0xffffffff;
    }
    if (this.distribute !== null) {
      h = (h * 31 + this.distribute) & 0xffffffff;
    }
    if (this.align !== null) {
      h = (h * 31 + this.align) & 0xffffffff;
    }
    if (this.gap !== null) {
      h = (h * 31 + this.gap.hash()) & 0xffffffff;
    }
    if (this.padding !== null) {
      h = (h * 31 + this.padding.hash()) & 0xffffffff;
    }
    if (this.grid !== null) {
      h = (h * 31 + this.grid.hash()) & 0xffffffff;
    }
    if (this.gridSpan !== null) {
      h = (h * 31 + this.gridSpan.hash()) & 0xffffffff;
    }
    if (this.aspectRatio !== null) {
      h = (h * 31 + hashFloat(this.aspectRatio)) & 0xffffffff;
    }
    if (this.isWrap !== null) {
      h = (h * 31 + hashBool(this.isWrap)) & 0xffffffff;
    }
    if (this.isVisible !== null) {
      h = (h * 31 + hashBool(this.isVisible)) & 0xffffffff;
    }
    if (this.opacity !== null) {
      h = (h * 31 + hashFloat(this.opacity)) & 0xffffffff;
    }
    if (this.fill !== null) {
      h = (h * 31 + this.fill.hash()) & 0xffffffff;
    }
    if (this.rotation !== null) {
      h = (h * 31 + this.rotation.hash()) & 0xffffffff;
    }
    if (this.skew !== null) {
      h = (h * 31 + this.skew.hash()) & 0xffffffff;
    }
    if (this.scale !== null) {
      h = (h * 31 + hashFloat(this.scale)) & 0xffffffff;
    }
    if (this.shadow !== null) {
      h = (h * 31 + this.shadow.hash()) & 0xffffffff;
    }
    if (this.border !== null) {
      h = (h * 31 + this.border.hash()) & 0xffffffff;
    }
    if (this.radius !== null) {
      h = (h * 31 + this.radius.hash()) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.position !== null) {
      h = (h * 31 + this.position.hash()) & 0xffffffff;
    }
    if (this.width !== null) {
      h = (h * 31 + this.width.hash()) & 0xffffffff;
    }
    if (this.height !== null) {
      h = (h * 31 + this.height.hash()) & 0xffffffff;
    }
    if (this.minWidth !== null) {
      h = (h * 31 + this.minWidth.hash()) & 0xffffffff;
    }
    if (this.minHeight !== null) {
      h = (h * 31 + this.minHeight.hash()) & 0xffffffff;
    }
    if (this.maxWidth !== null) {
      h = (h * 31 + this.maxWidth.hash()) & 0xffffffff;
    }
    if (this.maxHeight !== null) {
      h = (h * 31 + this.maxHeight.hash()) & 0xffffffff;
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
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this.scriptPtr !== null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
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
      type: NodeType.ARROW_SHAPE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
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
    if (this.stroke !== null) {
      propertyReprs.push(`stroke=${this.stroke.repr()}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<ArrowShape '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return ArrowShape.__packValue__(this);
  }

  static __packValue__(object: ArrowShape): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 250300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    if (object.baseType != null) {
      objectValue["7"] = object.baseType.toValue();
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
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["27"] = object.orderKey;
    if (object.scriptPtr != null) {
      objectValue["70"] = object.scriptPtr.toValue();
    }
    objectValue["101"] = object.name;
    if (object.position != null) {
      objectValue["110"] = object.position.toValue();
    }
    if (object.width != null) {
      objectValue["111"] = object.width.toValue();
    }
    if (object.height != null) {
      objectValue["112"] = object.height.toValue();
    }
    if (object.minWidth != null) {
      objectValue["113"] = object.minWidth.toValue();
    }
    if (object.minHeight != null) {
      objectValue["114"] = object.minHeight.toValue();
    }
    if (object.maxWidth != null) {
      objectValue["115"] = object.maxWidth.toValue();
    }
    if (object.maxHeight != null) {
      objectValue["116"] = object.maxHeight.toValue();
    }
    if (object.layout != null) {
      objectValue["120"] = object.layout;
    }
    if (object.direction != null) {
      objectValue["121"] = object.direction;
    }
    if (object.distribute != null) {
      objectValue["122"] = object.distribute;
    }
    if (object.align != null) {
      objectValue["123"] = object.align;
    }
    if (object.gap != null) {
      objectValue["124"] = object.gap.toValue();
    }
    if (object.padding != null) {
      objectValue["125"] = object.padding.toValue();
    }
    if (object.grid != null) {
      objectValue["126"] = object.grid.toValue();
    }
    if (object.gridSpan != null) {
      objectValue["127"] = object.gridSpan.toValue();
    }
    if (object.aspectRatio != null) {
      objectValue["128"] = object.aspectRatio;
    }
    if (object.isWrap != null) {
      objectValue["129"] = object.isWrap;
    }
    if (object.isVisible != null) {
      objectValue["140"] = object.isVisible;
    }
    if (object.opacity != null) {
      objectValue["141"] = object.opacity;
    }
    if (object.fill != null) {
      objectValue["142"] = object.fill.toValue();
    }
    if (object.rotation != null) {
      objectValue["143"] = object.rotation.toValue();
    }
    if (object.skew != null) {
      objectValue["144"] = object.skew.toValue();
    }
    if (object.scale != null) {
      objectValue["145"] = object.scale;
    }
    if (object.shadow != null) {
      objectValue["146"] = object.shadow.toValue();
    }
    if (object.border != null) {
      objectValue["147"] = object.border.toValue();
    }
    if (object.radius != null) {
      objectValue["148"] = object.radius.toValue();
    }
    if (object.stroke != null) {
      objectValue["180"] = object.stroke.toValue();
    }
    objectValue["200"] = object.startType;
    objectValue["201"] = object.start.toValue();
    objectValue["210"] = object.endType;
    objectValue["211"] = object.end.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ArrowShape {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
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
        ? _Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection)
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
    const scriptPtrValue = objectValue["70"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["7"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _NodeDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ArrowShape({
      startType: Number(objectValue["200"]),
      start: _Vector2.fromValue(objectValue["201"], _session, _supergraph, _graph, _connection),
      endType: Number(objectValue["210"]),
      end: _Vector2.fromValue(objectValue["211"], _session, _supergraph, _graph, _connection),
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
      space: unpackedSpacePtr,
      materialization: Number(objectValue["10"]),
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      orderKey: objectValue["27"],
      script: unpackedScriptPtr,
      definition: unpackedDefinitionPtr,
      baseType: unpackedBaseType,
      deletedAt: unpackedDeletedAt,
      id: String(objectValue["2"]),
      customValues: unpackedCustomValues,
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
  ): ArrowShape {
    return ArrowShape.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ArrowShapeProto {
    return ArrowShape.__packProto__(this);
  }

  static __packProto__(object: ArrowShape): ArrowShapeProto {
    const objectProto: Partial<ArrowShapeProto> = { metatype: 250300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
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
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    objectProto.name = object.name;
    if (object.position != null) {
      objectProto.position = object.position.toProto();
    }
    if (object.width != null) {
      objectProto.width = object.width.toProto();
    }
    if (object.height != null) {
      objectProto.height = object.height.toProto();
    }
    if (object.minWidth != null) {
      objectProto.minWidth = object.minWidth.toProto();
    }
    if (object.minHeight != null) {
      objectProto.minHeight = object.minHeight.toProto();
    }
    if (object.maxWidth != null) {
      objectProto.maxWidth = object.maxWidth.toProto();
    }
    if (object.maxHeight != null) {
      objectProto.maxHeight = object.maxHeight.toProto();
    }
    if (object.layout != null) {
      objectProto.layout = Number(object.layout) as LayoutProto;
    }
    if (object.direction != null) {
      objectProto.direction = Number(object.direction) as DirectionProto;
    }
    if (object.distribute != null) {
      objectProto.distribute = Number(object.distribute) as DistributeProto;
    }
    if (object.align != null) {
      objectProto.align = Number(object.align) as AlignProto;
    }
    if (object.gap != null) {
      objectProto.gap = object.gap.toProto();
    }
    if (object.padding != null) {
      objectProto.padding = object.padding.toProto();
    }
    if (object.grid != null) {
      objectProto.grid = object.grid.toProto();
    }
    if (object.gridSpan != null) {
      objectProto.gridSpan = object.gridSpan.toProto();
    }
    if (object.aspectRatio != null) {
      objectProto.aspectRatio = object.aspectRatio;
    }
    if (object.isWrap != null) {
      objectProto.isWrap = object.isWrap;
    }
    if (object.isVisible != null) {
      objectProto.isVisible = object.isVisible;
    }
    if (object.opacity != null) {
      objectProto.opacity = object.opacity;
    }
    if (object.fill != null) {
      objectProto.fill = object.fill.toProto();
    }
    if (object.rotation != null) {
      objectProto.rotation = object.rotation.toProto();
    }
    if (object.skew != null) {
      objectProto.skew = object.skew.toProto();
    }
    if (object.scale != null) {
      objectProto.scale = object.scale;
    }
    if (object.shadow != null) {
      objectProto.shadow = object.shadow.toProto();
    }
    if (object.border != null) {
      objectProto.border = object.border.toProto();
    }
    if (object.radius != null) {
      objectProto.radius = object.radius.toProto();
    }
    if (object.stroke != null) {
      objectProto.stroke = object.stroke.toProto();
    }
    objectProto.startType = Number(object.startType) as ArrowHeadTypeProto;
    objectProto.start = object.start.toProto();
    objectProto.endType = Number(object.endType) as ArrowHeadTypeProto;
    objectProto.end = object.end.toProto();
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
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
    const unpackedCustomValues = new Map();
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
      start: _Vector2.fromProto(objectProto.start!, _session, _supergraph, _graph, _connection),
      endType: Number(objectProto.endType) as ArrowHeadType,
      end: _Vector2.fromProto(objectProto.end!, _session, _supergraph, _graph, _connection),
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
          ? _Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
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
      baseType:
        objectProto.baseType != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.baseType!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      id: String(objectProto.id),
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:250300 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:250300 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:250300 ==== */
