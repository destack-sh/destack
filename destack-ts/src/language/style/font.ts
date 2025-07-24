import type {
  Branch,
  NodeClass,
  NodeReference,
  PackedCache,
  Session,
  Snapshot,
  Space,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  EnumType,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core";
import type { Length } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import type { Fill } from "@destack/language/style/fill";
import { Style } from "@destack/language/style/style";
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
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as FontStyle | null;
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
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 11 /* FontType.SANS */;
    }
    if (_type === null) {
      throw new Error(`Font.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name !== "NodeReference") {
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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
      // @ts-expect-error(readonly) */
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
      return this._session.graph.get(nodePtr) as Entity | null;
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
      return this._session.graph.get(nodePtr) as Space | null;
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
      return this._session.graph.get(nodePtr) as Entity | null;
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
      return this._session.graph.get(nodePtr) as Branch | null;
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
      return this._session.graph.get(nodePtr) as Snapshot | null;
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
      return this._session.graph.get(nodePtr) as FontStyle | null;
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
      return this._session.graph.get(nodePtr) as Entity | null;
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
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

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
  get updatedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
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
      return this._session.graph.get(nodePtr) as Script | null;
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
      return this._session.graph.get(nodePtr) as Script | null;
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
    createdBy?: Entity | NodeReference;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: Entity | NodeReference | null;
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
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null ? options.parent.toRef() : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name !== "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
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
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
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
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
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
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
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
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
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

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = this._session.actorPtr;
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
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorPtr;
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
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
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
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FONT_STYLE, FontStyle);
/* ==== DESTACK_GENERATED_END:NODE:2100500 ==== */
