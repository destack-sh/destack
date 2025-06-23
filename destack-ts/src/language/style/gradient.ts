import { IsTracked, SliderInputView, ACTIVE_SESSION, WizardView, IsVisual, Spatial, EnumType, IsOrdered, ArrowShape, MaterializationType, Entity, AnnotationShape, Space, StructType, CustomView, SplitView, Struct, NumberInputView, Theme, Axis2, LabelView, PlaneShape, ThreadView, Scene, NodeReference, Color, NodeType, LineShape, Graph, User, Layer, Agent, IsTaggable, Node, QueryConnection, CustomViewDefinition, StructFrozen, TextView, Canvas, Supergraph, IsDeletable, FrameView, BuiltinObject, Session, activeSession, Style } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12033 ==== */
export enum GradientType {
  STYLE = 2,
  LINEAR = 10,
  RADIAL = 11,
  CONIC = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:12033 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12015 ==== */
export class GradientStop extends StructFrozen {
  readonly color: Color | null;
  readonly position: number;

  constructor(options: {
    color?: Color | null,
    position: number,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        supergraph,
    );

    this.color = options.color ?? null;
    this.position = options.position;
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12015 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12016 ==== */
export class Gradient extends Struct {
  type: GradientType;
  get style(): GradientStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as GradientStyle | null;
      }
      return null;
  }

  set style(value: GradientStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  angle: number | null;
  stops: Array<GradientStop>;
  centerAnchor: Axis2 | null;

  constructor(options: {
    type?: GradientType,
    style?: GradientStyle | NodeReference | null,
    angle?: number | null,
    stops?: Array<GradientStop>,
    centerAnchor?: Axis2 | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        supergraph,
    );

    this.type = options.type ?? GradientType.LINEAR;
    this.stylePtr = options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? (options.style as NodeReference) : (options.style as Node).toRef()) : null;
    this.angle = options.angle ?? null;
    this.stops = options.stops ?? [];
    this.centerAnchor = options.centerAnchor ?? null;
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12016 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12025 ==== */
export class GradientStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
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
  type: GradientType;
  name: string;
  angle: number | null;
  stops: Array<GradientStop>;
  centerAnchor: Axis2 | null;
  dark: Gradient | null;

  constructor(options: {
    id: string,
    parent?: Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | NodeReference | null,
    space?: Space | NodeReference | null,
    materialization?: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
    deletedAt?: Temporal.ZonedDateTime | null,
    orderKey?: string,
    type?: GradientType,
    name: string,
    angle?: number | null,
    stops?: Array<GradientStop>,
    centerAnchor?: Axis2 | null,
    dark?: Gradient | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    super(
        // id
        options.id,
        // parent
        options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null,
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
    this.parentPtr = options.parent != null ? (options.parent.metatype == StructType.NODE_REFERENCE ? (options.parent as NodeReference) : (options.parent as Node).toRef()) : null;
    this.spacePtr = options.space != null ? (options.space.metatype == StructType.NODE_REFERENCE ? (options.space as NodeReference) : (options.space as Node).toRef()) : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
    this.createdAt = options.createdAt;
    this.createdByPtr = options.createdBy != null ? (options.createdBy.metatype == StructType.NODE_REFERENCE ? (options.createdBy as NodeReference) : (options.createdBy as Node).toRef()) : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr = options.updatedBy != null ? (options.updatedBy.metatype == StructType.NODE_REFERENCE ? (options.updatedBy as NodeReference) : (options.updatedBy as Node).toRef()) : null;
    this.deletedAt = options.deletedAt ?? null;
    this.orderKey = options.orderKey ?? "a0";
    this.type = options.type ?? GradientType.LINEAR;
    this.name = options.name;
    this.angle = options.angle ?? null;
    this.stops = options.stops ?? [];
    this.centerAnchor = options.centerAnchor ?? null;
    this.dark = options.dark ?? null;
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
    return new NodeReference(NodeType.GRADIENT_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12025 ==== */