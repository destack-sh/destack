import { IsOrdered, Axis2, Layer, IsTracked, Organization, Script, MaterializationType, Distribute, StructFrozen, Fill, FrameView, IsScriptable, Supergraph, Struct, Shadow, Folder, Space, PlaneShape, NodeType, Entity, Corners, Direction, LabelView, Node, Analytic, IsTaggable, AnnotationShape, Agent, StructType, IsFrozen, Value, QueryConnection, Particle, Position, CustomView, Event, Insets, Axis3, Grid, IsDeletable, Dimension, Graph, Canvas, NodeReference, IsOwnable, Layout, Window, View, Session, IsExtensible, User, Border, CustomViewDefinition, Align, BuiltinObject, Vector2, EnumType, Spatial, ContainerView, Role, Icon, GridSpan, IsVisual, SplitView, Team, Indexed } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:9011 ==== */
export enum SceneEventType {
  ENTERED = 1,
  EXITED = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:9011 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9011 ==== */
export class SceneEvent extends Node implements Spatial, Particle, Analytic, Indexed, Event, IsFrozen, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  type: SceneEventType;
  get node(): Scene | null {
      const nodePtr: NodeReference | null = this.nodePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | null;
      }
      return null;
  }

  set node(value: Scene) {
      if (value === null) {
          this.nodePtr = null;
      } else {
          this.nodePtr = value.toRef();
      }
  }
  ;
  nodePtr: NodeReference

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: SceneEventType,
    nodePtr: NodeReference,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.type = type;
    this.nodePtr = nodePtr;
  }


  static create(options: {
    type: SceneEventType,
    node: Scene | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): SceneEvent {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new SceneEvent(
      options.type,
      options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? options.node : options.node.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
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
    return new NodeReference(NodeType.SCENE_EVENT, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "SceneEvent[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:9011 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9010 ==== */
export class Scene extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsExtensible, IsOrdered, IsOwnable, IsTaggable, IsScriptable, IsVisual, View, ContainerView {
  readonly id: string;
  get parent(): Folder | Scene | Window | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | Scene | Window | null | null;
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
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  name: string;
  icon: Icon | null;
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
  get rootView(): CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null {
      const nodePtr: NodeReference | null = this.rootViewPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null;
      }
      return null;
  }

  set rootView(value: CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null) {
      if (value === null) {
          this.rootViewPtr = null;
      } else {
          this.rootViewPtr = value.toRef();
      }
  }
  ;
  rootViewPtr: NodeReference | null
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
    ownedByPtr: NodeReference | null,
    name: string,
    icon: Icon | null,
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
    rootViewPtr: NodeReference | null,
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
    this.ownedByPtr = ownedByPtr;
    this.name = name;
    this.icon = icon;
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
    this.rootViewPtr = rootViewPtr;
    this.scriptPtr = scriptPtr;
  }


  static create(options: {
    value?: Map<string, Value>,
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    name: string,
    icon?: Icon | null,
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
    rootView?: CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | NodeReference | null,
    script?: Script | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Scene {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Scene(
      options.value ?? new Map(),
      options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? options.ownedBy : options.ownedBy.toRef()) : null,
      options.name,
      options.icon ?? null,
      options.position ?? null,
      options.width ?? null,
      options.height ?? null,
      options.minWidth ?? null,
      options.minHeight ?? null,
      options.maxWidth ?? null,
      options.maxHeight ?? null,
      options.layout ?? null,
      options.direction ?? null,
      options.distribute ?? null,
      options.align ?? null,
      options.gap ?? null,
      options.padding ?? null,
      options.grid ?? null,
      options.gridSpan ?? null,
      options.aspectRatio ?? null,
      options.isWrap ?? null,
      options.isVisible ?? null,
      options.opacity ?? null,
      options.fill ?? null,
      options.rotation ?? null,
      options.skew ?? null,
      options.scale ?? null,
      options.shadow ?? null,
      options.border ?? null,
      options.radius ?? null,
      options.rootView != null ? (options.rootView.metatype == StructType.NODE_REFERENCE ? options.rootView : options.rootView.toRef()) : null,
      options.script != null ? (options.script.metatype == StructType.NODE_REFERENCE ? options.script : options.script.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
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
    return new NodeReference(NodeType.SCENE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:9010 ==== */