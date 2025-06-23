import { IsDeletable, SplitView, IsOrdered, CustomView, ThreadView, Entity, MaterializationType, Canvas, NodeType, QueryConnection, StructType, Space, Style, NodeReference, Graph, NumberInputView, User, IsTaggable, Struct, FrameView, StructFrozen, PlaneShape, Spatial, BuiltinObject, EnumType, LineShape, Scene, Layer, Agent, SliderInputView, TextView, Theme, activeSession, Node, IsTracked, Supergraph, WizardView, AnnotationShape, CustomViewDefinition, ACTIVE_SESSION, IsVisual, LabelView, Session, ArrowShape } from '@/language';
import { Temporal } from 'temporal-polyfill';

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
export class Transition extends Struct {
  type: TransitionType;
  get style(): TransitionStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as TransitionStyle | null;
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

  constructor(options: {
    type?: TransitionType,
    style?: TransitionStyle | NodeReference | null,
    delay?: number | null,
    duration?: number | null,
    ease?: Array<number>,
    stiffness?: number | null,
    damping?: number | null,
    mass?: number | null,
    bounce?: number | null,
    springType?: SpringType | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.type = options.type ?? TransitionType.TWEEN;
    this.stylePtr = options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? (options.style as NodeReference) : (options.style as Node).toRef()) : null;
    this.delay = options.delay ?? null;
    this.duration = options.duration ?? null;
    this.ease = options.ease ?? [];
    this.stiffness = options.stiffness ?? null;
    this.damping = options.damping ?? null;
    this.mass = options.mass ?? null;
    this.bounce = options.bounce ?? null;
    this.springType = options.springType ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12024 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12026 ==== */
export class TransitionStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
  readonly id: string;
  get parent(): Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | null | null;
      }
      return null;
  }
  ;
  readonly parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  readonly spacePtr: NodeReference | null
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
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly updatedByPtr: NodeReference | null
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
    type?: TransitionType,
    name: string,
    delay?: number | null,
    duration?: number | null,
    ease?: Array<number>,
    stiffness?: number | null,
    damping?: number | null,
    mass?: number | null,
    bounce?: number | null,
    springType?: SpringType | null,
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
    this.type = options.type ?? TransitionType.TWEEN;
    this.name = options.name;
    this.delay = options.delay ?? null;
    this.duration = options.duration ?? null;
    this.ease = options.ease ?? [];
    this.stiffness = options.stiffness ?? null;
    this.damping = options.damping ?? null;
    this.mass = options.mass ?? null;
    this.bounce = options.bounce ?? null;
    this.springType = options.springType ?? null;
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
      nodeType: NodeType.TRANSITION_STYLE,
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
/* ==== DESTACK_GENERATED_END:NODE:12026 ==== */