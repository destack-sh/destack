import { IsDeletable, SplitView, IsOrdered, CustomView, ThreadView, Entity, MaterializationType, Canvas, NodeType, QueryConnection, StructType, Space, Style, NodeReference, Graph, NumberInputView, User, IsTaggable, Struct, FrameView, Palette, StructFrozen, PlaneShape, Spatial, BuiltinObject, EnumType, LineShape, Scene, Layer, Agent, SliderInputView, TextView, Theme, activeSession, Node, IsTracked, Supergraph, WizardView, AnnotationShape, CustomViewDefinition, ACTIVE_SESSION, IsVisual, LabelView, Session, ArrowShape } from '@/language';
import { Temporal } from 'temporal-polyfill';

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
  type: ColorType;
  get style(): ColorStyle | null {
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
  ;
  stylePtr: NodeReference | null
  hue: ColorHue | null;
  shade: ColorShade | null;
  intent: ColorIntent | null;
  x: number | null;
  y: number | null;
  z: number | null;
  alpha: number | null;

  constructor(options: {
    type: ColorType,
    style?: ColorStyle | NodeReference | null,
    hue?: ColorHue | null,
    shade?: ColorShade | null,
    intent?: ColorIntent | null,
    x?: number | null,
    y?: number | null,
    z?: number | null,
    alpha?: number | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    super(
        // supergraph
        options._supergraph ?? null,
    );

    this.type = options.type;
    this.stylePtr = options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? (options.style as NodeReference) : (options.style as Node).toRef()) : null;
    this.hue = options.hue ?? null;
    this.shade = options.shade ?? null;
    this.intent = options.intent ?? null;
    this.x = options.x ?? null;
    this.y = options.y ?? null;
    this.z = options.z ?? null;
    this.alpha = options.alpha ?? null;
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
export class ColorStyle extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsVisual, Style {
  readonly id: string;
  get parent(): Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | Palette | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | Palette | null | null;
      }
      return null;
  }
  ;
  readonly parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  readonly spacePtr: NodeReference | null
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
  readonly createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  readonly updatedByPtr: NodeReference | null
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
    id: string,
    parent?: Scene | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Layer | Scene | Theme | Palette | NodeReference | null,
    space?: Space | NodeReference | null,
    materialization?: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdBy?: Agent | User | NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedBy?: Agent | User | NodeReference | null,
    deletedAt?: Temporal.ZonedDateTime | null,
    orderKey?: string,
    type: ColorType,
    name: string,
    hue?: ColorHue | null,
    shade?: ColorShade | null,
    intent?: ColorIntent | null,
    x?: number | null,
    y?: number | null,
    z?: number | null,
    alpha?: number | null,
    dark?: Color | null,
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
    this.hue = options.hue ?? null;
    this.shade = options.shade ?? null;
    this.intent = options.intent ?? null;
    this.x = options.x ?? null;
    this.y = options.y ?? null;
    this.z = options.z ?? null;
    this.alpha = options.alpha ?? null;
    this.dark = options.dark ?? null;
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