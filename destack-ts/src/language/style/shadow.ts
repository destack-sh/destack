import { Session, Spatial, Entity, IsDeletable, Color, CustomViewDefinition, CustomView, Canvas, Style, Layer, User, NodeReference, BuiltinObject, ArrowShape, Agent, MaterializationType, PlaneShape, IsTracked, Axis2, IsVisual, AnnotationShape, IsTaggable, Scene, Graph, SliderInputView, WizardView, FrameView, LabelView, LineShape, TextView, Struct, Theme, ThreadView, Supergraph, NumberInputView, QueryConnection, SplitView, NodeType, IsOrdered, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12030 ==== */
export enum ShadowType {
  STYLE = 2,
  BOX = 10,
  REALISTIC = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:12030 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12031 ==== */
export enum ShadowPosition {
  OUTSIDE = 1,
  INSIDE = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12031 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12012 ==== */
export class Shadow extends BuiltinObject {
  type: ShadowType;
  get style(): ShadowStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }

  set style(value: ShadowStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  color: Color | null;
  position: ShadowPosition;
  offset: Axis2 | null;
  blur: number | null;
  spread: number | null;
  diffusion: number | null;

  constructor(
    type: ShadowType,
    stylePtr: NodeReference | null,
    color: Color | null,
    position: ShadowPosition,
    offset: Axis2 | null,
    blur: number | null,
    spread: number | null,
    diffusion: number | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.color = color;
    this.position = position;
    this.offset = offset;
    this.blur = blur;
    this.spread = spread;
    this.diffusion = diffusion;
  }


  static create(): Shadow {

    return new Shadow();
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
/* ==== DESTACK_GENERATED_END:STRUCT:12012 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12024 ==== */
export class ShadowStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
  readonly id: string;
  get parent(): Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }

  set space(value: Space | null) {
      if (value === null) {
          this.spacePtr = null;
      } else {
          this.spacePtr = value.toRef();
      }
  }
  ;
  spacePtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  readonly deletedAt: Temporal.ZonedDateTime | null;
  readonly orderKey: string;
  type: ShadowType;
  name: string;
  color: Color | null;
  position: ShadowPosition;
  offset: Axis2 | null;
  blur: number | null;
  spread: number | null;
  diffusion: number | null;

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
    type: ShadowType,
    name: string,
    color: Color | null,
    position: ShadowPosition,
    offset: Axis2 | null,
    blur: number | null,
    spread: number | null,
    diffusion: number | null,
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
    this.position = position;
    this.offset = offset;
    this.blur = blur;
    this.spread = spread;
    this.diffusion = diffusion;
  }


  static create(): ShadowStyle {

    return new ShadowStyle();
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
    return new NodeReference(NodeType.SHADOW_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12024 ==== */