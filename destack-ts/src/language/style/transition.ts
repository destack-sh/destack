import { PlaneShape, Agent, IsTaggable, BuiltinObject, ACTIVE_SESSION, IsVisual, Session, Scene, StructFrozen, LineShape, IsOrdered, Graph, Struct, AnnotationShape, activeSession, StructType, NodeType, Canvas, FrameView, Supergraph, QueryConnection, SliderInputView, NodeReference, WizardView, MaterializationType, User, CustomView, Spatial, LabelView, ThreadView, Space, CustomViewDefinition, NumberInputView, ArrowShape, Theme, IsDeletable, Entity, SplitView, EnumType, Layer, TextView, Style, Node, IsTracked } from '@/language';
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
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