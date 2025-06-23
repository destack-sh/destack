import {
  Agent,
  Entity,
  Graph,
  IsDeletable,
  IsOwnable,
  IsReactable,
  IsTaggable,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Organization,
  QueryConnection,
  Role,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  Team,
  Text,
  Thread,
  TraitType,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:5510 ==== */
export class Message
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsOwnable, IsTaggable, IsReactable
{
  static metatype: NodeType = NodeType.MESSAGE;
  static __traits__: TraitType[] = [
    TraitType.SPATIAL,
    TraitType.TAGGABLE,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.OWNABLE,
    TraitType.DELETABLE,
    TraitType.REACTABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.THREAD];
  static __childTypes__: NodeType[] = [NodeType.TAGGING, NodeType.REACTION];
  static __ancestorTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE, NodeType.THREAD];
  static __descendantTypes__: NodeType[] = [NodeType.REACTION, NodeType.TAGGING];

  readonly id: string;
  get parent(): Thread | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Thread | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
    }
    return null;
  }

  set ownedBy(node: Role | Agent | Organization | Team | User | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  ownedByPtr: NodeReference | null;
  get thread(): Thread | null | null {
    const nodePtr: NodeReference | null = this.threadPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Thread | null | null;
    }
    return null;
  }

  set thread(node: Thread | null) {
    if (node === null) {
      this.threadPtr = null;
    } else {
      this.threadPtr = node.toRef();
    }
  }
  threadPtr: NodeReference | null;
  editedAt: Temporal.ZonedDateTime | null;
  get replyTo(): Message | null | null {
    const nodePtr: NodeReference | null = this.replyToPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null | null;
    }
    return null;
  }

  set replyTo(node: Message | null) {
    if (node === null) {
      this.replyToPtr = null;
    } else {
      this.replyToPtr = node.toRef();
    }
  }
  replyToPtr: NodeReference | null;
  get forwardedFrom(): Message | null | null {
    const nodePtr: NodeReference | null = this.forwardedFromPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null | null;
    }
    return null;
  }

  set forwardedFrom(node: Message | null) {
    if (node === null) {
      this.forwardedFromPtr = null;
    } else {
      this.forwardedFromPtr = node.toRef();
    }
  }
  forwardedFromPtr: NodeReference | null;
  text: Text | null;
  get node(): Node | null | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null | null;
    }
    return null;
  }

  set node(node: Node | null) {
    if (node === null) {
      this.nodePtr = null;
    } else {
      this.nodePtr = node.toRef();
    }
  }
  nodePtr: NodeReference | null;

  constructor(options: {
    id: string;
    parent?: Thread | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null;
    thread?: Thread | NodeReference | null;
    editedAt?: Temporal.ZonedDateTime | null;
    replyTo?: Message | NodeReference | null;
    forwardedFrom?: Message | NodeReference | null;
    text?: Text | null;
    node?: Node | NodeReference | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.deletedAt = options.deletedAt ?? null;
    this.ownedByPtr =
      options.ownedBy != null
        ? options.ownedBy.metatype == StructType.NODE_REFERENCE
          ? (options.ownedBy as NodeReference)
          : (options.ownedBy as Node).toRef()
        : null;
    this.threadPtr =
      options.thread != null
        ? options.thread.metatype == StructType.NODE_REFERENCE
          ? (options.thread as NodeReference)
          : (options.thread as Node).toRef()
        : null;
    this.editedAt = options.editedAt ?? null;
    this.replyToPtr =
      options.replyTo != null
        ? options.replyTo.metatype == StructType.NODE_REFERENCE
          ? (options.replyTo as NodeReference)
          : (options.replyTo as Node).toRef()
        : null;
    this.forwardedFromPtr =
      options.forwardedFrom != null
        ? options.forwardedFrom.metatype == StructType.NODE_REFERENCE
          ? (options.forwardedFrom as NodeReference)
          : (options.forwardedFrom as Node).toRef()
        : null;
    this.text = options.text ?? null;
    this.nodePtr =
      options.node != null
        ? options.node.metatype == StructType.NODE_REFERENCE
          ? (options.node as NodeReference)
          : (options.node as Node).toRef()
        : null;
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
      nodeType: NodeType.MESSAGE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "Message[id={this.id}]";
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
} /* ==== DESTACK_GENERATED_END:NODE:5510 ==== */
