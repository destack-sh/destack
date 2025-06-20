import { CustomView, User, Scene, NodeReference, Entity, NumberInputView, PlaneShape, Supergraph, EnumType, CustomViewDefinition, SplitView, AnnotationShape, Style, TextView, StructType, Space, IsTracked, IsTaggable, Spatial, Canvas, MaterializationType, Session, QueryConnection, ArrowShape, Layer, ThreadView, IsOrdered, IsDeletable, Theme, FrameView, Node, BuiltinObject, LineShape, Graph, Struct, Palette, Agent, NodeType, WizardView, IsVisual, StructFrozen, LabelView, SliderInputView } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

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

  constructor(
    type: ColorType,
    stylePtr: NodeReference | null,
    hue: ColorHue | null,
    shade: ColorShade | null,
    intent: ColorIntent | null,
    x: number | null,
    y: number | null,
    z: number | null,
    alpha: number | null,
    _supergraph: Supergraph | null
  ) {
    super(_supergraph);
    this.type = type;
    this.stylePtr = stylePtr;
    this.hue = hue;
    this.shade = shade;
    this.intent = intent;
    this.x = x;
    this.y = y;
    this.z = z;
    this.alpha = alpha;
  }


  static create(options: {
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
  }): Color {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Color(
      options.type,
      options.style != null ? (options.style.metatype == StructType.NODE_REFERENCE ? options.style : options.style.toRef()) : null,
      options.hue ?? null,
      options.shade ?? null,
      options.intent ?? null,
      options.x ?? null,
      options.y ?? null,
      options.z ?? null,
      options.alpha ?? null,
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
    type: ColorType,
    name: string,
    hue: ColorHue | null,
    shade: ColorShade | null,
    intent: ColorIntent | null,
    x: number | null,
    y: number | null,
    z: number | null,
    alpha: number | null,
    dark: Color | null,
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
    this.hue = hue;
    this.shade = shade;
    this.intent = intent;
    this.x = x;
    this.y = y;
    this.z = z;
    this.alpha = alpha;
    this.dark = dark;
  }


  static create(options: {
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
  }): ColorStyle {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new ColorStyle(
      options.type,
      options.name,
      options.hue ?? null,
      options.shade ?? null,
      options.intent ?? null,
      options.x ?? null,
      options.y ?? null,
      options.z ?? null,
      options.alpha ?? null,
      options.dark ?? null,
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
    return new NodeReference(NodeType.COLOR_STYLE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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