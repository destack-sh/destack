import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  Node,
  NodeType,
  StructFrozen,
  StructType,
} from "@destack/language/core/builtin";
import { Insets } from "@destack/language/core/common";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Color, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import { BorderProto, BorderStyleProto, BorderTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:12013 ==== */
/**
 * A border value.
 */
export class Border extends StructFrozen {
  static metatype: StructType = StructType.BORDER;
  static __isFrozen__: boolean = true;

  /**
   * Border.type
   */
  readonly type: BorderType;

  /**
   * style
   */
  get style(): BorderStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as BorderStyle | null;
    }
    return null;
  }
  readonly stylePtr: NodeReference | null;

  /**
   * Border.color
   */
  readonly color: Color | null;

  /**
   * Border.width
   */
  readonly width: Insets | null;

  constructor(options: {
    type?: BorderType;
    style?: BorderStyle | NodeReference | null;
    color?: Color | null;
    width?: Insets | null;
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
      _type = BorderType.SOLID;
    }
    if (_type === null) {
      throw new Error(`Border.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _width = options.width ?? null;
    this.width = _width;

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
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (
      (this.width == null) !== (other.width == null) ||
      (this.width != null && !this.width.equals(other.width))
    ) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${BorderType[this.type]}`);
      if (this.color !== null) {
        propertyReprs.push(`color=${this.color.repr()}`);
      }
      if (this.width !== null) {
        propertyReprs.push(`width=${this.width.repr()}`);
      }
      if (this.style !== null) {
        propertyReprs.push(`style=${this.style?.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<Border ${propertyReprs.join(" ")}>`;
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
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.width !== null) {
      h = (h * 31 + this.width.hash()) & 0xffffffff;
    }
    if (this.stylePtr !== null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
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
      this._value = Border.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Border): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12013;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    if (object.width != null) {
      objectValue["51"] = object.width.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Border {
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["51"];
    const unpackedWidth =
      widthValue != undefined
        ? Insets.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const stylePtrValue = objectValue["41"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Border({
      type: Number(objectValue["30"]),
      color: unpackedColor,
      width: unpackedWidth,
      style: unpackedStylePtr,
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
  ): Border {
    return Border.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): BorderProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Border.__packProto__(this);
    }
    return this._proto as BorderProto;
  }

  static __packProto__(object: Border): BorderProto {
    const objectProto: Partial<BorderProto> = { metatype: 12013 };
    objectProto.type = Number(object.type) as BorderTypeProto;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.width != null) {
      objectProto.width = object.width.toProto();
    }
    return objectProto as BorderProto;
  }

  static __unpackProto__(
    objectProto: BorderProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Border {
    return new Border({
      type: Number(objectProto.type) as BorderType,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Insets.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
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
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: BorderProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Border {
    return Border.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Border {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = BorderProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.BORDER, Border);
/* ==== DESTACK_GENERATED_END:STRUCT:12013 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12050 ==== */
/**
 * BorderType
 */
export enum BorderType {
  STYLE = 2,
  SOLID = 10,
  DASHED = 11,
  DOTTED = 12,
  DOUBLE = 13,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.BORDER_TYPE, BorderType);
/* ==== DESTACK_GENERATED_END:ENUM:12050 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12063 ==== */
/**
 * A border style.
 */
export class BorderStyle extends Style {
  static metatype: NodeType = NodeType.BORDER_STYLE;

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
   * BorderStyle.type
   */
  type: BorderType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * BorderStyle.style
   */
  get style(): BorderStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as BorderStyle | null;
    }
    return null;
  }
  set style(node: BorderStyle | null) {
    if (node === null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = node.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * BorderStyle.color
   */
  color: Color | null;

  /**
   * BorderStyle.width
   */
  width: Insets | null;

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
    type?: BorderType;
    name: string;
    style?: BorderStyle | NodeReference | null;
    color?: Color | null;
    width?: Insets | null;
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
      throw new Error(`BorderStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = BorderType.SOLID;
    }
    if (_type === null) {
      throw new Error(`BorderStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`BorderStyle.name is required`);
    }
    this.name = _name;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _width = options.width ?? null;
    this.width = _width;

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
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    if (
      (this.width == null) !== (other.width == null) ||
      (this.width != null && !this.width.equals(other.width))
    ) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
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
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    if (this.width !== null) {
      h = (h * 31 + this.width.hash()) & 0xffffffff;
    }
    if (this.stylePtr !== null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
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

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.BORDER_STYLE,
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
    propertyReprs.push(`type=${BorderType[this.type]}`);
    if (this.color !== null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    if (this.width !== null) {
      propertyReprs.push(`width=${this.width.repr()}`);
    }
    if (this.style !== null) {
      propertyReprs.push(`style=${this.style?.repr()}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<BorderStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return BorderStyle.__packValue__(this);
  }

  static __packValue__(object: BorderStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12063;
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
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    if (object.width != null) {
      objectValue["51"] = object.width.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): BorderStyle {
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["51"];
    const unpackedWidth =
      widthValue != undefined
        ? Insets.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const stylePtrValue = objectValue["41"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
    return new BorderStyle({
      type: Number(objectValue["30"]),
      color: unpackedColor,
      width: unpackedWidth,
      style: unpackedStylePtr,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
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
  ): BorderStyle {
    return BorderStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): BorderStyleProto {
    return BorderStyle.__packProto__(this);
  }

  static __packProto__(object: BorderStyle): BorderStyleProto {
    const objectProto: Partial<BorderStyleProto> = { metatype: 12063 };
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
    objectProto.type = Number(object.type) as BorderTypeProto;
    objectProto.name = object.name;
    if (object.stylePtr != null) {
      objectProto.stylePtr = object.stylePtr.toProto();
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    if (object.width != null) {
      objectProto.width = object.width.toProto();
    }
    return objectProto as BorderStyleProto;
  }

  static __unpackProto__(
    objectProto: BorderStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): BorderStyle {
    return new BorderStyle({
      type: Number(objectProto.type) as BorderType,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Insets.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
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
      id: String(objectProto.id),
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
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: BorderStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): BorderStyle {
    return BorderStyle.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): BorderStyle {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = BorderStyleProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.BORDER_STYLE, BorderStyle);
/* ==== DESTACK_GENERATED_END:NODE:12063 ==== */
