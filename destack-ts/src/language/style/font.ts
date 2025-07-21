import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import type { Length } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Fill } from "@destack/language/style/fill";
import { Style } from "@destack/language/style/style";
import {
  FontProto,
  FontSizeProto,
  FontStyleProto,
  FontTypeProto,
  FontWeightProto,
  MaterializationProto,
  TextAlignProto,
  TextDecorationProto,
  TextTransformProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2100200 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100200 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100201 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100201 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100202 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100202 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100203 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100203 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100204 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100204 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100205 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100205 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2100500 ==== */
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
   * Font.style
   */
  get style(): FontStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
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
    _cson?: any | null;
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
      _type = 11 /* FontType.SANS */;
    }
    if (_type === null) {
      throw new Error(`Font.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name != "NodeReference") {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style as NodeReference | null;
    let _weight = options.weight ?? null;
    if (_weight === null) {
      _weight = 400 /* FontWeight.NORMAL */;
    }
    this.weight = _weight;
    let _color = options.color ?? null;
    this.color = _color;
    let _size = options.size ?? null;
    if (_size === null) {
      _size = 16 /* FontSize.BASE */;
    }
    this.size = _size;
    let _align = options.align ?? null;
    if (_align === null) {
      _align = 1 /* TextAlign.LEFT */;
    }
    this.align = _align;
    let _lineHeight = options.lineHeight ?? null;
    this.lineHeight = _lineHeight;
    let _letterSpacing = options.letterSpacing ?? null;
    this.letterSpacing = _letterSpacing;
    let _decoration = options.decoration ?? null;
    if (_decoration === null) {
      _decoration = 1 /* TextDecoration.NONE */;
    }
    this.decoration = _decoration;
    let _transform = options.transform ?? null;
    if (_transform === null) {
      _transform = 1 /* TextTransform.NONE */;
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
    this._cson = options._cson ?? null;
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
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.weight != null) {
        propertyReprs.push(`weight=${FontWeight[this.weight]}`);
      }
      if (this.color != null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      if (this.size != null) {
        propertyReprs.push(`size=${FontSize[this.size]}`);
      }
      if (this.align != null) {
        propertyReprs.push(`align=${TextAlign[this.align]}`);
      }
      if (this.lineHeight != null) {
        propertyReprs.push(`lineHeight=${this.lineHeight.repr()}`);
      }
      if (this.letterSpacing != null) {
        propertyReprs.push(`letterSpacing=${this.letterSpacing.repr()}`);
      }
      if (this.decoration != null) {
        propertyReprs.push(`decoration=${TextDecoration[this.decoration]}`);
      }
      if (this.transform != null) {
        propertyReprs.push(`transform=${TextTransform[this.transform]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Font ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.stylePtr != null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.weight != null) {
      h = (h * 31 + this.weight) & 0xffffffff;
    }
    if (this.color != null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.size != null) {
      h = (h * 31 + this.size) & 0xffffffff;
    }
    if (this.align != null) {
      h = (h * 31 + this.align) & 0xffffffff;
    }
    if (this.lineHeight != null) {
      h = (h * 31 + this.lineHeight.hash()) & 0xffffffff;
    }
    if (this.letterSpacing != null) {
      h = (h * 31 + this.letterSpacing.hash()) & 0xffffffff;
    }
    if (this.decoration != null) {
      h = (h * 31 + this.decoration) & 0xffffffff;
    }
    if (this.transform != null) {
      h = (h * 31 + this.transform) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Font.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Font): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2100500;
    objectCson["100"] = object.type;
    if (object.stylePtr != null) {
      objectCson["101"] = object.stylePtr.toCson();
    }
    if (object.weight != null) {
      objectCson["102"] = object.weight;
    }
    if (object.color != null) {
      objectCson["103"] = object.color.toCson();
    }
    if (object.size != null) {
      objectCson["104"] = object.size;
    }
    if (object.align != null) {
      objectCson["105"] = object.align;
    }
    if (object.lineHeight != null) {
      objectCson["106"] = object.lineHeight.toCson();
    }
    if (object.letterSpacing != null) {
      objectCson["107"] = object.letterSpacing.toCson();
    }
    if (object.decoration != null) {
      objectCson["108"] = object.decoration;
    }
    if (object.transform != null) {
      objectCson["109"] = object.transform;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Font {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const stylePtrValue = objectCson["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromCson(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const weightValue = objectCson["102"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectCson["103"];
    const unpackedColor =
      colorValue != undefined
        ? _Fill.fromCson(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const sizeValue = objectCson["104"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const alignValue = objectCson["105"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const lineHeightValue = objectCson["106"];
    const unpackedLineHeight =
      lineHeightValue != undefined
        ? _Length.fromCson(lineHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const letterSpacingValue = objectCson["107"];
    const unpackedLetterSpacing =
      letterSpacingValue != undefined
        ? _Length.fromCson(letterSpacingValue, _session, _supergraph, _graph, _connection)
        : null;
    const decorationValue = objectCson["108"];
    const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : null;
    const transformValue = objectCson["109"];
    const unpackedTransform = transformValue != undefined ? Number(transformValue) : null;
    return new Font({
      type: Number(objectCson["100"]),
      style: unpackedStylePtr,
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Font {
    return Font.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Font.__packProto__(this);
    }
    return this._proto as FontProto;
  }

  static __packProto__(object: Font): FontProto {
    const objectProto: Partial<FontProto> = { metatype: 2100500 };
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    return new Font({
      type: Number(objectProto.type) as FontType,
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
      weight: objectProto.weight != undefined ? (Number(objectProto.weight) as FontWeight) : null,
      color:
        objectProto.color != undefined
          ? _Fill.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FontSize) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as TextAlign) : null,
      lineHeight:
        objectProto.lineHeight != undefined
          ? _Length.fromProto(objectProto.lineHeight!, _session, _supergraph, _graph, _connection)
          : null,
      letterSpacing:
        objectProto.letterSpacing != undefined
          ? _Length.fromProto(
              objectProto.letterSpacing!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
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
/* ==== DESTACK_GENERATED_END:STRUCT:2100500 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2100500 ==== */
/**
 * A font style.
 */
export class FontStyle extends Style {
  static metatype: NodeType = NodeType.FONT_STYLE;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): FontStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as FontStyle | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  set script(node: Script | null) {
    if (node === null) {
      this.scriptPtr = null;
    } else {
      this.scriptPtr = node.toRef();
    }
  }
  /**
   * The Script of this Entity.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

  /**
   * FontStyle.type
   */
  /**
   * FontStyle.type
   */
  get type(): FontType {
    return this._type;
  }
  set type(value: FontType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: FontType;

  /**
   * FontStyle.weight
   */
  /**
   * FontStyle.weight
   */
  get weight(): FontWeight | null {
    return this._weight;
  }
  set weight(value: FontWeight | null) {
    const prop = (this.constructor as NodeClass).__properties__["weight"];
    this._session.updateSetProperty(this, prop, value);
    this._weight = value;
  }
  _weight: FontWeight | null;

  /**
   * FontStyle.color
   */
  /**
   * FontStyle.color
   */
  get color(): Fill | null {
    return this._color;
  }
  set color(value: Fill | null) {
    const prop = (this.constructor as NodeClass).__properties__["color"];
    this._session.updateSetProperty(this, prop, value);
    this._color = value;
  }
  _color: Fill | null;

  /**
   * FontStyle.size
   */
  /**
   * FontStyle.size
   */
  get size(): FontSize | null {
    return this._size;
  }
  set size(value: FontSize | null) {
    const prop = (this.constructor as NodeClass).__properties__["size"];
    this._session.updateSetProperty(this, prop, value);
    this._size = value;
  }
  _size: FontSize | null;

  /**
   * FontStyle.align
   */
  /**
   * FontStyle.align
   */
  get align(): TextAlign | null {
    return this._align;
  }
  set align(value: TextAlign | null) {
    const prop = (this.constructor as NodeClass).__properties__["align"];
    this._session.updateSetProperty(this, prop, value);
    this._align = value;
  }
  _align: TextAlign | null;

  /**
   * FontStyle.lineHeight
   */
  /**
   * FontStyle.lineHeight
   */
  get lineHeight(): Length | null {
    return this._lineHeight;
  }
  set lineHeight(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["line_height"];
    this._session.updateSetProperty(this, prop, value);
    this._lineHeight = value;
  }
  _lineHeight: Length | null;

  /**
   * FontStyle.letterSpacing
   */
  /**
   * FontStyle.letterSpacing
   */
  get letterSpacing(): Length | null {
    return this._letterSpacing;
  }
  set letterSpacing(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["letter_spacing"];
    this._session.updateSetProperty(this, prop, value);
    this._letterSpacing = value;
  }
  _letterSpacing: Length | null;

  /**
   * FontStyle.decoration
   */
  /**
   * FontStyle.decoration
   */
  get decoration(): TextDecoration | null {
    return this._decoration;
  }
  set decoration(value: TextDecoration | null) {
    const prop = (this.constructor as NodeClass).__properties__["decoration"];
    this._session.updateSetProperty(this, prop, value);
    this._decoration = value;
  }
  _decoration: TextDecoration | null;

  /**
   * FontStyle.transform
   */
  /**
   * FontStyle.transform
   */
  get transform(): TextTransform | null {
    return this._transform;
  }
  set transform(value: TextTransform | null) {
    const prop = (this.constructor as NodeClass).__properties__["transform"];
    this._session.updateSetProperty(this, prop, value);
    this._transform = value;
  }
  _transform: TextTransform | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: FontStyle | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    type?: FontType;
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
        ? options.parent.constructor.name == "NodeReference"
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
      // _isNew
      options.id == null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name != "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for FontStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`FontStyle.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`FontStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for FontStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`FontStyle.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for FontStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`FontStyle.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name != "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name != "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "FontStyle";
    }
    if (_name === null) {
      throw new Error(`FontStyle.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`FontStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name != "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name != "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 11 /* FontType.SANS */;
    }
    if (_type === null) {
      throw new Error(`FontStyle.type is required`);
    }
    this._type = _type;
    let _weight = options.weight ?? null;
    if (_weight === null) {
      _weight = 400 /* FontWeight.NORMAL */;
    }
    this._weight = _weight;
    let _color = options.color ?? null;
    this._color = _color;
    let _size = options.size ?? null;
    if (_size === null) {
      _size = 16 /* FontSize.BASE */;
    }
    this._size = _size;
    let _align = options.align ?? null;
    if (_align === null) {
      _align = 1 /* TextAlign.LEFT */;
    }
    this._align = _align;
    let _lineHeight = options.lineHeight ?? null;
    this._lineHeight = _lineHeight;
    let _letterSpacing = options.letterSpacing ?? null;
    this._letterSpacing = _letterSpacing;
    let _decoration = options.decoration ?? null;
    if (_decoration === null) {
      _decoration = 1 /* TextDecoration.NONE */;
    }
    this._decoration = _decoration;
    let _transform = options.transform ?? null;
    if (_transform === null) {
      _transform = 1 /* TextTransform.NONE */;
    }
    this._transform = _transform;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(
          `FontStyle.createdAt and FontStyle.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name == "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._weight === other._weight)) {
      return false;
    }
    if (
      (this._color == null) !== (other._color == null) ||
      (this._color != null && !this._color.equals(other._color))
    ) {
      return false;
    }
    if (!(this._size === other._size)) {
      return false;
    }
    if (!(this._align === other._align)) {
      return false;
    }
    if (
      (this._lineHeight == null) !== (other._lineHeight == null) ||
      (this._lineHeight != null && !this._lineHeight.equals(other._lineHeight))
    ) {
      return false;
    }
    if (
      (this._letterSpacing == null) !== (other._letterSpacing == null) ||
      (this._letterSpacing != null && !this._letterSpacing.equals(other._letterSpacing))
    ) {
      return false;
    }
    if (!(this._decoration === other._decoration)) {
      return false;
    }
    if (!(this._transform === other._transform)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._weight != null) {
      h = (h * 31 + this._weight) & 0xffffffff;
    }
    if (this._color != null) {
      h = (h * 31 + this._color.hash()) & 0xffffffff;
    }
    if (this._size != null) {
      h = (h * 31 + this._size) & 0xffffffff;
    }
    if (this._align != null) {
      h = (h * 31 + this._align) & 0xffffffff;
    }
    if (this._lineHeight != null) {
      h = (h * 31 + this._lineHeight.hash()) & 0xffffffff;
    }
    if (this._letterSpacing != null) {
      h = (h * 31 + this._letterSpacing.hash()) & 0xffffffff;
    }
    if (this._decoration != null) {
      h = (h * 31 + this._decoration) & 0xffffffff;
    }
    if (this._transform != null) {
      h = (h * 31 + this._transform) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.FONT_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${FontType[this.type]}`);
    if (this.weight != null) {
      propertyReprs.push(`weight=${FontWeight[this.weight]}`);
    }
    if (this.color != null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    if (this.size != null) {
      propertyReprs.push(`size=${FontSize[this.size]}`);
    }
    if (this.align != null) {
      propertyReprs.push(`align=${TextAlign[this.align]}`);
    }
    if (this.lineHeight != null) {
      propertyReprs.push(`lineHeight=${this.lineHeight.repr()}`);
    }
    if (this.letterSpacing != null) {
      propertyReprs.push(`letterSpacing=${this.letterSpacing.repr()}`);
    }
    if (this.decoration != null) {
      propertyReprs.push(`decoration=${TextDecoration[this.decoration]}`);
    }
    if (this.transform != null) {
      propertyReprs.push(`transform=${TextTransform[this.transform]}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<FontStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return FontStyle.__packCson__(this);
  }

  static __packCson__(object: FontStyle): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 2100500;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    objectCson["100"] = object._type;
    if (object._weight != null) {
      objectCson["102"] = object._weight;
    }
    if (object._color != null) {
      objectCson["103"] = object._color.toCson();
    }
    if (object._size != null) {
      objectCson["104"] = object._size;
    }
    if (object._align != null) {
      objectCson["105"] = object._align;
    }
    if (object._lineHeight != null) {
      objectCson["106"] = object._lineHeight.toCson();
    }
    if (object._letterSpacing != null) {
      objectCson["107"] = object._letterSpacing.toCson();
    }
    if (object._decoration != null) {
      objectCson["108"] = object._decoration;
    }
    if (object._transform != null) {
      objectCson["109"] = object._transform;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const weightValue = objectCson["102"];
    const unpackedWeight = weightValue != undefined ? Number(weightValue) : null;
    const colorValue = objectCson["103"];
    const unpackedColor =
      colorValue != undefined
        ? _Fill.fromCson(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const sizeValue = objectCson["104"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const alignValue = objectCson["105"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const lineHeightValue = objectCson["106"];
    const unpackedLineHeight =
      lineHeightValue != undefined
        ? _Length.fromCson(lineHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const letterSpacingValue = objectCson["107"];
    const unpackedLetterSpacing =
      letterSpacingValue != undefined
        ? _Length.fromCson(letterSpacingValue, _session, _supergraph, _graph, _connection)
        : null;
    const decorationValue = objectCson["108"];
    const unpackedDecoration = decorationValue != undefined ? Number(decorationValue) : null;
    const transformValue = objectCson["109"];
    const unpackedTransform = transformValue != undefined ? Number(transformValue) : null;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new FontStyle({
      type: Number(objectCson["100"]),
      weight: unpackedWeight,
      color: unpackedColor,
      size: unpackedSize,
      align: unpackedAlign,
      lineHeight: unpackedLineHeight,
      letterSpacing: unpackedLetterSpacing,
      decoration: unpackedDecoration,
      transform: unpackedTransform,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FontStyle {
    return FontStyle.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): FontStyleProto {
    return FontStyle.__packProto__(this);
  }

  static __packProto__(object: FontStyle): FontStyleProto {
    const objectProto: Partial<FontStyleProto> = { metatype: 2100500 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.type = Number(object._type) as FontTypeProto;
    if (object._weight != null) {
      objectProto.weight = Number(object._weight) as FontWeightProto;
    }
    if (object._color != null) {
      objectProto.color = object._color.toProto();
    }
    if (object._size != null) {
      objectProto.size = Number(object._size) as FontSizeProto;
    }
    if (object._align != null) {
      objectProto.align = Number(object._align) as TextAlignProto;
    }
    if (object._lineHeight != null) {
      objectProto.lineHeight = object._lineHeight.toProto();
    }
    if (object._letterSpacing != null) {
      objectProto.letterSpacing = object._letterSpacing.toProto();
    }
    if (object._decoration != null) {
      objectProto.decoration = Number(object._decoration) as TextDecorationProto;
    }
    if (object._transform != null) {
      objectProto.transform = Number(object._transform) as TextTransformProto;
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new FontStyle({
      type: Number(objectProto.type) as FontType,
      weight: objectProto.weight != undefined ? (Number(objectProto.weight) as FontWeight) : null,
      color:
        objectProto.color != undefined
          ? _Fill.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FontSize) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as TextAlign) : null,
      lineHeight:
        objectProto.lineHeight != undefined
          ? _Length.fromProto(objectProto.lineHeight!, _session, _supergraph, _graph, _connection)
          : null,
      letterSpacing:
        objectProto.letterSpacing != undefined
          ? _Length.fromProto(
              objectProto.letterSpacing!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
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
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:2100500 ==== */
