import { Canvas } from "@destack/language/canvas";
import {
  Align,
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Direction,
  Distribute,
  Graph,
  Grid,
  GridSpan,
  HasIcon,
  Icon,
  Insets,
  IsOwnable,
  IsOwner,
  IsSubject,
  Layout,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Position,
  QueryConnection,
  Session,
  StructType,
  Supergraph,
  TraitType,
  Value,
  Vector2,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Border, Fill, Shadow } from "@destack/language/style";
import { ContainerView } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:9020 ==== */
/**
 * LayerType
 */
export enum LayerType {
  GENERAL = 1,
  SHAPE = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:9020 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9020 ==== */
/**
 * A Layer is a named container for Views.
 */
export class Layer extends Node implements ContainerView, HasIcon, IsOwnable {
  static metatype: NodeType = NodeType.LAYER;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.VIEW,
    TraitType.ENTITY,
    TraitType.TAGGABLE,
    TraitType.CONTAINER_VIEW,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.EXTENSIBLE,
    TraitType.ORDERED,
    TraitType.SCRIPTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.CANVAS, NodeType.SCENE];
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
    NodeType.VARIANT,
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

  /**
   * Layer.parent
   */
  get parent(): Scene | Canvas | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | Canvas | null;
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
  readonly materialization: MaterializationType;

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
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
    }
    return null;
  }
  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;

  /**
   * Layer.type
   */
  type: LayerType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

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
    parent?: Scene | Canvas | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    value?: Map<string, Value>;
    orderKey?: string;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    type?: LayerType;
    name: string;
    icon?: Icon | null;
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
      throw new Error(`Layer.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _value = options.value ?? null;
    if (_value === null) {
      throw new Error(`Layer.value is required`);
    }
    this.value = _value;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Layer.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = LayerType.GENERAL;
    }
    if (_type === null) {
      throw new Error(`Layer.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Layer.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
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
      nodeType: NodeType.LAYER,
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

  toValue(): { [key: string]: any } {
    return Layer.__packValue__(this);
  }

  static __packValue__(object: Layer): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9020;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt !== null) {
      objectValue["20"] = object.deletedAt.toString();
    }
    if (object.value) {
      const packedValue: { [key: string]: any } = {};
      for (const [key, value] of Object.entries(object.value)) {
        packedValue[String(String(key))] = value.toValue();
      }
      objectValue["21"] = packedValue;
    }
    objectValue["22"] = object.orderKey;
    if (object.ownedByPtr !== null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.position !== null) {
      objectValue["40"] = object.position.toValue();
    }
    if (object.width !== null) {
      objectValue["41"] = object.width.toValue();
    }
    if (object.height !== null) {
      objectValue["42"] = object.height.toValue();
    }
    if (object.minWidth !== null) {
      objectValue["43"] = object.minWidth.toValue();
    }
    if (object.minHeight !== null) {
      objectValue["44"] = object.minHeight.toValue();
    }
    if (object.maxWidth !== null) {
      objectValue["45"] = object.maxWidth.toValue();
    }
    if (object.maxHeight !== null) {
      objectValue["46"] = object.maxHeight.toValue();
    }
    if (object.layout !== null) {
      objectValue["50"] = object.layout;
    }
    if (object.direction !== null) {
      objectValue["51"] = object.direction;
    }
    if (object.distribute !== null) {
      objectValue["52"] = object.distribute;
    }
    if (object.align !== null) {
      objectValue["53"] = object.align;
    }
    if (object.gap !== null) {
      objectValue["54"] = object.gap.toValue();
    }
    if (object.padding !== null) {
      objectValue["55"] = object.padding.toValue();
    }
    if (object.grid !== null) {
      objectValue["56"] = object.grid.toValue();
    }
    if (object.gridSpan !== null) {
      objectValue["57"] = object.gridSpan.toValue();
    }
    if (object.aspectRatio !== null) {
      objectValue["58"] = object.aspectRatio;
    }
    if (object.isWrap !== null) {
      objectValue["59"] = object.isWrap;
    }
    if (object.isVisible !== null) {
      objectValue["60"] = object.isVisible;
    }
    if (object.opacity !== null) {
      objectValue["61"] = object.opacity;
    }
    if (object.fill !== null) {
      objectValue["62"] = object.fill.toValue();
    }
    if (object.rotation !== null) {
      objectValue["63"] = object.rotation.toValue();
    }
    if (object.skew !== null) {
      objectValue["64"] = object.skew.toValue();
    }
    if (object.scale !== null) {
      objectValue["65"] = object.scale;
    }
    if (object.shadow !== null) {
      objectValue["66"] = object.shadow.toValue();
    }
    if (object.border !== null) {
      objectValue["67"] = object.border.toValue();
    }
    if (object.radius !== null) {
      objectValue["68"] = object.radius.toValue();
    }
    if (object.scriptPtr !== null) {
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
  ): Layer {
    const layoutValue = objectValue["50"];
    const unpackedLayout = layoutValue !== undefined ? Number(layoutValue) : null;
    const directionValue = objectValue["51"];
    const unpackedDirection = directionValue !== undefined ? Number(directionValue) : null;
    const distributeValue = objectValue["52"];
    const unpackedDistribute = distributeValue !== undefined ? Number(distributeValue) : null;
    const alignValue = objectValue["53"];
    const unpackedAlign = alignValue !== undefined ? Number(alignValue) : null;
    const gapValue = objectValue["54"];
    const unpackedGap =
      gapValue !== undefined ? Axis2.fromValue(gapValue, _session, _supergraph, _graph, _connection) : null;
    const paddingValue = objectValue["55"];
    const unpackedPadding =
      paddingValue !== undefined ? Insets.fromValue(paddingValue, _session, _supergraph, _graph, _connection) : null;
    const gridValue = objectValue["56"];
    const unpackedGrid =
      gridValue !== undefined ? Grid.fromValue(gridValue, _session, _supergraph, _graph, _connection) : null;
    const gridSpanValue = objectValue["57"];
    const unpackedGridSpan =
      gridSpanValue !== undefined
        ? GridSpan.fromValue(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectValue["58"];
    const unpackedAspectRatio = aspectRatioValue !== undefined ? aspectRatioValue : null;
    const isWrapValue = objectValue["59"];
    const unpackedIsWrap = isWrapValue !== undefined ? isWrapValue : null;
    const isVisibleValue = objectValue["60"];
    const unpackedIsVisible = isVisibleValue !== undefined ? isVisibleValue : null;
    const opacityValue = objectValue["61"];
    const unpackedOpacity = opacityValue !== undefined ? opacityValue : null;
    const fillValue = objectValue["62"];
    const unpackedFill =
      fillValue !== undefined ? Fill.fromValue(fillValue, _session, _supergraph, _graph, _connection) : null;
    const rotationValue = objectValue["63"];
    const unpackedRotation =
      rotationValue !== undefined ? Axis3.fromValue(rotationValue, _session, _supergraph, _graph, _connection) : null;
    const skewValue = objectValue["64"];
    const unpackedSkew =
      skewValue !== undefined ? Vector2.fromValue(skewValue, _session, _supergraph, _graph, _connection) : null;
    const scaleValue = objectValue["65"];
    const unpackedScale = scaleValue !== undefined ? scaleValue : null;
    const shadowValue = objectValue["66"];
    const unpackedShadow =
      shadowValue !== undefined ? Shadow.fromValue(shadowValue, _session, _supergraph, _graph, _connection) : null;
    const borderValue = objectValue["67"];
    const unpackedBorder =
      borderValue !== undefined ? Border.fromValue(borderValue, _session, _supergraph, _graph, _connection) : null;
    const radiusValue = objectValue["68"];
    const unpackedRadius =
      radiusValue !== undefined ? Corners.fromValue(radiusValue, _session, _supergraph, _graph, _connection) : null;
    const positionValue = objectValue["40"];
    const unpackedPosition =
      positionValue !== undefined
        ? Position.fromValue(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["41"];
    const unpackedWidth =
      widthValue !== undefined ? Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection) : null;
    const heightValue = objectValue["42"];
    const unpackedHeight =
      heightValue !== undefined ? Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection) : null;
    const minWidthValue = objectValue["43"];
    const unpackedMinWidth =
      minWidthValue !== undefined
        ? Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["44"];
    const unpackedMinHeight =
      minHeightValue !== undefined
        ? Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["45"];
    const unpackedMaxWidth =
      maxWidthValue !== undefined
        ? Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["46"];
    const unpackedMaxHeight =
      maxHeightValue !== undefined
        ? Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue !== undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const unpackedValue: { [key: string]: any } = {};
    if (objectValue["21"] !== undefined) {
      for (const [key, value] of Object.entries(objectValue["21"])) {
        unpackedValue[String(key)] = Value.fromValue(value, _session, _supergraph, _graph, _connection);
      }
    }
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const scriptValue = objectValue["200"];
    const unpackedScript =
      scriptValue !== undefined
        ? NodeReference.fromValue(scriptValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByValue = objectValue["25"];
    const unpackedOwnedBy =
      ownedByValue !== undefined
        ? NodeReference.fromValue(ownedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Layer({
      type: Number(objectValue["30"]),
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
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      value: unpackedValue,
      icon: unpackedIcon,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      script: unpackedScript,
      ownedBy: unpackedOwnedBy,
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
  ): Layer {
    return Layer.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:9020 ==== */
