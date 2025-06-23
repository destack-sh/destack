import {
  Agent,
  AnnotationShape,
  ArrowShape,
  Axis2,
  Canvas,
  Color,
  CustomView,
  CustomViewDefinition,
  Entity,
  FrameView,
  Graph,
  IsDeletable,
  IsOrdered,
  IsTaggable,
  IsTracked,
  IsVisual,
  LabelView,
  Layer,
  LineShape,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  NumberInputView,
  PlaneShape,
  QueryConnection,
  Scene,
  Session,
  SliderInputView,
  Space,
  Spatial,
  SplitView,
  Struct,
  StructType,
  Style,
  Supergraph,
  TextView,
  Theme,
  ThreadView,
  TraitType,
  User,
  WizardView,
} from "@/language";
import { Temporal } from "temporal-polyfill";

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
export class Shadow extends Struct {
  static metatype: StructType = StructType.SHADOW;
  static __isFrozen__: boolean = false;

  type: ShadowType;
  get style(): ShadowStyle | null | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as ShadowStyle | null;
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
  stylePtr: NodeReference | null;
  color: Color | null;
  position: ShadowPosition;
  offset: Axis2 | null;
  blur: number | null;
  spread: number | null;
  diffusion: number | null;

  constructor(options: {
    type?: ShadowType;
    style?: ShadowStyle | NodeReference | null;
    color?: Color | null;
    position?: ShadowPosition;
    offset?: Axis2 | null;
    blur?: number | null;
    spread?: number | null;
    diffusion?: number | null;
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = ShadowType.BOX;
    }
    if (_type === null) {
      throw new Error(`Shadow.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = ShadowPosition.OUTSIDE;
    }
    if (_position === null) {
      throw new Error(`Shadow.position is required`);
    }
    this.position = _position;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _blur = options.blur ?? null;
    this.blur = _blur;
    let _spread = options.spread ?? null;
    this.spread = _spread;
    let _diffusion = options.diffusion ?? null;
    this.diffusion = _diffusion;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12012 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12024 ==== */
export class ShadowStyle
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style
{
  static metatype: NodeType = NodeType.SHADOW_STYLE;
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

  get parent():
    | Scene
    | CustomViewDefinition
    | CustomView
    | FrameView
    | LabelView
    | SplitView
    | TextView
    | NumberInputView
    | SliderInputView
    | WizardView
    | ThreadView
    | AnnotationShape
    | ArrowShape
    | Canvas
    | LineShape
    | PlaneShape
    | Layer
    | Scene
    | Theme
    | null
    | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Scene
        | CustomViewDefinition
        | CustomView
        | FrameView
        | LabelView
        | SplitView
        | TextView
        | NumberInputView
        | SliderInputView
        | WizardView
        | ThreadView
        | AnnotationShape
        | ArrowShape
        | Canvas
        | LineShape
        | PlaneShape
        | Layer
        | Scene
        | Theme
        | null
        | null;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
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

  constructor(options: {
    id?: string;
    parent?:
      | Scene
      | CustomViewDefinition
      | CustomView
      | FrameView
      | LabelView
      | SplitView
      | TextView
      | NumberInputView
      | SliderInputView
      | WizardView
      | ThreadView
      | AnnotationShape
      | ArrowShape
      | Canvas
      | LineShape
      | PlaneShape
      | Layer
      | Scene
      | Theme
      | NodeReference
      | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: ShadowType;
    name: string;
    color?: Color | null;
    position?: ShadowPosition;
    offset?: Axis2 | null;
    blur?: number | null;
    spread?: number | null;
    diffusion?: number | null;
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
      throw new Error(`ShadowStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ShadowStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = ShadowType.BOX;
    }
    if (_type === null) {
      throw new Error(`ShadowStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ShadowStyle.name is required`);
    }
    this.name = _name;
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = ShadowPosition.OUTSIDE;
    }
    if (_position === null) {
      throw new Error(`ShadowStyle.position is required`);
    }
    this.position = _position;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _blur = options.blur ?? null;
    this.blur = _blur;
    let _spread = options.spread ?? null;
    this.spread = _spread;
    let _diffusion = options.diffusion ?? null;
    this.diffusion = _diffusion;
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
      nodeType: NodeType.SHADOW_STYLE,
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
/* ==== DESTACK_GENERATED_END:NODE:12024 ==== */
