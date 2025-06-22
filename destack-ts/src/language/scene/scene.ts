import { PlaneShape, Agent, IsTaggable, ContainerView, IsScriptable, BuiltinObject, ACTIVE_SESSION, IsVisual, Folder, Organization, Layout, Align, Session, Corners, View, StructFrozen, Vector2, IsFrozen, IsOrdered, Graph, Distribute, Struct, GridSpan, AnnotationShape, Event, activeSession, Particle, Axis2, StructType, Insets, Border, NodeType, Window, Indexed, Canvas, Role, FrameView, Dimension, Icon, Direction, Supergraph, Axis3, QueryConnection, NodeReference, MaterializationType, IsOwnable, Value, User, Analytic, Position, CustomView, IsExtensible, Spatial, LabelView, Team, Fill, Space, CustomViewDefinition, Shadow, IsDeletable, Entity, Grid, EnumType, SplitView, Script, Layer, Node, IsTracked } from '@/language';
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

  set node(node: Scene) {
      this.nodePtr = node.toRef();
  }
  ;
  nodePtr: NodeReference

  constructor(options: {
    type: SceneEventType,
    node: Scene | NodeReference,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.nodePtr = options.node != null ? (options.node.metatype == StructType.NODE_REFERENCE ? (options.node as NodeReference) : (options.node as Node).toRef()) : null;
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

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
      if (node === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = node.toRef();
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

  set rootView(node: CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null) {
      if (node === null) {
          this.rootViewPtr = null;
      } else {
          this.rootViewPtr = node.toRef();
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

  set script(node: Script | null) {
      if (node === null) {
          this.scriptPtr = null;
      } else {
          this.scriptPtr = node.toRef();
      }
  }
  ;
  scriptPtr: NodeReference | null

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.value = options.value ?? new Map();
    this.ownedByPtr = options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? (options.ownedBy as NodeReference) : (options.ownedBy as Node).toRef()) : null;
    this.name = options.name;
    this.icon = options.icon ?? null;
    this.position = options.position ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
    this.minWidth = options.minWidth ?? null;
    this.minHeight = options.minHeight ?? null;
    this.maxWidth = options.maxWidth ?? null;
    this.maxHeight = options.maxHeight ?? null;
    this.layout = options.layout ?? null;
    this.direction = options.direction ?? null;
    this.distribute = options.distribute ?? null;
    this.align = options.align ?? null;
    this.gap = options.gap ?? null;
    this.padding = options.padding ?? null;
    this.grid = options.grid ?? null;
    this.gridSpan = options.gridSpan ?? null;
    this.aspectRatio = options.aspectRatio ?? null;
    this.isWrap = options.isWrap ?? null;
    this.isVisible = options.isVisible ?? null;
    this.opacity = options.opacity ?? null;
    this.fill = options.fill ?? null;
    this.rotation = options.rotation ?? null;
    this.skew = options.skew ?? null;
    this.scale = options.scale ?? null;
    this.shadow = options.shadow ?? null;
    this.border = options.border ?? null;
    this.radius = options.radius ?? null;
    this.rootViewPtr = options.rootView != null ? (options.rootView.metatype == StructType.NODE_REFERENCE ? (options.rootView as NodeReference) : (options.rootView as Node).toRef()) : null;
    this.scriptPtr = options.script != null ? (options.script.metatype == StructType.NODE_REFERENCE ? (options.script as NodeReference) : (options.script as Node).toRef()) : null;
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