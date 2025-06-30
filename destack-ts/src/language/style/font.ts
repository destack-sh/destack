import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  EnumType,
  Graph,
  IsSubject,
  Length,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
} from "@destack/language/core";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Fill, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  FontProto,
  FontSizeProto,
  FontStyleProto,
  FontTypeProto,
  FontWeightProto,
  TextAlignProto,
  TextDecorationProto,
  TextTransformProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:12014 ==== */
/**
 * A font value.
 */
export class Font extends StructFrozen {
  static metatype: StructType = StructType.FONT;
  static __isFrozen__: boolean = true;

  /**
   * Font.type
   */
  readonly type: FontType;

  /**
   * style
   */
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
  readonly stylePtr: NodeReference | null;

  /**
   * Font.weight
   */
  readonly weight: FontWeight | null;

  /**
   * Font.color
   */
  readonly color: Fill | null;

  /**
   * Font.size
   */
  readonly size: FontSize | null;

  /**
   * Font.align
   */
  readonly align: TextAlign | null;

  /**
   * Font.lineHeight
   */
  readonly lineHeight: Length | null;

  /**
   * Font.letterSpacing
   */
  readonly letterSpacing: Length | null;

  /**
   * Font.decoration
   */
  readonly decoration: TextDecoration | null;

  /**
   * Font.transform
   */
  readonly transform: TextTransform | null;

  constructor(options: {
    type?: FontType;
    style?: FontStyle | NodeReference | null;
    weight?: FontWeight | null;
    color?: Fill | null;
    size?: FontSize | null;
    align?: TextAlign | null;
    lineHeight?: Length | null;
    letterSpacing?: Length | null;
    decoration?: TextDecoration | null;
    transform?: TextTransform | null;
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = FontType.SANS;
    }
    if (_type === null) {
      throw new Error(`Font.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _weight = options.weight ?? null;
    if (_weight === null) {
      _weight = FontWeight.NORMAL;
    }
    this.weight = _weight;
    let _color = options.color ?? null;
    this.color = _color;
    let _size = options.size ?? null;
    if (_size === null) {
      _size = FontSize.BASE;
    }
    this.size = _size;
    let _align = options.align ?? null;
    if (_align === null) {
      _align = TextAlign.LEFT;
    }
    this.align = _align;
    let _lineHeight = options.lineHeight ?? null;
    this.lineHeight = _lineHeight;
    let _letterSpacing = options.letterSpacing ?? null;
    this.letterSpacing = _letterSpacing;
    let _decoration = options.decoration ?? null;
    if (_decoration === null) {
      _decoration = TextDecoration.NONE;
    }
    this.decoration = _decoration;
    let _transform = options.transform ?? null;
    if (_transform === null) {
      _transform = TextTransform.NONE;
    }
    this.transform = _transform;

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
    if (!(this.weight === other.weight)) {
      return false;
    }
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (!(this.size === other.size)) {
      return false;
    }
    if (!(this.align === other.align)) {
      return false;
    }
    if (
      (this.lineHeight == null) !== (other.lineHeight == null) ||
      (this.lineHeight != null && !this.lineHeight.equals(other.lineHeight))
    ) {
      return false;
    }
    if (
      (this.letterSpacing == null) !== (other.letterSpacing == null) ||
      (this.letterSpacing != null && !this.letterSpacing.equals(other.letterSpacing))
    ) {
      return false;
    }
    if (!(this.decoration === other.decoration)) {
      return false;
    }
    if (!(this.transform === other.transform)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${FontType[this.type]}`);
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.weight !== null) {
        propertyReprs.push(`weight=${FontWeight[this.weight]}`);
      }
      if (this.color !== null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      if (this.size !== null) {
        propertyReprs.push(`size=${FontSize[this.size]}`);
      }
      if (this.align !== null) {
        propertyReprs.push(`align=${TextAlign[this.align]}`);
      }
      if (this.lineHeight !== null) {
        propertyReprs.push(`lineHeight=${this.lineHeight.repr()}`);
      }
      if (this.letterSpacing !== null) {
        propertyReprs.push(`letterSpacing=${this.letterSpacing.repr()}`);
      }
      if (this.decoration !== null) {
        propertyReprs.push(`decoration=${TextDecoration[this.decoration]}`);
      }
      if (this.transform !== null) {
        propertyReprs.push(`transform=${TextTransform[this.transform]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Font ${propertyReprs.join(" ")}>`;
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
    if (this.weight !== null) {
      h = (h * 31 + this.weight) & 0xffffffff;
    }
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.size !== null) {
      h = (h * 31 + this.size) & 0xffffffff;
    }
    if (this.align !== null) {
      h = (h * 31 + this.align) & 0xffffffff;
    }
    if (this.lineHeight !== null) {
      h = (h * 31 + this.lineHeight.hash()) & 0xffffffff;
    }
    if (this.letterSpacing !== null) {
      h = (h * 31 + this.letterSpacing.hash()) & 0xffffffff;
    }
    if (this.decoration !== null) {
      h = (h * 31 + this.decoration) & 0xffffffff;
    }
    if (this.transform !== null) {
      h = (h * 31 + this.transform) & 0xffffffff;
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
      this._value = Font.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Font): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12014;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.weight != null) {
      objectValue["50"] = object.weight;
    }
    if (object.color != null) {
      objectValue["51"] = object.color.toValue();
    }
    if (object.size != null) {
      objectValue["52"] = object.size;
    }
    if (object.align != null) {
      objectValue["53"] = object.align;
    }
    if (object.lineHeight != null) {
      objectValue["54"] = object.lineHeight.toValue();
    }
    if (object.letterSpacing != null) {
      objectValue["55"] = object.letterSpacing.toValue();
    }
    if (object.decoration != null) {
      objectValue["56"] = object.decoration;
    }
    if (object.transform != null) {
      objectValue["57"] = object.transform;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Font {
    const stylePtrValue = objectValue["41"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const weightValue = objectValue["50"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectValue["51"];
    const unpackedColor =
      colorValue != undefined
        ? Fill.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const sizeValue = objectValue["52"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const alignValue = objectValue["53"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const lineHeightValue = objectValue["54"];
    const unpackedLineHeight =
      lineHeightValue != undefined
        ? Length.fromValue(lineHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const letterSpacingValue = objectValue["55"];
    const unpackedLetterSpacing =
      letterSpacingValue != undefined
        ? Length.fromValue(letterSpacingValue, _session, _supergraph, _graph, _connection)
        : null;
    const decorationValue = objectValue["56"];
    const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : null;
    const transformValue = objectValue["57"];
    const unpackedTransform = transformValue != undefined ? Number(transformValue) : null;
    return new Font({
      type: Number(objectValue["30"]),
      style: unpackedStylePtr,
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
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
  ): Font {
    return Font.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Font.__packProto__(this);
    }
    return this._proto as FontProto;
  }

  static __packProto__(object: Font): FontProto {
    const objectProto: Partial<FontProto> = { metatype: 12014 };
    objectProto.type = Number(object.type) as FontTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.weight != null) {
      objectProto.weight = Number(object.weight) as FontWeightProto;
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.size != null) {
      objectProto.size = Number(object.size) as FontSizeProto;
    }
    if (object.align != null) {
      objectProto.align = Number(object.align) as TextAlignProto;
    }
    if (object.lineHeight != null) {
      objectProto.lineHeight = object.lineHeight.toProto();
    }
    if (object.letterSpacing != null) {
      objectProto.letterSpacing = object.letterSpacing.toProto();
    }
    if (object.decoration != null) {
      objectProto.decoration = Number(object.decoration) as TextDecorationProto;
    }
    if (object.transform != null) {
      objectProto.transform = Number(object.transform) as TextTransformProto;
    }
    return objectProto as FontProto;
  }

  static __unpackProto__(
    objectProto: FontProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Font {
    return new Font({
      type: Number(objectProto.type) as FontType,
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(
              objectProto.stylePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      weight: objectProto.weight != undefined ? (Number(objectProto.weight) as FontWeight) : null,
      color:
        objectProto.color != undefined
          ? Fill.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FontSize) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as TextAlign) : null,
      lineHeight:
        objectProto.lineHeight != undefined
          ? Length.fromProto(objectProto.lineHeight!, _session, _supergraph, _graph, _connection)
          : null,
      letterSpacing:
        objectProto.letterSpacing != undefined
          ? Length.fromProto(objectProto.letterSpacing!, _session, _supergraph, _graph, _connection)
          : null,
      decoration:
        objectProto.decoration != undefined
          ? (Number(objectProto.decoration) as TextDecoration)
          : null,
      transform:
        objectProto.transform != undefined
          ? (Number(objectProto.transform) as TextTransform)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: FontProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Font {
    return Font.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Font {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FontProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.FONT, Font);
/* ==== DESTACK_GENERATED_END:STRUCT:12014 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12040 ==== */
/**
 * FontType
 */
export enum FontType {
  SERIF = 10,
  SANS = 11,
  MONO = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FONT_TYPE, FontType);
/* ==== DESTACK_GENERATED_END:ENUM:12040 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12041 ==== */
/**
 * FontWeight
 */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FONT_WEIGHT, FontWeight);
/* ==== DESTACK_GENERATED_END:ENUM:12041 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12042 ==== */
/**
 * FontSize
 */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FONT_SIZE, FontSize);
/* ==== DESTACK_GENERATED_END:ENUM:12042 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12043 ==== */
/**
 * TextAlign
 */
export enum TextAlign {
  LEFT = 1,
  CENTER = 2,
  RIGHT = 3,
  JUSTIFY = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TEXT_ALIGN, TextAlign);
/* ==== DESTACK_GENERATED_END:ENUM:12043 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12044 ==== */
/**
 * TextDecoration
 */
export enum TextDecoration {
  NONE = 1,
  UNDERLINE = 2,
  STRIKETHROUGH = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TEXT_DECORATION, TextDecoration);
/* ==== DESTACK_GENERATED_END:ENUM:12044 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12045 ==== */
/**
 * TextTransform
 */
export enum TextTransform {
  NONE = 1,
  UPPERCASE = 2,
  LOWERCASE = 3,
  CAPITALIZE = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TEXT_TRANSFORM, TextTransform);
/* ==== DESTACK_GENERATED_END:ENUM:12045 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12062 ==== */
/**
 * A font style.
 */
export class FontStyle extends Style {
  static metatype: NodeType = NodeType.FONT_STYLE;

  /**
   * Style.parent
   */
  get parent(): Scene | View | Theme | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | View | Theme | null;
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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * FontStyle.type
   */
  type: FontType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * FontStyle.weight
   */
  weight: FontWeight | null;

  /**
   * FontStyle.color
   */
  color: Fill | null;

  /**
   * FontStyle.size
   */
  size: FontSize | null;

  /**
   * FontStyle.align
   */
  align: TextAlign | null;

  /**
   * FontStyle.lineHeight
   */
  lineHeight: Length | null;

  /**
   * FontStyle.letterSpacing
   */
  letterSpacing: Length | null;

  /**
   * FontStyle.decoration
   */
  decoration: TextDecoration | null;

  /**
   * FontStyle.transform
   */
  transform: TextTransform | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: FontType;
    name: string;
    weight?: FontWeight | null;
    color?: Fill | null;
    size?: FontSize | null;
    align?: TextAlign | null;
    lineHeight?: Length | null;
    letterSpacing?: Length | null;
    decoration?: TextDecoration | null;
    transform?: TextTransform | null;
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`FontStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = FontType.SANS;
    }
    if (_type === null) {
      throw new Error(`FontStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`FontStyle.name is required`);
    }
    this.name = _name;
    let _weight = options.weight ?? null;
    if (_weight === null) {
      _weight = FontWeight.NORMAL;
    }
    this.weight = _weight;
    let _color = options.color ?? null;
    this.color = _color;
    let _size = options.size ?? null;
    if (_size === null) {
      _size = FontSize.BASE;
    }
    this.size = _size;
    let _align = options.align ?? null;
    if (_align === null) {
      _align = TextAlign.LEFT;
    }
    this.align = _align;
    let _lineHeight = options.lineHeight ?? null;
    this.lineHeight = _lineHeight;
    let _letterSpacing = options.letterSpacing ?? null;
    this.letterSpacing = _letterSpacing;
    let _decoration = options.decoration ?? null;
    if (_decoration === null) {
      _decoration = TextDecoration.NONE;
    }
    this.decoration = _decoration;
    let _transform = options.transform ?? null;
    if (_transform === null) {
      _transform = TextTransform.NONE;
    }
    this.transform = _transform;

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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.weight === other.weight)) {
      return false;
    }
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (!(this.size === other.size)) {
      return false;
    }
    if (!(this.align === other.align)) {
      return false;
    }
    if (
      (this.lineHeight == null) !== (other.lineHeight == null) ||
      (this.lineHeight != null && !this.lineHeight.equals(other.lineHeight))
    ) {
      return false;
    }
    if (
      (this.letterSpacing == null) !== (other.letterSpacing == null) ||
      (this.letterSpacing != null && !this.letterSpacing.equals(other.letterSpacing))
    ) {
      return false;
    }
    if (!(this.decoration === other.decoration)) {
      return false;
    }
    if (!(this.transform === other.transform)) {
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
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.weight !== null) {
      h = (h * 31 + this.weight) & 0xffffffff;
    }
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.size !== null) {
      h = (h * 31 + this.size) & 0xffffffff;
    }
    if (this.align !== null) {
      h = (h * 31 + this.align) & 0xffffffff;
    }
    if (this.lineHeight !== null) {
      h = (h * 31 + this.lineHeight.hash()) & 0xffffffff;
    }
    if (this.letterSpacing !== null) {
      h = (h * 31 + this.letterSpacing.hash()) & 0xffffffff;
    }
    if (this.decoration !== null) {
      h = (h * 31 + this.decoration) & 0xffffffff;
    }
    if (this.transform !== null) {
      h = (h * 31 + this.transform) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
    return new NodeReference({
      nodeType: NodeType.FONT_STYLE,
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
    propertyReprs.push(`type=${FontType[this.type]}`);
    if (this.weight !== null) {
      propertyReprs.push(`weight=${FontWeight[this.weight]}`);
    }
    if (this.color !== null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    if (this.size !== null) {
      propertyReprs.push(`size=${FontSize[this.size]}`);
    }
    if (this.align !== null) {
      propertyReprs.push(`align=${TextAlign[this.align]}`);
    }
    if (this.lineHeight !== null) {
      propertyReprs.push(`lineHeight=${this.lineHeight.repr()}`);
    }
    if (this.letterSpacing !== null) {
      propertyReprs.push(`letterSpacing=${this.letterSpacing.repr()}`);
    }
    if (this.decoration !== null) {
      propertyReprs.push(`decoration=${TextDecoration[this.decoration]}`);
    }
    if (this.transform !== null) {
      propertyReprs.push(`transform=${TextTransform[this.transform]}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<FontStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return FontStyle.__packValue__(this);
  }

  static __packValue__(object: FontStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12062;
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
    objectValue["22"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.weight != null) {
      objectValue["50"] = object.weight;
    }
    if (object.color != null) {
      objectValue["51"] = object.color.toValue();
    }
    if (object.size != null) {
      objectValue["52"] = object.size;
    }
    if (object.align != null) {
      objectValue["53"] = object.align;
    }
    if (object.lineHeight != null) {
      objectValue["54"] = object.lineHeight.toValue();
    }
    if (object.letterSpacing != null) {
      objectValue["55"] = object.letterSpacing.toValue();
    }
    if (object.decoration != null) {
      objectValue["56"] = object.decoration;
    }
    if (object.transform != null) {
      objectValue["57"] = object.transform;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    const weightValue = objectValue["50"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectValue["51"];
    const unpackedColor =
      colorValue != undefined
        ? Fill.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const sizeValue = objectValue["52"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const alignValue = objectValue["53"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const lineHeightValue = objectValue["54"];
    const unpackedLineHeight =
      lineHeightValue != undefined
        ? Length.fromValue(lineHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const letterSpacingValue = objectValue["55"];
    const unpackedLetterSpacing =
      letterSpacingValue != undefined
        ? Length.fromValue(letterSpacingValue, _session, _supergraph, _graph, _connection)
        : null;
    const decorationValue = objectValue["56"];
    const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : null;
    const transformValue = objectValue["57"];
    const unpackedTransform = transformValue != undefined ? Number(transformValue) : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new FontStyle({
      type: Number(objectValue["30"]),
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
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
  ): FontStyle {
    return FontStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontStyleProto {
    return FontStyle.__packProto__(this);
  }

  static __packProto__(object: FontStyle): FontStyleProto {
    const objectProto: Partial<FontStyleProto> = { metatype: 12062 };
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
    objectProto.type = Number(object.type) as FontTypeProto;
    objectProto.name = object.name;
    if (object.weight != null) {
      objectProto.weight = Number(object.weight) as FontWeightProto;
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.size != null) {
      objectProto.size = Number(object.size) as FontSizeProto;
    }
    if (object.align != null) {
      objectProto.align = Number(object.align) as TextAlignProto;
    }
    if (object.lineHeight != null) {
      objectProto.lineHeight = object.lineHeight.toProto();
    }
    if (object.letterSpacing != null) {
      objectProto.letterSpacing = object.letterSpacing.toProto();
    }
    if (object.decoration != null) {
      objectProto.decoration = Number(object.decoration) as TextDecorationProto;
    }
    if (object.transform != null) {
      objectProto.transform = Number(object.transform) as TextTransformProto;
    }
    return objectProto as FontStyleProto;
  }

  static __unpackProto__(
    objectProto: FontStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    return new FontStyle({
      type: Number(objectProto.type) as FontType,
      weight: objectProto.weight != undefined ? (Number(objectProto.weight) as FontWeight) : null,
      color:
        objectProto.color != undefined
          ? Fill.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FontSize) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as TextAlign) : null,
      lineHeight:
        objectProto.lineHeight != undefined
          ? Length.fromProto(objectProto.lineHeight!, _session, _supergraph, _graph, _connection)
          : null,
      letterSpacing:
        objectProto.letterSpacing != undefined
          ? Length.fromProto(objectProto.letterSpacing!, _session, _supergraph, _graph, _connection)
          : null,
      decoration:
        objectProto.decoration != undefined
          ? (Number(objectProto.decoration) as TextDecoration)
          : null,
      transform:
        objectProto.transform != undefined
          ? (Number(objectProto.transform) as TextTransform)
          : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
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
    objectProto: FontStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    return FontStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): FontStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FontStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FONT_STYLE, FontStyle);
/* ==== DESTACK_GENERATED_END:NODE:12062 ==== */
