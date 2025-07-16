import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Axis2,
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
import type { Color } from "@destack/language/style/color";
import type { Palette } from "@destack/language/style/palette";
import { Style } from "@destack/language/style/style";
import type { Theme } from "@destack/language/style/theme";
import type { Space } from "@destack/language/universe";
import type { View } from "@destack/language/view";
import {
  MaterializationProto,
  ShadowPositionProto,
  ShadowProto,
  ShadowStyleProto,
  ShadowTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2100207 ==== */
/**
 * ShadowType
 */
export enum ShadowType {
  BOX = 10,
  REALISTIC = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SHADOW_TYPE, ShadowType);
/* ==== DESTACK_GENERATED_END:ENUM:2100207 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2100208 ==== */
/**
 * ShadowPosition
 */
export enum ShadowPosition {
  OUTSIDE = 1,
  INSIDE = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.SHADOW_POSITION, ShadowPosition);
/* ==== DESTACK_GENERATED_END:ENUM:2100208 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2100700 ==== */
/**
 * A shadow value.
 */
export class Shadow extends StructFrozen {
  static metatype: StructType = StructType.SHADOW;
  static __isFrozen__: boolean = true;

  /**
   * Shadow.type
   */
  readonly type: ShadowType;

  /**
   * Shadow.style
   */
  get style(): ShadowStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr != null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as ShadowStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Shadow.color
   */
  readonly color: Color | null;

  /**
   * Shadow.position
   */
  readonly position: ShadowPosition;

  /**
   * Shadow.offset
   */
  readonly offset: Axis2 | null;

  /**
   * Shadow.blur
   */
  readonly blur: number | null;

  /**
   * Shadow.spread
   */
  readonly spread: number | null;

  /**
   * Shadow.diffusion
   */
  readonly diffusion: number | null;

  constructor(options: {
    type?: ShadowType;
    style?: ShadowStyle | NodeReference | null;
    color?: Color | null;
    position?: ShadowPosition;
    offset?: Axis2 | null;
    blur?: number | null;
    spread?: number | null;
    diffusion?: number | null;
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
      _type = 10 /* ShadowType.BOX */;
    }
    if (_type === null) {
      throw new Error(`Shadow.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style.metatype != StructType.NODE_REFERENCE) {
      _style = (_style as Node).toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = 1 /* ShadowPosition.OUTSIDE */;
    }
    if (_position === null) {
      throw new Error(`Shadow.position is required`);
    }
    this.position = _position;
    let _offset = options.offset ?? null;
    this.offset = _offset;
    let _blur = options.blur ?? null;
    this.blur = _blur;
    let _spread = options.spread ?? null;
    this.spread = _spread;
    let _diffusion = options.diffusion ?? null;
    this.diffusion = _diffusion;

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
    if (!(this.position === other.position)) {
      return false;
    }
    if (
      (this.offset == null) !== (other.offset == null) ||
      (this.offset != null && !this.offset.equals(other.offset))
    ) {
      return false;
    }
    if (!(this.blur === other.blur)) {
      return false;
    }
    if (!(this.spread === other.spread)) {
      return false;
    }
    if (
      (this.diffusion == null) !== (other.diffusion == null) ||
      (this.diffusion != null &&
        !(this.diffusion === other.diffusion || Math.abs(this.diffusion - other.diffusion) < 1e-10))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ShadowType[this.type]}`);
      if (this.style != null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      if (this.color != null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      propertyReprs.push(`position=${ShadowPosition[this.position]}`);
      if (this.offset != null) {
        propertyReprs.push(`offset=${this.offset.repr()}`);
      }
      if (this.blur != null) {
        propertyReprs.push(`blur=${this.blur}`);
      }
      if (this.spread != null) {
        propertyReprs.push(`spread=${this.spread}`);
      }
      if (this.diffusion != null) {
        propertyReprs.push(`diffusion=${this.diffusion}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Shadow ${propertyReprs.join(" ")}>`;
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
    h = (h * 31 + this.position) & 0xffffffff;
    if (this.offset != null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.blur != null) {
      h = (h * 31 + hashInt(this.blur)) & 0xffffffff;
    }
    if (this.spread != null) {
      h = (h * 31 + hashInt(this.spread)) & 0xffffffff;
    }
    if (this.diffusion != null) {
      h = (h * 31 + hashFloat(this.diffusion)) & 0xffffffff;
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
      this._value = Shadow.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Shadow): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100700;
    objectValue["100"] = object.type;
    if (object.stylePtr != null) {
      objectValue["101"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["102"] = object.color.toValue();
    }
    objectValue["103"] = object.position;
    if (object.offset != null) {
      objectValue["104"] = object.offset.toValue();
    }
    if (object.blur != null) {
      objectValue["105"] = object.blur;
    }
    if (object.spread != null) {
      objectValue["106"] = object.spread;
    }
    if (object.diffusion != null) {
      objectValue["107"] = object.diffusion;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Shadow {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
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
    const offsetValue = objectValue["104"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const blurValue = objectValue["105"];
    const unpackedBlur = blurValue != undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["106"];
    const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["107"];
    const unpackedDiffusion = diffusionValue != undefined ? diffusionValue : null;
    return new Shadow({
      type: Number(objectValue["100"]),
      style: unpackedStylePtr,
      color: unpackedColor,
      position: Number(objectValue["103"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
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
  ): Shadow {
    return Shadow.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ShadowProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Shadow.__packProto__(this);
    }
    return this._proto as ShadowProto;
  }

  static __packProto__(object: Shadow): ShadowProto {
    const objectProto: Partial<ShadowProto> = { metatype: 2100700 };
    objectProto.type = Number(object.type) as ShadowTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    objectProto.position = Number(object.position) as ShadowPositionProto;
    if (object.offset != null) {
      objectProto.offset = object.offset.toProto();
    }
    if (object.blur != null) {
      objectProto.blur = object.blur;
    }
    if (object.spread != null) {
      objectProto.spread = object.spread;
    }
    if (object.diffusion != null) {
      objectProto.diffusion = object.diffusion;
    }
    return objectProto as ShadowProto;
  }

  static __unpackProto__(
    objectProto: ShadowProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Shadow {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    return new Shadow({
      type: Number(objectProto.type) as ShadowType,
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
      position: Number(objectProto.position) as ShadowPosition,
      offset:
        objectProto.offset != undefined
          ? _Axis2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
      spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
      diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ShadowProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Shadow {
    return Shadow.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Shadow {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ShadowProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SHADOW, Shadow);
/* ==== DESTACK_GENERATED_END:STRUCT:2100700 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2100700 ==== */
/**
 * A shadow style.
 */
export class ShadowStyle extends Style {
  static metatype: NodeType = NodeType.SHADOW_STYLE;

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
  get precededBy(): ShadowStyle | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as ShadowStyle | null;
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
   * ShadowStyle.type
   */
  /**
   * ShadowStyle.type
   */
  get type(): ShadowType {
    return this._type;
  }
  set type(value: ShadowType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: ShadowType;

  /**
   * ShadowStyle.color
   */
  /**
   * ShadowStyle.color
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
   * ShadowStyle.position
   */
  /**
   * ShadowStyle.position
   */
  get position(): ShadowPosition {
    return this._position;
  }
  set position(value: ShadowPosition) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: ShadowPosition;

  /**
   * ShadowStyle.offset
   */
  /**
   * ShadowStyle.offset
   */
  get offset(): Axis2 | null {
    return this._offset;
  }
  set offset(value: Axis2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["offset"];
    this._session.updateSetProperty(this, prop, value);
    this._offset = value;
  }
  _offset: Axis2 | null;

  /**
   * ShadowStyle.blur
   */
  /**
   * ShadowStyle.blur
   */
  get blur(): number | null {
    return this._blur;
  }
  set blur(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["blur"];
    this._session.updateSetProperty(this, prop, value);
    this._blur = value;
  }
  _blur: number | null;

  /**
   * ShadowStyle.spread
   */
  /**
   * ShadowStyle.spread
   */
  get spread(): number | null {
    return this._spread;
  }
  set spread(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["spread"];
    this._session.updateSetProperty(this, prop, value);
    this._spread = value;
  }
  _spread: number | null;

  /**
   * ShadowStyle.diffusion
   */
  /**
   * ShadowStyle.diffusion
   */
  get diffusion(): number | null {
    return this._diffusion;
  }
  set diffusion(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["diffusion"];
    this._session.updateSetProperty(this, prop, value);
    this._diffusion = value;
  }
  _diffusion: number | null;

  constructor(options: {
    id?: string;
    parent?: Scene | View | Theme | Palette | NodeReference | null;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    precededBy?: ShadowStyle | NodeReference | null;
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
    type?: ShadowType;
    color?: Color | null;
    position?: ShadowPosition;
    offset?: Axis2 | null;
    blur?: number | null;
    spread?: number | null;
    diffusion?: number | null;
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
        throw new Error(`ShadowStyle has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`ShadowStyle has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ShadowStyle.space is required`);
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
      throw new Error(`ShadowStyle.materialization is required`);
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
      throw new Error(`ShadowStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "ShadowStyle";
    }
    if (_name === null) {
      throw new Error(`ShadowStyle.name is required`);
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
      throw new Error(`ShadowStyle.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 10 /* ShadowType.BOX */;
    }
    if (_type === null) {
      throw new Error(`ShadowStyle.type is required`);
    }
    this._type = _type;
    let _color = options.color ?? null;
    this._color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = 1 /* ShadowPosition.OUTSIDE */;
    }
    if (_position === null) {
      throw new Error(`ShadowStyle.position is required`);
    }
    this._position = _position;
    let _offset = options.offset ?? null;
    this._offset = _offset;
    let _blur = options.blur ?? null;
    this._blur = _blur;
    let _spread = options.spread ?? null;
    this._spread = _spread;
    let _diffusion = options.diffusion ?? null;
    this._diffusion = _diffusion;

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
          `ShadowStyle.createdAt and ShadowStyle.updatedAt are required for existing Nodes`,
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
    if (
      (this._color == null) !== (other._color == null) ||
      (this._color != null && !this._color.equals(other._color))
    ) {
      return false;
    }
    if (!(this._position === other._position)) {
      return false;
    }
    if (
      (this._offset == null) !== (other._offset == null) ||
      (this._offset != null && !this._offset.equals(other._offset))
    ) {
      return false;
    }
    if (!(this._blur === other._blur)) {
      return false;
    }
    if (!(this._spread === other._spread)) {
      return false;
    }
    if (
      (this._diffusion == null) !== (other._diffusion == null) ||
      (this._diffusion != null &&
        !(
          this._diffusion === other._diffusion ||
          Math.abs(this._diffusion - other._diffusion) < 1e-10
        ))
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
    if (this._color != null) {
      h = (h * 31 + this._color.hash()) & 0xffffffff;
    }
    h = (h * 31 + this._position) & 0xffffffff;
    if (this._offset != null) {
      h = (h * 31 + this._offset.hash()) & 0xffffffff;
    }
    if (this._blur != null) {
      h = (h * 31 + hashInt(this._blur)) & 0xffffffff;
    }
    if (this._spread != null) {
      h = (h * 31 + hashInt(this._spread)) & 0xffffffff;
    }
    if (this._diffusion != null) {
      h = (h * 31 + hashFloat(this._diffusion)) & 0xffffffff;
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
      type: NodeType.SHADOW_STYLE,
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
    propertyReprs.push(`type=${ShadowType[this.type]}`);
    if (this.color != null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    propertyReprs.push(`position=${ShadowPosition[this.position]}`);
    if (this.offset != null) {
      propertyReprs.push(`offset=${this.offset.repr()}`);
    }
    if (this.blur != null) {
      propertyReprs.push(`blur=${this.blur}`);
    }
    if (this.spread != null) {
      propertyReprs.push(`spread=${this.spread}`);
    }
    if (this.diffusion != null) {
      propertyReprs.push(`diffusion=${this.diffusion}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<ShadowStyle "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return ShadowStyle.__packValue__(this);
  }

  static __packValue__(object: ShadowStyle): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2100700;
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
    if (object._color != null) {
      objectValue["200"] = object._color.toValue();
    }
    objectValue["201"] = object._position;
    if (object._offset != null) {
      objectValue["202"] = object._offset.toValue();
    }
    if (object._blur != null) {
      objectValue["203"] = object._blur;
    }
    if (object._spread != null) {
      objectValue["204"] = object._spread;
    }
    if (object._diffusion != null) {
      objectValue["205"] = object._diffusion;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ShadowStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const colorValue = objectValue["200"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const offsetValue = objectValue["202"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const blurValue = objectValue["203"];
    const unpackedBlur = blurValue != undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["204"];
    const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["205"];
    const unpackedDiffusion = diffusionValue != undefined ? diffusionValue : null;
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
    return new ShadowStyle({
      type: Number(objectValue["100"]),
      color: unpackedColor,
      position: Number(objectValue["201"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
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
  ): ShadowStyle {
    return ShadowStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ShadowStyleProto {
    return ShadowStyle.__packProto__(this);
  }

  static __packProto__(object: ShadowStyle): ShadowStyleProto {
    const objectProto: Partial<ShadowStyleProto> = { metatype: 2100700 };
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
    objectProto.type = Number(object._type) as ShadowTypeProto;
    if (object._color != null) {
      objectProto.color = object._color.toProto();
    }
    objectProto.position = Number(object._position) as ShadowPositionProto;
    if (object._offset != null) {
      objectProto.offset = object._offset.toProto();
    }
    if (object._blur != null) {
      objectProto.blur = object._blur;
    }
    if (object._spread != null) {
      objectProto.spread = object._spread;
    }
    if (object._diffusion != null) {
      objectProto.diffusion = object._diffusion;
    }
    return objectProto as ShadowStyleProto;
  }

  static __unpackProto__(
    objectProto: ShadowStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ShadowStyle {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
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
    return new ShadowStyle({
      type: Number(objectProto.type) as ShadowType,
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      position: Number(objectProto.position) as ShadowPosition,
      offset:
        objectProto.offset != undefined
          ? _Axis2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
      spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
      diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
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
    objectProto: ShadowStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ShadowStyle {
    return ShadowStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ShadowStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ShadowStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SHADOW_STYLE, ShadowStyle);
/* ==== DESTACK_GENERATED_END:NODE:2100700 ==== */
