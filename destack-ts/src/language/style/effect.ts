import { IsTracked, SliderInputView, ACTIVE_SESSION, Transition, WizardView, IsVisual, Spatial, EnumType, IsOrdered, ArrowShape, MaterializationType, Entity, AnnotationShape, Space, StructType, CustomView, Vector2, SplitView, Struct, NumberInputView, Theme, LabelView, PlaneShape, ThreadView, Scene, NodeReference, Axis3, NodeType, LineShape, Graph, User, Layer, Agent, IsTaggable, Node, QueryConnection, CustomViewDefinition, StructFrozen, TextView, Canvas, Supergraph, IsDeletable, FrameView, BuiltinObject, Session, activeSession, Style } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12046 ==== */
export enum EffectType {
  STYLE = 2,
  APPEAR = 10,
  ENTER = 11,
  EXIT = 12,
  HOVER = 20,
  PRESS = 21,
  DRAG = 22,
  FOCUS = 23,
  LOOP = 30,
}
/* ==== DESTACK_GENERATED_END:ENUM:12046 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12047 ==== */
export enum RepeatType {
  LOOP = 1,
  REVERSE = 2,
  MIRROR = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:12047 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12048 ==== */
export enum TextSplitType {
  CHAR = 1,
  WORD = 2,
  LINE = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:12048 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12049 ==== */
export enum OffscreenBehavior {
  PLAY = 1,
  PAUSE = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12049 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12025 ==== */
export class Effect extends Struct {
  type: EffectType;
  get style(): EffectStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as EffectStyle | null;
      }
      return null;
  }

  set style(value: EffectStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  opacity: number | null;
  offset: Vector2 | null;
  scale: number | null;
  rotate: Axis3 | null;
  skew: Vector2 | null;
  perspective: number | null;
  delay: Temporal.Duration | null;
  duration: number | null;
  threshold: number | null;
  once: boolean | null;
  repeat: RepeatType | null;
  split: TextSplitType | null;
  offscreen: OffscreenBehavior | null;
  transition: Transition | null;

  constructor(options: {
    type: EffectType,
    style?: EffectStyle | NodeReference | null,
    opacity?: number | null,
    offset?: Vector2 | null,
    scale?: number | null,
    rotate?: Axis3 | null,
    skew?: Vector2 | null,
    perspective?: number | null,
    delay?: Temporal.Duration | null,
    duration?: number | null,
    threshold?: number | null,
    once?: boolean | null,
    repeat?: RepeatType | null,
    split?: TextSplitType | null,
    offscreen?: OffscreenBehavior | null,
    transition?: Transition | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        supergraph,
    );

    this.type = options.type;
    this.stylePtr = options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? (options.style as NodeReference) : (options.style as Node).toRef()) : null;
    this.opacity = options.opacity ?? null;
    this.offset = options.offset ?? null;
    this.scale = options.scale ?? null;
    this.rotate = options.rotate ?? null;
    this.skew = options.skew ?? null;
    this.perspective = options.perspective ?? null;
    this.delay = options.delay ?? null;
    this.duration = options.duration ?? null;
    this.threshold = options.threshold ?? null;
    this.once = options.once ?? null;
    this.repeat = options.repeat ?? null;
    this.split = options.split ?? null;
    this.offscreen = options.offscreen ?? null;
    this.transition = options.transition ?? null;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12025 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12027 ==== */
export class EffectStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
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
  type: EffectType;
  name: string;
  opacity: number | null;
  offset: Vector2 | null;
  scale: number | null;
  rotate: Axis3 | null;
  skew: Vector2 | null;
  perspective: number | null;
  delay: Temporal.Duration | null;
  duration: number | null;
  threshold: number | null;
  once: boolean | null;
  repeat: RepeatType | null;
  split: TextSplitType | null;
  offscreen: OffscreenBehavior | null;
  transition: Transition | null;

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
    type: EffectType,
    name: string,
    opacity?: number | null,
    offset?: Vector2 | null,
    scale?: number | null,
    rotate?: Axis3 | null,
    skew?: Vector2 | null,
    perspective?: number | null,
    delay?: Temporal.Duration | null,
    duration?: number | null,
    threshold?: number | null,
    once?: boolean | null,
    repeat?: RepeatType | null,
    split?: TextSplitType | null,
    offscreen?: OffscreenBehavior | null,
    transition?: Transition | null,
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
    this.type = options.type;
    this.name = options.name;
    this.opacity = options.opacity ?? null;
    this.offset = options.offset ?? null;
    this.scale = options.scale ?? null;
    this.rotate = options.rotate ?? null;
    this.skew = options.skew ?? null;
    this.perspective = options.perspective ?? null;
    this.delay = options.delay ?? null;
    this.duration = options.duration ?? null;
    this.threshold = options.threshold ?? null;
    this.once = options.once ?? null;
    this.repeat = options.repeat ?? null;
    this.split = options.split ?? null;
    this.offscreen = options.offscreen ?? null;
    this.transition = options.transition ?? null;
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
    return new NodeReference(NodeType.EFFECT_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12027 ==== */