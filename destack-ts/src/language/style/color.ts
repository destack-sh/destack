import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsSubject,
  NodeReference,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import { EnumType, Node, NodeType, StructFrozen, StructType } from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Space } from "@destack/language/space";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { View } from "@destack/language/view";
import {
  ColorHueProto,
  ColorIntentProto,
  ColorProto,
  ColorShadeProto,
  ColorStyleProto,
  ColorTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:270300 ==== */
/**
 * A color value.
 */
export class Color extends StructFrozen {
  static metatype: StructType = StructType.COLOR;
  static __isFrozen__: boolean = true;

  /**
   * Color.type
   */
  readonly type: ColorType;

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
  readonly stylePtr: NodeReference | null;

  /**
   * Color.hue
   */
  readonly hue: ColorHue | null;

  /**
   * Color.shade
   */
  readonly shade: ColorShade | null;

  /**
   * Color.intent
   */
  readonly intent: ColorIntent | null;

  /**
   * Color.x
   */
  readonly x: number | null;

  /**
   * Color.y
   */
  readonly y: number | null;

  /**
   * Color.z
   */
  readonly z: number | null;

  /**
   * Color.alpha
   */
  readonly alpha: number | null;

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
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
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
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    if (!(this.hue === other.hue)) {
      return false;
    }
    if (!(this.shade === other.shade)) {
      return false;
    }
    if (!(this.intent === other.intent)) {
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
      (this.alpha != null &&
        !(this.alpha === other.alpha || Math.abs(this.alpha - other.alpha) < 1e-10))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ColorType[this.type]}`);
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.hue !== null) {
        propertyReprs.push(`hue=${ColorHue[this.hue]}`);
      }
      if (this.shade !== null) {
        propertyReprs.push(`shade=${ColorShade[this.shade]}`);
      }
      if (this.intent !== null) {
        propertyReprs.push(`intent=${ColorIntent[this.intent]}`);
      }
      if (this.x !== null) {
        propertyReprs.push(`x=${this.x}`);
      }
      if (this.y !== null) {
        propertyReprs.push(`y=${this.y}`);
      }
      if (this.z !== null) {
        propertyReprs.push(`z=${this.z}`);
      }
      if (this.alpha !== null) {
        propertyReprs.push(`alpha=${this.alpha}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Color ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.stylePtr !== null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.hue !== null) {
      h = (h * 31 + this.hue) & 0xffffffff;
    }
    if (this.shade !== null) {
      h = (h * 31 + this.shade) & 0xffffffff;
    }
    if (this.intent !== null) {
      h = (h * 31 + this.intent) & 0xffffffff;
    }
    if (this.x !== null) {
      h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    }
    if (this.y !== null) {
      h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    }
    if (this.z !== null) {
      h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    }
    if (this.alpha !== null) {
      h = (h * 31 + hashFloat(this.alpha)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Color.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Color): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270300;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const stylePtrValue = objectValue["42"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
    return new Color({
      type: Number(objectValue["30"]),
      style: unpackedStylePtr,
      hue: unpackedHue,
      shade: unpackedShade,
      intent: unpackedIntent,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      alpha: unpackedAlpha,
      _value: objectValue,
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
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Color.__packProto__(this);
    }
    return this._proto as ColorProto;
  }

  static __packProto__(object: Color): ColorProto {
    const objectProto: Partial<ColorProto> = { metatype: 270300 };
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new Color({
      type: Number(objectProto.type) as ColorType,
      style:
        objectProto.stylePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.stylePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      hue: objectProto.hue != undefined ? (Number(objectProto.hue) as ColorHue) : null,
      shade: objectProto.shade != undefined ? (Number(objectProto.shade) as ColorShade) : null,
      intent: objectProto.intent != undefined ? (Number(objectProto.intent) as ColorIntent) : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      z: objectProto.z != undefined ? objectProto.z : null,
      alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
      _proto: objectProto,
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

  static fromProtoString(packedProtoString: string): Color {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ColorProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  static fromHex(hex: string): Color {
    const [r, g, b, a] = hexToRgb(hex);
    return new Color({ type: ColorType.RGB, x: r, y: g, z: b, alpha: a });
  }

  static fromHue(hue: ColorHue, shade?: ColorShade | null): Color {
    return new Color({ type: ColorType.BUILTIN, hue, shade });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.COLOR, Color);
/* ==== DESTACK_GENERATED_END:STRUCT:270300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:270300 ==== */
/**
 * A color style, with an optional dark variant.
 */
export class ColorStyle extends Style {
  static metatype: NodeType = NodeType.COLOR_STYLE;

  /**
   * ColorStyle.parent
   */
  get parent(): Scene | View | Theme | Palette | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | View | Theme | Palette | null;
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
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
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
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * ColorStyle.type
   */
  type: ColorType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * ColorStyle.hue
   */
  hue: ColorHue | null;

  /**
   * ColorStyle.shade
   */
  shade: ColorShade | null;

  /**
   * ColorStyle.intent
   */
  intent: ColorIntent | null;

  /**
   * ColorStyle.x
   */
  x: number | null;

  /**
   * ColorStyle.y
   */
  y: number | null;

  /**
   * ColorStyle.z
   */
  z: number | null;

  /**
   * ColorStyle.alpha
   */
  alpha: number | null;

  /**
   * ColorStyle.dark
   */
  dark: Color | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference | null;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
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
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
      }
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
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.hue === other.hue)) {
      return false;
    }
    if (!(this.shade === other.shade)) {
      return false;
    }
    if (!(this.intent === other.intent)) {
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
      (this.alpha != null &&
        !(this.alpha === other.alpha || Math.abs(this.alpha - other.alpha) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.dark == null) !== (other.dark == null) ||
      (this.dark != null && !this.dark.equals(other.dark))
    ) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.hue !== null) {
      h = (h * 31 + this.hue) & 0xffffffff;
    }
    if (this.shade !== null) {
      h = (h * 31 + this.shade) & 0xffffffff;
    }
    if (this.intent !== null) {
      h = (h * 31 + this.intent) & 0xffffffff;
    }
    if (this.x !== null) {
      h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    }
    if (this.y !== null) {
      h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    }
    if (this.z !== null) {
      h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    }
    if (this.alpha !== null) {
      h = (h * 31 + hashFloat(this.alpha)) & 0xffffffff;
    }
    if (this.dark !== null) {
      h = (h * 31 + this.dark.hash()) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${ColorType[this.type]}`);
    if (this.hue !== null) {
      propertyReprs.push(`hue=${ColorHue[this.hue]}`);
    }
    if (this.shade !== null) {
      propertyReprs.push(`shade=${ColorShade[this.shade]}`);
    }
    if (this.intent !== null) {
      propertyReprs.push(`intent=${ColorIntent[this.intent]}`);
    }
    if (this.x !== null) {
      propertyReprs.push(`x=${this.x}`);
    }
    if (this.y !== null) {
      propertyReprs.push(`y=${this.y}`);
    }
    if (this.z !== null) {
      propertyReprs.push(`z=${this.z}`);
    }
    if (this.alpha !== null) {
      propertyReprs.push(`alpha=${this.alpha}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<ColorStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return ColorStyle.__packValue__(this);
  }

  static __packValue__(object: ColorStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 270300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["24"] = object.orderKey;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
    const darkValue = objectValue["60"];
    const unpackedDark =
      darkValue != undefined
        ? _Color.fromValue(darkValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new ColorStyle({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      hue: unpackedHue,
      shade: unpackedShade,
      intent: unpackedIntent,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      alpha: unpackedAlpha,
      dark: unpackedDark,
      space: unpackedSpacePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["24"],
      deletedAt: unpackedDeletedAt,
      id: String(objectValue["2"]),
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
    const objectProto: Partial<ColorStyleProto> = { metatype: 270300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    return new ColorStyle({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      type: Number(objectProto.type) as ColorType,
      hue: objectProto.hue != undefined ? (Number(objectProto.hue) as ColorHue) : null,
      shade: objectProto.shade != undefined ? (Number(objectProto.shade) as ColorShade) : null,
      intent: objectProto.intent != undefined ? (Number(objectProto.intent) as ColorIntent) : null,
      x: objectProto.x != undefined ? objectProto.x : null,
      y: objectProto.y != undefined ? objectProto.y : null,
      z: objectProto.z != undefined ? objectProto.z : null,
      alpha: objectProto.alpha != undefined ? objectProto.alpha : null,
      dark:
        objectProto.dark != undefined
          ? _Color.fromProto(objectProto.dark!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      id: String(objectProto.id),
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

  static fromProtoString(packedProtoString: string): ColorStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ColorStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  static fromColor(options: { name: string; color: Color; dark?: Color | null }): ColorStyle {
    return new ColorStyle({
      name: options.name,
      type: options.color.type,
      hue: options.color.hue,
      shade: options.color.shade,
      intent: options.color.intent,
      x: options.color.x,
      y: options.color.y,
      z: options.color.z,
      alpha: options.color.alpha,
      dark: options.dark ?? null,
    });
  }

  static fromHex(options: { name: string; hex: string; dark?: string | null }): ColorStyle {
    return ColorStyle.fromColor({
      name: options.name,
      color: Color.fromHex(options.hex),
      dark: options.dark ? Color.fromHex(options.dark) : null,
    });
  }

  static fromHue(options: {
    name: string;
    hue: ColorHue;
    shade?: ColorShade | null;
    darkShade?: ColorShade | null;
  }): ColorStyle {
    return ColorStyle.fromColor({
      name: options.name,
      color: Color.fromHue(options.hue, options.shade ?? null),
      dark: options.darkShade ? Color.fromHue(options.hue, options.darkShade) : null,
    });
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.COLOR_STYLE, ColorStyle);
/* ==== DESTACK_GENERATED_END:NODE:270300 ==== */

/** y-encoded sRGB → linear */
function srgbToLinear(c: number): number {
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/** linear → y-encoded sRGB */
function linearToSrgb(c: number): number {
  return c <= 0.0031308 ? c * 12.92 : 1.055 * Math.pow(c, 1 / 2.4) - 0.055;
}

/** avoid tiny negatives after matrices */
function clamp01(x: number): number {
  return Math.max(0.0, Math.min(1.0, x));
}

const SRGB_TO_XYZ = [
  [0.4124564, 0.3575761, 0.1804375],
  [0.2126729, 0.7151522, 0.072175],
  [0.0193339, 0.119192, 0.9503041],
] as const;

const XYZ_TO_SRGB = [
  [3.2406, -1.5372, -0.4986],
  [-0.9689, 1.8758, 0.0415],
  [0.0557, -0.204, 1.057],
] as const;

const P3_TO_XYZ = [
  [0.48657095, 0.26566769, 0.19821729],
  [0.22897456, 0.69173852, 0.07928691],
  [0.0, 0.04511338, 1.04394437],
] as const;

const XYZ_TO_P3 = [
  [2.49349691, -0.93138362, -0.40271078],
  [-0.82948897, 1.762664, 0.02362468],
  [0.03584583, -0.07617239, 0.95688452],
] as const;

/** Hex → linear-space floats 0-1 (optional alpha) */
export function hexToRgb(hex: string): [number, number, number, number | null] {
  if (hex.length === 6) {
    const r = parseInt(hex.slice(0, 2), 16) / 255.0;
    const g = parseInt(hex.slice(2, 4), 16) / 255.0;
    const b = parseInt(hex.slice(4, 6), 16) / 255.0;
    return [r, g, b, null];
  } else if (hex.length === 8) {
    const r = parseInt(hex.slice(0, 2), 16) / 255.0;
    const g = parseInt(hex.slice(2, 4), 16) / 255.0;
    const b = parseInt(hex.slice(4, 6), 16) / 255.0;
    const a = parseInt(hex.slice(6, 8), 16) / 255.0;
    return [r, g, b, a];
  } else {
    throw new Error(`invalid hex color: ${hex}`);
  }
}

/** 8-bit sRGB → hex */
export function rgbToHex(r: number, g: number, b: number, a?: number): string {
  if (a === undefined) {
    return `${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}`;
  } else {
    return `${r.toString(16).padStart(2, "0")}${g.toString(16).padStart(2, "0")}${b.toString(16).padStart(2, "0")}${a.toString(16).padStart(2, "0")}`;
  }
}

/** 8-bit sRGB → HSL (h° 0-360, s|l 0-1) */
export function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rF = r / 255.0;
  const gF = g / 255.0;
  const bF = b / 255.0;
  const cMax = Math.max(rF, gF, bF);
  const cMin = Math.min(rF, gF, bF);
  const delta = cMax - cMin;

  let h: number;
  if (delta === 0) {
    h = 0.0;
  } else if (cMax === rF) {
    h = ((gF - bF) / delta) % 6;
  } else if (cMax === gF) {
    h = (bF - rF) / delta + 2;
  } else {
    h = (rF - gF) / delta + 4;
  }
  h *= 60.0;

  const l = (cMax + cMin) / 2.0;
  const s = delta === 0 ? 0.0 : delta / (1.0 - Math.abs(2.0 * l - 1.0));

  return [h, s, l];
}

/** HSL (h° 0-360, s|l 0-1) → linear-space floats 0-1 */
export function hslToRgb(h: number, s: number, l: number): [number, number, number] {
  const c = (1.0 - Math.abs(2.0 * l - 1.0)) * s;
  const x = c * (1.0 - Math.abs(((h / 60.0) % 2.0) - 1.0));
  const m = l - c / 2.0;

  let r1: number, g1: number, b1: number;
  if (0 <= h && h < 60) {
    [r1, g1, b1] = [c, x, 0];
  } else if (60 <= h && h < 120) {
    [r1, g1, b1] = [x, c, 0];
  } else if (120 <= h && h < 180) {
    [r1, g1, b1] = [0, c, x];
  } else if (180 <= h && h < 240) {
    [r1, g1, b1] = [0, x, c];
  } else if (240 <= h && h < 300) {
    [r1, g1, b1] = [x, 0, c];
  } else {
    [r1, g1, b1] = [c, 0, x];
  }

  return [r1 + m, g1 + m, b1 + m];
}

function matMul(
  v: [number, number, number],
  m: readonly [
    readonly [number, number, number],
    readonly [number, number, number],
    readonly [number, number, number],
  ],
): [number, number, number] {
  const x = v[0] * m[0][0] + v[1] * m[0][1] + v[2] * m[0][2];
  const y = v[0] * m[1][0] + v[1] * m[1][1] + v[2] * m[1][2];
  const z = v[0] * m[2][0] + v[1] * m[2][1] + v[2] * m[2][2];
  return [x, y, z];
}

/** y-encoded sRGB (0-1) → y-encoded Display-P3 (0-1) */
export function rgbToP3(r: number, g: number, b: number): [number, number, number] {
  // sRGB y → linear
  const rl = srgbToLinear(r);
  const gl = srgbToLinear(g);
  const bl = srgbToLinear(b);

  // linear sRGB → XYZ → linear P3
  const [X, Y, Z] = matMul([rl, gl, bl], SRGB_TO_XYZ);
  const [rp3L, gp3L, bp3L] = matMul([X, Y, Z], XYZ_TO_P3);

  // linear P3 → y; clamp
  return [clamp01(linearToSrgb(rp3L)), clamp01(linearToSrgb(gp3L)), clamp01(linearToSrgb(bp3L))];
}

/** y-encoded Display-P3 (0-1) → y-encoded sRGB (0-1) */
export function p3ToRgb(rp3: number, gp3: number, bp3: number): [number, number, number] {
  // P3 y → linear
  const rp3L = srgbToLinear(rp3);
  const gp3L = srgbToLinear(gp3);
  const bp3L = srgbToLinear(bp3);

  // linear P3 → XYZ → linear sRGB
  const [X, Y, Z] = matMul([rp3L, gp3L, bp3L], P3_TO_XYZ);
  const [rL, gL, bL] = matMul([X, Y, Z], XYZ_TO_SRGB);

  // linear sRGB → y; clamp
  return [clamp01(linearToSrgb(rL)), clamp01(linearToSrgb(gL)), clamp01(linearToSrgb(bL))];
}

/** HSL → y-encoded Display-P3 (0-1) */
export function hslToP3(h: number, s: number, l: number): [number, number, number] {
  return rgbToP3(...hslToRgb(h, s, l));
}

/** y-encoded Display-P3 (0-1) → HSL */
export function p3ToHsl(rp3: number, gp3: number, bp3: number): [number, number, number] {
  const [r, g, b] = p3ToRgb(rp3, gp3, bp3);
  return rgbToHsl(Math.round(r * 255), Math.round(g * 255), Math.round(b * 255));
}

/* ==== DESTACK_GENERATED_START:ENUM:270000 ==== */
/**
 * ColorType
 */
export enum ColorType {
  BUILTIN = 1,
  RGB = 10,
  HSL = 11,
  P3 = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.COLOR_TYPE, ColorType);
/* ==== DESTACK_GENERATED_END:ENUM:270000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270002 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:270003 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:270003 ==== */
