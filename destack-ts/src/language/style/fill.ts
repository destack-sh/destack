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
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Color } from "@destack/language/style/color";
import type { Gradient } from "@destack/language/style/gradient";
import { Style } from "@destack/language/style/style";
import {
  FillPositionProto,
  FillProto,
  FillSizeProto,
  FillStyleProto,
  FillTypeProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
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
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as FillStyle | null;
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
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as File | null;
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
      throw new Error(`Fill.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _gradient = options.gradient ?? null;
    this.gradient = _gradient;
    let _image = options.image ?? null;
    if (_image != null && _image.metatype != StructType.NODE_REFERENCE) {
      _image = (_image as Node).toRef();
    }
    this.imagePtr = _image;
    let _position = options.position ?? null;
    this.position = _position;
    let _size = options.size ?? null;
    this.size = _size;

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
      // @ts-expect-error(readonly)
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

  toValue(): { readonly [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Fill.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Fill): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100400;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["102"] = object.color.toValue();
    }
    if (object.gradient != null) {
      objectValue["103"] = object.gradient.toValue();
    }
    if (object.imagePtr != null) {
      objectValue["104"] = object.imagePtr.toValue();
    }
    if (object.position != null) {
      objectValue["105"] = object.position;
    }
    if (object.size != null) {
      objectValue["106"] = object.size;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Fill {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    const stylePtrValue = objectValue["101"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? _NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const colorValue = objectValue["102"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const gradientValue = objectValue["103"];
    const unpackedGradient =
      gradientValue != undefined
        ? _Gradient.fromValue(gradientValue, _session, _supergraph, _graph, _connection)
        : null;
    const imagePtrValue = objectValue["104"];
    const unpackedImagePtr =
      imagePtrValue != undefined
        ? _NodeReference.fromValue(imagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["105"];
    const unpackedPosition = positionValue != undefined ? Number(positionValue) : null;
    const sizeValue = objectValue["106"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    return new Fill({
      type: Number(objectValue["100"]),
      style: unpackedStylePtr,
      color: unpackedColor,
      gradient: unpackedGradient,
      image: unpackedImagePtr,
      position: unpackedPosition,
      size: unpackedSize,
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
  ): Fill {
    return Fill.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FillProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Fill.__packProto__(this);
    }
    return this._proto as FillProto;
  }

  static __packProto__(object: Fill): FillProto {
    const objectProto: Partial<FillProto> = { metatype: 2100400 };
    objectProto.type = Number(object.type) as FillTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.gradient != null) {
      objectProto.gradient = object.gradient.toProto();
    }
    if (object.imagePtr != null) {
      objectProto.imagePtr = object.imagePtr.toProto();
    }
    if (object.position != null) {
      objectProto.position = Number(object.position) as FillPositionProto;
    }
    if (object.size != null) {
      objectProto.size = Number(object.size) as FillSizeProto;
    }
    return objectProto as FillProto;
  }

  static __unpackProto__(
    objectProto: FillProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Fill {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    return new Fill({
      type: Number(objectProto.type) as FillType,
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
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      gradient:
        objectProto.gradient != undefined
          ? _Gradient.fromProto(objectProto.gradient!, _session, _supergraph, _graph, _connection)
          : null,
      image:
        objectProto.imagePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.imagePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      position:
        objectProto.position != undefined ? (Number(objectProto.position) as FillPosition) : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FillSize) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: FillProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Fill {
    return Fill.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Fill {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FillProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
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
  get precededBy(): FillStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as FillStyle | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instantiationRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instantiationRootPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instantiationRootPtr: NodeReference | null;

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
      return this._supergraph.get(nodePtr.id) as File | null;
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
    instantiationRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    orderKey?: string;
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type: FillType;
    color?: Color | null;
    gradient?: Gradient | null;
    image?: File | NodeReference | null;
    position?: FillPosition | null;
    size?: FillSize | null;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for FillStyle`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`FillStyle.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`FillStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
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
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
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
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instantiationRoot = options.instantiationRoot ?? null;
    if (_instantiationRoot != null && _instantiationRoot.metatype != StructType.NODE_REFERENCE) {
      _instantiationRoot = (_instantiationRoot as Node).toRef();
    }
    this.instantiationRootPtr = _instantiationRoot;
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
      throw new Error(`FillStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "FillStyle";
    }
    if (_name === null) {
      throw new Error(`FillStyle.name is required`);
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
      throw new Error(`FillStyle.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
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
    if (_image != null && _image.metatype != StructType.NODE_REFERENCE) {
      _image = (_image as Node).toRef();
    }
    this._imagePtr = _image;
    let _position = options.position ?? null;
    this._position = _position;
    let _size = options.size ?? null;
    this._size = _size;

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
          `FillStyle.createdAt and FillStyle.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
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
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<FillStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return FillStyle.__packValue__(this);
  }

  static __packValue__(object: FillStyle): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100400;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectValue["11"] = object.definitionPtr.toValue();
    }
    objectValue["12"] = object.branchPtr.toValue();
    objectValue["13"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["14"] = object.precededByPtr.toValue();
    }
    if (object.instantiationRootPtr != null) {
      objectValue["15"] = object.instantiationRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["31"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
    if (object._color != null) {
      objectValue["200"] = object._color.toValue();
    }
    if (object._gradient != null) {
      objectValue["201"] = object._gradient.toValue();
    }
    if (object._imagePtr != null) {
      objectValue["202"] = object._imagePtr.toValue();
    }
    if (object._position != null) {
      objectValue["203"] = object._position;
    }
    if (object._size != null) {
      objectValue["204"] = object._size;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FillStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    const colorValue = objectValue["200"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const gradientValue = objectValue["201"];
    const unpackedGradient =
      gradientValue != undefined
        ? _Gradient.fromValue(gradientValue, _session, _supergraph, _graph, _connection)
        : null;
    const imagePtrValue = objectValue["202"];
    const unpackedImagePtr =
      imagePtrValue != undefined
        ? _NodeReference.fromValue(imagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["203"];
    const unpackedPosition = positionValue != undefined ? Number(positionValue) : null;
    const sizeValue = objectValue["204"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instantiationRootPtrValue = objectValue["15"];
    const unpackedInstantiationRootPtr =
      instantiationRootPtrValue != undefined
        ? _NodeReference.fromValue(
            instantiationRootPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new FillStyle({
      type: Number(objectValue["100"]),
      color: unpackedColor,
      gradient: unpackedGradient,
      image: unpackedImagePtr,
      position: unpackedPosition,
      size: unpackedSize,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromValue(
        objectValue["12"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromValue(
        objectValue["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instantiationRoot: unpackedInstantiationRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      orderKey: objectValue["31"],
      isExtensible: objectValue["90"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      script: unpackedScriptPtr,
      customValues: unpackedCustomValues,
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
  ): FillStyle {
    return FillStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FillStyleProto {
    return FillStyle.__packProto__(this);
  }

  static __packProto__(object: FillStyle): FillStyleProto {
    const objectProto: Partial<FillStyleProto> = { metatype: 2100400 };
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
    if (object.instantiationRootPtr != null) {
      objectProto.instantiationRootPtr = object.instantiationRootPtr.toProto();
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
    objectProto.type = Number(object._type) as FillTypeProto;
    if (object._color != null) {
      objectProto.color = object._color.toProto();
    }
    if (object._gradient != null) {
      objectProto.gradient = object._gradient.toProto();
    }
    if (object._imagePtr != null) {
      objectProto.imagePtr = object._imagePtr.toProto();
    }
    if (object._position != null) {
      objectProto.position = Number(object._position) as FillPositionProto;
    }
    if (object._size != null) {
      objectProto.size = Number(object._size) as FillSizeProto;
    }
    return objectProto as FillStyleProto;
  }

  static __unpackProto__(
    objectProto: FillStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FillStyle {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const _Gradient = STRUCT_CLASS_BY_TYPE[StructType.GRADIENT] as typeof Gradient;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new FillStyle({
      type: Number(objectProto.type) as FillType,
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      gradient:
        objectProto.gradient != undefined
          ? _Gradient.fromProto(objectProto.gradient!, _session, _supergraph, _graph, _connection)
          : null,
      image:
        objectProto.imagePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.imagePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      position:
        objectProto.position != undefined ? (Number(objectProto.position) as FillPosition) : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FillSize) : null,
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
      instantiationRoot:
        objectProto.instantiationRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instantiationRootPtr!,
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
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      isExtensible: objectProto.isExtensible,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FillStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FillStyle {
    return FillStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): FillStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FillStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILL_STYLE, FillStyle);
/* ==== DESTACK_GENERATED_END:NODE:2100400 ==== */
