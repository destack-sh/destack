import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  IsSubject,
  Node,
  NodeType,
  Struct,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Axis2 } from "@destack/language/core/common";
import {
  registerEnumClass,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Color, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import {
  ShadowPositionProto,
  ShadowProto,
  ShadowStyleProto,
  ShadowTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:12012 ==== */
/**
 * A shadow value.
 */
export class Shadow extends Struct {
  static metatype: StructType = StructType.SHADOW;
  static __isFrozen__: boolean = false;

  /**
   * Shadow.type
   */
  type: ShadowType;

  /**
   * style
   */
  get style(): ShadowStyle | null {
    const nodePtr: NodeReference | null = this.stylePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as ShadowStyle | null;
    }
    return null;
  }
  set style(value: ShadowStyle | null) {
    if (value == null) {
      this.stylePtr = null;
    } else {
      this.stylePtr = value.toRef();
    }
  }
  stylePtr: NodeReference | null;

  /**
   * Shadow.color
   */
  color: Color | null;

  /**
   * Shadow.position
   */
  position: ShadowPosition;

  /**
   * Shadow.offset
   */
  offset: Axis2 | null;

  /**
   * Shadow.blur
   */
  blur: number | null;

  /**
   * Shadow.spread
   */
  spread: number | null;

  /**
   * Shadow.diffusion
   */
  diffusion: number | null;

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
      _type = ShadowType.BOX;
    }
    if (_type === null) {
      throw new Error(`Shadow.type is required`);
    }
    this.type = _type;
    let _style = options.style ?? null;
    if (_style != null && _style instanceof Node) {
      _style = _style.toRef();
    }
    this.stylePtr = _style;
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = ShadowPosition.OUTSIDE;
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
    // ...
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
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${ShadowType[this.type]}`);
    if (this.style !== null) {
      propertyReprs.push(`style=${this.style.repr()}`);
    }
    if (this.color !== null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    propertyReprs.push(`position=${ShadowPosition[this.position]}`);
    if (this.offset !== null) {
      propertyReprs.push(`offset=${this.offset.repr()}`);
    }
    if (this.blur !== null) {
      propertyReprs.push(`blur=${this.blur}`);
    }
    if (this.spread !== null) {
      propertyReprs.push(`spread=${this.spread}`);
    }
    if (this.diffusion !== null) {
      propertyReprs.push(`diffusion=${this.diffusion}`);
    }
    return `<Shadow ${propertyReprs.join(" ")}>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.stylePtr !== null) {
      h = (h * 31 + hashString(this.stylePtr.id)) & 0xffffffff;
    }
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    h = (h * 31 + this.position) & 0xffffffff;
    if (this.offset !== null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.blur !== null) {
      h = (h * 31 + hashInt(this.blur)) & 0xffffffff;
    }
    if (this.spread !== null) {
      h = (h * 31 + hashInt(this.spread)) & 0xffffffff;
    }
    if (this.diffusion !== null) {
      h = (h * 31 + hashFloat(this.diffusion)) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return Shadow.__packValue__(this);
  }

  static __packValue__(object: Shadow): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12012;
    objectValue["30"] = object.type;
    if (object.stylePtr != null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    objectValue["51"] = object.position;
    if (object.offset != null) {
      objectValue["52"] = object.offset.toValue();
    }
    if (object.blur != null) {
      objectValue["53"] = object.blur;
    }
    if (object.spread != null) {
      objectValue["54"] = object.spread;
    }
    if (object.diffusion != null) {
      objectValue["55"] = object.diffusion;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Shadow {
    const stylePtrValue = objectValue["41"];
    const unpackedStylePtr =
      stylePtrValue != undefined
        ? NodeReference.fromValue(stylePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const offsetValue = objectValue["52"];
    const unpackedOffset =
      offsetValue != undefined
        ? Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const blurValue = objectValue["53"];
    const unpackedBlur = blurValue != undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["54"];
    const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["55"];
    const unpackedDiffusion = diffusionValue != undefined ? diffusionValue : null;
    return new Shadow({
      type: Number(objectValue["30"]),
      style: unpackedStylePtr,
      color: unpackedColor,
      position: Number(objectValue["51"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Shadow {
    return Shadow.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ShadowProto {
    return Shadow.__packProto__(this);
  }

  static __packProto__(object: Shadow): ShadowProto {
    const objectProto: Partial<ShadowProto> = { metatype: 12012 };
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
    return new Shadow({
      type: Number(objectProto.type) as ShadowType,
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
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      position: Number(objectProto.position) as ShadowPosition,
      offset:
        objectProto.offset != undefined
          ? Axis2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
      spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
      diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:12012 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12060 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12060 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12061 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:12061 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12060 ==== */
/**
 * A shadow style.
 */
export class ShadowStyle extends Node implements Style {
  static metatype: NodeType = NodeType.SHADOW_STYLE;
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
    NodeType.POLYGON_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.THEME,
    NodeType.THREAD_VIEW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.CANVAS,
    NodeType.LAYER,
    NodeType.TEXT_VIEW,
    NodeType.SPLIT_VIEW,
    NodeType.WIZARD_VIEW,
    NodeType.SCENE,
  ];
  static __childTypes__: NodeType[] = [NodeType.TAGGING];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.LINE_SHAPE,
    NodeType.POLYGON_SHAPE,
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
    NodeType.SPLIT_VIEW,
    NodeType.SCENE,
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
   * ShadowStyle.type
   */
  type: ShadowType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * ShadowStyle.color
   */
  color: Color | null;

  /**
   * ShadowStyle.position
   */
  position: ShadowPosition;

  /**
   * ShadowStyle.offset
   */
  offset: Axis2 | null;

  /**
   * ShadowStyle.blur
   */
  blur: number | null;

  /**
   * ShadowStyle.spread
   */
  spread: number | null;

  /**
   * ShadowStyle.diffusion
   */
  diffusion: number | null;

  constructor(options: {
    id?: string;
    parent?: Scene | (Node & View) | Theme | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    type?: ShadowType;
    name: string;
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
      throw new Error(`ShadowStyle.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = ShadowType.BOX;
    }
    if (_type === null) {
      throw new Error(`ShadowStyle.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ShadowStyle.name is required`);
    }
    this.name = _name;
    let _color = options.color ?? null;
    this.color = _color;
    let _position = options.position ?? null;
    if (_position === null) {
      _position = ShadowPosition.OUTSIDE;
    }
    if (_position === null) {
      throw new Error(`ShadowStyle.position is required`);
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
    h = (h * 31 + this.position) & 0xffffffff;
    if (this.offset !== null) {
      h = (h * 31 + this.offset.hash()) & 0xffffffff;
    }
    if (this.blur !== null) {
      h = (h * 31 + hashInt(this.blur)) & 0xffffffff;
    }
    if (this.spread !== null) {
      h = (h * 31 + hashInt(this.spread)) & 0xffffffff;
    }
    if (this.diffusion !== null) {
      h = (h * 31 + hashFloat(this.diffusion)) & 0xffffffff;
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
      nodeType: NodeType.SHADOW_STYLE,
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
    propertyReprs.push(`type=${ShadowType[this.type]}`);
    if (this.color !== null) {
      propertyReprs.push(`color=${this.color.repr()}`);
    }
    propertyReprs.push(`position=${ShadowPosition[this.position]}`);
    if (this.offset !== null) {
      propertyReprs.push(`offset=${this.offset.repr()}`);
    }
    if (this.blur !== null) {
      propertyReprs.push(`blur=${this.blur}`);
    }
    if (this.spread !== null) {
      propertyReprs.push(`spread=${this.spread}`);
    }
    if (this.diffusion !== null) {
      propertyReprs.push(`diffusion=${this.diffusion}`);
    }
    propertyReprs.push(`name=${this.name}`);
    return `<ShadowStyle '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return ShadowStyle.__packValue__(this);
  }

  static __packValue__(object: ShadowStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12060;
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
    if (object.color != null) {
      objectValue["50"] = object.color.toValue();
    }
    objectValue["51"] = object.position;
    if (object.offset != null) {
      objectValue["52"] = object.offset.toValue();
    }
    if (object.blur != null) {
      objectValue["53"] = object.blur;
    }
    if (object.spread != null) {
      objectValue["54"] = object.spread;
    }
    if (object.diffusion != null) {
      objectValue["55"] = object.diffusion;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ShadowStyle {
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue != undefined
        ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    const offsetValue = objectValue["52"];
    const unpackedOffset =
      offsetValue != undefined
        ? Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const blurValue = objectValue["53"];
    const unpackedBlur = blurValue != undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["54"];
    const unpackedSpread = spreadValue != undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["55"];
    const unpackedDiffusion = diffusionValue != undefined ? diffusionValue : null;
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
    return new ShadowStyle({
      type: Number(objectValue["30"]),
      color: unpackedColor,
      position: Number(objectValue["51"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
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
  ): ShadowStyle {
    return ShadowStyle.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ShadowStyleProto {
    return ShadowStyle.__packProto__(this);
  }

  static __packProto__(object: ShadowStyle): ShadowStyleProto {
    const objectProto: Partial<ShadowStyleProto> = { metatype: 12060 };
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
    objectProto.type = Number(object.type) as ShadowTypeProto;
    objectProto.name = object.name;
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
    return objectProto as ShadowStyleProto;
  }

  static __unpackProto__(
    objectProto: ShadowStyleProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ShadowStyle {
    return new ShadowStyle({
      type: Number(objectProto.type) as ShadowType,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      position: Number(objectProto.position) as ShadowPosition,
      offset:
        objectProto.offset != undefined
          ? Axis2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      blur: objectProto.blur != undefined ? Number(objectProto.blur) : null,
      spread: objectProto.spread != undefined ? Number(objectProto.spread) : null,
      diffusion: objectProto.diffusion != undefined ? objectProto.diffusion : null,
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
/* ==== DESTACK_GENERATED_END:NODE:12060 ==== */
