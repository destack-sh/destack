import { IsOrdered, Theme, Layer, Style, ThreadView, NumberInputView, File, IsTracked, MaterializationType, StructFrozen, FrameView, TextView, Supergraph, Struct, PlaneShape, Space, NodeType, Entity, Scene, LineShape, LabelView, Node, IsTaggable, ArrowShape, Agent, StructType, CustomView, QueryConnection, IsDeletable, Color, Graph, Canvas, NodeReference, WizardView, Session, User, CustomViewDefinition, BuiltinObject, Gradient, EnumType, Spatial, SplitView, SliderInputView, IsVisual, AnnotationShape } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12034 ==== */
export enum FillType {
  STYLE = 2,
  SOLID = 10,
  GRADIENT = 11,
  IMAGE = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:12034 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12035 ==== */
export enum FillPosition {
  TOP_LEFT = 1,
  TOP_CENTER = 2,
  TOP_RIGHT = 3,
  LEFT = 10,
  CENTER = 11,
  RIGHT = 12,
  BOTTOM_LEFT = 20,
  BOTTOM_CENTER = 21,
  BOTTOM_RIGHT = 22,
}
/* ==== DESTACK_GENERATED_END:ENUM:12035 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12036 ==== */
export enum FillSize {
  FILL = 1,
  STRETCH = 2,
  FIT = 3,
  TILE = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12036 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12017 ==== */
export class Fill extends Struct {
  type: FillType;
  get style(): FillStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as FillStyle | null;
      }
      return null;
  }

  set style(value: FillStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  color: Color | null;
  gradient: Gradient | null;
  get image(): File | null {
      const nodePtr: NodeReference | null = this.imagePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as File | null;
      }
      return null;
  }

  set image(value: File | null) {
      if (value == null) {
          this.imagePtr = null;
      } else {
          this.imagePtr = value.toRef();
      }
  }
  ;
  imagePtr: NodeReference | null
  position: FillPosition | null;
  size: FillSize | null;

  constructor(
    type: FillType,
    stylePtr: NodeReference | null,
    color: Color | null,
    gradient: Gradient | null,
    imagePtr: NodeReference | null,
    position: FillPosition | null,
    size: FillSize | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.color = color;
    this.gradient = gradient;
    this.imagePtr = imagePtr;
    this.position = position;
    this.size = size;
  }


  static create(options: {
    type: FillType,
    style?: FillStyle | NodeReference | null,
    color?: Color | null,
    gradient?: Gradient | null,
    image?: File | NodeReference | null,
    position?: FillPosition | null,
    size?: FillSize | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Fill {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Fill(
      options.type,
      options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? options.style : options.style.toRef()) : null,
      options.color ?? null,
      options.gradient ?? null,
      options.image != null ? (options.image.metatype == StructType.NODE_REFERENCE ? options.image : options.image.toRef()) : null,
      options.position ?? null,
      options.size ?? null,
      supergraph
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12017 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12021 ==== */
export class FillStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
  readonly id: string;
  get parent(): Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null | null;
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
  type: FillType;
  name: string;
  color: Color | null;
  gradient: Gradient | null;
  get image(): File | null | null {
      const nodePtr: NodeReference | null = this.imagePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as File | null | null;
      }
      return null;
  }

  set image(value: File | null) {
      if (value === null) {
          this.imagePtr = null;
      } else {
          this.imagePtr = value.toRef();
      }
  }
  ;
  imagePtr: NodeReference | null
  position: FillPosition | null;
  size: FillSize | null;

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
    type: FillType,
    name: string,
    color: Color | null,
    gradient: Gradient | null,
    imagePtr: NodeReference | null,
    position: FillPosition | null,
    size: FillSize | null,
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
    this.color = color;
    this.gradient = gradient;
    this.imagePtr = imagePtr;
    this.position = position;
    this.size = size;
  }


  static create(options: {
    type: FillType,
    name: string,
    color?: Color | null,
    gradient?: Gradient | null,
    image?: File | NodeReference | null,
    position?: FillPosition | null,
    size?: FillSize | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): FillStyle {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new FillStyle(
      options.type,
      options.name,
      options.color ?? null,
      options.gradient ?? null,
      options.image != null ? (options.image.metatype == StructType.NODE_REFERENCE ? options.image : options.image.toRef()) : null,
      options.position ?? null,
      options.size ?? null,
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
    return new NodeReference(NodeType.FILL_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12021 ==== */