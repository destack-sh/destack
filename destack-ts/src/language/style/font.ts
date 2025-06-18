import { IsTaggable, NumberInputView, ArrowShape, EnumType, StructType, CustomView, Scene, Theme, CustomViewDefinition, Node, Canvas, QueryConnection, Layer, IsDeletable, User, NodeReference, Graph, Spatial, Fill, LabelView, SplitView, Agent, IsOrdered, Space, StructFrozen, ThreadView, Struct, LineShape, BuiltinObject, AnnotationShape, IsVisual, SliderInputView, NodeType, TextView, Session, Style, MaterializationType, Entity, FrameView, Length, PlaneShape, Supergraph, WizardView, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:12026 ==== */
export enum FontType {
  STYLE = 2,
  SERIF = 10,
  SANS = 11,
  MONO = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:12026 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12024 ==== */
export enum FontWeight {
  THIN = 100,
  EXTRA_LIGHT = 200,
  LIGHT = 300,
  NORMAL = 400,
  MEDIUM = 500,
  SEMI_BOLD = 600,
  BOLD = 700,
  EXTRA_BOLD = 800,
  BLACK = 900,
}
/* ==== DESTACK_GENERATED_END:ENUM:12024 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12025 ==== */
export enum FontSize {
  XS = 12,
  SM = 14,
  BASE = 16,
  LG = 18,
  XL = 20,
  XL2 = 24,
  XL3 = 30,
  XL4 = 36,
  XL5 = 48,
  XL6 = 60,
  XL7 = 72,
}
/* ==== DESTACK_GENERATED_END:ENUM:12025 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12027 ==== */
export enum TextAlign {
  LEFT = 1,
  CENTER = 2,
  RIGHT = 3,
  JUSTIFY = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12027 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12028 ==== */
export enum TextDecoration {
  NONE = 1,
  UNDERLINE = 2,
  STRIKETHROUGH = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:12028 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12029 ==== */
export enum TextTransform {
  NONE = 1,
  UPPERCASE = 2,
  LOWERCASE = 3,
  CAPITALIZE = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12029 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12014 ==== */
export class Font extends Struct {
  type: FontType;
  get style(): FontStyle | null {
      const nodePtr: NodeReference | null = this.stylePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }

  set style(value: FontStyle | null) {
      if (value == null) {
          this.stylePtr = null;
      } else {
          this.stylePtr = value.toRef();
      }
  }
  ;
  stylePtr: NodeReference | null
  weight: FontWeight | null;
  color: Fill | null;
  size: FontSize | null;
  align: TextAlign | null;
  lineHeight: Length | null;
  letterSpacing: Length | null;
  decoration: TextDecoration | null;
  transform: TextTransform | null;

  constructor(
    type: FontType,
    stylePtr: NodeReference | null,
    weight: FontWeight | null,
    color: Fill | null,
    size: FontSize | null,
    align: TextAlign | null,
    lineHeight: Length | null,
    letterSpacing: Length | null,
    decoration: TextDecoration | null,
    transform: TextTransform | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.weight = weight;
    this.color = color;
    this.size = size;
    this.align = align;
    this.lineHeight = lineHeight;
    this.letterSpacing = letterSpacing;
    this.decoration = decoration;
    this.transform = transform;
  }


  static create(options: {
    type?: FontType,
    style?: FontStyle | NodeReference | null,
    weight?: FontWeight | null,
    color?: Fill | null,
    size?: FontSize | null,
    align?: TextAlign | null,
    lineHeight?: Length | null,
    letterSpacing?: Length | null,
    decoration?: TextDecoration | null,
    transform?: TextTransform | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }): Font {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Font(
      options.type ?? FontType.SANS,
      options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? options.style : options.style.toRef()) : null,
      options.weight ?? FontWeight.NORMAL,
      options.color ?? null,
      options.size ?? FontSize.BASE,
      options.align ?? TextAlign.LEFT,
      options.lineHeight ?? null,
      options.letterSpacing ?? null,
      options.decoration ?? TextDecoration.NONE,
      options.transform ?? TextTransform.NONE,
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
/* ==== DESTACK_GENERATED_END:STRUCT:12014 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12022 ==== */
export class FontStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
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
  type: FontType;
  name: string;
  weight: FontWeight | null;
  color: Fill | null;
  size: FontSize | null;
  align: TextAlign | null;
  lineHeight: Length | null;
  letterSpacing: Length | null;
  decoration: TextDecoration | null;
  transform: TextTransform | null;

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
    type: FontType,
    name: string,
    weight: FontWeight | null,
    color: Fill | null,
    size: FontSize | null,
    align: TextAlign | null,
    lineHeight: Length | null,
    letterSpacing: Length | null,
    decoration: TextDecoration | null,
    transform: TextTransform | null,
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
    this.weight = weight;
    this.color = color;
    this.size = size;
    this.align = align;
    this.lineHeight = lineHeight;
    this.letterSpacing = letterSpacing;
    this.decoration = decoration;
    this.transform = transform;
  }


  static create(options: {
    type?: FontType,
    name: string,
    weight?: FontWeight | null,
    color?: Fill | null,
    size?: FontSize | null,
    align?: TextAlign | null,
    lineHeight?: Length | null,
    letterSpacing?: Length | null,
    decoration?: TextDecoration | null,
    transform?: TextTransform | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): FontStyle {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new FontStyle(
      options.type ?? FontType.SANS,
      options.name,
      options.weight ?? FontWeight.NORMAL,
      options.color ?? null,
      options.size ?? FontSize.BASE,
      options.align ?? TextAlign.LEFT,
      options.lineHeight ?? null,
      options.letterSpacing ?? null,
      options.decoration ?? TextDecoration.NONE,
      options.transform ?? TextTransform.NONE,
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
    return new NodeReference(NodeType.FONT_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:12022 ==== */