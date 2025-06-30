import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { Graph, NodeReference, QueryConnection, Session, Supergraph } from "@destack/language/core";
import {
  HasName,
  IsSourceable,
  IsSpatial,
  IsSubject,
  Node,
  NodeType,
  PropertyReference,
  StructType,
} from "@destack/language/core/builtin";
import {
  CustomProperty,
  EditOperation,
  EditType,
  Entity,
  Metric,
  Value,
} from "@destack/language/core/common";
import { Script } from "@destack/language/logic";
import { registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  CustomEventDefinitionProto,
  EditEventProto,
  EditOperationProto,
  EditTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:51002 ==== */
/**
 * An Event represents something happening in a Space.
 */
export abstract class Event extends Node implements IsSpatial {
  static metatype: NodeType = NodeType.EVENT;

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
   * The Node this Event is about.
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
  declare nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EVENT, Event);
/* ==== DESTACK_GENERATED_END:NODE:51002 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4300 ==== */
/**
 * A CustomEventDefinition defines a kind of CustomEvent.
 */
export class CustomEventDefinition extends Entity implements IsSpatial, HasName, IsSourceable {
  static metatype: NodeType = NodeType.CUSTOM_EVENT_DEFINITION;

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
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
   */
  name: string;

  /**
   * IsSourceable.source
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    orderKey?: string;
    name: string;
    source?: Script | NodeReference | null;
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
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomEventDefinition.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomEventDefinition.name is required`);
    }
    this.name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source instanceof Node) {
      _source = _source.toRef();
    }
    this.sourcePtr = _source;

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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
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
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.CUSTOM_EVENT_DEFINITION,
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
    return `<CustomEventDefinition '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomEventDefinition.__packValue__(this);
  }

  static __packValue__(object: CustomEventDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4300;
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
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.sourcePtr != null) {
      objectValue["210"] = object.sourcePtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomEventDefinition {
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectValue["210"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
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
    return new CustomEventDefinition({
      space: unpackedSpacePtr,
      name: objectValue["31"],
      source: unpackedSourcePtr,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      orderKey: objectValue["22"],
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
  ): CustomEventDefinition {
    return CustomEventDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): CustomEventDefinitionProto {
    return CustomEventDefinition.__packProto__(this);
  }

  static __packProto__(object: CustomEventDefinition): CustomEventDefinitionProto {
    const objectProto: Partial<CustomEventDefinitionProto> = { metatype: 4300 };
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
    objectProto.orderKey = object.orderKey;
    objectProto.name = object.name;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    return objectProto as CustomEventDefinitionProto;
  }

  static __unpackProto__(
    objectProto: CustomEventDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomEventDefinition {
    return new CustomEventDefinition({
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
      source:
        objectProto.sourcePtr != undefined
          ? NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      orderKey: objectProto.orderKey,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomEventDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomEventDefinition {
    return CustomEventDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): CustomEventDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomEventDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_EVENT_DEFINITION, CustomEventDefinition);
/* ==== DESTACK_GENERATED_END:NODE:4300 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4301 ==== */
/**
 * An instance of a CustomEventDefinition.
 */
export abstract class CustomEvent extends Event {
  static metatype: NodeType = NodeType.CUSTOM_EVENT;

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
   * The Node this Event is about.
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
  declare nodePtr: NodeReference | null;

  /**
   * The CustomEventDefinition this CustomEvent is an instance of.
   */
  get definition(): CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEventDefinition | null;
    }
    return null;
  }
  set definition(node: CustomEventDefinition) {
    this.definitionPtr = node.toRef();
  }
  declare definitionPtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_EVENT, CustomEvent);
/* ==== DESTACK_GENERATED_END:NODE:4301 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4302 ==== */
/**
 * A Event of an Edit. Only EditEvents of Entities are allowed.
 */
export class EditEvent extends Event {
  static metatype: NodeType = NodeType.EDIT_EVENT;

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
   * Event.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

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
  readonly createdByPtr: NodeReference | null;

  /**
   * EditEvent.type
   */
  type: EditType;

  /**
   * EditEvent.operation
   */
  operation: EditOperation | null;

  /**
   * EditEvent.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set node(node: Node) {
    this.nodePtr = node.toRef();
  }
  nodePtr: NodeReference;

  /**
   * EditEvent.propPtr
   */
  propPtr: PropertyReference | null;

  /**
   * EditEvent.field
   */
  get field(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.fieldPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
    }
    return null;
  }
  set field(node: CustomProperty | null) {
    if (node === null) {
      this.fieldPtr = null;
    } else {
      this.fieldPtr = node.toRef();
    }
  }
  fieldPtr: NodeReference | null;

  /**
   * EditEvent.key
   */
  key: Value | null;

  /**
   * EditEvent.value
   */
  value: Value | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    type: EditType;
    operation?: EditOperation | null;
    node: Node | NodeReference;
    propPtr?: PropertyReference | null;
    field?: CustomProperty | NodeReference | null;
    key?: Value | null;
    value?: Value | null;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EditEvent.type is required`);
    }
    this.type = _type;
    let _operation = options.operation ?? null;
    this.operation = _operation;
    let _node = options.node;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    if (_node === null) {
      throw new Error(`EditEvent.node is required`);
    }
    this.nodePtr = _node;
    let _propPtr = options.propPtr ?? null;
    this.propPtr = _propPtr;
    let _field = options.field ?? null;
    if (_field != null && _field instanceof Node) {
      _field = _field.toRef();
    }
    this.fieldPtr = _field;
    let _key = options.key ?? null;
    this.key = _key;
    let _value = options.value ?? null;
    this.value = _value;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`{cls.__name__}.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
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
    if (!(this.operation === other.operation)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (
      (this.propPtr == null) !== (other.propPtr == null) ||
      (this.propPtr != null && !this.propPtr.equals(other.propPtr))
    ) {
      return false;
    }
    if (!(this.fieldPtr?.id === other.fieldPtr?.id)) {
      return false;
    }
    if (
      (this.key == null) !== (other.key == null) ||
      (this.key != null && !this.key.equals(other.key))
    ) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.operation !== null) {
      h = (h * 31 + this.operation) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    if (this.propPtr !== null) {
      h = (h * 31 + this.propPtr.hash()) & 0xffffffff;
    }
    if (this.fieldPtr !== null) {
      h = (h * 31 + hashString(this.fieldPtr.id)) & 0xffffffff;
    }
    if (this.key !== null) {
      h = (h * 31 + this.key.hash()) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
      nodeType: NodeType.EDIT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return "EditEvent[id={this.id}]";
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
    propertyReprs.push(`type=${EditType[this.type]}`);
    if (this.operation !== null) {
      propertyReprs.push(`operation=${EditOperation[this.operation]}`);
    }
    propertyReprs.push(`node=${this.node?.repr()}`);
    if (this.propPtr !== null) {
      propertyReprs.push(`propPtr=${this.propPtr.repr()}`);
    }
    if (this.field !== null) {
      propertyReprs.push(`field=${this.field?.repr()}`);
    }
    if (this.key !== null) {
      propertyReprs.push(`key=${this.key.repr()}`);
    }
    return `<EditEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return EditEvent.__packValue__(this);
  }

  static __packValue__(object: EditEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 4302;
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
    objectValue["30"] = object.type;
    if (object.operation != null) {
      objectValue["31"] = object.operation;
    }
    objectValue["35"] = object.nodePtr.toValue();
    if (object.propPtr != null) {
      objectValue["36"] = object.propPtr.toValue();
    }
    if (object.fieldPtr != null) {
      objectValue["37"] = object.fieldPtr.toValue();
    }
    if (object.key != null) {
      objectValue["38"] = object.key.toValue();
    }
    if (object.value != null) {
      objectValue["40"] = object.value.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    const operationValue = objectValue["31"];
    const unpackedOperation = operationValue != undefined ? Number(operationValue) : null;
    const propPtrValue = objectValue["36"];
    const unpackedPropPtr =
      propPtrValue != undefined
        ? PropertyReference.fromValue(propPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fieldPtrValue = objectValue["37"];
    const unpackedFieldPtr =
      fieldPtrValue != undefined
        ? NodeReference.fromValue(fieldPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["38"];
    const unpackedKey =
      keyValue != undefined
        ? Value.fromValue(keyValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["40"];
    const unpackedValue =
      valueValue != undefined
        ? Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new EditEvent({
      type: Number(objectValue["30"]),
      operation: unpackedOperation,
      node: NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
      propPtr: unpackedPropPtr,
      field: unpackedFieldPtr,
      key: unpackedKey,
      value: unpackedValue,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      space: unpackedSpacePtr,
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
  ): EditEvent {
    return EditEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EditEventProto {
    return EditEvent.__packProto__(this);
  }

  static __packProto__(object: EditEvent): EditEventProto {
    const objectProto: Partial<EditEventProto> = { metatype: 4302 };
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
    objectProto.type = Number(object.type) as EditTypeProto;
    if (object.operation != null) {
      objectProto.operation = Number(object.operation) as EditOperationProto;
    }
    objectProto.nodePtr = object.nodePtr.toProto();
    if (object.propPtr != null) {
      objectProto.propPtr = object.propPtr.toProto();
    }
    if (object.fieldPtr != null) {
      objectProto.fieldPtr = object.fieldPtr.toProto();
    }
    if (object.key != null) {
      objectProto.key = object.key.toProto();
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    return objectProto as EditEventProto;
  }

  static __unpackProto__(
    objectProto: EditEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    return new EditEvent({
      type: Number(objectProto.type) as EditType,
      operation:
        objectProto.operation != undefined
          ? (Number(objectProto.operation) as EditOperation)
          : null,
      node: NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      propPtr:
        objectProto.propPtr != undefined
          ? PropertyReference.fromProto(
              objectProto.propPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      field:
        objectProto.fieldPtr != undefined
          ? NodeReference.fromProto(
              objectProto.fieldPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key:
        objectProto.key != undefined
          ? Value.fromProto(objectProto.key!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
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
    objectProto: EditEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EditEvent {
    return EditEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): EditEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EditEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EDIT_EVENT, EditEvent);
/* ==== DESTACK_GENERATED_END:NODE:4302 ==== */

/* ==== DESTACK_GENERATED_START:NODE:4220 ==== */
/**
 * An Event that represents a Measurement.
 */
export abstract class MeasurementEvent extends Event {
  static metatype: NodeType = NodeType.MEASUREMENT_EVENT;

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
   * MeasurementEvent.definition
   */
  get definition(): Metric | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Metric | null;
    }
    return null;
  }
  declare readonly definitionPtr: NodeReference;

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
   * The Node this Event is about.
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
  declare nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEASUREMENT_EVENT, MeasurementEvent);
/* ==== DESTACK_GENERATED_END:NODE:4220 ==== */
