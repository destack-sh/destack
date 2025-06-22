import { PlaneShape, Agent, IsTaggable, BuiltinObject, ACTIVE_SESSION, IsVisual, Session, Scene, StructFrozen, LineShape, IsOrdered, Graph, Struct, AnnotationShape, activeSession, StructType, NodeType, Canvas, FrameView, Supergraph, QueryConnection, Length, SliderInputView, NodeReference, WizardView, MaterializationType, User, CustomView, Spatial, LabelView, ThreadView, Fill, Space, CustomViewDefinition, NumberInputView, ArrowShape, Theme, IsDeletable, Entity, SplitView, EnumType, Layer, TextView, Style, Node, IsTracked } from '@/language';
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
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

  constructor(options: {
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
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
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