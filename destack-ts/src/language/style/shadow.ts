import {
  Axis2,
  Graph,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Struct,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { Color, Style, Theme } from "@destack/language/style";
import { View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:12030 ==== */
/**
 * ShadowType
 */
export enum ShadowType {
  STYLE = 2,
  BOX = 10,
  REALISTIC = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:12030 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12031 ==== */
/**
 * ShadowPosition
 */
export enum ShadowPosition {
  OUTSIDE = 1,
  INSIDE = 2,
}
/* ==== DESTACK_GENERATED_END:ENUM:12031 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12012 ==== */
/**
 * A shadow value.
 */
export class Shadow extends Struct {
  static metatype: StructType = StructType.SHADOW;
  static __isFrozen__: boolean = false;

  /**
   * ShadowBase.type
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
   * ShadowBase.color
   */
  color: Color | null;

  /**
   * ShadowBase.position
   */
  position: ShadowPosition;

  /**
   * ShadowBase.offset
   */
  offset: Axis2 | null;

  /**
   * ShadowBase.blur
   */
  blur: number | null;

  /**
   * ShadowBase.spread
   */
  spread: number | null;

  /**
   * ShadowBase.diffusion
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
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
    if (object.stylePtr !== null) {
      objectValue["41"] = object.stylePtr.toValue();
    }
    if (object.color !== null) {
      objectValue["50"] = object.color.toValue();
    }
    objectValue["51"] = object.position;
    if (object.offset !== null) {
      objectValue["52"] = object.offset.toValue();
    }
    if (object.blur !== null) {
      objectValue["53"] = object.blur;
    }
    if (object.spread !== null) {
      objectValue["54"] = object.spread;
    }
    if (object.diffusion !== null) {
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
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue !== undefined ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
    const offsetValue = objectValue["52"];
    const unpackedOffset =
      offsetValue !== undefined ? Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection) : null;
    const blurValue = objectValue["53"];
    const unpackedBlur = blurValue !== undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["54"];
    const unpackedSpread = spreadValue !== undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["55"];
    const unpackedDiffusion = diffusionValue !== undefined ? diffusionValue : null;
    const styleValue = objectValue["41"];
    const unpackedStyle =
      styleValue !== undefined ? NodeReference.fromValue(styleValue, _session, _supergraph, _graph, _connection) : null;
    return new Shadow({
      type: Number(objectValue["30"]),
      color: unpackedColor,
      position: Number(objectValue["51"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
      style: unpackedStyle,
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:12012 ==== */

/* ==== DESTACK_GENERATED_START:NODE:12024 ==== */
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
   * ShadowBase.type
   */
  type: ShadowType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * ShadowBase.color
   */
  color: Color | null;

  /**
   * ShadowBase.position
   */
  position: ShadowPosition;

  /**
   * ShadowBase.offset
   */
  offset: Axis2 | null;

  /**
   * ShadowBase.blur
   */
  blur: number | null;

  /**
   * ShadowBase.spread
   */
  spread: number | null;

  /**
   * ShadowBase.diffusion
   */
  diffusion: number | null;

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
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`ShadowStyle.materialization is required`);
    }
    this.materialization = _materialization;
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
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
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

  toValue(): { [key: string]: any } {
    return ShadowStyle.__packValue__(this);
  }

  static __packValue__(object: ShadowStyle): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 12024;
    objectValue["2"] = String(object.id);
    if (object.parentPtr !== null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr !== null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr !== null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr !== null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt !== null) {
      objectValue["20"] = object.deletedAt.toString();
    }
    objectValue["22"] = object.orderKey;
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.color !== null) {
      objectValue["50"] = object.color.toValue();
    }
    objectValue["51"] = object.position;
    if (object.offset !== null) {
      objectValue["52"] = object.offset.toValue();
    }
    if (object.blur !== null) {
      objectValue["53"] = object.blur;
    }
    if (object.spread !== null) {
      objectValue["54"] = object.spread;
    }
    if (object.diffusion !== null) {
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
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue !== undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const colorValue = objectValue["50"];
    const unpackedColor =
      colorValue !== undefined ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
    const offsetValue = objectValue["52"];
    const unpackedOffset =
      offsetValue !== undefined ? Axis2.fromValue(offsetValue, _session, _supergraph, _graph, _connection) : null;
    const blurValue = objectValue["53"];
    const unpackedBlur = blurValue !== undefined ? Number(blurValue) : null;
    const spreadValue = objectValue["54"];
    const unpackedSpread = spreadValue !== undefined ? Number(spreadValue) : null;
    const diffusionValue = objectValue["55"];
    const unpackedDiffusion = diffusionValue !== undefined ? diffusionValue : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue !== undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue !== undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue !== undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue !== undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ShadowStyle({
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      type: Number(objectValue["30"]),
      color: unpackedColor,
      position: Number(objectValue["51"]),
      offset: unpackedOffset,
      blur: unpackedBlur,
      spread: unpackedSpread,
      diffusion: unpackedDiffusion,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
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
}
/* ==== DESTACK_GENERATED_END:NODE:12024 ==== */
