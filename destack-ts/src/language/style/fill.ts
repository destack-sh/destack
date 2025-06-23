import {
  Agent,
  AnnotationShape,
  ArrowShape,
  Canvas,
  Color,
  CustomView,
  CustomViewDefinition,
  Entity,
  File,
  FrameView,
  Gradient,
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

/* ==== DESTACK_GENERATED_START:ENUM:12034 ==== */
export enum FillType {
  STYLE = 2,
  SOLID = 10,
  GRADIENT = 11,
  IMAGE = 12,
} /* ==== DESTACK_GENERATED_END:ENUM:12034 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12035 ==== */
export enum FillPosition {
  TOP_LEFT = 1,
  TOP_CENTER = 2,
  TOP_RIGHT = 3,
  LEFT = 10,
  CENTER = 11,
  RIGHT = 12,
  BOTTOM_LEFT = 20,
  BOTTOM_CENTER = 21,
  BOTTOM_RIGHT = 22,
} /* ==== DESTACK_GENERATED_END:ENUM:12035 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12036 ==== */
export enum FillSize {
  FILL = 1,
  STRETCH = 2,
  FIT = 3,
  TILE = 4,
} /* ==== DESTACK_GENERATED_END:ENUM:12036 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12017 ==== */
export class Fill extends Struct {
  static metatype: StructType = StructType.FILL;
  static __isFrozen__: boolean = false;

  type: FillType;
  get style(): FillStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as FillStyle | null;
    }
    return null;
  }

  set style(value: FillStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;
  color: Color | null;
  gradient: Gradient | null;
  get image(): File | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as File | null;
    }
    return null;
  }

  set image(value: File | null) {
    if (value == null) {
      this.imagePtr = null;
    } else {
      this.imagePtr = value.toRef();
    }
  }
  imagePtr: NodeReference | null;
  position: FillPosition | null;
  size: FillSize | null;

  constructor(options: {
    type: FillType;
    style?: FillStyle | NodeReference | null;
    color?: Color | null;
    gradient?: Gradient | null;
    image?: File | NodeReference | null;
    position?: FillPosition | null;
    size?: FillSize | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    this.type = options.type;
    this.stylePtr =
      options.style != null
        ? options.style.metatype == StructType.NODE_REFERENCE
          ? (options.style as NodeReference)
          : (options.style as Node).toRef()
        : null;
    this.color = options.color ?? null;
    this.gradient = options.gradient ?? null;
    this.imagePtr =
      options.image != null
        ? options.image.metatype == StructType.NODE_REFERENCE
          ? (options.image as NodeReference)
          : (options.image as Node).toRef()
        : null;
    this.position = options.position ?? null;
    this.size = options.size ?? null;
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
} /* ==== DESTACK_GENERATED_END:STRUCT:12017 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12021 ==== */
export class FillStyle
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style
{
  static metatype: NodeType = NodeType.FILL_STYLE;
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

  readonly id: string;
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
  type: FillType;
  name: string;
  color: Color | null;
  gradient: Gradient | null;
  get image(): File | null | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as File | null | null;
    }
    return null;
  }

  set image(node: File | null) {
    if (node === null) {
      this.imagePtr = null;
    } else {
      this.imagePtr = node.toRef();
    }
  }
  imagePtr: NodeReference | null;
  position: FillPosition | null;
  size: FillSize | null;

  constructor(options: {
    id: string;
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
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type: FillType;
    name: string;
    color?: Color | null;
    gradient?: Gradient | null;
    image?: File | NodeReference | null;
    position?: FillPosition | null;
    size?: FillSize | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
    this.createdAt = options.createdAt;
    this.createdByPtr =
      options.createdBy != null
        ? options.createdBy.metatype == StructType.NODE_REFERENCE
          ? (options.createdBy as NodeReference)
          : (options.createdBy as Node).toRef()
        : null;
    this.updatedAt = options.updatedAt;
    this.updatedByPtr =
      options.updatedBy != null
        ? options.updatedBy.metatype == StructType.NODE_REFERENCE
          ? (options.updatedBy as NodeReference)
          : (options.updatedBy as Node).toRef()
        : null;
    this.deletedAt = options.deletedAt ?? null;
    this.orderKey = options.orderKey ?? "a0";
    this.type = options.type;
    this.name = options.name;
    this.color = options.color ?? null;
    this.gradient = options.gradient ?? null;
    this.imagePtr =
      options.image != null
        ? options.image.metatype == StructType.NODE_REFERENCE
          ? (options.image as NodeReference)
          : (options.image as Node).toRef()
        : null;
    this.position = options.position ?? null;
    this.size = options.size ?? null;
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
      nodeType: NodeType.FILL_STYLE,
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
} /* ==== DESTACK_GENERATED_END:NODE:12021 ==== */
