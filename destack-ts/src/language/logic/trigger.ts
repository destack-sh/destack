import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Condition,
  EventStatus,
  Graph,
  Icon,
  IsSpatial,
  IsSubject,
  NodeClass,
  NodeDefinitionReference,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Action } from "@destack/language/logic/action";
import type { Service } from "@destack/language/logic/service";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Client, Space } from "@destack/language/universe";
import { MaterializationProto, TriggerProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:105000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:105000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:107001 ==== */
/**
 * A TriggerEvent is an Event that corresponds to a Trigger.
 */
export abstract class TriggerEvent extends Event {
  static metatype: NodeType = NodeType.TRIGGER_EVENT;

  /**
   * Event.parent
   */
  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  /**
   * The Snapshot this Event originated from.
   */
  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Event.createdBy
   */
  abstract get createdBy(): (Entity & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * Event.client
   */
  abstract get client(): Client | null;
  declare readonly clientPtr: NodeReference | null;

  /**
   * Event.clientNonce
   */
  declare readonly clientNonce: string | null;

  /**
   * The status of the Event.
   */
  declare readonly status: EventStatus;

  /**
   * TriggerEvent.node
   */
  abstract get node(): Trigger | null;
  declare readonly nodePtr: NodeReference;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TRIGGER_EVENT, TriggerEvent);
/* ==== DESTACK_GENERATED_END:NODE:107001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:107000 ==== */
/**
 * A Trigger is a dynamic event to run something.
 */
export class Trigger extends Entity implements IsSpatial {
  static metatype: NodeType = NodeType.TRIGGER;

  /**
   * Entity.parent
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
  get predecessor(): Trigger | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Trigger | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): Trigger | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Trigger | null;
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
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Subject that created this Entity.
   */
  get createdBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Subject that last updated this Entity.
   */
  get updatedBy(): (Entity & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * Trigger.name
   */
  /**
   * Trigger.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * Trigger.icon
   */
  /**
   * Trigger.icon
   */
  get icon(): Icon | null {
    return this._icon;
  }
  set icon(value: Icon | null) {
    const prop = (this.constructor as NodeClass).__properties__["icon"];
    this._session.updateSetProperty(this, prop, value);
    this._icon = value;
  }
  _icon: Icon | null;

  /**
   * Trigger.event
   */
  /**
   * Trigger.event
   */
  get event(): NodeDefinitionReference | null {
    return this._event;
  }
  set event(value: NodeDefinitionReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["event"];
    this._session.updateSetProperty(this, prop, value);
    this._event = value;
  }
  _event: NodeDefinitionReference | null;

  /**
   * Trigger.where
   */
  /**
   * Trigger.where
   */
  get where(): Condition | null {
    return this._where;
  }
  set where(value: Condition | null) {
    const prop = (this.constructor as NodeClass).__properties__["where"];
    this._session.updateSetProperty(this, prop, value);
    this._where = value;
  }
  _where: Condition | null;

  /**
   * Trigger.target
   */
  get target(): Action | Service | null {
    const nodePtr: NodeReference | null = this.targetPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Action | Service | null;
    }
    return null;
  }
  set target(node: Action | Service | null) {
    if (node === null) {
      this.targetPtr = null;
    } else {
      this.targetPtr = node.toRef();
    }
  }
  /**
   * Trigger.target
   */
  get targetPtr(): NodeReference | null {
    return this._targetPtr;
  }
  set targetPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["target"];
    this._session.updateSetProperty(this, prop, value);
    this._targetPtr = value;
  }
  _targetPtr: NodeReference | null;

  /**
   * Trigger.arguments
   */
  /**
   * Trigger.arguments
   */
  get arguments(): { readonly [key: string]: Value } {
    return this._arguments;
  }
  set arguments(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["arguments"];
    this._session.updateSetProperty(this, prop, value);
    this._arguments = value;
  }
  _arguments: { readonly [key: string]: Value };

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: Trigger | NodeReference | null;
    template?: Trigger | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsSubject) | NodeReference | null;
    name: string;
    icon?: Icon | null;
    event?: NodeDefinitionReference | null;
    where?: Condition | null;
    target?: Action | Service | NodeReference | null;
    arguments?: { readonly [key: string]: Value };
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
      throw new Error(`Trigger.materialization is required`);
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
    let _name = options.name;
    if (_name === null) {
      throw new Error(`Trigger.name is required`);
    }
    this._name = _name;
    let _icon = options.icon ?? null;
    this._icon = _icon;
    let _event = options.event ?? null;
    this._event = _event;
    let _where = options.where ?? null;
    this._where = _where;
    let _target = options.target ?? null;
    if (_target != null && _target.metatype != StructType.NODE_REFERENCE) {
      _target = (_target as Node).toRef();
    }
    this._targetPtr = _target;
    let _arguments = options.arguments ?? null;
    if (_arguments === null) {
      _arguments = {};
    }
    this._arguments = _arguments;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`Trigger.createdAt and Trigger.updatedAt are required for existing Nodes`);
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
    if (!(this._name === other._name)) {
      return false;
    }
    if (
      (this._icon == null) !== (other._icon == null) ||
      (this._icon != null && !this._icon.equals(other._icon))
    ) {
      return false;
    }
    if (
      (this._event == null) !== (other._event == null) ||
      (this._event != null && !this._event.equals(other._event))
    ) {
      return false;
    }
    if (
      (this._where == null) !== (other._where == null) ||
      (this._where != null && !this._where.equals(other._where))
    ) {
      return false;
    }
    if (!(this._targetPtr?.id === other._targetPtr?.id)) {
      return false;
    }
    if (Object.keys(this._arguments).length !== Object.keys(other._arguments).length) {
      return false;
    }
    for (const key in this._arguments) {
      if (!(key in other._arguments)) {
        return false;
      }
      if (!this._arguments[key].equals(other._arguments[key])) {
        return false;
      }
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._icon !== null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
    }
    if (this._event !== null) {
      h = (h * 31 + this._event.hash()) & 0xffffffff;
    }
    if (this._where !== null) {
      h = (h * 31 + this._where.hash()) & 0xffffffff;
    }
    if (this._targetPtr !== null) {
      h = (h * 31 + hashString(this._targetPtr.id)) & 0xffffffff;
    }
    if (this._arguments && Object.keys(this._arguments).length > 0) {
      for (const [_key, _value] of Object.entries(this._arguments)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
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
      type: NodeType.TRIGGER,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
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
    let lastNode: Node | null = this;
    while (node !== null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Trigger "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Trigger.__packValue__(this);
  }

  static __packValue__(object: Trigger): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 107000;
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
    objectValue["101"] = object._name;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    if (object._event != null) {
      objectValue["110"] = object._event.toValue();
    }
    if (object._where != null) {
      objectValue["111"] = object._where.toValue();
    }
    if (object._targetPtr != null) {
      objectValue["120"] = object._targetPtr.toValue();
    }
    if (Object.keys(object._arguments).length > 0) {
      const packedArguments: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._arguments)) {
        packedArguments[String(String(key))] = value.toValue();
      }
      objectValue["121"] = packedArguments;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Trigger {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const eventValue = objectValue["110"];
    const unpackedEvent =
      eventValue != undefined
        ? _NodeDefinitionReference.fromValue(eventValue, _session, _supergraph, _graph, _connection)
        : null;
    const whereValue = objectValue["111"];
    const unpackedWhere =
      whereValue != undefined
        ? _Condition.fromValue(whereValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetPtrValue = objectValue["120"];
    const unpackedTargetPtr =
      targetPtrValue != undefined
        ? _NodeReference.fromValue(targetPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedArguments = {} as any;
    if (objectValue["121"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["121"])) {
        unpackedArguments[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
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
    return new Trigger({
      name: objectValue["101"],
      icon: unpackedIcon,
      event: unpackedEvent,
      where: unpackedWhere,
      target: unpackedTargetPtr,
      arguments: unpackedArguments,
      space: unpackedSpacePtr,
      parent: unpackedParentPtr,
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
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<TriggerProto> = { metatype: 107000 };
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
    objectProto.name = object._name;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
    }
    if (object._event != null) {
      objectProto.event = object._event.toProto();
    }
    if (object._where != null) {
      objectProto.where = object._where.toProto();
    }
    if (object._targetPtr != null) {
      objectProto.targetPtr = object._targetPtr.toProto();
    }
    if (object._arguments) {
      objectProto.arguments = {} as any;
      for (const [key, value] of Object.entries(object._arguments)) {
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedArguments = {} as any;
    if (objectProto.arguments) {
      for (const [key, value] of Object.entries(objectProto.arguments)) {
        unpackedArguments.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Trigger({
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      event:
        objectProto.event != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.event!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      where:
        objectProto.where != undefined
          ? _Condition.fromProto(objectProto.where!, _session, _supergraph, _graph, _connection)
          : null,
      target:
        objectProto.targetPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.targetPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      arguments: unpackedArguments,
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
/* ==== DESTACK_GENERATED_END:NODE:107000 ==== */
