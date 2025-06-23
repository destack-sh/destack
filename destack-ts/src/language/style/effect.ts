import {
  Axis3,
  Entity,
  Graph,
  IsDeletable,
  IsOrdered,
  IsSubject,
  IsTaggable,
  IsTracked,
  IsVisual,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Scene,
  Session,
  Space,
  Spatial,
  Struct,
  StructType,
  Style,
  Supergraph,
  Theme,
  TraitType,
  Transition,
  Vector2,
  View,
} from "@/language";
import { Temporal } from "temporal-polyfill";

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
  static metatype: StructType = StructType.EFFECT;
  static __isFrozen__: boolean = false;

  type: EffectType;
  get style(): EffectStyle | null | null {
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
  stylePtr: NodeReference | null;
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
    type: EffectType;
    style?: EffectStyle | NodeReference | null;
    opacity?: number | null;
    offset?: Vector2 | null;
    scale?: number | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: number | null;
    delay?: Temporal.Duration | null;
    duration?: number | null;
    threshold?: number | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Effect.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _scale = options.scale ?? null;
    this.scale = _scale;
    let _rotate = options.rotate ?? null;
    this.rotate = _rotate;
    let _skew = options.skew ?? null;
    this.skew = _skew;
    let _perspective = options.perspective ?? null;
    this.perspective = _perspective;
    let _delay = options.delay ?? null;
    this.delay = _delay;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _threshold = options.threshold ?? null;
    this.threshold = _threshold;
    let _once = options.once ?? null;
    this.once = _once;
    let _repeat = options.repeat ?? null;
    this.repeat = _repeat;
    let _split = options.split ?? null;
    this.split = _split;
    let _offscreen = options.offscreen ?? null;
    this.offscreen = _offscreen;
    let _transition = options.transition ?? null;
    this.transition = _transition;

    // identity
    // ...
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
export class EffectStyle
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style
{
  static metatype: NodeType = NodeType.EFFECT_STYLE;
  static __traits__: TraitType[] = [
    TraitType.STYLE,
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.ORDERED,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.THEME,
    NodeType.THREAD_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.CANVAS,
    NodeType.TEXT_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.LAYER,
  ];
  static __childTypes__: NodeType[] = [NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.TEXT_VIEW,
    NodeType.THEME,
    NodeType.FOLDER,
    NodeType.THREAD_VIEW,
    NodeType.CANVAS,
  ];
  static __descendantTypes__: NodeType[] = [NodeType.TAGGING];

  get parent(): Scene | (Node & View) | Theme | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | (Node & View) | Theme | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
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
    id?: string;
    parent?: Scene | (Node & View) | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type: EffectType;
    name: string;
    opacity?: number | null;
    offset?: Vector2 | null;
    scale?: number | null;
    rotate?: Axis3 | null;
    skew?: Vector2 | null;
    perspective?: number | null;
    delay?: Temporal.Duration | null;
    duration?: number | null;
    threshold?: number | null;
    once?: boolean | null;
    repeat?: RepeatType | null;
    split?: TextSplitType | null;
    offscreen?: OffscreenBehavior | null;
    transition?: Transition | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
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
      options.id != null || options._graph != null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`EffectStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`EffectStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EffectStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`EffectStyle.name is required`);
    }
    this.name = _name;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _scale = options.scale ?? null;
    this.scale = _scale;
    let _rotate = options.rotate ?? null;
    this.rotate = _rotate;
    let _skew = options.skew ?? null;
    this.skew = _skew;
    let _perspective = options.perspective ?? null;
    this.perspective = _perspective;
    let _delay = options.delay ?? null;
    this.delay = _delay;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _threshold = options.threshold ?? null;
    this.threshold = _threshold;
    let _once = options.once ?? null;
    this.once = _once;
    let _repeat = options.repeat ?? null;
    this.repeat = _repeat;
    let _split = options.split ?? null;
    this.split = _split;
    let _offscreen = options.offscreen ?? null;
    this.offscreen = _offscreen;
    let _transition = options.transition ?? null;
    this.transition = _transition;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
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
      nodeType: NodeType.EFFECT_STYLE,
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
/* ==== DESTACK_GENERATED_END:NODE:12027 ==== */
