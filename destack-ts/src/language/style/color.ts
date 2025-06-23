import {
  Agent,
  AnnotationShape,
  ArrowShape,
  Canvas,
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
  Palette,
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

/* ==== DESTACK_GENERATED_START:ENUM:12020 ==== */
export enum ColorType {
  BUILTIN = 1,
  STYLE = 2,
  FIELD = 3,
  RGB = 10,
  HSL = 11,
  P3 = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:12020 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12022 ==== */
export enum ColorHue {
  GRAY = 30,
  RED = 31,
  ORANGE = 32,
  AMBER = 33,
  YELLOW = 34,
  LIME = 35,
  GREEN = 36,
  EMERALD = 37,
  TEAL = 38,
  CYAN = 39,
  SKY = 40,
  BLUE = 41,
  INDIGO = 42,
  VIOLET = 43,
  PURPLE = 44,
  FUCHSIA = 45,
  PINK = 46,
  ROSE = 47,
}
/* ==== DESTACK_GENERATED_END:ENUM:12022 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12021 ==== */
export enum ColorShade {
  S25 = 25,
  S50 = 50,
  S100 = 100,
  S200 = 200,
  S300 = 300,
  S400 = 400,
  S500 = 500,
  S600 = 600,
  S700 = 700,
  S800 = 800,
  S900 = 900,
  S950 = 950,
}
/* ==== DESTACK_GENERATED_END:ENUM:12021 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12023 ==== */
export enum ColorIntent {
  PRIMARY = 1,
  SECONDARY = 2,
  NEUTRAL = 3,
  SUCCESS = 10,
  INFO = 11,
  WARNING = 12,
  ERROR = 13,
}
/* ==== DESTACK_GENERATED_END:ENUM:12023 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12011 ==== */
export class Color extends Struct {
  static metatype: StructType = StructType.COLOR;
  static __isFrozen__: boolean = false;

  type: ColorType;
  get style(): ColorStyle | null | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as ColorStyle | null;
    }
    return null;
  }

  set style(value: ColorStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;
  hue: ColorHue | null;
  shade: ColorShade | null;
  intent: ColorIntent | null;
  x: number | null;
  y: number | null;
  z: number | null;
  alpha: number | null;

  constructor(options: {
    type: ColorType;
    style?: ColorStyle | NodeReference | null;
    hue?: ColorHue | null;
    shade?: ColorShade | null;
    intent?: ColorIntent | null;
    x?: number | null;
    y?: number | null;
    z?: number | null;
    alpha?: number | null;
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
      throw new Error(`Color.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _hue = options.hue ?? null;
    this.hue = _hue;
    let _shade = options.shade ?? null;
    this.shade = _shade;
    let _intent = options.intent ?? null;
    this.intent = _intent;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;
    let _z = options.z ?? null;
    this.z = _z;
    let _alpha = options.alpha ?? null;
    this.alpha = _alpha;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12011 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12020 ==== */
export class ColorStyle
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style
{
  static metatype: NodeType = NodeType.COLOR_STYLE;
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
    NodeType.LABEL_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.TEXT_VIEW,
    NodeType.THEME,
    NodeType.THREAD_VIEW,
    NodeType.PALETTE,
    NodeType.CANVAS,
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
    NodeType.PALETTE,
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
    | Palette
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
        | Palette
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
  type: ColorType;
  name: string;
  hue: ColorHue | null;
  shade: ColorShade | null;
  intent: ColorIntent | null;
  x: number | null;
  y: number | null;
  z: number | null;
  alpha: number | null;
  dark: Color | null;

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
      | Palette
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
    type: ColorType;
    name: string;
    hue?: ColorHue | null;
    shade?: ColorShade | null;
    intent?: ColorIntent | null;
    x?: number | null;
    y?: number | null;
    z?: number | null;
    alpha?: number | null;
    dark?: Color | null;
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
      throw new Error(`ColorStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ColorStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`ColorStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ColorStyle.name is required`);
    }
    this.name = _name;
    let _hue = options.hue ?? null;
    this.hue = _hue;
    let _shade = options.shade ?? null;
    this.shade = _shade;
    let _intent = options.intent ?? null;
    this.intent = _intent;
    let _x = options.x ?? null;
    this.x = _x;
    let _y = options.y ?? null;
    this.y = _y;
    let _z = options.z ?? null;
    this.z = _z;
    let _alpha = options.alpha ?? null;
    this.alpha = _alpha;
    let _dark = options.dark ?? null;
    this.dark = _dark;
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
      nodeType: NodeType.COLOR_STYLE,
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
/* ==== DESTACK_GENERATED_END:NODE:12020 ==== */
