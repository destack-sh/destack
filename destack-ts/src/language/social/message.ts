import {
  Entity,
  Graph,
  IsDeletable,
  IsOwnable,
  IsOwner,
  IsReactable,
  IsSubject,
  IsTaggable,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  Text,
  TraitType,
} from "@destack/language/core";
import { Thread } from "@destack/language/social";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:5510 ==== */
/**
 * A Message about something (usually in a Thread or a Channel).
 */
export class Message extends Node implements Spatial, Entity, IsOwnable, IsDeletable, IsTaggable, IsReactable {
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

  /**
   * Message.parent
   */
  get parent(): Thread | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Thread | null;
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
   * Message.thread
   */
  get thread(): Thread | null {
    const nodePtr: NodeReference | null = this.threadPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Thread | null;
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

  /**
   * Message.editedAt
   */
  editedAt: Temporal.ZonedDateTime | null;

  /**
   * Message.replyTo
   */
  get replyTo(): Message | null {
    const nodePtr: NodeReference | null = this.replyToPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null;
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

  /**
   * Message.forwardedFrom
   */
  get forwardedFrom(): Message | null {
    const nodePtr: NodeReference | null = this.forwardedFromPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null;
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

  /**
   * Message.text
   */
  text: Text | null;

  /**
   * Message.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
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
    id?: string;
    parent?: Thread | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Node & IsOwner) | NodeReference | null;
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
      throw new Error(`Message.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy instanceof Node) {
      _ownedBy = _ownedBy.toRef();
    }
    this.ownedByPtr = _ownedBy;
    let _thread = options.thread ?? null;
    if (_thread != null && _thread instanceof Node) {
      _thread = _thread.toRef();
    }
    this.threadPtr = _thread;
    let _editedAt = options.editedAt ?? null;
    this.editedAt = _editedAt;
    let _replyTo = options.replyTo ?? null;
    if (_replyTo != null && _replyTo instanceof Node) {
      _replyTo = _replyTo.toRef();
    }
    this.replyToPtr = _replyTo;
    let _forwardedFrom = options.forwardedFrom ?? null;
    if (_forwardedFrom != null && _forwardedFrom instanceof Node) {
      _forwardedFrom = _forwardedFrom.toRef();
    }
    this.forwardedFromPtr = _forwardedFrom;
    let _text = options.text ?? null;
    this.text = _text;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;

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
}
/* ==== DESTACK_GENERATED_END:NODE:5510 ==== */
