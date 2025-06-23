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
  StructFrozen,
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
  static metatype: StructType = StructType.GRADIENT_STOP;
  static __isFrozen__: boolean = true;

  readonly color: Color | null;
  readonly position: number;

  constructor(options: {
    color?: Color | null;
    position: number;
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
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position;
    if (_position === null) {
      throw new Error(`GradientStop.position is required`);
    }
    this.position = _position;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12015 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12016 ==== */
export class Gradient extends Struct {
  static metatype: StructType = StructType.GRADIENT;
  static __isFrozen__: boolean = false;

  type: GradientType;
  get style(): GradientStyle | null | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as GradientStyle | null;
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
  stylePtr: NodeReference | null;
  angle: number | null;
  stops: Array<GradientStop>;
  centerAnchor: Axis2 | null;

  constructor(options: {
    type?: GradientType;
    style?: GradientStyle | NodeReference | null;
    angle?: number | null;
    stops?: Array<GradientStop>;
    centerAnchor?: Axis2 | null;
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
      _type = GradientType.LINEAR;
    }
    if (_type === null) {
      throw new Error(`Gradient.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _angle = options.angle ?? null;
    this.angle = _angle;
    let _stops = options.stops ?? null;
    if (_stops === null) {
      throw new Error(`Gradient.stops is required`);
    }
    this.stops = _stops;
    let _centerAnchor = options.centerAnchor ?? null;
    this.centerAnchor = _centerAnchor;
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
/* ==== DESTACK_GENERATED_END:STRUCT:12016 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12025 ==== */
export class GradientStyle
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style
{
  static metatype: NodeType = NodeType.GRADIENT_STYLE;
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
  type: GradientType;
  name: string;
  angle: number | null;
  stops: Array<GradientStop>;
  centerAnchor: Axis2 | null;
  dark: Gradient | null;

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
    type?: GradientType;
    name: string;
    angle?: number | null;
    stops?: Array<GradientStop>;
    centerAnchor?: Axis2 | null;
    dark?: Gradient | null;
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
      throw new Error(`GradientStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`GradientStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = GradientType.LINEAR;
    }
    if (_type === null) {
      throw new Error(`GradientStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`GradientStyle.name is required`);
    }
    this.name = _name;
    let _angle = options.angle ?? null;
    this.angle = _angle;
    let _stops = options.stops ?? null;
    if (_stops === null) {
      throw new Error(`GradientStyle.stops is required`);
    }
    this.stops = _stops;
    let _centerAnchor = options.centerAnchor ?? null;
    this.centerAnchor = _centerAnchor;
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
      nodeType: NodeType.GRADIENT_STYLE,
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
/* ==== DESTACK_GENERATED_END:NODE:12025 ==== */
