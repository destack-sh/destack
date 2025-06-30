import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  EnumType,
  HasName,
  IsRunnable,
  IsSpatial,
  IsSubject,
  Node,
  NodeDefinitionReference,
  NodeType,
  StructType,
} from "@destack/language/core/builtin";
import { Condition, Entity, Event, Value } from "@destack/language/core/common";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import { TriggerProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:3040 ==== */
/**
 * TriggerType
 */
export enum TriggerType {
  EVENT = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TRIGGER_TYPE, TriggerType);
/* ==== DESTACK_GENERATED_END:ENUM:3040 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3080 ==== */
/**
 * A Trigger is a dynamic event to run something.
 */
export class Trigger extends Entity implements IsSpatial, HasName {
  static metatype: NodeType = NodeType.TRIGGER;

  /**
   * Trait.parent
   */
  get parent(): Node | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
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
   * HasName.name
   */
  name: string;

  /**
   * Trigger.event
   */
  event: NodeDefinitionReference | null;

  /**
   * Trigger.where
   */
  where: Condition | null;

  /**
   * Trigger.target
   */
  get target(): (Node & IsRunnable) | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsRunnable) | null;
    }
    return null;
  }
  set target(node: Node & IsRunnable) {
    this.targetPtr = node.toRef();
  }
  targetPtr: NodeReference;

  /**
   * Trigger.arguments
   */
  arguments: Map<string, Value>;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    name: string;
    event?: NodeDefinitionReference | null;
    where?: Condition | null;
    target: (Node & IsRunnable) | NodeReference;
    arguments?: Map<string, Value>;
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
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Trigger.name is required`);
    }
    this.name = _name;
    let _event = options.event ?? null;
    this.event = _event;
    let _where = options.where ?? null;
    this.where = _where;
    let _target = options.target;
    if (_target != null && _target instanceof Node) {
      _target = _target.toRef();
    }
    if (_target === null) {
      throw new Error(`Trigger.target is required`);
    }
    this.targetPtr = _target;
    let _arguments = options.arguments ?? null;
    if (_arguments === null) {
      _arguments = new Map();
    }
    this.arguments = _arguments;

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
    if (
      (this.event == null) !== (other.event == null) ||
      (this.event != null && !this.event.equals(other.event))
    ) {
      return false;
    }
    if (
      (this.where == null) !== (other.where == null) ||
      (this.where != null && !this.where.equals(other.where))
    ) {
      return false;
    }
    if (!(this.targetPtr.id === other.targetPtr.id)) {
      return false;
    }
    if (Object.keys(this.arguments).length !== Object.keys(other.arguments).length) {
      return false;
    }
    for (const key in this.arguments) {
      if (!(key in other.arguments)) {
        return false;
      }
      if (!this.arguments.get(key)!.equals(other.arguments.get(key)!)) {
        return false;
      }
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
    if (this.event !== null) {
      h = (h * 31 + this.event.hash()) & 0xffffffff;
    }
    if (this.where !== null) {
      h = (h * 31 + this.where.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.targetPtr.id)) & 0xffffffff;
    if (this.arguments && Object.keys(this.arguments).length > 0) {
      for (const [_key, _value] of Object.entries(this.arguments)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.TRIGGER,
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
    propertyReprs.push(`name=${this.name}`);
    return `<Trigger '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Trigger.__packValue__(this);
  }

  static __packValue__(object: Trigger): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 3080;
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
    objectValue["31"] = object.name;
    if (object.event != null) {
      objectValue["40"] = object.event.toValue();
    }
    if (object.where != null) {
      objectValue["41"] = object.where.toValue();
    }
    objectValue["50"] = object.targetPtr.toValue();
    if (object.arguments.size > 0) {
      const packedArguments: { [key: string]: any } = {};
      for (const [key, value] of object.arguments) {
        packedArguments[String(String(key))] = value.toValue();
      }
      objectValue["51"] = packedArguments;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Trigger {
    const eventValue = objectValue["40"];
    const unpackedEvent =
      eventValue != undefined
        ? NodeDefinitionReference.fromValue(eventValue, _session, _supergraph, _graph, _connection)
        : null;
    const whereValue = objectValue["41"];
    const unpackedWhere =
      whereValue != undefined
        ? Condition.fromValue(whereValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedArguments = new Map();
    if (objectValue["51"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["51"])) {
        unpackedArguments.set(
          String(key),
          Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Trigger({
      event: unpackedEvent,
      where: unpackedWhere,
      target: NodeReference.fromValue(
        objectValue["50"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      arguments: unpackedArguments,
      space: unpackedSpacePtr,
      name: objectValue["31"],
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
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
  ): Trigger {
    return Trigger.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TriggerProto {
    return Trigger.__packProto__(this);
  }

  static __packProto__(object: Trigger): TriggerProto {
    const objectProto: Partial<TriggerProto> = { metatype: 3080 };
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
    objectProto.name = object.name;
    if (object.event != null) {
      objectProto.event = object.event.toProto();
    }
    if (object.where != null) {
      objectProto.where = object.where.toProto();
    }
    objectProto.targetPtr = object.targetPtr.toProto();
    if (object.arguments) {
      objectProto.arguments = {};
      for (const [key, value] of object.arguments) {
        objectProto.arguments![String(key)] = value.toProto();
      }
    }
    return objectProto as TriggerProto;
  }

  static __unpackProto__(
    objectProto: TriggerProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Trigger {
    const unpackedArguments = new Map();
    if (objectProto.arguments) {
      for (const [key, value] of Object.entries(objectProto.arguments)) {
        unpackedArguments.set(
          String(key),
          Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Trigger({
      event:
        objectProto.event != undefined
          ? NodeDefinitionReference.fromProto(
              objectProto.event!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      where:
        objectProto.where != undefined
          ? Condition.fromProto(objectProto.where!, _session, _supergraph, _graph, _connection)
          : null,
      target: NodeReference.fromProto(
        objectProto.targetPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      arguments: unpackedArguments,
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
      name: objectProto.name,
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
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: TriggerProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Trigger {
    return Trigger.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Trigger {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TriggerProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TRIGGER, Trigger);
/* ==== DESTACK_GENERATED_END:NODE:3080 ==== */

/* ==== DESTACK_GENERATED_START:NODE:3081 ==== */
/**
 * A TriggerEvent is an Event that corresponds to a Trigger.
 */
export abstract class TriggerEvent extends Event {
  static metatype: NodeType = NodeType.TRIGGER_EVENT;

  /**
   * Node.parent
   */
  get parent(): Node | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  declare readonly parentPtr: NodeReference | null;

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
  declare readonly spacePtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  declare readonly createdByPtr: NodeReference | null;

  /**
   * TriggerEvent.node
   */
  get node(): Trigger | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Trigger | null;
    }
    return null;
  }
  set node(node: Trigger) {
    this.nodePtr = node.toRef();
  }
  declare nodePtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TRIGGER_EVENT, TriggerEvent);
/* ==== DESTACK_GENERATED_END:NODE:3081 ==== */
