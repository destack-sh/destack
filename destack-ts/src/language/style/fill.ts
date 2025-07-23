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
import type { File } from "@destack/language/data";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import type { Color } from "@destack/language/style/color";
import type { Gradient } from "@destack/language/style/gradient";
import { Style } from "@destack/language/style/style";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2100100 ==== */
/**
 * FillType
 */
export enum FillType {
  SOLID = 10,
  GRADIENT = 11,
  IMAGE = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILL_TYPE, FillType);
/* ==== DESTACK_GENERATED_END:ENUM:2100100 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100101 ==== */
/**
 * FillPosition
 */
export enum FillPosition {
  TOP_LEFT = 1,
  TOP_CENTER = 2,
  TOP_RIGHT = 3,
  LEFT = 10,
  CENTER = 11,
  RIGHT = 12,
  BOTTOM_LEFT = 20,
  BOTTOM_CENTER = 21,
  BOTTOM_RIGHT = 22,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILL_POSITION, FillPosition);
/* ==== DESTACK_GENERATED_END:ENUM:2100101 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100102 ==== */
/**
 * FillSize
 */
export enum FillSize {
  FILL = 1,
  STRETCH = 2,
  FIT = 3,
  TILE = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILL_SIZE, FillSize);
/* ==== DESTACK_GENERATED_END:ENUM:2100102 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2100400 ==== */
/**
 * A fill value.
 */
export class Fill extends StructFrozen {
  static metatype: StructType = StructType.FILL;
  static __isFrozen__: boolean = true;

  /**
   * Fill.type
   */
  readonly type: FillType;

  /**
   * Fill.style
   */
  get style(): FillStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as FillStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Fill.color
   */
  readonly color: Color | null;

  /**
   * Fill.gradient
   */
  readonly gradient: Gradient | null;

  /**
   * Fill.image
   */
  get image(): File | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr != null) {
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as File | null;
    }
    return null;
  }
  readonly imagePtr: NodeReference | null;

  /**
   * Fill.position
   */
  readonly position: FillPosition | null;

  /**
   * Fill.size
   */
  readonly size: FillSize | null;

  constructor(options: {
    type: FillType;
    style?: FillStyle | NodeReference | null;
    color?: Color | null;
    gradient?: Gradient | null;
    image?: File | NodeReference | null;
    position?: FillPosition | null;
    size?: FillSize | null;
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
      throw new Error(`Fill.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.constructor.name !== "NodeReference") {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style as NodeReference | null;
    let _color = options.color ?? null;
    this.color = _color;
    let _gradient = options.gradient ?? null;
    this.gradient = _gradient;
    let _image = options.image ?? null;
    if (_image != null && _image.constructor.name !== "NodeReference") {
      _image = (_image as Node).toRef();
    }
    this.imagePtr = _image as NodeReference | null;
    let _position = options.position ?? null;
    this.position = _position;
    let _size = options.size ?? null;
    this.size = _size;

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
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (
      (this.gradient == null) !== (other.gradient == null) ||
      (this.gradient != null && !this.gradient.equals(other.gradient))
    ) {
      return false;
    }
    if (!(this.imagePtr?.id === other.imagePtr?.id)) {
      return false;
    }
    if (!(this.position === other.position)) {
      return false;
    }
    if (!(this.size === other.size)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${FillType[this.type]}`);
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.color != null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      if (this.gradient != null) {
        propertyReprs.push(`gradient=${this.gradient.repr()}`);
      }
      if (this.image != null) {
        propertyReprs.push(`image=${this.image?.repr()}`);
      }
      if (this.position != null) {
        propertyReprs.push(`position=${FillPosition[this.position]}`);
      }
      if (this.size != null) {
        propertyReprs.push(`size=${FillSize[this.size]}`);
      }
      // @ts-expect-error(readonly) */
      this._repr = `<Fill ${propertyReprs.join(" ")}>`;
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
    if (this.color != null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.gradient != null) {
      h = (h * 31 + this.gradient.hash()) & 0xffffffff;
    }
    if (this.imagePtr != null) {
      h = (h * 31 + hashString(this.imagePtr.id)) & 0xffffffff;
    }
    if (this.position != null) {
      h = (h * 31 + this.position) & 0xffffffff;
    }
    if (this.size != null) {
      h = (h * 31 + this.size) & 0xffffffff;
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
registerStructClass(StructType.FILL, Fill);
/* ==== DESTACK_GENERATED_END:STRUCT:2100400 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2100400 ==== */
/**
 * A fill style.
 */
export class FillStyle extends Style {
  static metatype: NodeType = NodeType.FILL_STYLE;

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
  get precededBy(): FillStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as FillStyle | null;
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
   * FillStyle.type
   */
  /**
   * FillStyle.type
   */
  get type(): FillType {
    return this._type;
  }
  set type(value: FillType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: FillType;

  /**
   * FillStyle.color
   */
  /**
   * FillStyle.color
   */
  get color(): Color | null {
    return this._color;
  }
  set color(value: Color | null) {
    const prop = (this.constructor as NodeClass).__properties__["color"];
    this._session.updateSetProperty(this, prop, value);
    this._color = value;
  }
  _color: Color | null;

  /**
   * FillStyle.gradient
   */
  /**
   * FillStyle.gradient
   */
  get gradient(): Gradient | null {
    return this._gradient;
  }
  set gradient(value: Gradient | null) {
    const prop = (this.constructor as NodeClass).__properties__["gradient"];
    this._session.updateSetProperty(this, prop, value);
    this._gradient = value;
  }
  _gradient: Gradient | null;

  /**
   * FillStyle.image
   */
  get image(): File | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as File | null;
    }
    return null;
  }
  set image(node: File | null) {
    if (node === null) {
      this.imagePtr = null;
    } else {
      this.imagePtr = node.toRef();
    }
  }
  /**
   * FillStyle.image
   */
  get imagePtr(): NodeReference | null {
    return this._imagePtr;
  }
  set imagePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["image"];
    this._session.updateSetProperty(this, prop, value);
    this._imagePtr = value;
  }
  _imagePtr: NodeReference | null;

  /**
   * FillStyle.position
   */
  /**
   * FillStyle.position
   */
  get position(): FillPosition | null {
    return this._position;
  }
  set position(value: FillPosition | null) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: FillPosition | null;

  /**
   * FillStyle.size
   */
  /**
   * FillStyle.size
   */
  get size(): FillSize | null {
    return this._size;
  }
  set size(value: FillSize | null) {
    const prop = (this.constructor as NodeClass).__properties__["size"];
    this._session.updateSetProperty(this, prop, value);
    this._size = value;
  }
  _size: FillSize | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: FillStyle | NodeReference | null;
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
    type: FillType;
    color?: Color | null;
    gradient?: Gradient | null;
    image?: File | NodeReference | null;
    position?: FillPosition | null;
    size?: FillSize | null;
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
        throw new Error(`no active Space for FillStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`FillStyle.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`FillStyle.materialization is required`);
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
        throw new Error(`no active Branch for FillStyle`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`FillStyle.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for FillStyle`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`FillStyle.snapshot is required`);
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
      _name = "FillStyle";
    }
    if (_name === null) {
      throw new Error(`FillStyle.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`FillStyle.orderKey is required`);
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`FillStyle.type is required`);
    }
    this._type = _type;
    let _color = options.color ?? null;
    this._color = _color;
    let _gradient = options.gradient ?? null;
    this._gradient = _gradient;
    let _image = options.image ?? null;
    if (_image != null && _image.constructor.name !== "NodeReference") {
      _image = (_image as Node).toRef();
    }
    this._imagePtr = _image as NodeReference | null;
    let _position = options.position ?? null;
    this._position = _position;
    let _size = options.size ?? null;
    this._size = _size;

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
          `FillStyle.createdAt and FillStyle.updatedAt are required for existing Nodes`,
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
    if (
      (this._color == null) !== (other._color == null) ||
      (this._color != null && !this._color.equals(other._color))
    ) {
      return false;
    }
    if (
      (this._gradient == null) !== (other._gradient == null) ||
      (this._gradient != null && !this._gradient.equals(other._gradient))
    ) {
      return false;
    }
    if (!(this._imagePtr?.id === other._imagePtr?.id)) {
      return false;
    }
    if (!(this._position === other._position)) {
      return false;
    }
    if (!(this._size === other._size)) {
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
    if (this._color != null) {
      h = (h * 31 + this._color.hash()) & 0xffffffff;
    }
    if (this._gradient != null) {
      h = (h * 31 + this._gradient.hash()) & 0xffffffff;
    }
    if (this._imagePtr != null) {
      h = (h * 31 + hashString(this._imagePtr.id)) & 0xffffffff;
    }
    if (this._position != null) {
      h = (h * 31 + this._position) & 0xffffffff;
    }
    if (this._size != null) {
      h = (h * 31 + this._size) & 0xffffffff;
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
      type: NodeType.FILL_STYLE,
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
    propertyReprs.push(`type=${FillType[this.type]}`);
    if (this.color != null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    if (this.gradient != null) {
      propertyReprs.push(`gradient=${this.gradient.repr()}`);
    }
    if (this.image != null) {
      propertyReprs.push(`image=${this.image?.repr()}`);
    }
    if (this.position != null) {
      propertyReprs.push(`position=${FillPosition[this.position]}`);
    }
    if (this.size != null) {
      propertyReprs.push(`size=${FillSize[this.size]}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<FillStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILL_STYLE, FillStyle);
/* ==== DESTACK_GENERATED_END:NODE:2100400 ==== */
