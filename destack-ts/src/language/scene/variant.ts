import {
  Entity,
  Graph,
  HasIcon,
  HasName,
  HasSlug,
  Icon,
  IsDeletable,
  IsOwnable,
  IsOwner,
  IsSubject,
  Length,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Layer, Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { CustomViewDefinition } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:9030 ==== */
/**
 * VariantType
 */
export enum VariantType {
  DYNAMIC = 1,
  BREAKPOINT = 2,
  PLATFORM = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:9030 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:9031 ==== */
/**
 * VariantStateType
 */
export enum VariantStateType {
  LOADING = 10,
  ERROR = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:9031 ==== */

/* ==== DESTACK_GENERATED_START:NODE:9030 ==== */
/**
 * A Variant is an alternative presentation of a visual.
 */
export class Variant extends Node implements Spatial, Entity, HasName, HasSlug, HasIcon, IsOwnable, IsDeletable {
  static metatype: NodeType = NodeType.VARIANT;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.CUSTOM_VIEW_DEFINITION, NodeType.SCENE, NodeType.LAYER];
  static __childTypes__: NodeType[] = [];
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
  static __descendantTypes__: NodeType[] = [];

  /**
   * Variant.parent
   */
  get parent(): Scene | Layer | CustomViewDefinition | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | Layer | CustomViewDefinition | null;
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
   * IsOwnable.ownedBy
   */
  get ownedBy(): (Node & IsOwner) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsOwner) | null;
    }
    return null;
  }
  set ownedBy(node: (Node & IsOwner) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;

  /**
   * Variant.type
   */
  type: VariantType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * HasSlug.slug
   */
  slug: string | null;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * Variant.maxWidth
   */
  maxWidth: Length | null;

  /**
   * Variant.maxHeight
   */
  maxHeight: Length | null;

  /**
   * Variant.minWidth
   */
  minWidth: Length | null;

  /**
   * Variant.minHeight
   */
  minHeight: Length | null;

  constructor(options: {
    id?: string;
    parent?: Scene | Layer | CustomViewDefinition | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    type: VariantType;
    name: string;
    slug?: string | null;
    icon?: Icon | null;
    maxWidth?: Length | null;
    maxHeight?: Length | null;
    minWidth?: Length | null;
    minHeight?: Length | null;
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
      throw new Error(`Variant.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Variant.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Variant.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _maxWidth = options.maxWidth ?? null;
    this.maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this.maxHeight = _maxHeight;
    let _minWidth = options.minWidth ?? null;
    this.minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this.minHeight = _minHeight;

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
      nodeType: NodeType.VARIANT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
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
    return Variant.__packValue__(this);
  }

  static __packValue__(object: Variant): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 9030;
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
    if (object.ownedByPtr !== null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.slug !== null) {
      objectValue["33"] = object.slug;
    }
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.maxWidth !== null) {
      objectValue["50"] = object.maxWidth.toValue();
    }
    if (object.maxHeight !== null) {
      objectValue["51"] = object.maxHeight.toValue();
    }
    if (object.minWidth !== null) {
      objectValue["52"] = object.minWidth.toValue();
    }
    if (object.minHeight !== null) {
      objectValue["53"] = object.minHeight.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Variant {
    const maxWidthValue = objectValue["50"];
    const unpackedMaxWidth =
      maxWidthValue !== undefined ? Length.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection) : null;
    const maxHeightValue = objectValue["51"];
    const unpackedMaxHeight =
      maxHeightValue !== undefined
        ? Length.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectValue["52"];
    const unpackedMinWidth =
      minWidthValue !== undefined ? Length.fromValue(minWidthValue, _session, _supergraph, _graph, _connection) : null;
    const minHeightValue = objectValue["53"];
    const unpackedMinHeight =
      minHeightValue !== undefined
        ? Length.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const slugValue = objectValue["33"];
    const unpackedSlug = slugValue !== undefined ? slugValue : null;
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue !== undefined ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection) : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt = deletedAtValue !== undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
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
    const ownedByValue = objectValue["25"];
    const unpackedOwnedBy =
      ownedByValue !== undefined
        ? NodeReference.fromValue(ownedByValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Variant({
      type: Number(objectValue["30"]),
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      name: objectValue["31"],
      slug: unpackedSlug,
      icon: unpackedIcon,
      deletedAt: unpackedDeletedAt,
      parent: unpackedParent,
      space: unpackedSpace,
      createdBy: unpackedCreatedBy,
      updatedBy: unpackedUpdatedBy,
      ownedBy: unpackedOwnedBy,
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
  ): Variant {
    return Variant.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:9030 ==== */
