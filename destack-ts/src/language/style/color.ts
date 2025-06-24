import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  EnumType,
  Graph,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Struct,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { registerEnumClass, registerNodeClass, registerStructClass } from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Palette, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  ColorHueProto,
  ColorIntentProto,
  ColorProto,
  ColorShadeProto,
  ColorStyleProto,
  ColorTypeProto,
  MaterializationTypeProto,
} from "@destack/proto";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12020 ==== */
/**
 * ColorType
 */
export enum ColorType {
  BUILTIN = 1,
  STYLE = 2,
  FIELD = 3,
  RGB = 10,
  HSL = 11,
  P3 = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.COLOR_TYPE, ColorType);
/* ==== DESTACK_GENERATED_END:ENUM:12020 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12022 ==== */
/**
 * ColorHue
 */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.COLOR_HUE, ColorHue);
/* ==== DESTACK_GENERATED_END:ENUM:12022 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12021 ==== */
/**
 * ColorShade
 */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.COLOR_SHADE, ColorShade);
/* ==== DESTACK_GENERATED_END:ENUM:12021 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12023 ==== */
/**
 * ColorIntent
 */
export enum ColorIntent {
  PRIMARY = 1,
  SECONDARY = 2,
  NEUTRAL = 3,
  SUCCESS = 10,
  INFO = 11,
  WARNING = 12,
  ERROR = 13,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.COLOR_INTENT, ColorIntent);
/* ==== DESTACK_GENERATED_END:ENUM:12023 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12011 ==== */
/**
 * A color value.
 */
export class Color extends Struct {
  static metatype: StructType = StructType.COLOR;
  static __isFrozen__: boolean = false;

  /**
   * ColorBase.type
   */
  type: ColorType;

  /**
   * style
   */
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
  stylePtr: NodeReference | null;

  /**
   * ColorBase.hue
   */
  hue: ColorHue | null;

  /**
   * ColorBase.shade
   */
  shade: ColorShade | null;

  /**
   * ColorBase.intent
   */
  intent: ColorIntent | null;

  /**
   * ColorBase.x
   */
  x: number | null;

  /**
   * ColorBase.y
   */
  y: number | null;

  /**
   * ColorBase.z
   */
  z: number | null;

  /**
   * ColorBase.alpha
   */
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if ((this.hue == null) !== (other.hue == null) || (this.hue != null && !(this.hue === other.hue))) {
      return false;
    }
    if ((this.shade == null) !== (other.shade == null) || (this.shade != null && !(this.shade === other.shade))) {
      return false;
    }
    if ((this.intent == null) !== (other.intent == null) || (this.intent != null && !(this.intent === other.intent))) {
      return false;
    }
    if (
      (this.x == null) !== (other.x == null) ||
      (this.x != null && !(this.x === other.x || Math.abs(this.x - other.x) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.y == null) !== (other.y == null) ||
      (this.y != null && !(this.y === other.y || Math.abs(this.y - other.y) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.z == null) !== (other.z == null) ||
      (this.z != null && !(this.z === other.z || Math.abs(this.z - other.z) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.alpha == null) !== (other.alpha == null) ||
      (this.alpha != null && !(this.alpha === other.alpha || Math.abs(this.alpha - other.alpha) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.stylePtr == null) !== (other.stylePtr == null) ||
      (this.stylePtr != null && !(this.stylePtr.id === other.stylePtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return Color.__packValue__(this);
  }

  static __packValue__(object: Color): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12011;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["42"] = object.stylePtr.toValue();
    }
    if (object.hue != null) {
      objectValue["50"] = object.hue;
    }
    if (object.shade != null) {
      objectValue["51"] = object.shade;
    }
    if (object.intent != null) {
      objectValue["52"] = object.intent;
    }
    if (object.x != null) {
      objectValue["55"] = object.x;
    }
    if (object.y != null) {
      objectValue["56"] = object.y;
    }
    if (object.z != null) {
      objectValue["57"] = object.z;
    }
    if (object.alpha != null) {
      objectValue["58"] = object.alpha;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Color {
    const hueValue = objectValue["50"];
    const unpackedHue = hueValue != undefined ? Number(hueValue) : null;
    const shadeValue = objectValue["51"];
    const unpackedShade = shadeValue != undefined ? Number(shadeValue) : null;
    const intentValue = objectValue["52"];
    const unpackedIntent = intentValue != undefined ? Number(intentValue) : null;
    const xValue = objectValue["55"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["56"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectValue["57"];
    const unpackedZ = zValue != undefined ? zValue : null;
    const alphaValue = objectValue["58"];
    const unpackedAlpha = alphaValue != undefined ? alphaValue : null;
    const styleValue = objectValue["42"];
    const unpackedStyle =
      styleValue != undefined ? NodeReference.fromValue(styleValue, _session, _supergraph, _graph, _connection) : null;
    return new Color({
      type: Number(objectValue["30"]),
      hue: unpackedHue,
      shade: unpackedShade,
      intent: unpackedIntent,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      alpha: unpackedAlpha,
      style: unpackedStyle,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Color {
    return Color.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ColorProto {
    return Color.__packProto__(this);
  }

  static __packProto__(object: Color): ColorProto {
    const objectProto: Partial<ColorProto> = { metatype: 12011 };
    objectProto.type = Number(object.type) as ColorTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.hue != null) {
      objectProto.hue = Number(object.hue) as ColorHueProto;
    }
    if (object.shade != null) {
      objectProto.shade = Number(object.shade) as ColorShadeProto;
    }
    if (object.intent != null) {
      objectProto.intent = Number(object.intent) as ColorIntentProto;
    }
    if (object.x != null) {
      objectProto.x = object.x;
    }
    if (object.y != null) {
      objectProto.y = object.y;
    }
    if (object.z != null) {
      objectProto.z = object.z;
    }
    if (object.alpha != null) {
      objectProto.alpha = object.alpha;
    }
    return objectProto as ColorProto;
  }

  static __unpackProto__(
    objectProto: ColorProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Color {
    return new Color({
      type: Number(objectProto.type) as ColorType,
      hue: objectProto.hue != undefined ? (Number(objectProto.hue) as ColorHue) : null,
      shade: objectProto.shade != undefined ? (Number(objectProto.shade) as ColorShade) : null,
      intent: objectProto.intent != undefined ? (Number(objectProto.intent) as ColorIntent) : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      z: objectProto.z != undefined ? objectProto.z : null,
      alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(objectProto.stylePtr!, _session, _supergraph, _graph, _connection)
          : null,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ColorProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Color {
    return Color.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.COLOR, Color);
/* ==== DESTACK_GENERATED_END:STRUCT:12011 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12020 ==== */
/**
 * A color style, with an optional dark variant.
 */
export class ColorStyle extends Node implements Style {
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

  /**
   * ColorStyle.parent
   */
  get parent(): Scene | (Node & View) | Theme | Palette | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | (Node & View) | Theme | Palette | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * ColorBase.type
   */
  type: ColorType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * ColorBase.hue
   */
  hue: ColorHue | null;

  /**
   * ColorBase.shade
   */
  shade: ColorShade | null;

  /**
   * ColorBase.intent
   */
  intent: ColorIntent | null;

  /**
   * ColorBase.x
   */
  x: number | null;

  /**
   * ColorBase.y
   */
  y: number | null;

  /**
   * ColorBase.z
   */
  z: number | null;

  /**
   * ColorBase.alpha
   */
  alpha: number | null;

  /**
   * ColorStyle.dark
   */
  dark: Color | null;

  constructor(options: {
    id?: string;
    parent?: Scene | (Node & View) | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if ((this.dark == null) !== (other.dark == null) || (this.dark != null && !this.dark.equals(other.dark))) {
      return false;
    }
    if (!(this.materialization === other.materialization)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if ((this.hue == null) !== (other.hue == null) || (this.hue != null && !(this.hue === other.hue))) {
      return false;
    }
    if ((this.shade == null) !== (other.shade == null) || (this.shade != null && !(this.shade === other.shade))) {
      return false;
    }
    if ((this.intent == null) !== (other.intent == null) || (this.intent != null && !(this.intent === other.intent))) {
      return false;
    }
    if (
      (this.x == null) !== (other.x == null) ||
      (this.x != null && !(this.x === other.x || Math.abs(this.x - other.x) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.y == null) !== (other.y == null) ||
      (this.y != null && !(this.y === other.y || Math.abs(this.y - other.y) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.z == null) !== (other.z == null) ||
      (this.z != null && !(this.z === other.z || Math.abs(this.z - other.z) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.alpha == null) !== (other.alpha == null) ||
      (this.alpha != null && !(this.alpha === other.alpha || Math.abs(this.alpha - other.alpha) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
    ) {
      return false;
    }
    return true;
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

  toValue(): { [key: string]: any } {
    return ColorStyle.__packValue__(this);
  }

  static __packValue__(object: ColorStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12020;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString();
    }
    objectValue["22"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.hue != null) {
      objectValue["50"] = object.hue;
    }
    if (object.shade != null) {
      objectValue["51"] = object.shade;
    }
    if (object.intent != null) {
      objectValue["52"] = object.intent;
    }
    if (object.x != null) {
      objectValue["55"] = object.x;
    }
    if (object.y != null) {
      objectValue["56"] = object.y;
    }
    if (object.z != null) {
      objectValue["57"] = object.z;
    }
    if (object.alpha != null) {
      objectValue["58"] = object.alpha;
    }
    if (object.dark != null) {
      objectValue["60"] = object.dark.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ColorStyle {
    const darkValue = objectValue["60"];
    const unpackedDark =
      darkValue != undefined ? Color.fromValue(darkValue, _session, _supergraph, _graph, _connection) : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const hueValue = objectValue["50"];
    const unpackedHue = hueValue != undefined ? Number(hueValue) : null;
    const shadeValue = objectValue["51"];
    const unpackedShade = shadeValue != undefined ? Number(shadeValue) : null;
    const intentValue = objectValue["52"];
    const unpackedIntent = intentValue != undefined ? Number(intentValue) : null;
    const xValue = objectValue["55"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["56"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectValue["57"];
    const unpackedZ = zValue != undefined ? zValue : null;
    const alphaValue = objectValue["58"];
    const unpackedAlpha = alphaValue != undefined ? alphaValue : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ColorStyle({
      dark: unpackedDark,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      hue: unpackedHue,
      shade: unpackedShade,
      intent: unpackedIntent,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      alpha: unpackedAlpha,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ColorStyle {
    return ColorStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ColorStyleProto {
    return ColorStyle.__packProto__(this);
  }

  static __packProto__(object: ColorStyle): ColorStyleProto {
    const objectProto: Partial<ColorStyleProto> = { metatype: 12020 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationTypeProto;
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.orderKey = object.orderKey;
    objectProto.type = Number(object.type) as ColorTypeProto;
    objectProto.name = object.name;
    if (object.hue != null) {
      objectProto.hue = Number(object.hue) as ColorHueProto;
    }
    if (object.shade != null) {
      objectProto.shade = Number(object.shade) as ColorShadeProto;
    }
    if (object.intent != null) {
      objectProto.intent = Number(object.intent) as ColorIntentProto;
    }
    if (object.x != null) {
      objectProto.x = object.x;
    }
    if (object.y != null) {
      objectProto.y = object.y;
    }
    if (object.z != null) {
      objectProto.z = object.z;
    }
    if (object.alpha != null) {
      objectProto.alpha = object.alpha;
    }
    if (object.dark != null) {
      objectProto.dark = object.dark.toProto();
    }
    return objectProto as ColorStyleProto;
  }

  static __unpackProto__(
    objectProto: ColorStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ColorStyle {
    return new ColorStyle({
      dark:
        objectProto.dark != undefined
          ? Color.fromProto(objectProto.dark!, _session, _supergraph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      type: Number(objectProto.type) as ColorType,
      hue: objectProto.hue != undefined ? (Number(objectProto.hue) as ColorHue) : null,
      shade: objectProto.shade != undefined ? (Number(objectProto.shade) as ColorShade) : null,
      intent: objectProto.intent != undefined ? (Number(objectProto.intent) as ColorIntent) : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      z: objectProto.z != undefined ? objectProto.z : null,
      alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ColorStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ColorStyle {
    return ColorStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.COLOR_STYLE, ColorStyle);
/* ==== DESTACK_GENERATED_END:NODE:12020 ==== */
