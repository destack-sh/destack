import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  IsDeletable,
  IsOwnable,
  IsOwner,
  IsReactable,
  IsSpatial,
  IsSubject,
  IsTaggable,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  Entity,
  Graph,
  Node,
  NodeReference,
  NodeType,
  StructType,
  Text,
} from "@destack/language/core";
import { registerNodeClass } from "@destack/language/registry";
import type { Thread } from "@destack/language/social";
import type { Space } from "@destack/language/space";
import { MessageProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:120100 ==== */
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
    if (!(this.threadPtr?.id === other.threadPtr?.id)) {
      return false;
    }
    if (!(this.editedAt === other.editedAt)) {
      return false;
    }
    if (!(this.replyToPtr?.id === other.replyToPtr?.id)) {
      return false;
    }
    if (!(this.forwardedFromPtr?.id === other.forwardedFromPtr?.id)) {
      return false;
    }
    if (
      (this.text == null) !== (other.text == null) ||
      (this.text != null && !this.text.equals(other.text))
    ) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.ownedByPtr?.id === other.ownedByPtr?.id)) {
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
    if (this.threadPtr !== null) {
      h = (h * 31 + hashString(this.threadPtr.id)) & 0xffffffff;
    }
    if (this.editedAt !== null) {
      h = (h * 31 + hashString(this.editedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.replyToPtr !== null) {
      h = (h * 31 + hashString(this.replyToPtr.id)) & 0xffffffff;
    }
    if (this.forwardedFromPtr !== null) {
      h = (h * 31 + hashString(this.forwardedFromPtr.id)) & 0xffffffff;
    }
    if (this.text !== null) {
      h = (h * 31 + this.text.hash()) & 0xffffffff;
    }
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.ownedByPtr !== null) {
      h = (h * 31 + hashString(this.ownedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
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
    objectValue["1"] = 120100;
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
    if (object.ownedByPtr != null) {
      objectValue["25"] = object.ownedByPtr.toValue();
    }
    if (object.threadPtr != null) {
      objectValue["35"] = object.threadPtr.toValue();
    }
    if (object.editedAt != null) {
      objectValue["40"] = object.editedAt.toString({ timeZoneName: "never" });
    }
    if (object.replyToPtr != null) {
      objectValue["50"] = object.replyToPtr.toValue();
    }
    if (object.forwardedFromPtr != null) {
      objectValue["51"] = object.forwardedFromPtr.toValue();
    }
    if (object.text != null) {
      objectValue["61"] = object.text.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["62"] = object.nodePtr.toValue();
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const threadPtrValue = objectValue["35"];
    const unpackedThreadPtr =
      threadPtrValue != undefined
        ? NodeReference.fromValue(threadPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const editedAtValue = objectValue["40"];
    const unpackedEditedAt =
      editedAtValue != undefined
        ? Temporal.Instant.from(editedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const replyToPtrValue = objectValue["50"];
    const unpackedReplyToPtr =
      replyToPtrValue != undefined
        ? NodeReference.fromValue(replyToPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const forwardedFromPtrValue = objectValue["51"];
    const unpackedForwardedFromPtr =
      forwardedFromPtrValue != undefined
        ? NodeReference.fromValue(forwardedFromPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const textValue = objectValue["61"];
    const unpackedText =
      textValue != undefined
        ? Text.fromValue(textValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodePtrValue = objectValue["62"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const ownedByPtrValue = objectValue["25"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? NodeReference.fromValue(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
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
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
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
    const objectProto: Partial<MessageProto> = { metatype: 120100 };
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
    if (object.ownedByPtr != null) {
      objectProto.ownedByPtr = object.ownedByPtr.toProto();
    }
    if (object.threadPtr != null) {
      objectProto.threadPtr = object.threadPtr.toProto();
    }
    if (object.editedAt != null) {
      objectProto.editedAt = packProtoTimestamp(object.editedAt);
    }
    if (object.replyToPtr != null) {
      objectProto.replyToPtr = object.replyToPtr.toProto();
    }
    if (object.forwardedFromPtr != null) {
      objectProto.forwardedFromPtr = object.forwardedFromPtr.toProto();
    }
    if (object.text != null) {
      objectProto.text = object.text.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
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
    return new Message({
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
      thread:
        objectProto.threadPtr != undefined
          ? NodeReference.fromProto(
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
          ? NodeReference.fromProto(
              objectProto.replyToPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      forwardedFrom:
        objectProto.forwardedFromPtr != undefined
          ? NodeReference.fromProto(
              objectProto.forwardedFromPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      text:
        objectProto.text != undefined
          ? Text.fromProto(objectProto.text!, _session, _supergraph, _graph, _connection)
          : null,
      node:
        objectProto.nodePtr != undefined
          ? NodeReference.fromProto(
              objectProto.nodePtr!,
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
/* ==== DESTACK_GENERATED_END:NODE:120100 ==== */
