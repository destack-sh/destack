import {
  Entity,
  Graph,
  HasIcon,
  HasName,
  HasSlug,
  Icon,
  IsDeletable,
  IsFollowable,
  IsOwner,
  IsScriptable,
  IsSubject,
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
import { Folder } from "@destack/language/folder";
import { Cursor, Script } from "@destack/language/logic";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:600 ==== */
/**
 * An Agent is an identity for a bot.
 */
export class Agent
  extends Node
  implements Spatial, Entity, HasName, HasIcon, HasSlug, IsOwner, IsFollowable, IsScriptable, IsDeletable, IsSubject
{
  static metatype: NodeType = NodeType.AGENT;
  static __traits__: TraitType[] = [
    TraitType.SCRIPTABLE,
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.SUBJECT,
    TraitType.OWNER,
    TraitType.FOLLOWABLE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.FOLDER];
  static __childTypes__: NodeType[] = [
    NodeType.ENTITLEMENT,
    NodeType.SANCTION,
    NodeType.SCRIPT,
    NodeType.FOLLOW,
    NodeType.CLIENT,
  ];
  static __ancestorTypes__: NodeType[] = [NodeType.FOLDER, NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [
    NodeType.OPTION,
    NodeType.CLIENT,
    NodeType.ENTITLEMENT,
    NodeType.FIELD,
    NodeType.TAGGING,
    NodeType.SCRIPT,
    NodeType.FOLLOW,
    NodeType.SANCTION,
  ];

  /**
   * Agent.parent
   */
  get parent(): Folder | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Folder | null;
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
   * Agent.name
   */
  name: string;

  /**
   * Agent.slug
   */
  slug: string;

  /**
   * HasIcon.icon
   */
  icon: Icon | null;

  /**
   * Agent.cursor
   */
  get cursor(): (Node & Cursor) | null {
    const nodePtr: NodeReference | null = this.cursorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & Cursor) | null;
    }
    return null;
  }
  set cursor(node: (Node & Cursor) | null) {
    if (node === null) {
      this.cursorPtr = null;
    } else {
      this.cursorPtr = node.toRef();
    }
  }
  cursorPtr: NodeReference | null;

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
    parent?: Folder | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    name: string;
    slug: string;
    icon?: Icon | null;
    cursor?: (Node & Cursor) | NodeReference | null;
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
      throw new Error(`Agent.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Agent.name is required`);
    }
    this.name = _name;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Agent.slug is required`);
    }
    this.slug = _slug;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _cursor = options.cursor ?? null;
    if (_cursor != null && _cursor instanceof Node) {
      _cursor = _cursor.toRef();
    }
    this.cursorPtr = _cursor;
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
      nodeType: NodeType.AGENT,
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
    return Agent.__packValue__(this);
  }

  static __packValue__(object: Agent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 600;
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
    objectValue["31"] = object.name;
    objectValue["33"] = object.slug;
    if (object.icon !== null) {
      objectValue["34"] = object.icon.toValue();
    }
    if (object.cursorPtr !== null) {
      objectValue["52"] = object.cursorPtr.toValue();
    }
    if (object.scriptPtr !== null) {
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
  ): Agent {
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
    const cursorValue = objectValue["52"];
    const unpackedCursor =
      cursorValue !== undefined
        ? NodeReference.fromValue(cursorValue, _session, _supergraph, _graph, _connection)
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
    const scriptValue = objectValue["200"];
    const unpackedScript =
      scriptValue !== undefined
        ? NodeReference.fromValue(scriptValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Agent({
      name: objectValue["31"],
      slug: objectValue["33"],
      id: String(objectValue["2"]),
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      icon: unpackedIcon,
      deletedAt: unpackedDeletedAt,
      parent: unpackedParent,
      cursor: unpackedCursor,
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
  ): Agent {
    return Agent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:NODE:600 ==== */
