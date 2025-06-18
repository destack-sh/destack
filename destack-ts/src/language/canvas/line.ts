import { PlaneShape, LabelView, Graph, IsTaggable, IsShape, IsDeletable, Position, MaterializationType, Spatial, StructFrozen, CustomViewDefinition, IsScriptable, Struct, EnumType, Script, IsTracked, Entity, IsVisual, Vector2, QueryConnection, NodeReference, SplitView, Node, Scene, Layer, CustomView, Align, FrameView, Dimension, AnnotationShape, IsOrdered, NodeType, Session, View, ContentView, Agent, Space, User, Window, StructType, Canvas, Color, Supergraph, BuiltinObject } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:11010 ==== */
export enum LineType {
  SOLID = 1,
  DASHED = 2,
  DOTTED = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:11010 ==== */

/* ==== DESTACK_GENERATED_START:NODE:11010 ==== */
export class LineShape extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsScriptable, IsVisual, View, ContentView, IsShape {
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
  readonly orderKey: string;
  type: LineType;
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
  points: Array<Vector2>;
  color: Color | null;
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
    orderKey: string,
    type: LineType,
    name: string,
    position: Position | null,
    width: Dimension | null,
    height: Dimension | null,
    minWidth: Dimension | null,
    minHeight: Dimension | null,
    maxWidth: Dimension | null,
    maxHeight: Dimension | null,
    align: Align | null,
    isVisible: boolean | null,
    opacity: number | null,
    points: Array<Vector2>,
    color: Color | null,
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
    this.orderKey = orderKey;
    this.type = type;
    this.name = name;
    this.position = position;
    this.width = width;
    this.height = height;
    this.minWidth = minWidth;
    this.minHeight = minHeight;
    this.maxWidth = maxWidth;
    this.maxHeight = maxHeight;
    this.align = align;
    this.isVisible = isVisible;
    this.opacity = opacity;
    this.points = points;
    this.color = color;
    this.scriptPtr = scriptPtr;
  }


  static create(options: {
    type: LineType,
    name: string,
    position?: Position | null,
    width?: Dimension | null,
    height?: Dimension | null,
    minWidth?: Dimension | null,
    minHeight?: Dimension | null,
    maxWidth?: Dimension | null,
    maxHeight?: Dimension | null,
    align?: Align | null,
    isVisible?: boolean | null,
    opacity?: number | null,
    points?: Array<Vector2>,
    color?: Color | null,
    script?: Script | NodeReference | null
  }): LineShape {

    return new LineShape(

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
    return new NodeReference(NodeType.LINE_SHAPE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:11010 ==== */