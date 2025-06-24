import {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Entity,
  Graph,
  Grid,
  GridSpan,
  Insets,
  IsDeletable,
  IsExtensible,
  IsOrdered,
  IsScriptable,
  IsSubject,
  IsTaggable,
  IsTracked,
  IsVisual,
  Layout,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Position,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
  Value,
  Vector2,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Border, Fill, Shadow } from "@destack/language/style";
import { ContainerView, View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:10020 ==== */
export class FrameView
  extends Node
  implements
    Spatial,
    Entity,
    IsTracked,
    IsDeletable,
    IsExtensible,
    IsOrdered,
    IsTaggable,
    IsScriptable,
    IsVisual,
    View,
    ContainerView
{
  static metatype: NodeType = NodeType.FRAME_VIEW;
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
    TraitType.SCRIPTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.CANVAS,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
  ];
  static __childTypes__: NodeType[] = [
    NodeType.FIELD,
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
    NodeType.PLANE_SHAPE,
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
  ];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.FOLDER,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.CANVAS,
    NodeType.LAYER,
  ];
  static __descendantTypes__: NodeType[] = [
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.VARIANT,
    NodeType.SHADOW_STYLE,
    NodeType.FIELD,
    NodeType.TEXT_VIEW,
    NodeType.OPTION,
    NodeType.THREAD_VIEW,
    NodeType.PALETTE,
    NodeType.TAGGING,
    NodeType.COLOR_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.CANVAS,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
  ];

  get parent(): Window | Scene | Layer | (Node & ContainerView) | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Window | Scene | Layer | (Node & ContainerView) | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  value: Map<string, Value>;
  readonly orderKey: string;
  name: string;
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  layout: Layout | null;
  direction: Direction | null;
  distribute: Distribute | null;
  align: Align | null;
  gap: Axis2 | null;
  padding: Insets | null;
  grid: Grid | null;
  gridSpan: GridSpan | null;
  aspectRatio: number | null;
  isWrap: boolean | null;
  isVisible: boolean | null;
  opacity: number | null;
  fill: Fill | null;
  rotation: Axis3 | null;
  skew: Vector2 | null;
  scale: number | null;
  shadow: Shadow | null;
  border: Border | null;
  radius: Corners | null;
  get script(): Script | null | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null | null;
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
    materialization?: MaterializationType;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`FrameView.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _value = options.value ?? null;
    if (_value === null) {
      throw new Error(`FrameView.value is required`);
    }
    this.value = _value;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`FrameView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`FrameView.name is required`);
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
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FRAME_VIEW,
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
}
/* ==== DESTACK_GENERATED_END:NODE:10020 ==== */
