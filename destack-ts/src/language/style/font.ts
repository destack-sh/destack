import { IsTracked, SliderInputView, ACTIVE_SESSION, WizardView, IsVisual, Length, Spatial, EnumType, IsOrdered, ArrowShape, MaterializationType, Entity, AnnotationShape, Space, StructType, CustomView, SplitView, Struct, NumberInputView, Theme, LabelView, PlaneShape, ThreadView, Scene, NodeReference, NodeType, LineShape, Graph, User, Layer, Agent, IsTaggable, Fill, Node, QueryConnection, CustomViewDefinition, StructFrozen, TextView, Canvas, Supergraph, IsDeletable, FrameView, BuiltinObject, Session, activeSession, Style } from '@/language';
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
          return this._supergraph.get(nodePtr.id) as FontStyle | null;
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

  constructor(options: {
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
  }) {
    super(
        // supergraph
        supergraph,
    );

    this.type = options.type ?? FontType.SANS;
    this.stylePtr = options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? (options.style as NodeReference) : (options.style as Node).toRef()) : null;
    this.weight = options.weight ?? FontWeight.NORMAL;
    this.color = options.color ?? null;
    this.size = options.size ?? FontSize.BASE;
    this.align = options.align ?? TextAlign.LEFT;
    this.lineHeight = options.lineHeight ?? null;
    this.letterSpacing = options.letterSpacing ?? null;
    this.decoration = options.decoration ?? TextDecoration.NONE;
    this.transform = options.transform ?? TextTransform.NONE;
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
    this.type = options.type ?? FontType.SANS;
    this.name = options.name;
    this.weight = options.weight ?? FontWeight.NORMAL;
    this.color = options.color ?? null;
    this.size = options.size ?? FontSize.BASE;
    this.align = options.align ?? TextAlign.LEFT;
    this.lineHeight = options.lineHeight ?? null;
    this.letterSpacing = options.letterSpacing ?? null;
    this.decoration = options.decoration ?? TextDecoration.NONE;
    this.transform = options.transform ?? TextTransform.NONE;
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