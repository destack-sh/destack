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
import { Style } from "@destack/language/style/style";
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
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as ColorStyle | null;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Color.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name != "NodeReference") {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style as NodeReference | null;
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

    /* identity */
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
      // @ts-expect-error(readonly) */
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
  get precededBy(): ColorStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as ColorStyle | null;
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
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: ColorStyle | NodeReference | null;
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
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null
        ? options.parent.constructor.name == "NodeReference"
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
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
        throw new Error(`no active Space for ColorStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ColorStyle.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`ColorStyle.materialization is required`);
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
        throw new Error(`no active Branch for ColorStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`ColorStyle.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for ColorStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`ColorStyle.snapshot is required`);
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
      _name = "ColorStyle";
    }
    if (_name === null) {
      throw new Error(`ColorStyle.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ColorStyle.orderKey is required`);
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
          `ColorStyle.createdAt and ColorStyle.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name == "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : this._session.actorPtr;
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
      type: NodeType.COLOR_STYLE,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<ColorStyle "${this.path}" ${propertyReprs.join(" ")}>`;
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
