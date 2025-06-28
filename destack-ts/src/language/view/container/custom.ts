import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  IsCustomNode,
  IsCustomNodeDefinition,
  IsSubject,
  Node,
  NodeType,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Grid,
  GridSpan,
  Insets,
  Layout,
  Position,
  Value,
  Vector2,
} from "@destack/language/core/common";
import { Folder } from "@destack/language/folder";
import { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Border, Fill, Shadow } from "@destack/language/style";
import { ContainerView } from "@destack/language/view/container";
import {
  AlignProto,
  CustomViewDefinitionProto,
  CustomViewProto,
  DirectionProto,
  DistributeProto,
  LayoutProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:10000 ==== */
/**
 * A definition for a custom View type.
 */
export class CustomViewDefinition extends Node implements ContainerView, IsCustomNodeDefinition {
  static metatype: NodeType = NodeType.CUSTOM_VIEW_DEFINITION;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.VIEW,
    TraitType.ENTITY,
    TraitType.TAGGABLE,
    TraitType.CONTAINER_VIEW,
    TraitType.TRACKED,
    TraitType.CUSTOM_NODE_DEFINITION,
    TraitType.DELETABLE,
    TraitType.EXTENSIBLE,
    TraitType.ORDERED,
    TraitType.SCRIPTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SCENE];
  static __childTypes__: NodeType[] = [
    NodeType.CUSTOM_PROPERTY,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.POLYGON_SHAPE,
    NodeType.TAGGING,
    NodeType.SCRIPT,
    NodeType.VARIANT,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.STROKE_STYLE,
  ];
  static __ancestorTypes__: NodeType[] = [
    NodeType.WINDOW,
    NodeType.SPACE,
    NodeType.SCENE,
    NodeType.FOLDER,
  ];
  static __descendantTypes__: NodeType[] = [
    NodeType.LINE_SHAPE,
    NodeType.POLYGON_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.FONT_STYLE,
    NodeType.CUSTOM_VIEW,
    NodeType.BORDER_STYLE,
    NodeType.WIZARD_VIEW,
    NodeType.SHADOW_STYLE,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.GRADIENT_STYLE,
    NodeType.LABEL_VIEW,
    NodeType.TRANSITION_STYLE,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.EFFECT_STYLE,
    NodeType.STROKE_STYLE,
    NodeType.LAYER,
    NodeType.CUSTOM_PROPERTY,
    NodeType.TEXT_VIEW,
    NodeType.CUSTOM_OPTION,
    NodeType.VARIANT,
    NodeType.THREAD_VIEW,
    NodeType.TAGGING,
    NodeType.COLOR_STYLE,
    NodeType.CANVAS,
    NodeType.FILL_STYLE,
  ];

  /**
   * CustomViewDefinition.parent
   */
  get parent(): Folder | Scene | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | Scene | null;
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
   * A custom View's prototype is the default template new CustomView instances are based on.
   */
  get prototype(): CustomView | null {
    const nodePtr: NodeReference | null = this.prototypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomView | null;
    }
    return null;
  }
  set prototype(node: CustomView | null) {
    if (node === null) {
      this.prototypePtr = null;
    } else {
      this.prototypePtr = node.toRef();
    }
  }
  prototypePtr: NodeReference | null;

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
   * IsExtensible.value
   */
  value: Map<string, Value>;

  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
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

  constructor(options: {
    id?: string;
    parent?: Folder | Scene | NodeReference | null;
    space?: Space | NodeReference | null;
    prototype?: CustomView | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    value?: Map<string, Value>;
    orderKey?: string;
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
    script?: Script | NodeReference | null;
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
    let _prototype = options.prototype ?? null;
    if (_prototype != null && _prototype instanceof Node) {
      _prototype = _prototype.toRef();
    }
    this.prototypePtr = _prototype;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _value = options.value ?? null;
    if (_value === null) {
      _value = new Map();
    }
    this.value = _value;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomViewDefinition.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomViewDefinition.name is required`);
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
    let _script = options.script ?? null;
    if (_script != null && _script instanceof Node) {
      _script = _script.toRef();
    }
    this.scriptPtr = _script;

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
    if (!(this.prototypePtr?.id === other.prototypePtr?.id)) {
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
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (Object.keys(this.value).length !== Object.keys(other.value).length) {
      return false;
    }
    for (const key in this.value) {
      if (!(key in other.value)) {
        return false;
      }
      if (!this.value.get(key)!.equals(other.value.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.prototypePtr !== null) {
      h = (h * 31 + hashString(this.prototypePtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
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
    if (this.scriptPtr !== null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.value && Object.keys(this.value).length > 0) {
      for (const [_key, _value] of Object.entries(this.value)) {
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
    return new NodeReference({
      nodeType: NodeType.CUSTOM_VIEW_DEFINITION,
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
    return `<CustomViewDefinition '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomViewDefinition.__packValue__(this);
  }

  static __packValue__(object: CustomViewDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 10000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.prototypePtr != null) {
      objectValue["6"] = object.prototypePtr.toValue();
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
    if (object.value.size > 0) {
      const packedValue: { [key: string]: any } = {};
      for (const [key, value] of object.value) {
        packedValue[String(String(key))] = value.toValue();
      }
      objectValue["21"] = packedValue;
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.position != null) {
      objectValue["40"] = object.position.toValue();
    }
    if (object.width != null) {
      objectValue["41"] = object.width.toValue();
    }
    if (object.height != null) {
      objectValue["42"] = object.height.toValue();
    }
    if (object.minWidth != null) {
      objectValue["43"] = object.minWidth.toValue();
    }
    if (object.minHeight != null) {
      objectValue["44"] = object.minHeight.toValue();
    }
    if (object.maxWidth != null) {
      objectValue["45"] = object.maxWidth.toValue();
    }
    if (object.maxHeight != null) {
      objectValue["46"] = object.maxHeight.toValue();
    }
    if (object.layout != null) {
      objectValue["50"] = object.layout;
    }
    if (object.direction != null) {
      objectValue["51"] = object.direction;
    }
    if (object.distribute != null) {
      objectValue["52"] = object.distribute;
    }
    if (object.align != null) {
      objectValue["53"] = object.align;
    }
    if (object.gap != null) {
      objectValue["54"] = object.gap.toValue();
    }
    if (object.padding != null) {
      objectValue["55"] = object.padding.toValue();
    }
    if (object.grid != null) {
      objectValue["56"] = object.grid.toValue();
    }
    if (object.gridSpan != null) {
      objectValue["57"] = object.gridSpan.toValue();
    }
    if (object.aspectRatio != null) {
      objectValue["58"] = object.aspectRatio;
    }
    if (object.isWrap != null) {
      objectValue["59"] = object.isWrap;
    }
    if (object.isVisible != null) {
      objectValue["60"] = object.isVisible;
    }
    if (object.opacity != null) {
      objectValue["61"] = object.opacity;
    }
    if (object.fill != null) {
      objectValue["62"] = object.fill.toValue();
    }
    if (object.rotation != null) {
      objectValue["63"] = object.rotation.toValue();
    }
    if (object.skew != null) {
      objectValue["64"] = object.skew.toValue();
    }
    if (object.scale != null) {
      objectValue["65"] = object.scale;
    }
    if (object.shadow != null) {
      objectValue["66"] = object.shadow.toValue();
    }
    if (object.border != null) {
      objectValue["67"] = object.border.toValue();
    }
    if (object.radius != null) {
      objectValue["68"] = object.radius.toValue();
    }
    if (object.scriptPtr != null) {
      objectValue["200"] = object.scriptPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomViewDefinition {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const prototypePtrValue = objectValue["6"];
    const unpackedPrototypePtr =
      prototypePtrValue != undefined
        ? NodeReference.fromValue(prototypePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const layoutValue = objectValue["50"];
    const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : null;
    const directionValue = objectValue["51"];
    const unpackedDirection = directionValue != undefined ? Number(directionValue) : null;
    const distributeValue = objectValue["52"];
    const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : null;
    const alignValue = objectValue["53"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const gapValue = objectValue["54"];
    const unpackedGap =
      gapValue != undefined
        ? Axis2.fromValue(gapValue, _session, _supergraph, _graph, _connection)
        : null;
    const paddingValue = objectValue["55"];
    const unpackedPadding =
      paddingValue != undefined
        ? Insets.fromValue(paddingValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridValue = objectValue["56"];
    const unpackedGrid =
      gridValue != undefined
        ? Grid.fromValue(gridValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridSpanValue = objectValue["57"];
    const unpackedGridSpan =
      gridSpanValue != undefined
        ? GridSpan.fromValue(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectValue["58"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const isWrapValue = objectValue["59"];
    const unpackedIsWrap = isWrapValue != undefined ? isWrapValue : null;
    const isVisibleValue = objectValue["60"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectValue["61"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const fillValue = objectValue["62"];
    const unpackedFill =
      fillValue != undefined
        ? Fill.fromValue(fillValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectValue["63"];
    const unpackedRotation =
      rotationValue != undefined
        ? Axis3.fromValue(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectValue["64"];
    const unpackedSkew =
      skewValue != undefined
        ? Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectValue["65"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const shadowValue = objectValue["66"];
    const unpackedShadow =
      shadowValue != undefined
        ? Shadow.fromValue(shadowValue, _session, _supergraph, _graph, _connection)
        : null;
    const borderValue = objectValue["67"];
    const unpackedBorder =
      borderValue != undefined
        ? Border.fromValue(borderValue, _session, _supergraph, _graph, _connection)
        : null;
    const radiusValue = objectValue["68"];
    const unpackedRadius =
      radiusValue != undefined
        ? Corners.fromValue(radiusValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["40"];
    const unpackedPosition =
      positionValue != undefined
        ? Position.fromValue(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["41"];
    const unpackedWidth =
      widthValue != undefined
        ? Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectValue["42"];
    const unpackedHeight =
      heightValue != undefined
        ? Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectValue["43"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["44"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["45"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["46"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
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
    const scriptPtrValue = objectValue["200"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const unpackedValue = new Map();
    if (objectValue["21"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["21"])) {
        unpackedValue.set(
          String(key),
          Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomViewDefinition({
      parent: unpackedParentPtr,
      prototype: unpackedPrototypePtr,
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
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
      script: unpackedScriptPtr,
      deletedAt: unpackedDeletedAt,
      value: unpackedValue,
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
  ): CustomViewDefinition {
    return CustomViewDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): CustomViewDefinitionProto {
    return CustomViewDefinition.__packProto__(this);
  }

  static __packProto__(object: CustomViewDefinition): CustomViewDefinitionProto {
    const objectProto: Partial<CustomViewDefinitionProto> = { metatype: 10000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.prototypePtr != null) {
      objectProto.prototypePtr = object.prototypePtr.toProto();
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
    if (object.value) {
      objectProto.value = {};
      for (const [key, value] of object.value) {
        objectProto.value![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
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
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    return objectProto as CustomViewDefinitionProto;
  }

  static __unpackProto__(
    objectProto: CustomViewDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomViewDefinition {
    const unpackedValue = new Map();
    if (objectProto.value) {
      for (const [key, value] of Object.entries(objectProto.value)) {
        unpackedValue.set(
          String(key),
          Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomViewDefinition({
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
      prototype:
        objectProto.prototypePtr != undefined
          ? NodeReference.fromProto(
              objectProto.prototypePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      layout: objectProto.layout != undefined ? (Number(objectProto.layout) as Layout) : null,
      direction:
        objectProto.direction != undefined ? (Number(objectProto.direction) as Direction) : null,
      distribute:
        objectProto.distribute != undefined ? (Number(objectProto.distribute) as Distribute) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as Align) : null,
      gap:
        objectProto.gap != undefined
          ? Axis2.fromProto(objectProto.gap!, _session, _supergraph, _graph, _connection)
          : null,
      padding:
        objectProto.padding != undefined
          ? Insets.fromProto(objectProto.padding!, _session, _supergraph, _graph, _connection)
          : null,
      grid:
        objectProto.grid != undefined
          ? Grid.fromProto(objectProto.grid!, _session, _supergraph, _graph, _connection)
          : null,
      gridSpan:
        objectProto.gridSpan != undefined
          ? GridSpan.fromProto(objectProto.gridSpan!, _session, _supergraph, _graph, _connection)
          : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      fill:
        objectProto.fill != undefined
          ? Fill.fromProto(objectProto.fill!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? Axis3.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      shadow:
        objectProto.shadow != undefined
          ? Shadow.fromProto(objectProto.shadow!, _session, _supergraph, _graph, _connection)
          : null,
      border:
        objectProto.border != undefined
          ? Border.fromProto(objectProto.border!, _session, _supergraph, _graph, _connection)
          : null,
      radius:
        objectProto.radius != undefined
          ? Corners.fromProto(objectProto.radius!, _session, _supergraph, _graph, _connection)
          : null,
      position:
        objectProto.position != undefined
          ? Position.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Dimension.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? Dimension.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? Dimension.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? Dimension.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? Dimension.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? Dimension.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
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
      id: String(objectProto.id),
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
      script:
        objectProto.scriptPtr != undefined
          ? NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      value: unpackedValue,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomViewDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomViewDefinition {
    return CustomViewDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): CustomViewDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomViewDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_VIEW_DEFINITION, CustomViewDefinition);
/* ==== DESTACK_GENERATED_END:NODE:10000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:10001 ==== */
/**
 * CustomView
 */
export class CustomView extends Node implements ContainerView, IsCustomNode {
  static metatype: NodeType = NodeType.CUSTOM_VIEW;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.VIEW,
    TraitType.ENTITY,
    TraitType.TAGGABLE,
    TraitType.CONTAINER_VIEW,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.EXTENSIBLE,
    TraitType.ORDERED,
    TraitType.CUSTOM_NODE,
    TraitType.SCRIPTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.POLYGON_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.LAYER,
    NodeType.CUSTOM_VIEW,
    NodeType.CANVAS,
    NodeType.SPLIT_VIEW,
    NodeType.SCENE,
  ];
  static __childTypes__: NodeType[] = [
    NodeType.CUSTOM_PROPERTY,
    NodeType.CUSTOM_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.TEXT_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.THREAD_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.CANVAS,
    NodeType.LINE_SHAPE,
    NodeType.POLYGON_SHAPE,
    NodeType.TAGGING,
    NodeType.SCRIPT,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.STROKE_STYLE,
  ];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.POLYGON_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.FOLDER,
    NodeType.LABEL_VIEW,
    NodeType.CANVAS,
    NodeType.CUSTOM_VIEW,
    NodeType.LAYER,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.SPLIT_VIEW,
    NodeType.SCENE,
  ];
  static __descendantTypes__: NodeType[] = [
    NodeType.LINE_SHAPE,
    NodeType.POLYGON_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.FONT_STYLE,
    NodeType.CUSTOM_VIEW,
    NodeType.BORDER_STYLE,
    NodeType.WIZARD_VIEW,
    NodeType.SHADOW_STYLE,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.GRADIENT_STYLE,
    NodeType.LABEL_VIEW,
    NodeType.TRANSITION_STYLE,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.EFFECT_STYLE,
    NodeType.STROKE_STYLE,
    NodeType.LAYER,
    NodeType.CUSTOM_PROPERTY,
    NodeType.TEXT_VIEW,
    NodeType.CUSTOM_OPTION,
    NodeType.VARIANT,
    NodeType.THREAD_VIEW,
    NodeType.TAGGING,
    NodeType.COLOR_STYLE,
    NodeType.CANVAS,
    NodeType.FILL_STYLE,
  ];

  /**
   * View.parent
   */
  get parent(): Window | Scene | Layer | (Node & ContainerView) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Window
        | Scene
        | Layer
        | (Node & ContainerView)
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
   * The CustomViewDefinition this CustomView is an instance of.
   */
  get definition(): CustomViewDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomViewDefinition | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference;

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
   * IsExtensible.value
   */
  value: Map<string, Value>;

  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
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

  constructor(options: {
    id?: string;
    parent?: Window | Scene | Layer | (Node & ContainerView) | NodeReference | null;
    space?: Space | NodeReference | null;
    definition?: CustomViewDefinition | NodeReference;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    value?: Map<string, Value>;
    orderKey?: string;
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
    script?: Script | NodeReference | null;
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
    let _definition = options.definition ?? null;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    if (_definition === null) {
      throw new Error(`CustomView.definition is required`);
    }
    this.definitionPtr = _definition;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _value = options.value ?? null;
    if (_value === null) {
      _value = new Map();
    }
    this.value = _value;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomView.name is required`);
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
    let _script = options.script ?? null;
    if (_script != null && _script instanceof Node) {
      _script = _script.toRef();
    }
    this.scriptPtr = _script;

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
    if (!(this.definitionPtr.id === other.definitionPtr.id)) {
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
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.scriptPtr?.id === other.scriptPtr?.id)) {
      return false;
    }
    if (Object.keys(this.value).length !== Object.keys(other.value).length) {
      return false;
    }
    for (const key in this.value) {
      if (!(key in other.value)) {
        return false;
      }
      if (!this.value.get(key)!.equals(other.value.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
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
    if (this.scriptPtr !== null) {
      h = (h * 31 + hashString(this.scriptPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.value && Object.keys(this.value).length > 0) {
      for (const [_key, _value] of Object.entries(this.value)) {
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
    return new NodeReference({
      nodeType: NodeType.CUSTOM_VIEW,
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
    propertyReprs.push(`name=${this.name}`);
    return `<CustomView '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomView.__packValue__(this);
  }

  static __packValue__(object: CustomView): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 10001;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["6"] = object.definitionPtr.toValue();
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
    if (object.value.size > 0) {
      const packedValue: { [key: string]: any } = {};
      for (const [key, value] of object.value) {
        packedValue[String(String(key))] = value.toValue();
      }
      objectValue["21"] = packedValue;
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.position != null) {
      objectValue["40"] = object.position.toValue();
    }
    if (object.width != null) {
      objectValue["41"] = object.width.toValue();
    }
    if (object.height != null) {
      objectValue["42"] = object.height.toValue();
    }
    if (object.minWidth != null) {
      objectValue["43"] = object.minWidth.toValue();
    }
    if (object.minHeight != null) {
      objectValue["44"] = object.minHeight.toValue();
    }
    if (object.maxWidth != null) {
      objectValue["45"] = object.maxWidth.toValue();
    }
    if (object.maxHeight != null) {
      objectValue["46"] = object.maxHeight.toValue();
    }
    if (object.layout != null) {
      objectValue["50"] = object.layout;
    }
    if (object.direction != null) {
      objectValue["51"] = object.direction;
    }
    if (object.distribute != null) {
      objectValue["52"] = object.distribute;
    }
    if (object.align != null) {
      objectValue["53"] = object.align;
    }
    if (object.gap != null) {
      objectValue["54"] = object.gap.toValue();
    }
    if (object.padding != null) {
      objectValue["55"] = object.padding.toValue();
    }
    if (object.grid != null) {
      objectValue["56"] = object.grid.toValue();
    }
    if (object.gridSpan != null) {
      objectValue["57"] = object.gridSpan.toValue();
    }
    if (object.aspectRatio != null) {
      objectValue["58"] = object.aspectRatio;
    }
    if (object.isWrap != null) {
      objectValue["59"] = object.isWrap;
    }
    if (object.isVisible != null) {
      objectValue["60"] = object.isVisible;
    }
    if (object.opacity != null) {
      objectValue["61"] = object.opacity;
    }
    if (object.fill != null) {
      objectValue["62"] = object.fill.toValue();
    }
    if (object.rotation != null) {
      objectValue["63"] = object.rotation.toValue();
    }
    if (object.skew != null) {
      objectValue["64"] = object.skew.toValue();
    }
    if (object.scale != null) {
      objectValue["65"] = object.scale;
    }
    if (object.shadow != null) {
      objectValue["66"] = object.shadow.toValue();
    }
    if (object.border != null) {
      objectValue["67"] = object.border.toValue();
    }
    if (object.radius != null) {
      objectValue["68"] = object.radius.toValue();
    }
    if (object.scriptPtr != null) {
      objectValue["200"] = object.scriptPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomView {
    const layoutValue = objectValue["50"];
    const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : null;
    const directionValue = objectValue["51"];
    const unpackedDirection = directionValue != undefined ? Number(directionValue) : null;
    const distributeValue = objectValue["52"];
    const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : null;
    const alignValue = objectValue["53"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const gapValue = objectValue["54"];
    const unpackedGap =
      gapValue != undefined
        ? Axis2.fromValue(gapValue, _session, _supergraph, _graph, _connection)
        : null;
    const paddingValue = objectValue["55"];
    const unpackedPadding =
      paddingValue != undefined
        ? Insets.fromValue(paddingValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridValue = objectValue["56"];
    const unpackedGrid =
      gridValue != undefined
        ? Grid.fromValue(gridValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridSpanValue = objectValue["57"];
    const unpackedGridSpan =
      gridSpanValue != undefined
        ? GridSpan.fromValue(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectValue["58"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const isWrapValue = objectValue["59"];
    const unpackedIsWrap = isWrapValue != undefined ? isWrapValue : null;
    const isVisibleValue = objectValue["60"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectValue["61"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const fillValue = objectValue["62"];
    const unpackedFill =
      fillValue != undefined
        ? Fill.fromValue(fillValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectValue["63"];
    const unpackedRotation =
      rotationValue != undefined
        ? Axis3.fromValue(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectValue["64"];
    const unpackedSkew =
      skewValue != undefined
        ? Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectValue["65"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const shadowValue = objectValue["66"];
    const unpackedShadow =
      shadowValue != undefined
        ? Shadow.fromValue(shadowValue, _session, _supergraph, _graph, _connection)
        : null;
    const borderValue = objectValue["67"];
    const unpackedBorder =
      borderValue != undefined
        ? Border.fromValue(borderValue, _session, _supergraph, _graph, _connection)
        : null;
    const radiusValue = objectValue["68"];
    const unpackedRadius =
      radiusValue != undefined
        ? Corners.fromValue(radiusValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["40"];
    const unpackedPosition =
      positionValue != undefined
        ? Position.fromValue(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["41"];
    const unpackedWidth =
      widthValue != undefined
        ? Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectValue["42"];
    const unpackedHeight =
      heightValue != undefined
        ? Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectValue["43"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["44"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["45"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["46"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
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
    const scriptPtrValue = objectValue["200"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const unpackedValue = new Map();
    if (objectValue["21"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["21"])) {
        unpackedValue.set(
          String(key),
          Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomView({
      definition: NodeReference.fromValue(
        objectValue["6"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
      script: unpackedScriptPtr,
      deletedAt: unpackedDeletedAt,
      value: unpackedValue,
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
  ): CustomView {
    return CustomView.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CustomViewProto {
    return CustomView.__packProto__(this);
  }

  static __packProto__(object: CustomView): CustomViewProto {
    const objectProto: Partial<CustomViewProto> = { metatype: 10001 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.definitionPtr = object.definitionPtr.toProto();
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
    if (object.value) {
      objectProto.value = {};
      for (const [key, value] of object.value) {
        objectProto.value![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
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
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    return objectProto as CustomViewProto;
  }

  static __unpackProto__(
    objectProto: CustomViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomView {
    const unpackedValue = new Map();
    if (objectProto.value) {
      for (const [key, value] of Object.entries(objectProto.value)) {
        unpackedValue.set(
          String(key),
          Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomView({
      definition: NodeReference.fromProto(
        objectProto.definitionPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      layout: objectProto.layout != undefined ? (Number(objectProto.layout) as Layout) : null,
      direction:
        objectProto.direction != undefined ? (Number(objectProto.direction) as Direction) : null,
      distribute:
        objectProto.distribute != undefined ? (Number(objectProto.distribute) as Distribute) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as Align) : null,
      gap:
        objectProto.gap != undefined
          ? Axis2.fromProto(objectProto.gap!, _session, _supergraph, _graph, _connection)
          : null,
      padding:
        objectProto.padding != undefined
          ? Insets.fromProto(objectProto.padding!, _session, _supergraph, _graph, _connection)
          : null,
      grid:
        objectProto.grid != undefined
          ? Grid.fromProto(objectProto.grid!, _session, _supergraph, _graph, _connection)
          : null,
      gridSpan:
        objectProto.gridSpan != undefined
          ? GridSpan.fromProto(objectProto.gridSpan!, _session, _supergraph, _graph, _connection)
          : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      fill:
        objectProto.fill != undefined
          ? Fill.fromProto(objectProto.fill!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? Axis3.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
      shadow:
        objectProto.shadow != undefined
          ? Shadow.fromProto(objectProto.shadow!, _session, _supergraph, _graph, _connection)
          : null,
      border:
        objectProto.border != undefined
          ? Border.fromProto(objectProto.border!, _session, _supergraph, _graph, _connection)
          : null,
      radius:
        objectProto.radius != undefined
          ? Corners.fromProto(objectProto.radius!, _session, _supergraph, _graph, _connection)
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
      position:
        objectProto.position != undefined
          ? Position.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Dimension.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? Dimension.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? Dimension.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? Dimension.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? Dimension.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? Dimension.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
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
      id: String(objectProto.id),
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
      script:
        objectProto.scriptPtr != undefined
          ? NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      value: unpackedValue,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomView {
    return CustomView.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CustomView {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomViewProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_VIEW, CustomView);
/* ==== DESTACK_GENERATED_END:NODE:10001 ==== */
