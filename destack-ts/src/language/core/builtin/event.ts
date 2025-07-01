import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { NodeType, StructType } from "@destack/language/core/builtin/common";
import { Entity, Metric } from "@destack/language/core/builtin/entity";
import { Node } from "@destack/language/core/builtin/node";
import type {
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import type {
  HasName,
  IsCustomizable,
  IsExtensible,
  IsSourceable,
  IsSpatial,
  IsSubject,
} from "@destack/language/core/builtin/trait";
import { EditOperation, EditType } from "@destack/language/core/common/edit";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import {
  CustomEventDefinitionProto,
  EditEventProto,
  EditOperationProto,
  EditTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:3 ==== */
/**
 * An Event is an immutable record of something happening to an Entity.
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
/* ==== DESTACK_GENERATED_END:NODE:3 ==== */

/* ==== DESTACK_GENERATED_START:NODE:200 ==== */
/**
 * A CustomEventDefinition defines a kind of CustomEvent with custom Properties.
 */
export class CustomEventDefinition
  extends Entity
  implements IsSpatial, HasName, IsSourceable, IsCustomizable
{
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
   * IsCustomizable.value
   */
  value: Map<string, Value>;

  /**
   * IsOrdered.orderKey
   */
  readonly orderKey: string;

  /**
   * HasName.name
   */
  name: string;

  /**
   * A custom Event's prototype is the default template new CustomEvent instances are based on.
   */
  get prototype(): Node | null {
    const nodePtr: NodeReference | null = this.prototypePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  set prototype(node: Node | null) {
    if (node === null) {
      this.prototypePtr = null;
    } else {
      this.prototypePtr = node.toRef();
    }
  }
  prototypePtr: NodeReference | null;

  /**
   * CustomEventDefinition.baseType
   */
  baseType: NodeDefinitionReference | null;

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
    value?: Map<string, Value>;
    orderKey?: string;
    name: string;
    prototype?: Node | NodeReference | null;
    baseType?: NodeDefinitionReference | null;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _value = options.value ?? null;
    if (_value === null) {
      _value = new Map();
    }
    this.value = _value;
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
    let _prototype = options.prototype ?? null;
    if (_prototype != null && _prototype.metatype != StructType.NODE_REFERENCE) {
      _prototype = (_prototype as Node).toRef();
    }
    this.prototypePtr = _prototype;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
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
    if (!(this.prototypePtr?.id === other.prototypePtr?.id)) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
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
    if (Object.keys(this.value).length !== Object.keys(other.value).length) {
      return false;
    }
    for (const key in this.value) {
      if (!(key in other.value)) {
        return false;
      }
      if (!this.value.get(key)!.equals(other.value.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.prototypePtr !== null) {
      h = (h * 31 + hashString(this.prototypePtr.id)) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this.value && Object.keys(this.value).length > 0) {
      for (const [_key, _value] of Object.entries(this.value)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
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
    objectValue["1"] = 200;
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
    if (object.value.size > 0) {
      const packedValue: { [key: string]: any } = {};
      for (const [key, value] of object.value) {
        packedValue[String(String(key))] = value.toValue();
      }
      objectValue["21"] = packedValue;
    }
    objectValue["22"] = object.orderKey;
    objectValue["31"] = object.name;
    if (object.prototypePtr != null) {
      objectValue["40"] = object.prototypePtr.toValue();
    }
    if (object.baseType != null) {
      objectValue["41"] = object.baseType.toValue();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const prototypePtrValue = objectValue["40"];
    const unpackedPrototypePtr =
      prototypePtrValue != undefined
        ? _NodeReference.fromValue(prototypePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["41"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _NodeDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectValue["210"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedValue = new Map();
    if (objectValue["21"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["21"])) {
        unpackedValue.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CustomEventDefinition({
      prototype: unpackedPrototypePtr,
      baseType: unpackedBaseType,
      space: unpackedSpacePtr,
      name: objectValue["31"],
      source: unpackedSourcePtr,
      value: unpackedValue,
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
    const objectProto: Partial<CustomEventDefinitionProto> = { metatype: 200 };
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
    if (object.value) {
      objectProto.value = {};
      for (const [key, value] of object.value) {
        objectProto.value![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    objectProto.name = object.name;
    if (object.prototypePtr != null) {
      objectProto.prototypePtr = object.prototypePtr.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
    }
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedValue = new Map();
    if (objectProto.value) {
      for (const [key, value] of Object.entries(objectProto.value)) {
        unpackedValue.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomEventDefinition({
      prototype:
        objectProto.prototypePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.prototypePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      baseType:
        objectProto.baseType != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.baseType!,
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
      name: objectProto.name,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      value: unpackedValue,
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
/* ==== DESTACK_GENERATED_END:NODE:200 ==== */

/* ==== DESTACK_GENERATED_START:NODE:201 ==== */
/**
 * A CustomEvent is an instance of a CustomEventDefinition.
 */
export abstract class CustomEvent extends Event implements IsCustomizable, IsExtensible {
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
   * IsCustomizable.value
   */
  declare value: Map<string, Value>;

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
/* ==== DESTACK_GENERATED_END:NODE:201 ==== */

/* ==== DESTACK_GENERATED_START:NODE:21 ==== */
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
/* ==== DESTACK_GENERATED_END:NODE:21 ==== */

/* ==== DESTACK_GENERATED_START:NODE:93001 ==== */
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
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
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`EditEvent.node is required`);
    }
    this.nodePtr = _node;
    let _propPtr = options.propPtr ?? null;
    this.propPtr = _propPtr;
    let _field = options.field ?? null;
    if (_field != null && _field.metatype != StructType.NODE_REFERENCE) {
      _field = (_field as Node).toRef();
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
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
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
    objectValue["1"] = 93001;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const operationValue = objectValue["31"];
    const unpackedOperation = operationValue != undefined ? Number(operationValue) : null;
    const propPtrValue = objectValue["36"];
    const unpackedPropPtr =
      propPtrValue != undefined
        ? _PropertyReference.fromValue(propPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fieldPtrValue = objectValue["37"];
    const unpackedFieldPtr =
      fieldPtrValue != undefined
        ? _NodeReference.fromValue(fieldPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["38"];
    const unpackedKey =
      keyValue != undefined
        ? _Value.fromValue(keyValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["40"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new EditEvent({
      type: Number(objectValue["30"]),
      operation: unpackedOperation,
      node: _NodeReference.fromValue(objectValue["35"], _session, _supergraph, _graph, _connection),
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
    const objectProto: Partial<EditEventProto> = { metatype: 93001 };
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    return new EditEvent({
      type: Number(objectProto.type) as EditType,
      operation:
        objectProto.operation != undefined
          ? (Number(objectProto.operation) as EditOperation)
          : null,
      node: _NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      propPtr:
        objectProto.propPtr != undefined
          ? _PropertyReference.fromProto(
              objectProto.propPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      field:
        objectProto.fieldPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.fieldPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key:
        objectProto.key != undefined
          ? _Value.fromProto(objectProto.key!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
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
      id: String(objectProto.id),
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
/* ==== DESTACK_GENERATED_END:NODE:93001 ==== */
