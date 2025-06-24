import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  Dimension,
  Graph,
  IsSubject,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Position,
  QueryConnection,
  Session,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { ContainerView, InputView } from "@destack/language/view";
import { MaterializationTypeProto, NumberInputViewProto } from "@destack/proto";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:10400 ==== */
/**
 * A general number input View.
 */
export class NumberInputView extends Node implements InputView {
  static metatype: NodeType = NodeType.NUMBER_INPUT_VIEW;
  static __traits__: TraitType[] = [
    TraitType.INPUT_VIEW,
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.VIEW,
    TraitType.ENTITY,
    TraitType.TAGGABLE,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.ORDERED,
    TraitType.SCRIPTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.CANVAS,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
  ];
  static __childTypes__: NodeType[] = [
    NodeType.TAGGING,
    NodeType.SCRIPT,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.SHADOW_STYLE,
  ];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.FOLDER,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.CANVAS,
    NodeType.LAYER,
  ];
  static __descendantTypes__: NodeType[] = [
    NodeType.OPTION,
    NodeType.FIELD,
    NodeType.TAGGING,
    NodeType.COLOR_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.SCRIPT,
  ];

  /**
   * View.parent
   */
  get parent(): Window | Scene | Layer | (Node & ContainerView) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Window | Scene | Layer | (Node & ContainerView) | null;
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
   * HasName.name
   */
  name: string;

  /**
   * View.position
   */
  position: Position | null;

  /**
   * View.width
   */
  width: Dimension | null;

  /**
   * View.height
   */
  height: Dimension | null;

  /**
   * View.minWidth
   */
  minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  maxHeight: Dimension | null;

  /**
   * InputView.isVisible
   */
  isVisible: boolean | null;

  /**
   * InputView.opacity
   */
  opacity: number | null;

  /**
   * NumberInputView.value
   */
  value: string | null;

  /**
   * NumberInputView.placeholder
   */
  placeholder: string | null;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
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
  scriptPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Window | Scene | Layer | (Node & ContainerView) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    name: string;
    position?: Position | null;
    width?: Dimension | null;
    height?: Dimension | null;
    minWidth?: Dimension | null;
    minHeight?: Dimension | null;
    maxWidth?: Dimension | null;
    maxHeight?: Dimension | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    value?: string | null;
    placeholder?: string | null;
    script?: Script | NodeReference | null;
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
      throw new Error(`NumberInputView.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`NumberInputView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`NumberInputView.name is required`);
    }
    this.name = _name;
    let _position = options.position ?? null;
    this.position = _position;
    let _width = options.width ?? null;
    this.width = _width;
    let _height = options.height ?? null;
    this.height = _height;
    let _minWidth = options.minWidth ?? null;
    this.minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this.minHeight = _minHeight;
    let _maxWidth = options.maxWidth ?? null;
    this.maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this.maxHeight = _maxHeight;
    let _isVisible = options.isVisible ?? null;
    this.isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _value = options.value ?? null;
    this.value = _value;
    let _placeholder = options.placeholder ?? null;
    this.placeholder = _placeholder;
    let _script = options.script ?? null;
    if (_script != null && _script instanceof Node) {
      _script = _script.toRef();
    }
    this.scriptPtr = _script;

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
    if ((this.value == null) !== (other.value == null) || (this.value != null && !(this.value === other.value))) {
      return false;
    }
    if (
      (this.placeholder == null) !== (other.placeholder == null) ||
      (this.placeholder != null && !(this.placeholder === other.placeholder))
    ) {
      return false;
    }
    if (
      (this.isVisible == null) !== (other.isVisible == null) ||
      (this.isVisible != null && !(this.isVisible === other.isVisible))
    ) {
      return false;
    }
    if (
      (this.opacity == null) !== (other.opacity == null) ||
      (this.opacity != null && !(this.opacity === other.opacity || Math.abs(this.opacity - other.opacity) < 1e-10))
    ) {
      return false;
    }
    if (
      (this.position == null) !== (other.position == null) ||
      (this.position != null && !this.position.equals(other.position))
    ) {
      return false;
    }
    if ((this.width == null) !== (other.width == null) || (this.width != null && !this.width.equals(other.width))) {
      return false;
    }
    if (
      (this.height == null) !== (other.height == null) ||
      (this.height != null && !this.height.equals(other.height))
    ) {
      return false;
    }
    if (
      (this.minWidth == null) !== (other.minWidth == null) ||
      (this.minWidth != null && !this.minWidth.equals(other.minWidth))
    ) {
      return false;
    }
    if (
      (this.minHeight == null) !== (other.minHeight == null) ||
      (this.minHeight != null && !this.minHeight.equals(other.minHeight))
    ) {
      return false;
    }
    if (
      (this.maxWidth == null) !== (other.maxWidth == null) ||
      (this.maxWidth != null && !this.maxWidth.equals(other.maxWidth))
    ) {
      return false;
    }
    if (
      (this.maxHeight == null) !== (other.maxHeight == null) ||
      (this.maxHeight != null && !this.maxHeight.equals(other.maxHeight))
    ) {
      return false;
    }
    if (!(this.materialization === other.materialization)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.spacePtr == null) !== (other.spacePtr == null) ||
      (this.spacePtr != null && !(this.spacePtr.id === other.spacePtr.id))
    ) {
      return false;
    }
    if (
      (this.scriptPtr == null) !== (other.scriptPtr == null) ||
      (this.scriptPtr != null && !(this.scriptPtr.id === other.scriptPtr.id))
    ) {
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
      nodeType: NodeType.NUMBER_INPUT_VIEW,
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
    return NumberInputView.__packValue__(this);
  }

  static __packValue__(object: NumberInputView): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 10400;
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
    objectValue["31"] = object.name;
    if (object.position != null) {
      objectValue["40"] = object.position.toValue();
    }
    if (object.width != null) {
      objectValue["41"] = object.width.toValue();
    }
    if (object.height != null) {
      objectValue["42"] = object.height.toValue();
    }
    if (object.minWidth != null) {
      objectValue["43"] = object.minWidth.toValue();
    }
    if (object.minHeight != null) {
      objectValue["44"] = object.minHeight.toValue();
    }
    if (object.maxWidth != null) {
      objectValue["45"] = object.maxWidth.toValue();
    }
    if (object.maxHeight != null) {
      objectValue["46"] = object.maxHeight.toValue();
    }
    if (object.isVisible != null) {
      objectValue["60"] = object.isVisible;
    }
    if (object.opacity != null) {
      objectValue["61"] = object.opacity;
    }
    if (object.value != null) {
      objectValue["100"] = object.value;
    }
    if (object.placeholder != null) {
      objectValue["101"] = object.placeholder;
    }
    if (object.scriptPtr != null) {
      objectValue["200"] = object.scriptPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberInputView {
    const valueValue = objectValue["100"];
    const unpackedValue = valueValue != undefined ? valueValue : null;
    const placeholderValue = objectValue["101"];
    const unpackedPlaceholder = placeholderValue != undefined ? placeholderValue : null;
    const isVisibleValue = objectValue["60"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectValue["61"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const positionValue = objectValue["40"];
    const unpackedPosition =
      positionValue != undefined ? Position.fromValue(positionValue, _session, _supergraph, _graph, _connection) : null;
    const widthValue = objectValue["41"];
    const unpackedWidth =
      widthValue != undefined ? Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection) : null;
    const heightValue = objectValue["42"];
    const unpackedHeight =
      heightValue != undefined ? Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection) : null;
    const minWidthValue = objectValue["43"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["44"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["45"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["46"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    const parentValue = objectValue["3"];
    const unpackedParent =
      parentValue != undefined
        ? NodeReference.fromValue(parentValue, _session, _supergraph, _graph, _connection)
        : null;
    const spaceValue = objectValue["5"];
    const unpackedSpace =
      spaceValue != undefined ? NodeReference.fromValue(spaceValue, _session, _supergraph, _graph, _connection) : null;
    const createdByValue = objectValue["16"];
    const unpackedCreatedBy =
      createdByValue != undefined
        ? NodeReference.fromValue(createdByValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByValue = objectValue["18"];
    const unpackedUpdatedBy =
      updatedByValue != undefined
        ? NodeReference.fromValue(updatedByValue, _session, _supergraph, _graph, _connection)
        : null;
    const scriptValue = objectValue["200"];
    const unpackedScript =
      scriptValue != undefined
        ? NodeReference.fromValue(scriptValue, _session, _supergraph, _graph, _connection)
        : null;
    return new NumberInputView({
      value: unpackedValue,
      placeholder: unpackedPlaceholder,
      isVisible: unpackedIsVisible,
      opacity: unpackedOpacity,
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      orderKey: objectValue["22"],
      deletedAt: unpackedDeletedAt,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      script: unpackedScript,
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
  ): NumberInputView {
    return NumberInputView.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): NumberInputViewProto {
    return NumberInputView.__packProto__(this);
  }

  static __packProto__(object: NumberInputView): NumberInputViewProto {
    const objectProto: Partial<NumberInputViewProto> = { metatype: 10400 };
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
    objectProto.name = object.name;
    if (object.position != null) {
      objectProto.position = object.position.toProto();
    }
    if (object.width != null) {
      objectProto.width = object.width.toProto();
    }
    if (object.height != null) {
      objectProto.height = object.height.toProto();
    }
    if (object.minWidth != null) {
      objectProto.minWidth = object.minWidth.toProto();
    }
    if (object.minHeight != null) {
      objectProto.minHeight = object.minHeight.toProto();
    }
    if (object.maxWidth != null) {
      objectProto.maxWidth = object.maxWidth.toProto();
    }
    if (object.maxHeight != null) {
      objectProto.maxHeight = object.maxHeight.toProto();
    }
    if (object.isVisible != null) {
      objectProto.isVisible = object.isVisible;
    }
    if (object.opacity != null) {
      objectProto.opacity = object.opacity;
    }
    if (object.value != null) {
      objectProto.value = object.value;
    }
    if (object.placeholder != null) {
      objectProto.placeholder = object.placeholder;
    }
    if (object.scriptPtr != null) {
      objectProto.scriptPtr = object.scriptPtr.toProto();
    }
    return objectProto as NumberInputViewProto;
  }

  static __unpackProto__(
    objectProto: NumberInputViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberInputView {
    return new NumberInputView({
      value: objectProto.value != undefined ? objectProto.value : null,
      placeholder: objectProto.placeholder != undefined ? objectProto.placeholder : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      position:
        objectProto.position != undefined
          ? Position.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? Dimension.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? Dimension.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? Dimension.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? Dimension.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? Dimension.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? Dimension.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      deletedAt: objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      script:
        objectProto.scriptPtr != undefined
          ? NodeReference.fromProto(objectProto.scriptPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: NumberInputViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NumberInputView {
    return NumberInputView.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  // ...

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.NUMBER_INPUT_VIEW, NumberInputView);
/* ==== DESTACK_GENERATED_END:NODE:10400 ==== */
