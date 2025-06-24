import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  Graph,
  IsSubject,
  Length,
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
  MaterializationTypeProto,
  TextAlignProto,
  TextDecorationProto,
  TextTransformProto,
} from "@destack/proto";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12026 ==== */
/**
 * FontType
 */
export enum FontType {
  STYLE = 2,
  SERIF = 10,
  SANS = 11,
  MONO = 12,
}
/* ==== DESTACK_GENERATED_END:ENUM:12026 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12024 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:12024 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12025 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:12025 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12027 ==== */
/**
 * TextAlign
 */
export enum TextAlign {
  LEFT = 1,
  CENTER = 2,
  RIGHT = 3,
  JUSTIFY = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12027 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12028 ==== */
/**
 * TextDecoration
 */
export enum TextDecoration {
  NONE = 1,
  UNDERLINE = 2,
  STRIKETHROUGH = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:12028 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12029 ==== */
/**
 * TextTransform
 */
export enum TextTransform {
  NONE = 1,
  UPPERCASE = 2,
  LOWERCASE = 3,
  CAPITALIZE = 4,
}
/* ==== DESTACK_GENERATED_END:ENUM:12029 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12014 ==== */
/**
 * A font value.
 */
export class Font extends Struct {
  static metatype: StructType = StructType.FONT;
  static __isFrozen__: boolean = false;

  /**
   * FontBase.type
   */
  type: FontType;

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
  set style(value: FontStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * FontBase.weight
   */
  weight: FontWeight | null;

  /**
   * FontBase.color
   */
  color: Fill | null;

  /**
   * FontBase.size
   */
  size: FontSize | null;

  /**
   * FontBase.align
   */
  align: TextAlign | null;

  /**
   * FontBase.lineHeight
   */
  lineHeight: Length | null;

  /**
   * FontBase.letterSpacing
   */
  letterSpacing: Length | null;

  /**
   * FontBase.decoration
   */
  decoration: TextDecoration | null;

  /**
   * FontBase.transform
   */
  transform: TextTransform | null;

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

  toValue(): { [key: string]: any } {
    return Font.__packValue__(this);
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
    const weightValue = objectValue["50"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectValue["51"];
    const unpackedColor =
      colorValue != undefined ? Fill.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
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
    const styleValue = objectValue["41"];
    const unpackedStyle =
      styleValue != undefined ? NodeReference.fromValue(styleValue, _session, _supergraph, _graph, _connection) : null;
    return new Font({
      type: Number(objectValue["30"]),
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
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
  ): Font {
    return Font.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontProto {
    return Font.__packProto__(this);
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
      decoration: objectProto.decoration != undefined ? (Number(objectProto.decoration) as TextDecoration) : null,
      transform: objectProto.transform != undefined ? (Number(objectProto.transform) as TextTransform) : null,
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(objectProto.stylePtr!, _session, _supergraph, _graph, _connection)
          : null,
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12014 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12022 ==== */
/**
 * A font style.
 */
export class FontStyle extends Node implements Style {
  static metatype: NodeType = NodeType.FONT_STYLE;
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

  /**
   * Style.parent
   */
  get parent(): Scene | (Node & View) | Theme | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | (Node & View) | Theme | null;
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
   * FontBase.type
   */
  type: FontType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * FontBase.weight
   */
  weight: FontWeight | null;

  /**
   * FontBase.color
   */
  color: Fill | null;

  /**
   * FontBase.size
   */
  size: FontSize | null;

  /**
   * FontBase.align
   */
  align: TextAlign | null;

  /**
   * FontBase.lineHeight
   */
  lineHeight: Length | null;

  /**
   * FontBase.letterSpacing
   */
  letterSpacing: Length | null;

  /**
   * FontBase.decoration
   */
  decoration: TextDecoration | null;

  /**
   * FontBase.transform
   */
  transform: TextTransform | null;

  constructor(options: {
    id?: string;
    parent?: Scene | (Node & View) | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`FontStyle.materialization is required`);
    }
    this.materialization = _materialization;
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

  toValue(): { [key: string]: any } {
    return FontStyle.__packValue__(this);
  }

  static __packValue__(object: FontStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12022;
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
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const weightValue = objectValue["50"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectValue["51"];
    const unpackedColor =
      colorValue != undefined ? Fill.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
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
    return new FontStyle({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
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
  ): FontStyle {
    return FontStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontStyleProto {
    return FontStyle.__packProto__(this);
  }

  static __packProto__(object: FontStyle): FontStyleProto {
    const objectProto: Partial<FontStyleProto> = { metatype: 12022 };
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
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      decoration: objectProto.decoration != undefined ? (Number(objectProto.decoration) as TextDecoration) : null,
      transform: objectProto.transform != undefined ? (Number(objectProto.transform) as TextTransform) : null,
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
    objectProto: FontStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    return FontStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:12022 ==== */
