import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
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
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Scene } from "@destack/language/scene";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  ColorHueProto,
  ColorIntentProto,
  ColorProto,
  ColorShadeProto,
  ColorStyleProto,
  ColorTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:2100300 ==== */
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
   * Color.style
   */
  get style(): ColorStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
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
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.hue != null) {
        propertyReprs.push(`hue=${ColorHue[this.hue]}`);
      }
      if (this.shade != null) {
        propertyReprs.push(`shade=${ColorShade[this.shade]}`);
      }
      if (this.intent != null) {
        propertyReprs.push(`intent=${ColorIntent[this.intent]}`);
      }
      if (this.x != null) {
        propertyReprs.push(`x=${this.x}`);
      }
      if (this.y != null) {
        propertyReprs.push(`y=${this.y}`);
      }
      if (this.z != null) {
        propertyReprs.push(`z=${this.z}`);
      }
      if (this.alpha != null) {
        propertyReprs.push(`alpha=${this.alpha}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Color ${propertyReprs.join(" ")}>`;
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
    if (this.hue != null) {
      h = (h * 31 + this.hue) & 0xffffffff;
    }
    if (this.shade != null) {
      h = (h * 31 + this.shade) & 0xffffffff;
    }
    if (this.intent != null) {
      h = (h * 31 + this.intent) & 0xffffffff;
    }
    if (this.x != null) {
      h = (h * 31 + hashFloat(this.x)) & 0xffffffff;
    }
    if (this.y != null) {
      h = (h * 31 + hashFloat(this.y)) & 0xffffffff;
    }
    if (this.z != null) {
      h = (h * 31 + hashFloat(this.z)) & 0xffffffff;
    }
    if (this.alpha != null) {
      h = (h * 31 + hashFloat(this.alpha)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Color.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Color): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100300;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.hue != null) {
      objectValue["102"] = object.hue;
    }
    if (object.shade != null) {
      objectValue["103"] = object.shade;
    }
    if (object.intent != null) {
      objectValue["104"] = object.intent;
    }
    if (object.x != null) {
      objectValue["105"] = object.x;
    }
    if (object.y != null) {
      objectValue["106"] = object.y;
    }
    if (object.z != null) {
      objectValue["107"] = object.z;
    }
    if (object.alpha != null) {
      objectValue["108"] = object.alpha;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Color {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const stylePtrValue = objectValue["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const hueValue = objectValue["102"];
    const unpackedHue = hueValue != undefined ? Number(hueValue) : null;
    const shadeValue = objectValue["103"];
    const unpackedShade = shadeValue != undefined ? Number(shadeValue) : null;
    const intentValue = objectValue["104"];
    const unpackedIntent = intentValue != undefined ? Number(intentValue) : null;
    const xValue = objectValue["105"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["106"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectValue["107"];
    const unpackedZ = zValue != undefined ? zValue : null;
    const alphaValue = objectValue["108"];
    const unpackedAlpha = alphaValue != undefined ? alphaValue : null;
    return new Color({
      type: Number(objectValue["100"]),
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
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<ColorProto> = { metatype: 2100300 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:2100300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2100300 ==== */
/**
 * A color style, with an optional dark variant.
 */
export class ColorStyle extends Style {
  static metatype: NodeType = NodeType.COLOR_STYLE;

  /**
   * Style.parent
   */
  get parent(): Scene | View | Theme | Palette | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this CustomEntity is an instance of.
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
   * Entity.materialization
   */
  readonly materialization: Materialization;

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
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get precededBy(): ColorStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as ColorStyle | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

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
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

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
   * Entity.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * The main / root Script of this Node.
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
   * The main / root Script of this Node.
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
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * ColorStyle.type
   */
  /**
   * ColorStyle.type
   */
  get type(): ColorType {
    return this._type;
  }
  set type(value: ColorType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: ColorType;

  /**
   * ColorStyle.hue
   */
  /**
   * ColorStyle.hue
   */
  get hue(): ColorHue | null {
    return this._hue;
  }
  set hue(value: ColorHue | null) {
    const prop = (this.constructor as NodeClass).__properties__["hue"];
    this._session.updateSetProperty(this, prop, value);
    this._hue = value;
  }
  _hue: ColorHue | null;

  /**
   * ColorStyle.shade
   */
  /**
   * ColorStyle.shade
   */
  get shade(): ColorShade | null {
    return this._shade;
  }
  set shade(value: ColorShade | null) {
    const prop = (this.constructor as NodeClass).__properties__["shade"];
    this._session.updateSetProperty(this, prop, value);
    this._shade = value;
  }
  _shade: ColorShade | null;

  /**
   * ColorStyle.intent
   */
  /**
   * ColorStyle.intent
   */
  get intent(): ColorIntent | null {
    return this._intent;
  }
  set intent(value: ColorIntent | null) {
    const prop = (this.constructor as NodeClass).__properties__["intent"];
    this._session.updateSetProperty(this, prop, value);
    this._intent = value;
  }
  _intent: ColorIntent | null;

  /**
   * ColorStyle.x
   */
  /**
   * ColorStyle.x
   */
  get x(): number | null {
    return this._x;
  }
  set x(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["x"];
    this._session.updateSetProperty(this, prop, value);
    this._x = value;
  }
  _x: number | null;

  /**
   * ColorStyle.y
   */
  /**
   * ColorStyle.y
   */
  get y(): number | null {
    return this._y;
  }
  set y(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["y"];
    this._session.updateSetProperty(this, prop, value);
    this._y = value;
  }
  _y: number | null;

  /**
   * ColorStyle.z
   */
  /**
   * ColorStyle.z
   */
  get z(): number | null {
    return this._z;
  }
  set z(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["z"];
    this._session.updateSetProperty(this, prop, value);
    this._z = value;
  }
  _z: number | null;

  /**
   * ColorStyle.alpha
   */
  /**
   * ColorStyle.alpha
   */
  get alpha(): number | null {
    return this._alpha;
  }
  set alpha(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["alpha"];
    this._session.updateSetProperty(this, prop, value);
    this._alpha = value;
  }
  _alpha: number | null;

  /**
   * ColorStyle.dark
   */
  /**
   * ColorStyle.dark
   */
  get dark(): Color | null {
    return this._dark;
  }
  set dark(value: Color | null) {
    const prop = (this.constructor as NodeClass).__properties__["dark"];
    this._session.updateSetProperty(this, prop, value);
    this._dark = value;
  }
  _dark: Color | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: ColorStyle | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    orderKey?: string;
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type: ColorType;
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
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`ColorStyle has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`ColorStyle has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ColorStyle.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`ColorStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ColorStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "ColorStyle";
    }
    if (_name === null) {
      throw new Error(`ColorStyle.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`ColorStyle.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`ColorStyle.type is required`);
    }
    this._type = _type;
    let _hue = options.hue ?? null;
    this._hue = _hue;
    let _shade = options.shade ?? null;
    this._shade = _shade;
    let _intent = options.intent ?? null;
    this._intent = _intent;
    let _x = options.x ?? null;
    this._x = _x;
    let _y = options.y ?? null;
    this._y = _y;
    let _z = options.z ?? null;
    this._z = _z;
    let _alpha = options.alpha ?? null;
    this._alpha = _alpha;
    let _dark = options.dark ?? null;
    this._dark = _dark;

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
          `ColorStyle.createdAt and ColorStyle.updatedAt are required for existing Nodes`,
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._hue === other._hue)) {
      return false;
    }
    if (!(this._shade === other._shade)) {
      return false;
    }
    if (!(this._intent === other._intent)) {
      return false;
    }
    if (
      (this._x == null) !== (other._x == null) ||
      (this._x != null && !(this._x === other._x || Math.abs(this._x - other._x) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._y == null) !== (other._y == null) ||
      (this._y != null && !(this._y === other._y || Math.abs(this._y - other._y) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._z == null) !== (other._z == null) ||
      (this._z != null && !(this._z === other._z || Math.abs(this._z - other._z) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._alpha == null) !== (other._alpha == null) ||
      (this._alpha != null &&
        !(this._alpha === other._alpha || Math.abs(this._alpha - other._alpha) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._dark == null) !== (other._dark == null) ||
      (this._dark != null && !this._dark.equals(other._dark))
    ) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._hue != null) {
      h = (h * 31 + this._hue) & 0xffffffff;
    }
    if (this._shade != null) {
      h = (h * 31 + this._shade) & 0xffffffff;
    }
    if (this._intent != null) {
      h = (h * 31 + this._intent) & 0xffffffff;
    }
    if (this._x != null) {
      h = (h * 31 + hashFloat(this._x)) & 0xffffffff;
    }
    if (this._y != null) {
      h = (h * 31 + hashFloat(this._y)) & 0xffffffff;
    }
    if (this._z != null) {
      h = (h * 31 + hashFloat(this._z)) & 0xffffffff;
    }
    if (this._alpha != null) {
      h = (h * 31 + hashFloat(this._alpha)) & 0xffffffff;
    }
    if (this._dark != null) {
      h = (h * 31 + this._dark.hash()) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.COLOR_STYLE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
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
    propertyReprs.push(`type=${ColorType[this.type]}`);
    if (this.hue != null) {
      propertyReprs.push(`hue=${ColorHue[this.hue]}`);
    }
    if (this.shade != null) {
      propertyReprs.push(`shade=${ColorShade[this.shade]}`);
    }
    if (this.intent != null) {
      propertyReprs.push(`intent=${ColorIntent[this.intent]}`);
    }
    if (this.x != null) {
      propertyReprs.push(`x=${this.x}`);
    }
    if (this.y != null) {
      propertyReprs.push(`y=${this.y}`);
    }
    if (this.z != null) {
      propertyReprs.push(`z=${this.z}`);
    }
    if (this.alpha != null) {
      propertyReprs.push(`alpha=${this.alpha}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<ColorStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return ColorStyle.__packValue__(this);
  }

  static __packValue__(object: ColorStyle): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["24"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["27"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
    if (object._hue != null) {
      objectValue["200"] = object._hue;
    }
    if (object._shade != null) {
      objectValue["201"] = object._shade;
    }
    if (object._intent != null) {
      objectValue["202"] = object._intent;
    }
    if (object._x != null) {
      objectValue["203"] = object._x;
    }
    if (object._y != null) {
      objectValue["204"] = object._y;
    }
    if (object._z != null) {
      objectValue["205"] = object._z;
    }
    if (object._alpha != null) {
      objectValue["206"] = object._alpha;
    }
    if (object._dark != null) {
      objectValue["207"] = object._dark.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ColorStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const hueValue = objectValue["200"];
    const unpackedHue = hueValue != undefined ? Number(hueValue) : null;
    const shadeValue = objectValue["201"];
    const unpackedShade = shadeValue != undefined ? Number(shadeValue) : null;
    const intentValue = objectValue["202"];
    const unpackedIntent = intentValue != undefined ? Number(intentValue) : null;
    const xValue = objectValue["203"];
    const unpackedX = xValue != undefined ? xValue : null;
    const yValue = objectValue["204"];
    const unpackedY = yValue != undefined ? yValue : null;
    const zValue = objectValue["205"];
    const unpackedZ = zValue != undefined ? zValue : null;
    const alphaValue = objectValue["206"];
    const unpackedAlpha = alphaValue != undefined ? alphaValue : null;
    const darkValue = objectValue["207"];
    const unpackedDark =
      darkValue != undefined
        ? _Color.fromValue(darkValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["24"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ColorStyle({
      type: Number(objectValue["100"]),
      hue: unpackedHue,
      shade: unpackedShade,
      intent: unpackedIntent,
      x: unpackedX,
      y: unpackedY,
      z: unpackedZ,
      alpha: unpackedAlpha,
      dark: unpackedDark,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      precededBy: unpackedPrecededByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      orderKey: objectValue["27"],
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<ColorStyleProto> = { metatype: 2100300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as ColorTypeProto;
    if (object._hue != null) {
      objectProto.hue = Number(object._hue) as ColorHueProto;
    }
    if (object._shade != null) {
      objectProto.shade = Number(object._shade) as ColorShadeProto;
    }
    if (object._intent != null) {
      objectProto.intent = Number(object._intent) as ColorIntentProto;
    }
    if (object._x != null) {
      objectProto.x = object._x;
    }
    if (object._y != null) {
      objectProto.y = object._y;
    }
    if (object._z != null) {
      objectProto.z = object._z;
    }
    if (object._alpha != null) {
      objectProto.alpha = object._alpha;
    }
    if (object._dark != null) {
      objectProto.dark = object._dark.toProto();
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ColorStyle({
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
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
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
      isExtensible: objectProto.isExtensible,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
/* ==== DESTACK_GENERATED_END:NODE:2100300 ==== */

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

/* ==== DESTACK_GENERATED_START:ENUM:2100000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100002 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100003 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2100003 ==== */
