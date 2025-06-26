import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  MaterializationType,
  Node,
  NodeType,
  Struct,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { File } from "@destack/language/data";
import { registerEnumClass, registerNodeClass, registerStructClass } from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Color, Gradient, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  FillPositionProto,
  FillProto,
  FillSizeProto,
  FillStyleProto,
  FillTypeProto,
  MaterializationTypeProto,
} from "@destack/proto";
import type { IMessageType } from "@protobuf-ts/runtime";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12034 ==== */
/**
 * FillType
 */
export enum FillType {
  STYLE = 2,
  SOLID = 10,
  GRADIENT = 11,
  IMAGE = 12,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILL_TYPE, FillType);
/* ==== DESTACK_GENERATED_END:ENUM:12034 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12035 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12035 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12036 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12036 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12017 ==== */
/**
 * A fill value.
 */
export class Fill extends Struct {
  static metatype: StructType = StructType.FILL;
  static __protoClass__ = FillProto as IMessageType<any>;
  static __isFrozen__: boolean = false;

  /**
   * FillBase.type
   */
  type: FillType;

  /**
   * style
   */
  get style(): FillStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as FillStyle | null;
    }
    return null;
  }
  set style(value: FillStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * FillBase.color
   */
  color: Color | null;

  /**
   * FillBase.gradient
   */
  gradient: Gradient | null;

  /**
   * image
   */
  get image(): File | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as File | null;
    }
    return null;
  }
  set image(value: File | null) {
    if (value == null) {
      this.imagePtr = null;
    } else {
      this.imagePtr = value.toRef();
    }
  }
  imagePtr: NodeReference | null;

  /**
   * FillBase.position
   */
  position: FillPosition | null;

  /**
   * FillBase.size
   */
  size: FillSize | null;

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
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _gradient = options.gradient ?? null;
    this.gradient = _gradient;
    let _image = options.image ?? null;
    if (_image != null && _image instanceof Node) {
      _image = _image.toRef();
    }
    this.imagePtr = _image;
    let _position = options.position ?? null;
    this.position = _position;
    let _size = options.size ?? null;
    this.size = _size;

    // identity
    // ...
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.stylePtr?.id === other.stylePtr?.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if ((this.color == null) !== (other.color == null) || (this.color != null && !this.color.equals(other.color))) {
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

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return Fill.__packValue__(this);
  }

  static __packValue__(object: Fill): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12017;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["42"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    if (object.gradient != null) {
      objectValue["51"] = object.gradient.toValue();
    }
    if (object.imagePtr != null) {
      objectValue["52"] = object.imagePtr.toValue();
    }
    if (object.position != null) {
      objectValue["53"] = object.position;
    }
    if (object.size != null) {
      objectValue["54"] = object.size;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Fill {
    const stylePtrValue = objectValue["42"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
    const gradientValue = objectValue["51"];
    const unpackedGradient =
      gradientValue != undefined ? Gradient.fromValue(gradientValue, _session, _supergraph, _graph, _connection) : null;
    const imagePtrValue = objectValue["52"];
    const unpackedImagePtr =
      imagePtrValue != undefined
        ? NodeReference.fromValue(imagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["53"];
    const unpackedPosition = positionValue != undefined ? Number(positionValue) : null;
    const sizeValue = objectValue["54"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    return new Fill({
      style: unpackedStylePtr,
      type: Number(objectValue["30"]),
      color: unpackedColor,
      gradient: unpackedGradient,
      image: unpackedImagePtr,
      position: unpackedPosition,
      size: unpackedSize,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Fill {
    return Fill.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FillProto {
    return Fill.__packProto__(this);
  }

  static __packProto__(object: Fill): FillProto {
    const objectProto: Partial<FillProto> = { metatype: 12017 };
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
    return new Fill({
      style:
        objectProto.stylePtr != undefined
          ? NodeReference.fromProto(objectProto.stylePtr!, _session, _supergraph, _graph, _connection)
          : null,
      type: Number(objectProto.type) as FillType,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      gradient:
        objectProto.gradient != undefined
          ? Gradient.fromProto(objectProto.gradient!, _session, _supergraph, _graph, _connection)
          : null,
      image:
        objectProto.imagePtr != undefined
          ? NodeReference.fromProto(objectProto.imagePtr!, _session, _supergraph, _graph, _connection)
          : null,
      position: objectProto.position != undefined ? (Number(objectProto.position) as FillPosition) : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FillSize) : null,
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.FILL, Fill);
/* ==== DESTACK_GENERATED_END:STRUCT:12017 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12021 ==== */
/**
 * A fill style.
 */
export class FillStyle extends Node implements Style {
  static metatype: NodeType = NodeType.FILL_STYLE;
  static __protoClass__ = FillStyleProto as IMessageType<any>;
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
   * FillBase.type
   */
  type: FillType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * FillBase.color
   */
  color: Color | null;

  /**
   * FillBase.gradient
   */
  gradient: Gradient | null;

  /**
   * FillBase.image
   */
  get image(): File | null {
    const nodePtr: NodeReference | null = this.imagePtr;
    if (nodePtr !== null) {
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
  imagePtr: NodeReference | null;

  /**
   * FillBase.position
   */
  position: FillPosition | null;

  /**
   * FillBase.size
   */
  size: FillSize | null;

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
    type: FillType;
    name: string;
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
      throw new Error(`FillStyle.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`FillStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`FillStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`FillStyle.name is required`);
    }
    this.name = _name;
    let _color = options.color ?? null;
    this.color = _color;
    let _gradient = options.gradient ?? null;
    this.gradient = _gradient;
    let _image = options.image ?? null;
    if (_image != null && _image instanceof Node) {
      _image = _image.toRef();
    }
    this.imagePtr = _image;
    let _position = options.position ?? null;
    this.position = _position;
    let _size = options.size ?? null;
    this.size = _size;

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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if ((this.color == null) !== (other.color == null) || (this.color != null && !this.color.equals(other.color))) {
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

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FILL_STYLE,
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
    return FillStyle.__packValue__(this);
  }

  static __packValue__(object: FillStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12021;
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
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    if (object.gradient != null) {
      objectValue["51"] = object.gradient.toValue();
    }
    if (object.imagePtr != null) {
      objectValue["52"] = object.imagePtr.toValue();
    }
    if (object.position != null) {
      objectValue["53"] = object.position;
    }
    if (object.size != null) {
      objectValue["54"] = object.size;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FillStyle {
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
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
    const gradientValue = objectValue["51"];
    const unpackedGradient =
      gradientValue != undefined ? Gradient.fromValue(gradientValue, _session, _supergraph, _graph, _connection) : null;
    const imagePtrValue = objectValue["52"];
    const unpackedImagePtr =
      imagePtrValue != undefined
        ? NodeReference.fromValue(imagePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectValue["53"];
    const unpackedPosition = positionValue != undefined ? Number(positionValue) : null;
    const sizeValue = objectValue["54"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    return new FillStyle({
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      color: unpackedColor,
      gradient: unpackedGradient,
      image: unpackedImagePtr,
      position: unpackedPosition,
      size: unpackedSize,
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
  ): FillStyle {
    return FillStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FillStyleProto {
    return FillStyle.__packProto__(this);
  }

  static __packProto__(object: FillStyle): FillStyleProto {
    const objectProto: Partial<FillStyleProto> = { metatype: 12021 };
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
    objectProto.type = Number(object.type) as FillTypeProto;
    objectProto.name = object.name;
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
    return objectProto as FillStyleProto;
  }

  static __unpackProto__(
    objectProto: FillStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FillStyle {
    return new FillStyle({
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      type: Number(objectProto.type) as FillType,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      gradient:
        objectProto.gradient != undefined
          ? Gradient.fromProto(objectProto.gradient!, _session, _supergraph, _graph, _connection)
          : null,
      image:
        objectProto.imagePtr != undefined
          ? NodeReference.fromProto(objectProto.imagePtr!, _session, _supergraph, _graph, _connection)
          : null,
      position: objectProto.position != undefined ? (Number(objectProto.position) as FillPosition) : null,
      size: objectProto.size != undefined ? (Number(objectProto.size) as FillSize) : null,
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILL_STYLE, FillStyle);
/* ==== DESTACK_GENERATED_END:NODE:12021 ==== */
