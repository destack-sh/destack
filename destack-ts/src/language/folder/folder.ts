import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  Entity,
  EnumType,
  HasIcon,
  HasName,
  HasSlug,
  IsDeletable,
  IsFollowable,
  IsJoinable,
  IsOrdered,
  IsOwnable,
  IsOwner,
  IsStarable,
  IsSubject,
  IsTaggable,
  MaterializationType,
  Node,
  NodeType,
  Spatial,
  StructType,
  TraitType,
} from "@destack/language/core/builtin";
import { Icon } from "@destack/language/core/common";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Scene } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { FolderProto, FolderTypeProto, MaterializationTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:1000 ==== */
/**
 * FolderType
 */
export enum FolderType {
  SYSTEM = 1,
  HOME = 2,
  GENERAL = 3,
  MODULE = 4,
  APP = 5,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FOLDER_TYPE, FolderType);
/* ==== DESTACK_GENERATED_END:ENUM:1000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1000 ==== */
/**
 * A Folder is a sub-space of a Space.
 */
export class Folder
  extends Node
  implements
    Spatial,
    Entity,
    HasIcon,
    HasSlug,
    HasName,
    IsTaggable,
    IsOwnable,
    IsJoinable,
    IsOrdered,
    IsDeletable,
    IsStarable,
    IsFollowable
{
  static metatype: NodeType = NodeType.FOLDER;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.JOINABLE,
    TraitType.ORDERED,
    TraitType.STARABLE,
    TraitType.FOLLOWABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __childTypes__: NodeType[] = [
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.ENTITLEMENT,
    NodeType.INVITE,
    NodeType.MEMBERSHIP,
    NodeType.PERMISSION,
    NodeType.ROLE,
    NodeType.SANCTION,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.FOLDER,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.ROUTE,
    NodeType.SCRIPT,
    NodeType.SCENE,
    NodeType.FOLLOW,
    NodeType.STAR,
    NodeType.THREAD,
    NodeType.AGENT,
  ];
  static __ancestorTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [
    NodeType.LINE_SHAPE,
    NodeType.PLANE_SHAPE,
    NodeType.ARROW_SHAPE,
    NodeType.ANNOTATION_SHAPE,
    NodeType.MESSAGE,
    NodeType.ROLE,
    NodeType.REACTION,
    NodeType.STAR,
    NodeType.PERMISSION,
    NodeType.CUSTOM_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.FOLLOW,
    NodeType.WIZARD_VIEW,
    NodeType.SANCTION,
    NodeType.NUMBER_INPUT_VIEW,
    NodeType.SLIDER_INPUT_VIEW,
    NodeType.FRAME_VIEW,
    NodeType.ENTITLEMENT,
    NodeType.LABEL_VIEW,
    NodeType.CANVAS,
    NodeType.SCENE,
    NodeType.SCRIPT,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
    NodeType.VARIANT,
    NodeType.BORDER_STYLE,
    NodeType.ACTION,
    NodeType.CUSTOM_ENTITY_DEFINITION,
    NodeType.CUSTOM_ENTITY,
    NodeType.ROUTE,
    NodeType.CUSTOM_PROPERTY,
    NodeType.AGENT,
    NodeType.TEXT_VIEW,
    NodeType.CUSTOM_OPTION,
    NodeType.CLIENT,
    NodeType.THREAD_VIEW,
    NodeType.FOLDER,
    NodeType.PALETTE,
    NodeType.TAG,
    NodeType.TAGGING,
    NodeType.MEMBERSHIP,
    NodeType.COLOR_STYLE,
    NodeType.FONT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.THREAD,
    NodeType.INVITE,
  ];

  /**
   * Folder.parent
   */
  get parent(): Space | Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | Folder | null;
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
   * Folder.type
   */
  type: FolderType;

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
   * Folder.mainScene
   */
  get mainScene(): Scene | null {
    const nodePtr: NodeReference | null = this.mainScenePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Scene | null;
    }
    return null;
  }
  set mainScene(node: Scene | null) {
    if (node === null) {
      this.mainScenePtr = null;
    } else {
      this.mainScenePtr = node.toRef();
    }
  }
  mainScenePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Space | Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
    type?: FolderType;
    name: string;
    slug?: string | null;
    icon?: Icon | null;
    mainScene?: Scene | NodeReference | null;
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
      throw new Error(`Folder.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Folder.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = FolderType.GENERAL;
    }
    if (_type === null) {
      throw new Error(`Folder.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Folder.name is required`);
    }
    this.name = _name;
    let _slug = options.slug ?? null;
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _mainScene = options.mainScene ?? null;
    if (_mainScene != null && _mainScene instanceof Node) {
      _mainScene = _mainScene.toRef();
    }
    this.mainScenePtr = _mainScene;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
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
    if (!(this.mainScenePtr?.id === other.mainScenePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.slug === other.slug)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.ownedByPtr?.id === other.ownedByPtr?.id)) {
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
      nodeType: NodeType.FOLDER,
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
    return Folder.__packValue__(this);
  }

  static __packValue__(object: Folder): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1000;
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
    if (object.ownedByPtr != null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    if (object.slug != null) {
      objectValue["33"] = object.slug;
    }
    if (object.icon != null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.mainScenePtr != null) {
      objectValue["41"] = object.mainScenePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Folder {
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const mainScenePtrValue = objectValue["41"];
    const unpackedMainScenePtr =
      mainScenePtrValue != undefined
        ? NodeReference.fromValue(mainScenePtrValue, _session, _supergraph, _graph, _connection)
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
    const iconValue = objectValue["34"];
    const unpackedIcon =
      iconValue != undefined
        ? Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const slugValue = objectValue["33"];
    const unpackedSlug = slugValue != undefined ? slugValue : null;
    const ownedByPtrValue = objectValue["25"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined ? Temporal.ZonedDateTime.from(deletedAtValue) : null;
    return new Folder({
      parent: unpackedParentPtr,
      type: Number(objectValue["30"]),
      mainScene: unpackedMainScenePtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      icon: unpackedIcon,
      slug: unpackedSlug,
      name: objectValue["31"],
      ownedBy: unpackedOwnedByPtr,
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
  ): Folder {
    return Folder.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FolderProto {
    return Folder.__packProto__(this);
  }

  static __packProto__(object: Folder): FolderProto {
    const objectProto: Partial<FolderProto> = { metatype: 1000 };
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
    if (object.ownedByPtr != null) {
      objectProto.ownedByPtr = object.ownedByPtr.toProto();
    }
    objectProto.type = Number(object.type) as FolderTypeProto;
    objectProto.name = object.name;
    if (object.slug != null) {
      objectProto.slug = object.slug;
    }
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.mainScenePtr != null) {
      objectProto.mainScenePtr = object.mainScenePtr.toProto();
    }
    return objectProto as FolderProto;
  }

  static __unpackProto__(
    objectProto: FolderProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Folder {
    return new Folder({
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
      type: Number(objectProto.type) as FolderType,
      mainScene:
        objectProto.mainScenePtr != undefined
          ? NodeReference.fromProto(
              objectProto.mainScenePtr!,
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
      materialization: Number(objectProto.materialization) as MaterializationType,
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
      icon:
        objectProto.icon != undefined
          ? Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      slug: objectProto.slug != undefined ? objectProto.slug : null,
      name: objectProto.name,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      orderKey: objectProto.orderKey,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FolderProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Folder {
    return Folder.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Folder {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FolderProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FOLDER, Folder);
/* ==== DESTACK_GENERATED_END:NODE:1000 ==== */
