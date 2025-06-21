import { Layer, Vector2, ThreadView, Agent, PlaneShape, Scene, Supergraph, StructFrozen, Transition, CustomViewDefinition, IsDeletable, EnumType, ArrowShape, Struct, SplitView, Session, Theme, TextView, NodeReference, User, MaterializationType, BuiltinObject, Spatial, Graph, WizardView, CustomView, SliderInputView, NodeType, LineShape, IsOrdered, Node, StructType, IsTracked, NumberInputView, QueryConnection, Style, IsVisual, LabelView, FrameView, Canvas, AnnotationShape, IsTaggable, Entity, Space, Axis3 } from '@/language';
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

  constructor(
    type: EffectType,
    stylePtr: NodeReference | null,
    opacity: number | null,
    offset: Vector2 | null,
    scale: number | null,
    rotate: Axis3 | null,
    skew: Vector2 | null,
    perspective: number | null,
    delay: Temporal.Duration | null,
    duration: number | null,
    threshold: number | null,
    once: boolean | null,
    repeat: RepeatType | null,
    split: TextSplitType | null,
    offscreen: OffscreenBehavior | null,
    transition: Transition | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.opacity = opacity;
    this.offset = offset;
    this.scale = scale;
    this.rotate = rotate;
    this.skew = skew;
    this.perspective = perspective;
    this.delay = delay;
    this.duration = duration;
    this.threshold = threshold;
    this.once = once;
    this.repeat = repeat;
    this.split = split;
    this.offscreen = offscreen;
    this.transition = transition;
  }


  static create(options: {
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
  }): Effect {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Effect(
      options.type,
      options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? options.style : options.style.toRef()) : null,
      options.opacity ?? null,
      options.offset ?? null,
      options.scale ?? null,
      options.rotate ?? null,
      options.skew ?? null,
      options.perspective ?? null,
      options.delay ?? null,
      options.duration ?? null,
      options.threshold ?? null,
      options.once ?? null,
      options.repeat ?? null,
      options.split ?? null,
      options.offscreen ?? null,
      options.transition ?? null,
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
    type: EffectType,
    name: string,
    opacity: number | null,
    offset: Vector2 | null,
    scale: number | null,
    rotate: Axis3 | null,
    skew: Vector2 | null,
    perspective: number | null,
    delay: Temporal.Duration | null,
    duration: number | null,
    threshold: number | null,
    once: boolean | null,
    repeat: RepeatType | null,
    split: TextSplitType | null,
    offscreen: OffscreenBehavior | null,
    transition: Transition | null,
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
    this.opacity = opacity;
    this.offset = offset;
    this.scale = scale;
    this.rotate = rotate;
    this.skew = skew;
    this.perspective = perspective;
    this.delay = delay;
    this.duration = duration;
    this.threshold = threshold;
    this.once = once;
    this.repeat = repeat;
    this.split = split;
    this.offscreen = offscreen;
    this.transition = transition;
  }


  static create(options: {
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
  }): EffectStyle {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new EffectStyle(
      options.type,
      options.name,
      options.opacity ?? null,
      options.offset ?? null,
      options.scale ?? null,
      options.rotate ?? null,
      options.skew ?? null,
      options.perspective ?? null,
      options.delay ?? null,
      options.duration ?? null,
      options.threshold ?? null,
      options.once ?? null,
      options.repeat ?? null,
      options.split ?? null,
      options.offscreen ?? null,
      options.transition ?? null,
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