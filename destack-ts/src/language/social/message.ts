import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsDeletable,
  IsOwnable,
  IsOwner,
  IsReactable,
  IsSpatial,
  IsSubject,
  IsTaggable,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Text,
} from "@destack/language/core";
import { Entity, Materialization, Node, NodeType, StructType } from "@destack/language/core";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Thread } from "@destack/language/social/thread";
import type { Space } from "@destack/language/universe";
import { MaterializationProto, MessageProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:220100 ==== */
/**
 * A Message about something (usually in a Thread or a Channel).
 */
export class Message
  extends Entity
  implements IsSpatial, IsOwnable, IsDeletable, IsTaggable, IsReactable
{
  static metatype: NodeType = NodeType.MESSAGE;

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
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): Message | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): Message | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Message | null;
    }
    return null;
  }
  readonly templatePtr: NodeReference | null;

  /**
   * The (root) Entity in this Entity's instance tree (not the template tree).
   */
  get instanceRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instanceRootPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instanceRootPtr: NodeReference | null;

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
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

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
  get threadPtr(): NodeReference | null {
    return this._threadPtr;
  }
  set threadPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["thread"];
    this._session.updateSetProperty(this, prop, value);
    this._threadPtr = value;
  }
  _threadPtr: NodeReference | null;

  /**
   * Message.editedAt
   */
  get editedAt(): Temporal.ZonedDateTime | null {
    return this._editedAt;
  }
  set editedAt(value: Temporal.ZonedDateTime | null) {
    const prop = (this.constructor as NodeClass).__properties__["edited_at"];
    this._session.updateSetProperty(this, prop, value);
    this._editedAt = value;
  }
  _editedAt: Temporal.ZonedDateTime | null;

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
  get replyToPtr(): NodeReference | null {
    return this._replyToPtr;
  }
  set replyToPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["reply_to"];
    this._session.updateSetProperty(this, prop, value);
    this._replyToPtr = value;
  }
  _replyToPtr: NodeReference | null;

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
  get forwardedFromPtr(): NodeReference | null {
    return this._forwardedFromPtr;
  }
  set forwardedFromPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["forwarded_from"];
    this._session.updateSetProperty(this, prop, value);
    this._forwardedFromPtr = value;
  }
  _forwardedFromPtr: NodeReference | null;

  /**
   * Message.text
   */
  get text(): Text | null {
    return this._text;
  }
  set text(value: Text | null) {
    const prop = (this.constructor as NodeClass).__properties__["text"];
    this._session.updateSetProperty(this, prop, value);
    this._text = value;
  }
  _text: Text | null;

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
  get nodePtr(): NodeReference | null {
    return this._nodePtr;
  }
  set nodePtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["node"];
    this._session.updateSetProperty(this, prop, value);
    this._nodePtr = value;
  }
  _nodePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Thread | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Message | NodeReference | null;
    template?: Message | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 32 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`Message.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _template = options.template ?? null;
    if (_template != null && _template.metatype != StructType.NODE_REFERENCE) {
      _template = (_template as Node).toRef();
    }
    this.templatePtr = _template;
    let _instanceRoot = options.instanceRoot ?? null;
    if (_instanceRoot != null && _instanceRoot.metatype != StructType.NODE_REFERENCE) {
      _instanceRoot = (_instanceRoot as Node).toRef();
    }
    this.instanceRootPtr = _instanceRoot;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _thread = options.thread ?? null;
    if (_thread != null && _thread.metatype != StructType.NODE_REFERENCE) {
      _thread = (_thread as Node).toRef();
    }
    this._threadPtr = _thread;
    let _editedAt = options.editedAt ?? null;
    this._editedAt = _editedAt;
    let _replyTo = options.replyTo ?? null;
    if (_replyTo != null && _replyTo.metatype != StructType.NODE_REFERENCE) {
      _replyTo = (_replyTo as Node).toRef();
    }
    this._replyToPtr = _replyTo;
    let _forwardedFrom = options.forwardedFrom ?? null;
    if (_forwardedFrom != null && _forwardedFrom.metatype != StructType.NODE_REFERENCE) {
      _forwardedFrom = (_forwardedFrom as Node).toRef();
    }
    this._forwardedFromPtr = _forwardedFrom;
    let _text = options.text ?? null;
    this._text = _text;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this._nodePtr = _node;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`Message.createdAt and Message.updatedAt are required for existing Nodes`);
      }
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
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._threadPtr?.id === other._threadPtr?.id)) {
      return false;
    }
    if (!(this._editedAt === other._editedAt)) {
      return false;
    }
    if (!(this._replyToPtr?.id === other._replyToPtr?.id)) {
      return false;
    }
    if (!(this._forwardedFromPtr?.id === other._forwardedFromPtr?.id)) {
      return false;
    }
    if (
      (this._text == null) !== (other._text == null) ||
      (this._text != null && !this._text.equals(other._text))
    ) {
      return false;
    }
    if (!(this._nodePtr?.id === other._nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this.templatePtr?.id === other.templatePtr?.id)) {
      return false;
    }
    if (!(this.instanceRootPtr?.id === other.instanceRootPtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this._threadPtr !== null) {
      h = (h * 31 + hashString(this._threadPtr.id)) & 0xffffffff;
    }
    if (this._editedAt !== null) {
      h = (h * 31 + hashString(this._editedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._replyToPtr !== null) {
      h = (h * 31 + hashString(this._replyToPtr.id)) & 0xffffffff;
    }
    if (this._forwardedFromPtr !== null) {
      h = (h * 31 + hashString(this._forwardedFromPtr.id)) & 0xffffffff;
    }
    if (this._text !== null) {
      h = (h * 31 + this._text.hash()) & 0xffffffff;
    }
    if (this._nodePtr !== null) {
      h = (h * 31 + hashString(this._nodePtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this._ownedByPtr !== null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr !== null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    if (this.templatePtr !== null) {
      h = (h * 31 + hashString(this.templatePtr.id)) & 0xffffffff;
    }
    if (this.instanceRootPtr !== null) {
      h = (h * 31 + hashString(this.instanceRootPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.MESSAGE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return `Message[id=${this.id}]`;
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
    if (this.ownedBy !== null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    if (propertyReprs.length > 0) {
      return `<Message '${this.path}' ${propertyReprs.join(" ")}>`;
    } else {
      return `<Message '${this.path}'>`;
    }
  }

  toValue(): { [key: string]: any } {
    return Message.__packValue__(this);
  }

  static __packValue__(object: Message): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 220100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    if (object.templatePtr != null) {
      objectValue["13"] = object.templatePtr.toValue();
    }
    if (object.instanceRootPtr != null) {
      objectValue["14"] = object.instanceRootPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectValue["28"] = object._ownedByPtr.toValue();
    }
    if (object._threadPtr != null) {
      objectValue["35"] = object._threadPtr.toValue();
    }
    if (object._editedAt != null) {
      objectValue["40"] = object._editedAt.toString({ timeZoneName: "never" });
    }
    if (object._replyToPtr != null) {
      objectValue["50"] = object._replyToPtr.toValue();
    }
    if (object._forwardedFromPtr != null) {
      objectValue["51"] = object._forwardedFromPtr.toValue();
    }
    if (object._text != null) {
      objectValue["61"] = object._text.toValue();
    }
    if (object._nodePtr != null) {
      objectValue["62"] = object._nodePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Message {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Text = STRUCT_CLASS_BY_TYPE[StructType.TEXT] as typeof Text;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const threadPtrValue = objectValue["35"];
    const unpackedThreadPtr =
      threadPtrValue != undefined
        ? _NodeReference.fromValue(threadPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const editedAtValue = objectValue["40"];
    const unpackedEditedAt =
      editedAtValue != undefined
        ? Temporal.Instant.from(editedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const replyToPtrValue = objectValue["50"];
    const unpackedReplyToPtr =
      replyToPtrValue != undefined
        ? _NodeReference.fromValue(replyToPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const forwardedFromPtrValue = objectValue["51"];
    const unpackedForwardedFromPtr =
      forwardedFromPtrValue != undefined
        ? _NodeReference.fromValue(
            forwardedFromPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const textValue = objectValue["61"];
    const unpackedText =
      textValue != undefined
        ? _Text.fromValue(textValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodePtrValue = objectValue["62"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["28"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const templatePtrValue = objectValue["13"];
    const unpackedTemplatePtr =
      templatePtrValue != undefined
        ? _NodeReference.fromValue(templatePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instanceRootPtrValue = objectValue["14"];
    const unpackedInstanceRootPtr =
      instanceRootPtrValue != undefined
        ? _NodeReference.fromValue(instanceRootPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Message({
      parent: unpackedParentPtr,
      thread: unpackedThreadPtr,
      editedAt: unpackedEditedAt,
      replyTo: unpackedReplyToPtr,
      forwardedFrom: unpackedForwardedFromPtr,
      text: unpackedText,
      node: unpackedNodePtr,
      space: unpackedSpacePtr,
      ownedBy: unpackedOwnedByPtr,
      deletedAt: unpackedDeletedAt,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      instanceRoot: unpackedInstanceRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
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
  ): Message {
    return Message.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): MessageProto {
    return Message.__packProto__(this);
  }

  static __packProto__(object: Message): MessageProto {
    const objectProto: Partial<MessageProto> = { metatype: 220100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
    }
    if (object.templatePtr != null) {
      objectProto.templatePtr = object.templatePtr.toProto();
    }
    if (object.instanceRootPtr != null) {
      objectProto.instanceRootPtr = object.instanceRootPtr.toProto();
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
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    if (object._threadPtr != null) {
      objectProto.threadPtr = object._threadPtr.toProto();
    }
    if (object._editedAt != null) {
      objectProto.editedAt = packProtoTimestamp(object._editedAt);
    }
    if (object._replyToPtr != null) {
      objectProto.replyToPtr = object._replyToPtr.toProto();
    }
    if (object._forwardedFromPtr != null) {
      objectProto.forwardedFromPtr = object._forwardedFromPtr.toProto();
    }
    if (object._text != null) {
      objectProto.text = object._text.toProto();
    }
    if (object._nodePtr != null) {
      objectProto.nodePtr = object._nodePtr.toProto();
    }
    return objectProto as MessageProto;
  }

  static __unpackProto__(
    objectProto: MessageProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Message {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Text = STRUCT_CLASS_BY_TYPE[StructType.TEXT] as typeof Text;
    return new Message({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      thread:
        objectProto.threadPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.threadPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      editedAt:
        objectProto.editedAt != undefined ? unpackProtoTimestamp(objectProto.editedAt!) : null,
      replyTo:
        objectProto.replyToPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.replyToPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      forwardedFrom:
        objectProto.forwardedFromPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.forwardedFromPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      text:
        objectProto.text != undefined
          ? _Text.fromProto(objectProto.text!, _session, _supergraph, _graph, _connection)
          : null,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.nodePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      template:
        objectProto.templatePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.templatePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instanceRoot:
        objectProto.instanceRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instanceRootPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
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
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MessageProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Message {
    return Message.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Message {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MessageProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MESSAGE, Message);
/* ==== DESTACK_GENERATED_END:NODE:220100 ==== */
