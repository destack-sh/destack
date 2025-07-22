import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  GraphConnection,
  IsActor,
  IsOwned,
  IsReactable,
  NodeClass,
  NodeReference,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  Event,
  EventStatus,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Client } from "@destack/language/universe";
import {
  EventStatusProto,
  MaterializationProto,
  ReactionAddedEventProto,
  ReactionEventProto,
  ReactionProto,
  ReactionRemovedEventProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1400000 ==== */
/**
 * A Reaction is a relationship between a Actor and a Reaction Node.
 */
export class Reaction extends Entity implements IsOwned {
  static metatype: NodeType = NodeType.REACTION;

  /**
   * Reaction.parent
   */
  get parent(): (Entity & IsReactable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsReactable) | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): Reaction | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Reaction | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
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
   * Entity.name
   */
  /**
   * Entity.name
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
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Script | null;
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
  /**
   * The Script of this Entity.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

  /**
   * Reaction.content
   */
  /**
   * Reaction.content
   */
  get content(): string {
    return this._content;
  }
  set content(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["content"];
    this._session.updateSetProperty(this, prop, value);
    this._content = value;
  }
  _content: string;

  constructor(options: {
    id?: string;
    parent?: (Entity & IsReactable) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Reaction | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    content: string;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.constructor.name == "NodeReference"
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // _isNew
      options.id == null,
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name != "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for Reaction`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Reaction.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Reaction.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for Reaction`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Reaction.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Reaction`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Reaction.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name != "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name != "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Reaction";
    }
    if (_name === null) {
      throw new Error(`Reaction.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Reaction.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.constructor.name != "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name != "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _content = options.content;
    if (_content === null) {
      throw new Error(`Reaction.content is required`);
    }
    this._content = _content;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(
          `Reaction.createdAt and Reaction.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy.constructor.name == "NodeReference"
            ? (options.updatedBy as NodeReference)
            : (options.updatedBy as Node).toRef()
          : null;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._content === other._content)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._content)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.REACTION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
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
    propertyReprs.push(`content=${`"${this.content}"`}`);
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Reaction "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return Reaction.__packCson__(this);
  }

  static __packCson__(object: Reaction): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1400000;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    objectCson["101"] = object._content;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Reaction {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new Reaction({
      parent: unpackedParentPtr,
      content: objectCson["101"],
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Reaction {
    return Reaction.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ReactionProto {
    return Reaction.__packProto__(this);
  }

  static __packProto__(object: Reaction): ReactionProto {
    const objectProto: Partial<ReactionProto> = { metatype: 1400000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.content = object._content;
    return objectProto as ReactionProto;
  }

  static __unpackProto__(
    objectProto: ReactionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Reaction {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _graph, _graph, _connection),
        );
      }
    }
    return new Reaction({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(objectProto.parentPtr!, _session, _graph, _graph, _connection)
          : null,
      content: objectProto.content,
      materialization: Number(objectProto.materialization) as Materialization,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedEpoch: Number(objectProto.updatedEpoch),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(objectProto.ownedByPtr!, _session, _graph, _graph, _connection)
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(objectProto.scriptPtr!, _session, _graph, _graph, _connection)
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(objectProto.sourcePtr!, _session, _graph, _graph, _connection)
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ReactionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Reaction {
    return Reaction.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Reaction {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ReactionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.REACTION, Reaction);
/* ==== DESTACK_GENERATED_END:NODE:1400000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1400001 ==== */
/**
 * ReactionEvent
 */
export class ReactionEvent extends Event {
  static metatype: NodeType = NodeType.REACTION_EVENT;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this Event is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Event originated from.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

  /**
   * The Snapshot this Event originated from.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Event that this Event follows.
   */
  get precededBy(): Event | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The Event that caused this Event (if any).
   */
  get causedBy(): Event | null {
    const nodePtr: NodeReference | null = this.causedByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Event | null;
    }
    return null;
  }
  readonly causedByPtr: NodeReference | null;

  /**
   * The time this Event was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Event was created (system time).
   */
  readonly createdEpoch: number;

  /**
   * The Actor that created this Event.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The Client that created this Event.
   */
  get client(): Client | null {
    const nodePtr: NodeReference | null = this.clientPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Client | null;
    }
    return null;
  }
  readonly clientPtr: NodeReference | null;

  /**
   * The nonce of the Client that created this Event.
   */
  readonly clientNonce: string | null;

  /**
   * The time in the Client when it created this Event.
   */
  readonly clientCreatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time in the Client when it created this Event.
   */
  readonly clientEpoch: number;

  /**
   * The status of the Event.
   */
  readonly status: EventStatus;

  /**
   * ReactionEvent.node
   */
  get node(): Reaction | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Reaction | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * ReactionEvent.content
   */
  readonly content: string;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node: Reaction | NodeReference;
    content: string;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      null,
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // _isNew
      options.id == null,
    );

    // properties
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name != "NodeReference") {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for ReactionEvent`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`ReactionEvent.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name != "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name != "NodeReference") {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for ReactionEvent`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`ReactionEvent.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for ReactionEvent`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`ReactionEvent.snapshot is required`);
    }
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name != "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _causedBy = options.causedBy ?? null;
    if (_causedBy != null && _causedBy.constructor.name != "NodeReference") {
      _causedBy = (_causedBy as Node).toRef();
    }
    this.causedByPtr = _causedBy as NodeReference | null;
    let _client = options.client ?? null;
    if (_client != null && _client.constructor.name != "NodeReference") {
      _client = (_client as Node).toRef();
    }
    this.clientPtr = _client as NodeReference | null;
    let _clientNonce = options.clientNonce ?? null;
    this.clientNonce = _clientNonce;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* EventStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`ReactionEvent.status is required`);
    }
    this.status = _status;
    let _node = options.node;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`ReactionEvent.node is required`);
    }
    this.nodePtr = _node as NodeReference;
    let _content = options.content;
    if (_content === null) {
      throw new Error(`ReactionEvent.content is required`);
    }
    this.content = _content;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.clientCreatedAt = now;
      this.clientEpoch = epoch;
    } else {
      if (
        options.createdAt == null ||
        options.createdEpoch == null ||
        options.clientCreatedAt == null ||
        options.clientEpoch == null
      ) {
        throw new Error(`ReactionEvent.createdAt is required for existing Events`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.constructor.name == "NodeReference"
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.clientCreatedAt = options.clientCreatedAt;
      this.clientEpoch = options.clientEpoch;
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.content === other.content)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.content)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.REACTION_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `ReactionEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<ReactionEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return ReactionEvent.__packCson__(this);
  }

  static __packCson__(object: ReactionEvent): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1400001;
    objectCson["2"] = String(object.id);
    objectCson["5"] = object.spacePtr.toCson();
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.causedByPtr != null) {
      objectCson["15"] = object.causedByPtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    if (object.clientPtr != null) {
      objectCson["23"] = object.clientPtr.toCson();
    }
    if (object.clientNonce != null) {
      objectCson["24"] = String(object.clientNonce);
    }
    objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectCson["26"] = object.clientEpoch;
    objectCson["40"] = object.status;
    objectCson["101"] = object.nodePtr.toCson();
    objectCson["102"] = object.content;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const causedByPtrValue = objectCson["15"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromCson(causedByPtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const clientPtrValue = objectCson["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromCson(clientPtrValue, _session, _graph, _connection)
        : null;
    const clientNonceValue = objectCson["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new ReactionEvent({
      node: _NodeReference.fromCson(objectCson["101"], _session, _graph, _connection),
      content: objectCson["102"],
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectCson["26"]),
      status: Number(objectCson["40"]),
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionEvent {
    return ReactionEvent.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ReactionEventProto {
    return ReactionEvent.__packProto__(this);
  }

  static __packProto__(object: ReactionEvent): ReactionEventProto {
    const objectProto: Partial<ReactionEventProto> = { metatype: 1400001 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    objectProto.status = Number(object.status) as EventStatusProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    objectProto.content = object.content;
    return objectProto as ReactionEventProto;
  }

  static __unpackProto__(
    objectProto: ReactionEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new ReactionEvent({
      node: _NodeReference.fromProto(objectProto.nodePtr!, _session, _graph, _graph, _connection),
      content: objectProto.content,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(objectProto.clientPtr!, _session, _graph, _graph, _connection)
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ReactionEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionEvent {
    return ReactionEvent.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ReactionEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ReactionEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.REACTION_EVENT, ReactionEvent);
/* ==== DESTACK_GENERATED_END:NODE:1400001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1400002 ==== */
/**
 * ReactionAddedEvent
 */
export class ReactionAddedEvent extends ReactionEvent {
  static metatype: NodeType = NodeType.REACTION_ADDED_EVENT;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node: Reaction | NodeReference;
    content: string;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(options);

    // properties

    // identity
    // ... (already set in parent)
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.content === other.content)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.content)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.REACTION_ADDED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `ReactionAddedEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<ReactionAddedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return ReactionAddedEvent.__packCson__(this);
  }

  static __packCson__(object: ReactionAddedEvent): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1400002;
    objectCson["2"] = String(object.id);
    objectCson["5"] = object.spacePtr.toCson();
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.causedByPtr != null) {
      objectCson["15"] = object.causedByPtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    if (object.clientPtr != null) {
      objectCson["23"] = object.clientPtr.toCson();
    }
    if (object.clientNonce != null) {
      objectCson["24"] = String(object.clientNonce);
    }
    objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectCson["26"] = object.clientEpoch;
    objectCson["40"] = object.status;
    objectCson["101"] = object.nodePtr.toCson();
    objectCson["102"] = object.content;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionAddedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const causedByPtrValue = objectCson["15"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromCson(causedByPtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const clientPtrValue = objectCson["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromCson(clientPtrValue, _session, _graph, _connection)
        : null;
    const clientNonceValue = objectCson["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new ReactionAddedEvent({
      node: _NodeReference.fromCson(objectCson["101"], _session, _graph, _connection),
      content: objectCson["102"],
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectCson["26"]),
      status: Number(objectCson["40"]),
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionAddedEvent {
    return ReactionAddedEvent.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ReactionAddedEventProto {
    return ReactionAddedEvent.__packProto__(this);
  }

  static __packProto__(object: ReactionAddedEvent): ReactionAddedEventProto {
    const objectProto: Partial<ReactionAddedEventProto> = { metatype: 1400002 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    objectProto.status = Number(object.status) as EventStatusProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    objectProto.content = object.content;
    return objectProto as ReactionAddedEventProto;
  }

  static __unpackProto__(
    objectProto: ReactionAddedEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionAddedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new ReactionAddedEvent({
      node: _NodeReference.fromProto(objectProto.nodePtr!, _session, _graph, _graph, _connection),
      content: objectProto.content,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(objectProto.clientPtr!, _session, _graph, _graph, _connection)
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ReactionAddedEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionAddedEvent {
    return ReactionAddedEvent.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ReactionAddedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ReactionAddedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.REACTION_ADDED_EVENT, ReactionAddedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1400002 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1400003 ==== */
/**
 * ReactionRemovedEvent
 */
export class ReactionRemovedEvent extends ReactionEvent {
  static metatype: NodeType = NodeType.REACTION_REMOVED_EVENT;

  constructor(options: {
    id?: string;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Event | NodeReference | null;
    causedBy?: Event | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    client?: Client | NodeReference | null;
    clientNonce?: string | null;
    clientCreatedAt?: Temporal.ZonedDateTime;
    clientEpoch?: number;
    status?: EventStatus;
    node: Reaction | NodeReference;
    content: string;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(options);

    // properties

    // identity
    // ... (already set in parent)
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.content === other.content)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.branchPtr.id === other.branchPtr.id)) {
      return false;
    }
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this.causedByPtr?.id === other.causedByPtr?.id)) {
      return false;
    }
    if (!(this.clientPtr?.id === other.clientPtr?.id)) {
      return false;
    }
    if (!(this.clientNonce === other.clientNonce)) {
      return false;
    }
    if (!(this.clientCreatedAt === other.clientCreatedAt)) {
      return false;
    }
    if (!(this.clientEpoch === other.clientEpoch)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.content)) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.branchPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
    }
    if (this.causedByPtr != null) {
      h = (h * 31 + hashString(this.causedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.clientPtr != null) {
      h = (h * 31 + hashString(this.clientPtr.id)) & 0xffffffff;
    }
    if (this.clientNonce != null) {
      h = (h * 31 + hashString(this.clientNonce.toString())) & 0xffffffff;
    }
    h =
      (h * 31 + hashString(this.clientCreatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashInt(this.clientEpoch)) & 0xffffffff;
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.REACTION_REMOVED_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _graph: this._graph,
    });
  }

  get _pathKey(): string {
    return `ReactionRemovedEvent[id=${this.id}]`;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
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
    propertyReprs.push(`createdEpoch=${this.createdEpoch}`);
    propertyReprs.push(`status=${EventStatus[this.status]}`);
    return `<ReactionRemovedEvent "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return ReactionRemovedEvent.__packCson__(this);
  }

  static __packCson__(object: ReactionRemovedEvent): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1400003;
    objectCson["2"] = String(object.id);
    objectCson["5"] = object.spacePtr.toCson();
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.causedByPtr != null) {
      objectCson["15"] = object.causedByPtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    if (object.clientPtr != null) {
      objectCson["23"] = object.clientPtr.toCson();
    }
    if (object.clientNonce != null) {
      objectCson["24"] = String(object.clientNonce);
    }
    objectCson["25"] = object.clientCreatedAt.toString({ timeZoneName: "never" });
    objectCson["26"] = object.clientEpoch;
    objectCson["40"] = object.status;
    objectCson["101"] = object.nodePtr.toCson();
    objectCson["102"] = object.content;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionRemovedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const causedByPtrValue = objectCson["15"];
    const unpackedCausedByPtr =
      causedByPtrValue != undefined
        ? _NodeReference.fromCson(causedByPtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const clientPtrValue = objectCson["23"];
    const unpackedClientPtr =
      clientPtrValue != undefined
        ? _NodeReference.fromCson(clientPtrValue, _session, _graph, _connection)
        : null;
    const clientNonceValue = objectCson["24"];
    const unpackedClientNonce = clientNonceValue != undefined ? String(clientNonceValue) : null;
    return new ReactionRemovedEvent({
      node: _NodeReference.fromCson(objectCson["101"], _session, _graph, _connection),
      content: objectCson["102"],
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
      precededBy: unpackedPrecededByPtr,
      causedBy: unpackedCausedByPtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      client: unpackedClientPtr,
      clientNonce: unpackedClientNonce,
      clientCreatedAt: Temporal.Instant.from(objectCson["25"]).toZonedDateTimeISO("UTC"),
      clientEpoch: Number(objectCson["26"]),
      status: Number(objectCson["40"]),
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): ReactionRemovedEvent {
    return ReactionRemovedEvent.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): ReactionRemovedEventProto {
    return ReactionRemovedEvent.__packProto__(this);
  }

  static __packProto__(object: ReactionRemovedEvent): ReactionRemovedEventProto {
    const objectProto: Partial<ReactionRemovedEventProto> = { metatype: 1400003 };
    objectProto.id = String(object.id);
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.causedByPtr != null) {
      objectProto.causedByPtr = object.causedByPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.clientPtr != null) {
      objectProto.clientPtr = object.clientPtr.toProto();
    }
    if (object.clientNonce != null) {
      objectProto.clientNonce = String(object.clientNonce);
    }
    objectProto.clientCreatedAt = packProtoTimestamp(object.clientCreatedAt);
    objectProto.clientEpoch = object.clientEpoch;
    objectProto.status = Number(object.status) as EventStatusProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    objectProto.content = object.content;
    return objectProto as ReactionRemovedEventProto;
  }

  static __unpackProto__(
    objectProto: ReactionRemovedEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionRemovedEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new ReactionRemovedEvent({
      node: _NodeReference.fromProto(objectProto.nodePtr!, _session, _graph, _graph, _connection),
      content: objectProto.content,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      causedBy:
        objectProto.causedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.causedByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      client:
        objectProto.clientPtr != undefined
          ? _NodeReference.fromProto(objectProto.clientPtr!, _session, _graph, _graph, _connection)
          : null,
      clientNonce: objectProto.clientNonce != undefined ? String(objectProto.clientNonce) : null,
      clientCreatedAt: unpackProtoTimestamp(objectProto.clientCreatedAt!),
      clientEpoch: Number(objectProto.clientEpoch),
      status: Number(objectProto.status) as EventStatus,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ReactionRemovedEventProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): ReactionRemovedEvent {
    return ReactionRemovedEvent.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ReactionRemovedEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ReactionRemovedEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.REACTION_REMOVED_EVENT, ReactionRemovedEvent);
/* ==== DESTACK_GENERATED_END:NODE:1400003 ==== */
