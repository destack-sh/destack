import {
  Agent,
  Align,
  AnnotationShape,
  Canvas,
  ContentView,
  CustomView,
  CustomViewDefinition,
  Dimension,
  Entity,
  FrameView,
  Graph,
  IsDeletable,
  IsOrdered,
  IsScriptable,
  IsShape,
  IsTaggable,
  IsTracked,
  IsVisual,
  LabelView,
  Layer,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  PlaneShape,
  Position,
  QueryConnection,
  Scene,
  Script,
  Session,
  Space,
  Spatial,
  SplitView,
  StructType,
  Supergraph,
  User,
  Vector2,
  View,
  Window,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:11012 ==== */
export enum ArrowHeadType {
  ARROW = 1,
  TRIANGLE = 2,
  DOT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:11012 ==== */

/* ==== DESTACK_GENERATED_START:NODE:11012 ==== */
export class ArrowShape
  extends Node
  implements
    Spatial,
    Entity,
    IsTracked,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsScriptable,
    IsVisual,
    View,
    ContentView,
    IsShape
{
  readonly id: string;
  get parent():
    | Window
    | Scene
    | Layer
    | CustomViewDefinition
    | CustomView
    | FrameView
    | LabelView
    | SplitView
    | AnnotationShape
    | Canvas
    | PlaneShape
    | Layer
    | Scene
    | null
    | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Window
        | Scene
        | Layer
        | CustomViewDefinition
        | CustomView
        | FrameView
        | LabelView
        | SplitView
        | AnnotationShape
        | Canvas
        | PlaneShape
        | Layer
        | Scene
        | null
        | null;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  readonly orderKey: string;
  name: string;
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  align: Align | null;
  isVisible: boolean | null;
  opacity: number | null;
  startType: ArrowHeadType;
  start: Vector2;
  endType: ArrowHeadType;
  end: Vector2;
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
    id: string;
    parent?:
      | Window
      | Scene
      | Layer
      | CustomViewDefinition
      | CustomView
      | FrameView
      | LabelView
      | SplitView
      | AnnotationShape
      | Canvas
      | PlaneShape
      | Layer
      | Scene
      | NodeReference
      | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    name: string;
    position?: Position | null;
    width?: Dimension | null;
    height?: Dimension | null;
    minWidth?: Dimension | null;
    minHeight?: Dimension | null;
    maxWidth?: Dimension | null;
    maxHeight?: Dimension | null;
    align?: Align | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
    script?: Script | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.deletedAt = options.deletedAt ?? null;
    this.orderKey = options.orderKey ?? "a0";
    this.name = options.name;
    this.position = options.position ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
    this.minWidth = options.minWidth ?? null;
    this.minHeight = options.minHeight ?? null;
    this.maxWidth = options.maxWidth ?? null;
    this.maxHeight = options.maxHeight ?? null;
    this.align = options.align ?? null;
    this.isVisible = options.isVisible ?? null;
    this.opacity = options.opacity ?? null;
    this.startType = options.startType;
    this.start = options.start;
    this.endType = options.endType;
    this.end = options.end;
    this.scriptPtr =
      options.script != null
        ? options.script.metatype == StructType.NODE_REFERENCE
          ? (options.script as NodeReference)
          : (options.script as Node).toRef()
        : null;
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
      nodeType: NodeType.ARROW_SHAPE,
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
/* ==== DESTACK_GENERATED_END:NODE:11012 ==== */
