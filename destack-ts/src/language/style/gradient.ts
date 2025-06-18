import { PlaneShape, LabelView, Graph, IsTaggable, IsDeletable, ThreadView, MaterializationType, Spatial, StructFrozen, CustomViewDefinition, Struct, EnumType, IsTracked, Style, Entity, IsVisual, QueryConnection, NodeReference, Axis2, SplitView, Node, Scene, Layer, CustomView, FrameView, Theme, WizardView, AnnotationShape, IsOrdered, NodeType, Session, NumberInputView, ArrowShape, Space, Agent, User, LineShape, TextView, StructType, Canvas, Color, SliderInputView, Supergraph, BuiltinObject } from '@/language';
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

  constructor(
    color: Color | null,
    position: number,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.color = color;
    this.position = position;
  }


  static create(options: {
    color?: Color | null,
    position: number
  }): GradientStop {

    return new GradientStop(

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
          return this._supergraph.get(nodePtr.id);
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

  constructor(
    type: GradientType,
    stylePtr: NodeReference | null,
    angle: number | null,
    stops: Array<GradientStop>,
    centerAnchor: Axis2 | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.angle = angle;
    this.stops = stops;
    this.centerAnchor = centerAnchor;
  }


  static create(options: {
    type?: GradientType,
    style?: GradientStyle | NodeReference | null,
    angle?: number | null,
    stops?: Array<GradientStop>,
    centerAnchor?: Axis2 | null
  }): Gradient {

    return new Gradient(

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
    type: GradientType,
    name: string,
    angle: number | null,
    stops: Array<GradientStop>,
    centerAnchor: Axis2 | null,
    dark: Gradient | null,
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
    this.angle = angle;
    this.stops = stops;
    this.centerAnchor = centerAnchor;
    this.dark = dark;
  }


  static create(options: {
    type?: GradientType,
    name: string,
    angle?: number | null,
    stops?: Array<GradientStop>,
    centerAnchor?: Axis2 | null,
    dark?: Gradient | null
  }): GradientStyle {

    return new GradientStyle(

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