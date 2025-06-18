import { Session, Spatial, Entity, IsDeletable, CustomViewDefinition, CustomView, Canvas, Style, Layer, User, NodeReference, BuiltinObject, ArrowShape, Agent, MaterializationType, PlaneShape, IsTracked, IsVisual, AnnotationShape, IsTaggable, Scene, Graph, SliderInputView, WizardView, FrameView, LabelView, LineShape, TextView, Struct, Theme, ThreadView, Supergraph, NumberInputView, QueryConnection, SplitView, NodeType, IsOrdered, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12043 ==== */
export enum TransitionType {
  STYLE = 2,
  TWEEN = 10,
  SPRING = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:12043 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12044 ==== */
export enum SpringType {
  TIME = 1,
  PHYSICS = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12044 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12024 ==== */
export class Transition extends BuiltinObject {
  type: TransitionType;
  get style(): TransitionStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }

  set style(value: TransitionStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  delay: number | null;
  duration: number | null;
  ease: Array<number>;
  stiffness: number | null;
  damping: number | null;
  mass: number | null;
  bounce: number | null;
  springType: SpringType | null;

  constructor(
    type: TransitionType,
    stylePtr: NodeReference | null,
    delay: number | null,
    duration: number | null,
    ease: Array<number>,
    stiffness: number | null,
    damping: number | null,
    mass: number | null,
    bounce: number | null,
    springType: SpringType | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.delay = delay;
    this.duration = duration;
    this.ease = ease;
    this.stiffness = stiffness;
    this.damping = damping;
    this.mass = mass;
    this.bounce = bounce;
    this.springType = springType;
  }


  static create(): Transition {

    return new Transition();
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
/* ==== DESTACK_GENERATED_END:STRUCT:12024 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12026 ==== */
export class TransitionStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
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
  type: TransitionType;
  name: string;
  delay: number | null;
  duration: number | null;
  ease: Array<number>;
  stiffness: number | null;
  damping: number | null;
  mass: number | null;
  bounce: number | null;
  springType: SpringType | null;

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
    type: TransitionType,
    name: string,
    delay: number | null,
    duration: number | null,
    ease: Array<number>,
    stiffness: number | null,
    damping: number | null,
    mass: number | null,
    bounce: number | null,
    springType: SpringType | null,
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
    this.delay = delay;
    this.duration = duration;
    this.ease = ease;
    this.stiffness = stiffness;
    this.damping = damping;
    this.mass = mass;
    this.bounce = bounce;
    this.springType = springType;
  }


  static create(): TransitionStyle {

    return new TransitionStyle();
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
    return new NodeReference(NodeType.TRANSITION_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12026 ==== */