import { PlaneShape, Graph, IsTaggable, IsDeletable, Position, Grid, MaterializationType, Spatial, StructFrozen, CustomViewDefinition, IsScriptable, Struct, GridSpan, EnumType, IsExtensible, Script, IsTracked, Entity, IsVisual, Vector2, Shadow, QueryConnection, NodeReference, Axis2, SplitView, Node, Value, Scene, Layer, CustomView, Align, FrameView, Fill, Dimension, Axis3, Distribute, Border, Layout, Direction, Corners, Insets, AnnotationShape, IsOrdered, NodeType, Session, View, ContainerView, Agent, Space, User, Window, StructType, Canvas, Supergraph, BuiltinObject } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:10030 ==== */
export class LabelView extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsExtensible, IsOrdered, IsTaggable, IsScriptable, IsVisual, View, ContainerView {
  readonly id: string;
  get parent(): Window | Scene | Layer | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Window | Scene | Layer | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
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

  set script(value: Script | null) {
      if (value === null) {
          this.scriptPtr = null;
      } else {
          this.scriptPtr = value.toRef();
      }
  }
  ;
  scriptPtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    value: Map<string, Value>,
    orderKey: string,
    name: string,
    position: Position | null,
    width: Dimension | null,
    height: Dimension | null,
    minWidth: Dimension | null,
    minHeight: Dimension | null,
    maxWidth: Dimension | null,
    maxHeight: Dimension | null,
    layout: Layout | null,
    direction: Direction | null,
    distribute: Distribute | null,
    align: Align | null,
    gap: Axis2 | null,
    padding: Insets | null,
    grid: Grid | null,
    gridSpan: GridSpan | null,
    aspectRatio: number | null,
    isWrap: boolean | null,
    isVisible: boolean | null,
    opacity: number | null,
    fill: Fill | null,
    rotation: Axis3 | null,
    skew: Vector2 | null,
    scale: number | null,
    shadow: Shadow | null,
    border: Border | null,
    radius: Corners | null,
    scriptPtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.value = value;
    this.orderKey = orderKey;
    this.name = name;
    this.position = position;
    this.width = width;
    this.height = height;
    this.minWidth = minWidth;
    this.minHeight = minHeight;
    this.maxWidth = maxWidth;
    this.maxHeight = maxHeight;
    this.layout = layout;
    this.direction = direction;
    this.distribute = distribute;
    this.align = align;
    this.gap = gap;
    this.padding = padding;
    this.grid = grid;
    this.gridSpan = gridSpan;
    this.aspectRatio = aspectRatio;
    this.isWrap = isWrap;
    this.isVisible = isVisible;
    this.opacity = opacity;
    this.fill = fill;
    this.rotation = rotation;
    this.skew = skew;
    this.scale = scale;
    this.shadow = shadow;
    this.border = border;
    this.radius = radius;
    this.scriptPtr = scriptPtr;
  }


  static create(options: {
    value?: Map<string, Value>,
    name: string,
    position?: Position | null,
    width?: Dimension | null,
    height?: Dimension | null,
    minWidth?: Dimension | null,
    minHeight?: Dimension | null,
    maxWidth?: Dimension | null,
    maxHeight?: Dimension | null,
    layout?: Layout | null,
    direction?: Direction | null,
    distribute?: Distribute | null,
    align?: Align | null,
    gap?: Axis2 | null,
    padding?: Insets | null,
    grid?: Grid | null,
    gridSpan?: GridSpan | null,
    aspectRatio?: number | null,
    isWrap?: boolean | null,
    isVisible?: boolean | null,
    opacity?: number | null,
    fill?: Fill | null,
    rotation?: Axis3 | null,
    skew?: Vector2 | null,
    scale?: number | null,
    shadow?: Shadow | null,
    border?: Border | null,
    radius?: Corners | null,
    script?: Script | NodeReference | null
  }): LabelView {

    return new LabelView(

    );
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.LABEL_VIEW, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:10030 ==== */