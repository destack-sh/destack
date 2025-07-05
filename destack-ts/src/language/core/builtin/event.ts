import {
  packProtoJson,
  packProtoTimestamp,
  unpackProtoJson,
  unpackProtoTimestamp,
} from "@destack/grpc";
import { NodeType, StructType } from "@destack/language/core/builtin/common";
import type { Snapshot } from "@destack/language/core/builtin/entity";
import { Entity, Materialization, Metric } from "@destack/language/core/builtin/entity";
import { Node } from "@destack/language/core/builtin/node";
import type {
  NodeDefinitionReference,
  NodeReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import type {
  IsCustomizable,
  IsExtensible,
  IsSourceable,
  IsSpatial,
  IsSubject,
} from "@destack/language/core/builtin/trait";
import type { Edit, Origin } from "@destack/language/core/common/edit";
import { ChangeDebounce, EditOperation, EditType } from "@destack/language/core/common/edit";
import type { Icon } from "@destack/language/core/common/icon";
import type {
  Aggregation,
  Condition,
  Expression,
  Join,
  Query,
  Select,
  Sort,
} from "@destack/language/core/common/query";
import { QueryType } from "@destack/language/core/common/query";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import {
  ChangeDebounceProto,
  ChangeEventProto,
  CustomEventDefinitionProto,
  EditEventProto,
  EditOperationProto,
  EditTypeProto,
  MaterializationProto,
  QueryEventProto,
  QueryTypeProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:3 ==== */
/**
 * An Event is an immutable datum of something happening to an Entity.
 */
export abstract class Event extends Node implements IsSpatial {
  static metatype: NodeType = NodeType.EVENT;

  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.EVENT, Event);
/* ==== DESTACK_GENERATED_END:NODE:3 ==== */

/* ==== DESTACK_GENERATED_START:NODE:102 ==== */
/**
 * A CustomEventDefinition defines a kind of CustomEvent with custom Properties.
 */
export class CustomEventDefinition
  extends Entity
  implements IsSpatial, IsSourceable, IsCustomizable
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
  get predecessor(): CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEventDefinition | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomEventDefinition | null;
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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): Map<string, Value> {
    return this.#customValues;
  }
  set customValues(value: Map<string, Value>) {
    const oldValue = this.#customValues;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["customValues"] === undefined) {
      this._dirty["customValues"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#customValues = value;
  }
  #customValues: Map<string, Value>;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * CustomEventDefinition.baseType
   */
  get baseType(): NodeDefinitionReference | null {
    return this.#baseType;
  }
  set baseType(value: NodeDefinitionReference | null) {
    const oldValue = this.#baseType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["baseType"] === undefined) {
      this._dirty["baseType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#baseType = value;
  }
  #baseType: NodeDefinitionReference | null;

  /**
   * CustomEventDefinition.baseTraits
   */
  get baseTraits(): Array<NodeDefinitionReference> {
    return this.#baseTraits;
  }
  set baseTraits(value: Array<NodeDefinitionReference>) {
    const oldValue = this.#baseTraits;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["baseTraits"] === undefined) {
      this._dirty["baseTraits"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#baseTraits = value;
  }
  #baseTraits: Array<NodeDefinitionReference>;

  /**
   * CustomEventDefinition.isAbstract
   */
  get isAbstract(): boolean {
    return this.#isAbstract;
  }
  set isAbstract(value: boolean) {
    const oldValue = this.#isAbstract;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isAbstract"] === undefined) {
      this._dirty["isAbstract"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#isAbstract = value;
  }
  #isAbstract: boolean;

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

  /**
   * CustomEventDefinition.name
   */
  get name(): string {
    return this.#name;
  }
  set name(value: string) {
    const oldValue = this.#name;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["name"] === undefined) {
      this._dirty["name"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#name = value;
  }
  #name: string;

  /**
   * CustomEventDefinition.icon
   */
  get icon(): Icon | null {
    return this.#icon;
  }
  set icon(value: Icon | null) {
    const oldValue = this.#icon;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["icon"] === undefined) {
      this._dirty["icon"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#icon = value;
  }
  #icon: Icon | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: CustomEventDefinition | NodeReference | null;
    template?: CustomEventDefinition | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    customValues?: Map<string, Value>;
    orderKey?: string;
    baseType?: NodeDefinitionReference | null;
    baseTraits?: Array<NodeDefinitionReference>;
    isAbstract?: boolean;
    source?: Script | NodeReference | null;
    name: string;
    icon?: Icon | null;
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
      throw new Error(`CustomEventDefinition.materialization is required`);
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
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.#customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomEventDefinition.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _baseType = options.baseType ?? null;
    this.#baseType = _baseType;
    let _baseTraits = options.baseTraits ?? null;
    if (_baseTraits === null) {
      _baseTraits = [];
    }
    this.#baseTraits = _baseTraits;
    let _isAbstract = options.isAbstract ?? null;
    if (_isAbstract === null) {
      _isAbstract = false;
    }
    if (_isAbstract === null) {
      throw new Error(`CustomEventDefinition.isAbstract is required`);
    }
    this.#isAbstract = _isAbstract;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomEventDefinition.name is required`);
    }
    this.#name = _name;
    let _icon = options.icon ?? null;
    this.#icon = _icon;

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
          `CustomEventDefinition.createdAt and CustomEventDefinition.updatedAt are required for existing Nodes`,
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
    if (
      (this.#baseType == null) !== (other.#baseType == null) ||
      (this.#baseType != null && !this.#baseType.equals(other.#baseType))
    ) {
      return false;
    }
    if (this.#baseTraits.length !== other.#baseTraits.length) {
      return false;
    }
    for (let i = 0; i < this.#baseTraits.length; i++) {
      if (!this.#baseTraits[i].equals(other.#baseTraits[i])) {
        return false;
      }
    }
    if (!(this.#isAbstract === other.#isAbstract)) {
      return false;
    }
    if (!(this.#name === other.#name)) {
      return false;
    }
    if (
      (this.#icon == null) !== (other.#icon == null) ||
      (this.#icon != null && !this.#icon.equals(other.#icon))
    ) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (Object.keys(this.#customValues).length !== Object.keys(other.#customValues).length) {
      return false;
    }
    for (const key in this.#customValues) {
      if (!(key in other.#customValues)) {
        return false;
      }
      if (!this.#customValues.get(key)!.equals(other.#customValues.get(key)!)) {
        return false;
      }
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
    if (this.#baseType !== null) {
      h = (h * 31 + this.#baseType.hash()) & 0xffffffff;
    }
    if (this.#baseTraits && this.#baseTraits.length > 0) {
      for (const _item of this.#baseTraits) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.#isAbstract)) & 0xffffffff;
    h = (h * 31 + hashString(this.#name)) & 0xffffffff;
    if (this.#icon !== null) {
      h = (h * 31 + this.#icon.hash()) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this.#customValues && Object.keys(this.#customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.#customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
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
      type: NodeType.CUSTOM_EVENT_DEFINITION,
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
    objectValue["1"] = 102;
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
    if (object.#customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.#customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["27"] = object.orderKey;
    if (object.#baseType != null) {
      objectValue["40"] = object.#baseType.toValue();
    }
    if (object.#baseTraits.length > 0) {
      const packedBaseTraits: any[] = [];
      for (const item of object.#baseTraits) {
        packedBaseTraits.push(item.toValue());
      }
      objectValue["41"] = packedBaseTraits;
    }
    objectValue["45"] = object.#isAbstract;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    objectValue["101"] = object.#name;
    if (object.#icon != null) {
      objectValue["102"] = object.#icon.toValue();
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const baseTypeValue = objectValue["40"];
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
    const unpackedBaseTraits: any[] = [];
    if (objectValue["41"] != undefined) {
      for (const item of objectValue["41"]) {
        unpackedBaseTraits.push(
          _NodeDefinitionReference.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new CustomEventDefinition({
      baseType: unpackedBaseType,
      baseTraits: unpackedBaseTraits,
      isAbstract: objectValue["45"],
      name: objectValue["101"],
      icon: unpackedIcon,
      space: unpackedSpacePtr,
      source: unpackedSourcePtr,
      customValues: unpackedCustomValues,
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
      parent: unpackedParentPtr,
      orderKey: objectValue["27"],
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
    const objectProto: Partial<CustomEventDefinitionProto> = { metatype: 102 };
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
    if (object.#customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.#customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    if (object.#baseType != null) {
      objectProto.baseType = object.#baseType.toProto();
    }
    if (object.#baseTraits) {
      const packedBaseTraits: any[] = [];
      for (const item of object.#baseTraits) {
        packedBaseTraits.push(item.toProto());
      }
      objectProto.baseTraits = packedBaseTraits;
    }
    objectProto.isAbstract = object.#isAbstract;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    objectProto.name = object.#name;
    if (object.#icon != null) {
      objectProto.icon = object.#icon.toProto();
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
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedBaseTraits: any[] = [];
    if (objectProto.baseTraits) {
      for (const item of objectProto.baseTraits) {
        unpackedBaseTraits.push(
          _NodeDefinitionReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomEventDefinition({
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
      baseTraits: unpackedBaseTraits,
      isAbstract: objectProto.isAbstract,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
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
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:102 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2000 ==== */
/**
 * A generic Signal of a CustomEventDefinition.
 * More specific base Event types will be instanced of that base type instead.
 */
export abstract class Signal extends Event implements IsExtensible {
  static metatype: NodeType = NodeType.SIGNAL;

  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get definition(): CustomEventDefinition | null;
  declare readonly definitionPtr: NodeReference;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  declare readonly baseType: NodeDefinitionReference | null;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  declare readonly customValues: Map<string, Value>;

  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SIGNAL, Signal);
/* ==== DESTACK_GENERATED_END:NODE:2000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2001 ==== */
/**
 * A recorded Edit of an Entity.
 */
export class EditEvent extends Event {
  static metatype: NodeType = NodeType.EDIT_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
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
   * The Snapshot this Event originated from.
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
  readonly type: EditType;

  /**
   * EditEvent.node
   */
  get node(): Entity | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference;

  /**
   * EditEvent.operation
   */
  readonly operation: EditOperation | null;

  /**
   * EditEvent.attribute
   */
  readonly attribute: PropertyReference | null;

  /**
   * EditEvent.key
   */
  readonly key: Value | null;

  /**
   * EditEvent.keyUnpacked
   */
  readonly keyUnpacked: any | null;

  /**
   * EditEvent.value
   */
  readonly value: Value | null;

  /**
   * The inverse Edit *if* it cannot be unambiguously derived from the Edit).
   */
  readonly undo: Edit | null;

  /**
   * EditEvent.ancestorsIds
   */
  readonly ancestorsIds: Array<string>;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    type: EditType;
    node: Entity | NodeReference;
    operation?: EditOperation | null;
    attribute?: PropertyReference | null;
    key?: Value | null;
    keyUnpacked?: any | null;
    value?: Value | null;
    undo?: Edit | null;
    ancestorsIds?: Array<string>;
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
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EditEvent.type is required`);
    }
    this.type = _type;
    let _node = options.node;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    if (_node === null) {
      throw new Error(`EditEvent.node is required`);
    }
    this.nodePtr = _node;
    let _operation = options.operation ?? null;
    this.operation = _operation;
    let _attribute = options.attribute ?? null;
    this.attribute = _attribute;
    let _key = options.key ?? null;
    this.key = _key;
    let _keyUnpacked = options.keyUnpacked ?? null;
    this.keyUnpacked = _keyUnpacked;
    let _value = options.value ?? null;
    this.value = _value;
    let _undo = options.undo ?? null;
    this.undo = _undo;
    let _ancestorsIds = options.ancestorsIds ?? null;
    if (_ancestorsIds === null) {
      _ancestorsIds = [];
    }
    this.ancestorsIds = _ancestorsIds;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`EditEvent.createdAt is required for existing Events`);
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
    if (!(this.nodePtr.id === other.nodePtr.id)) {
      return false;
    }
    if (!(this.operation === other.operation)) {
      return false;
    }
    if (
      (this.attribute == null) !== (other.attribute == null) ||
      (this.attribute != null && !this.attribute.equals(other.attribute))
    ) {
      return false;
    }
    if (
      (this.key == null) !== (other.key == null) ||
      (this.key != null && !this.key.equals(other.key))
    ) {
      return false;
    }
    if (!(this.keyUnpacked === other.keyUnpacked)) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (
      (this.undo == null) !== (other.undo == null) ||
      (this.undo != null && !this.undo.equals(other.undo))
    ) {
      return false;
    }
    if (this.ancestorsIds.length !== other.ancestorsIds.length) {
      return false;
    }
    for (let i = 0; i < this.ancestorsIds.length; i++) {
      if (!(this.ancestorsIds[i] === other.ancestorsIds[i])) {
        return false;
      }
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
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
    h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    if (this.operation !== null) {
      h = (h * 31 + this.operation) & 0xffffffff;
    }
    if (this.attribute !== null) {
      h = (h * 31 + this.attribute.hash()) & 0xffffffff;
    }
    if (this.key !== null) {
      h = (h * 31 + this.key.hash()) & 0xffffffff;
    }
    if (this.keyUnpacked !== null) {
      h = (h * 31 + hashString(JSON.stringify(this.keyUnpacked))) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.undo !== null) {
      h = (h * 31 + this.undo.hash()) & 0xffffffff;
    }
    if (this.ancestorsIds && this.ancestorsIds.length > 0) {
      for (const _item of this.ancestorsIds) {
        h = (h * 31 + hashString(_item.toString())) & 0xffffffff;
      }
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
      type: NodeType.EDIT_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
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
    propertyReprs.push(`node=${this.node?.repr()}`);
    if (this.operation !== null) {
      propertyReprs.push(`operation=${EditOperation[this.operation]}`);
    }
    if (this.attribute !== null) {
      propertyReprs.push(`attribute=${this.attribute.repr()}`);
    }
    if (this.key !== null) {
      propertyReprs.push(`key=${this.key.repr()}`);
    }
    if (this.keyUnpacked !== null) {
      propertyReprs.push(`keyUnpacked=${this.keyUnpacked}`);
    }
    return `<EditEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return EditEvent.__packValue__(this);
  }

  static __packValue__(object: EditEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2001;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["100"] = object.type;
    objectValue["101"] = object.nodePtr.toValue();
    if (object.operation != null) {
      objectValue["102"] = object.operation;
    }
    if (object.attribute != null) {
      objectValue["103"] = object.attribute.toValue();
    }
    if (object.key != null) {
      objectValue["104"] = object.key.toValue();
    }
    if (object.keyUnpacked != null) {
      objectValue["105"] = object.keyUnpacked;
    }
    if (object.value != null) {
      objectValue["110"] = object.value.toValue();
    }
    if (object.undo != null) {
      objectValue["120"] = object.undo.toValue();
    }
    if (object.ancestorsIds.length > 0) {
      const packedAncestorsIds: any[] = [];
      for (const item of object.ancestorsIds) {
        packedAncestorsIds.push(String(item));
      }
      objectValue["122"] = packedAncestorsIds;
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
    const _Edit = STRUCT_CLASS_BY_TYPE[StructType.EDIT] as typeof Edit;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const operationValue = objectValue["102"];
    const unpackedOperation = operationValue != undefined ? Number(operationValue) : null;
    const attributeValue = objectValue["103"];
    const unpackedAttribute =
      attributeValue != undefined
        ? _PropertyReference.fromValue(attributeValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["104"];
    const unpackedKey =
      keyValue != undefined
        ? _Value.fromValue(keyValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyUnpackedValue = objectValue["105"];
    const unpackedKeyUnpacked = keyUnpackedValue != undefined ? keyUnpackedValue : null;
    const valueValue = objectValue["110"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const undoValue = objectValue["120"];
    const unpackedUndo =
      undoValue != undefined
        ? _Edit.fromValue(undoValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedAncestorsIds: any[] = [];
    if (objectValue["122"] != undefined) {
      for (const item of objectValue["122"]) {
        unpackedAncestorsIds.push(String(item));
      }
    }
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
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new EditEvent({
      type: Number(objectValue["100"]),
      node: _NodeReference.fromValue(
        objectValue["101"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      operation: unpackedOperation,
      attribute: unpackedAttribute,
      key: unpackedKey,
      keyUnpacked: unpackedKeyUnpacked,
      value: unpackedValue,
      undo: unpackedUndo,
      ancestorsIds: unpackedAncestorsIds,
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      space: unpackedSpacePtr,
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
  ): EditEvent {
    return EditEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EditEventProto {
    return EditEvent.__packProto__(this);
  }

  static __packProto__(object: EditEvent): EditEventProto {
    const objectProto: Partial<EditEventProto> = { metatype: 2001 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.type = Number(object.type) as EditTypeProto;
    objectProto.nodePtr = object.nodePtr.toProto();
    if (object.operation != null) {
      objectProto.operation = Number(object.operation) as EditOperationProto;
    }
    if (object.attribute != null) {
      objectProto.attribute = object.attribute.toProto();
    }
    if (object.key != null) {
      objectProto.key = object.key.toProto();
    }
    if (object.keyUnpacked != null) {
      objectProto.keyUnpacked = packProtoJson(object.keyUnpacked);
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.undo != null) {
      objectProto.undo = object.undo.toProto();
    }
    if (object.ancestorsIds) {
      const packedAncestorsIds: any[] = [];
      for (const item of object.ancestorsIds) {
        packedAncestorsIds.push(String(item));
      }
      objectProto.ancestorsIds = packedAncestorsIds;
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
    const _Edit = STRUCT_CLASS_BY_TYPE[StructType.EDIT] as typeof Edit;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedAncestorsIds: any[] = [];
    if (objectProto.ancestorsIds) {
      for (const item of objectProto.ancestorsIds) {
        unpackedAncestorsIds.push(String(item));
      }
    }
    return new EditEvent({
      type: Number(objectProto.type) as EditType,
      node: _NodeReference.fromProto(
        objectProto.nodePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      operation:
        objectProto.operation != undefined
          ? (Number(objectProto.operation) as EditOperation)
          : null,
      attribute:
        objectProto.attribute != undefined
          ? _PropertyReference.fromProto(
              objectProto.attribute!,
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
      keyUnpacked:
        objectProto.keyUnpacked != undefined ? unpackProtoJson(objectProto.keyUnpacked!) : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      undo:
        objectProto.undo != undefined
          ? _Edit.fromProto(objectProto.undo!, _session, _supergraph, _graph, _connection)
          : null,
      ancestorsIds: unpackedAncestorsIds,
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
/* ==== DESTACK_GENERATED_END:NODE:2001 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2002 ==== */
/**
 * A recorded Change.
 */
export class ChangeEvent extends Event {
  static metatype: NodeType = NodeType.CHANGE_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
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
   * The Snapshot this Event originated from.
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
   * The Node this Event is about.
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * ChangeEvent.name
   */
  readonly name: string | null;

  /**
   * ChangeEvent.origin
   */
  readonly origin: Origin | null;

  /**
   * ChangeEvent.debounce
   */
  readonly debounce: ChangeDebounce | null;

  /**
   * ChangeEvent.editsIds
   */
  readonly editsIds: Array<string>;

  /**
   * ChangeEvent.nodesIds
   */
  readonly nodesIds: Array<string>;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    node?: Node | NodeReference | null;
    name?: string | null;
    origin?: Origin | null;
    debounce?: ChangeDebounce | null;
    editsIds?: Array<string>;
    nodesIds?: Array<string>;
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
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _name = options.name ?? null;
    this.name = _name;
    let _origin = options.origin ?? null;
    this.origin = _origin;
    let _debounce = options.debounce ?? null;
    this.debounce = _debounce;
    let _editsIds = options.editsIds ?? null;
    if (_editsIds === null) {
      _editsIds = [];
    }
    this.editsIds = _editsIds;
    let _nodesIds = options.nodesIds ?? null;
    if (_nodesIds === null) {
      _nodesIds = [];
    }
    this.nodesIds = _nodesIds;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`ChangeEvent.createdAt is required for existing Events`);
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
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.origin == null) !== (other.origin == null) ||
      (this.origin != null && !this.origin.equals(other.origin))
    ) {
      return false;
    }
    if (!(this.debounce === other.debounce)) {
      return false;
    }
    if (this.editsIds.length !== other.editsIds.length) {
      return false;
    }
    for (let i = 0; i < this.editsIds.length; i++) {
      if (!(this.editsIds[i] === other.editsIds[i])) {
        return false;
      }
    }
    if (this.nodesIds.length !== other.nodesIds.length) {
      return false;
    }
    for (let i = 0; i < this.nodesIds.length; i++) {
      if (!(this.nodesIds[i] === other.nodesIds[i])) {
        return false;
      }
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    if (this.name !== null) {
      h = (h * 31 + hashString(this.name)) & 0xffffffff;
    }
    if (this.origin !== null) {
      h = (h * 31 + this.origin.hash()) & 0xffffffff;
    }
    if (this.debounce !== null) {
      h = (h * 31 + this.debounce) & 0xffffffff;
    }
    if (this.editsIds && this.editsIds.length > 0) {
      for (const _item of this.editsIds) {
        h = (h * 31 + hashString(_item.toString())) & 0xffffffff;
      }
    }
    if (this.nodesIds && this.nodesIds.length > 0) {
      for (const _item of this.nodesIds) {
        h = (h * 31 + hashString(_item.toString())) & 0xffffffff;
      }
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
      type: NodeType.CHANGE_EVENT,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name ?? "ChangeEvent[id={this.id}]";
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
    if (this.name !== null) {
      propertyReprs.push(`name=${this.name}`);
    }
    if (propertyReprs.length > 0) {
      return `<ChangeEvent '${this.path}' ${propertyReprs.join(" ")}>`;
    } else {
      return `<ChangeEvent '${this.path}'>`;
    }
  }

  toValue(): { [key: string]: any } {
    return ChangeEvent.__packValue__(this);
  }

  static __packValue__(object: ChangeEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2002;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    if (object.nodePtr != null) {
      objectValue["101"] = object.nodePtr.toValue();
    }
    if (object.name != null) {
      objectValue["102"] = object.name;
    }
    if (object.origin != null) {
      objectValue["103"] = object.origin.toValue();
    }
    if (object.debounce != null) {
      objectValue["104"] = object.debounce;
    }
    if (object.editsIds.length > 0) {
      const packedEditsIds: any[] = [];
      for (const item of object.editsIds) {
        packedEditsIds.push(String(item));
      }
      objectValue["130"] = packedEditsIds;
    }
    if (object.nodesIds.length > 0) {
      const packedNodesIds: any[] = [];
      for (const item of object.nodesIds) {
        packedNodesIds.push(String(item));
      }
      objectValue["131"] = packedNodesIds;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Origin = STRUCT_CLASS_BY_TYPE[StructType.ORIGIN] as typeof Origin;
    const nameValue = objectValue["102"];
    const unpackedName = nameValue != undefined ? nameValue : null;
    const originValue = objectValue["103"];
    const unpackedOrigin =
      originValue != undefined
        ? _Origin.fromValue(originValue, _session, _supergraph, _graph, _connection)
        : null;
    const debounceValue = objectValue["104"];
    const unpackedDebounce = debounceValue != undefined ? Number(debounceValue) : null;
    const unpackedEditsIds: any[] = [];
    if (objectValue["130"] != undefined) {
      for (const item of objectValue["130"]) {
        unpackedEditsIds.push(String(item));
      }
    }
    const unpackedNodesIds: any[] = [];
    if (objectValue["131"] != undefined) {
      for (const item of objectValue["131"]) {
        unpackedNodesIds.push(String(item));
      }
    }
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
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ChangeEvent({
      name: unpackedName,
      origin: unpackedOrigin,
      debounce: unpackedDebounce,
      editsIds: unpackedEditsIds,
      nodesIds: unpackedNodesIds,
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      node: unpackedNodePtr,
      space: unpackedSpacePtr,
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
  ): ChangeEvent {
    return ChangeEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ChangeEventProto {
    return ChangeEvent.__packProto__(this);
  }

  static __packProto__(object: ChangeEvent): ChangeEventProto {
    const objectProto: Partial<ChangeEventProto> = { metatype: 2002 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    if (object.name != null) {
      objectProto.name = object.name;
    }
    if (object.origin != null) {
      objectProto.origin = object.origin.toProto();
    }
    if (object.debounce != null) {
      objectProto.debounce = Number(object.debounce) as ChangeDebounceProto;
    }
    if (object.editsIds) {
      const packedEditsIds: any[] = [];
      for (const item of object.editsIds) {
        packedEditsIds.push(String(item));
      }
      objectProto.editsIds = packedEditsIds;
    }
    if (object.nodesIds) {
      const packedNodesIds: any[] = [];
      for (const item of object.nodesIds) {
        packedNodesIds.push(String(item));
      }
      objectProto.nodesIds = packedNodesIds;
    }
    return objectProto as ChangeEventProto;
  }

  static __unpackProto__(
    objectProto: ChangeEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeEvent {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Origin = STRUCT_CLASS_BY_TYPE[StructType.ORIGIN] as typeof Origin;
    const unpackedEditsIds: any[] = [];
    if (objectProto.editsIds) {
      for (const item of objectProto.editsIds) {
        unpackedEditsIds.push(String(item));
      }
    }
    const unpackedNodesIds: any[] = [];
    if (objectProto.nodesIds) {
      for (const item of objectProto.nodesIds) {
        unpackedNodesIds.push(String(item));
      }
    }
    return new ChangeEvent({
      name: objectProto.name != undefined ? objectProto.name : null,
      origin:
        objectProto.origin != undefined
          ? _Origin.fromProto(objectProto.origin!, _session, _supergraph, _graph, _connection)
          : null,
      debounce:
        objectProto.debounce != undefined ? (Number(objectProto.debounce) as ChangeDebounce) : null,
      editsIds: unpackedEditsIds,
      nodesIds: unpackedNodesIds,
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
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: ChangeEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ChangeEvent {
    return ChangeEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): ChangeEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ChangeEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CHANGE_EVENT, ChangeEvent);
/* ==== DESTACK_GENERATED_END:NODE:2002 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2003 ==== */
/**
 * A recorded Query.
 */
export class QueryEvent extends Event {
  static metatype: NodeType = NodeType.QUERY_EVENT;

  /**
   * Event.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
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
   * The Snapshot this Event originated from.
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
   * QueryEvent.type
   */
  readonly type: QueryType;

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
  readonly nodePtr: NodeReference | null;

  /**
   * Name for this subquery. Must be unique within the parent Query.
   */
  readonly name: string;

  /**
   * QueryEvent.definition
   */
  readonly definition: NodeDefinitionReference;

  /**
   * Relative to parent Query.
   */
  readonly join: Join | null;

  /**
   * QueryEvent.select
   */
  readonly select: Select | null;

  /**
   * QueryEvent.subqueries
   */
  readonly subqueries: Array<Query>;

  /**
   * QueryEvent.where
   */
  readonly where: Condition | null;

  /**
   * QueryEvent.having
   */
  readonly having: Condition | null;

  /**
   * QueryEvent.groupBy
   */
  readonly groupBy: Array<Expression>;

  /**
   * QueryEvent.aggregation
   */
  readonly aggregation: Aggregation | null;

  /**
   * QueryEvent.sort
   */
  readonly sort: Array<Sort>;

  /**
   * QueryEvent.limit
   */
  readonly limit: number | null;

  /**
   * QueryEvent.offset
   */
  readonly offset: number | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    snapshot?: Snapshot | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    type: QueryType;
    node?: Node | NodeReference | null;
    name: string;
    definition: NodeDefinitionReference;
    join?: Join | null;
    select?: Select | null;
    subqueries?: Array<Query>;
    where?: Condition | null;
    having?: Condition | null;
    groupBy?: Array<Expression>;
    aggregation?: Aggregation | null;
    sort?: Array<Sort>;
    limit?: number | null;
    offset?: number | null;
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
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`QueryEvent.type is required`);
    }
    this.type = _type;
    let _node = options.node ?? null;
    if (_node != null && _node.metatype != StructType.NODE_REFERENCE) {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`QueryEvent.name is required`);
    }
    this.name = _name;
    let _definition = options.definition;
    if (_definition === null) {
      throw new Error(`QueryEvent.definition is required`);
    }
    this.definition = _definition;
    let _join = options.join ?? null;
    this.join = _join;
    let _select = options.select ?? null;
    this.select = _select;
    let _subqueries = options.subqueries ?? null;
    if (_subqueries === null) {
      _subqueries = [];
    }
    this.subqueries = _subqueries;
    let _where = options.where ?? null;
    this.where = _where;
    let _having = options.having ?? null;
    this.having = _having;
    let _groupBy = options.groupBy ?? null;
    if (_groupBy === null) {
      _groupBy = [];
    }
    this.groupBy = _groupBy;
    let _aggregation = options.aggregation ?? null;
    this.aggregation = _aggregation;
    let _sort = options.sort ?? null;
    if (_sort === null) {
      _sort = [];
    }
    this.sort = _sort;
    let _limit = options.limit ?? null;
    this.limit = _limit;
    let _offset = options.offset ?? null;
    this.offset = _offset;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
    } else {
      if (options.createdAt == null) {
        throw new Error(`QueryEvent.createdAt is required for existing Events`);
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
    if (!(this.name === other.name)) {
      return false;
    }
    if (!this.definition.equals(other.definition)) {
      return false;
    }
    if (
      (this.join == null) !== (other.join == null) ||
      (this.join != null && !this.join.equals(other.join))
    ) {
      return false;
    }
    if (
      (this.select == null) !== (other.select == null) ||
      (this.select != null && !this.select.equals(other.select))
    ) {
      return false;
    }
    if (this.subqueries.length !== other.subqueries.length) {
      return false;
    }
    for (let i = 0; i < this.subqueries.length; i++) {
      if (!this.subqueries[i].equals(other.subqueries[i])) {
        return false;
      }
    }
    if (
      (this.where == null) !== (other.where == null) ||
      (this.where != null && !this.where.equals(other.where))
    ) {
      return false;
    }
    if (
      (this.having == null) !== (other.having == null) ||
      (this.having != null && !this.having.equals(other.having))
    ) {
      return false;
    }
    if (this.groupBy.length !== other.groupBy.length) {
      return false;
    }
    for (let i = 0; i < this.groupBy.length; i++) {
      if (!this.groupBy[i].equals(other.groupBy[i])) {
        return false;
      }
    }
    if (
      (this.aggregation == null) !== (other.aggregation == null) ||
      (this.aggregation != null && !this.aggregation.equals(other.aggregation))
    ) {
      return false;
    }
    if (this.sort.length !== other.sort.length) {
      return false;
    }
    for (let i = 0; i < this.sort.length; i++) {
      if (!this.sort[i].equals(other.sort[i])) {
        return false;
      }
    }
    if (!(this.limit === other.limit)) {
      return false;
    }
    if (!(this.offset === other.offset)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
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
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.definition.hash()) & 0xffffffff;
    if (this.join !== null) {
      h = (h * 31 + this.join.hash()) & 0xffffffff;
    }
    if (this.select !== null) {
      h = (h * 31 + this.select.hash()) & 0xffffffff;
    }
    if (this.subqueries && this.subqueries.length > 0) {
      for (const _item of this.subqueries) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.where !== null) {
      h = (h * 31 + this.where.hash()) & 0xffffffff;
    }
    if (this.having !== null) {
      h = (h * 31 + this.having.hash()) & 0xffffffff;
    }
    if (this.groupBy && this.groupBy.length > 0) {
      for (const _item of this.groupBy) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.aggregation !== null) {
      h = (h * 31 + this.aggregation.hash()) & 0xffffffff;
    }
    if (this.sort && this.sort.length > 0) {
      for (const _item of this.sort) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.limit !== null) {
      h = (h * 31 + hashInt(this.limit)) & 0xffffffff;
    }
    if (this.offset !== null) {
      h = (h * 31 + hashInt(this.offset)) & 0xffffffff;
    }
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.snapshotPtr !== null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    if (this.nodePtr !== null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
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
      type: NodeType.QUERY_EVENT,
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
    propertyReprs.push(`type=${QueryType[this.type]}`);
    propertyReprs.push(`name=${this.name}`);
    propertyReprs.push(`definition=${this.definition.repr()}`);
    if (this.join !== null) {
      propertyReprs.push(`join=${this.join.repr()}`);
    }
    if (this.select !== null) {
      propertyReprs.push(`select=${this.select.repr()}`);
    }
    if (this.subqueries.length > 0) {
      propertyReprs.push(`subqueries=${this.subqueries.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.where !== null) {
      propertyReprs.push(`where=${this.where.repr()}`);
    }
    if (this.having !== null) {
      propertyReprs.push(`having=${this.having.repr()}`);
    }
    if (this.groupBy.length > 0) {
      propertyReprs.push(`groupBy=${this.groupBy.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.aggregation !== null) {
      propertyReprs.push(`aggregation=${this.aggregation.repr()}`);
    }
    if (this.sort.length > 0) {
      propertyReprs.push(`sort=${this.sort.map((_item) => _item.repr()).join(", ")}`);
    }
    if (this.limit !== null) {
      propertyReprs.push(`limit=${this.limit}`);
    }
    if (this.offset !== null) {
      propertyReprs.push(`offset=${this.offset}`);
    }
    return `<QueryEvent '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return QueryEvent.__packValue__(this);
  }

  static __packValue__(object: QueryEvent): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2003;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["100"] = object.type;
    if (object.nodePtr != null) {
      objectValue["101"] = object.nodePtr.toValue();
    }
    objectValue["102"] = object.name;
    objectValue["103"] = object.definition.toValue();
    if (object.join != null) {
      objectValue["104"] = object.join.toValue();
    }
    if (object.select != null) {
      objectValue["105"] = object.select.toValue();
    }
    if (object.subqueries.length > 0) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toValue());
      }
      objectValue["106"] = packedSubqueries;
    }
    if (object.where != null) {
      objectValue["110"] = object.where.toValue();
    }
    if (object.having != null) {
      objectValue["111"] = object.having.toValue();
    }
    if (object.groupBy.length > 0) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toValue());
      }
      objectValue["112"] = packedGroupBy;
    }
    if (object.aggregation != null) {
      objectValue["113"] = object.aggregation.toValue();
    }
    if (object.sort.length > 0) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toValue());
      }
      objectValue["114"] = packedSort;
    }
    if (object.limit != null) {
      objectValue["120"] = object.limit;
    }
    if (object.offset != null) {
      objectValue["121"] = object.offset;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryEvent {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Sort = STRUCT_CLASS_BY_TYPE[StructType.SORT] as typeof Sort;
    const _Select = STRUCT_CLASS_BY_TYPE[StructType.SELECT] as typeof Select;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const joinValue = objectValue["104"];
    const unpackedJoin =
      joinValue != undefined
        ? _Join.fromValue(joinValue, _session, _supergraph, _graph, _connection)
        : null;
    const selectValue = objectValue["105"];
    const unpackedSelect =
      selectValue != undefined
        ? _Select.fromValue(selectValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedSubqueries: any[] = [];
    if (objectValue["106"] != undefined) {
      for (const item of objectValue["106"]) {
        unpackedSubqueries.push(_Query.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const whereValue = objectValue["110"];
    const unpackedWhere =
      whereValue != undefined
        ? _Condition.fromValue(whereValue, _session, _supergraph, _graph, _connection)
        : null;
    const havingValue = objectValue["111"];
    const unpackedHaving =
      havingValue != undefined
        ? _Condition.fromValue(havingValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedGroupBy: any[] = [];
    if (objectValue["112"] != undefined) {
      for (const item of objectValue["112"]) {
        unpackedGroupBy.push(
          _Expression.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const aggregationValue = objectValue["113"];
    const unpackedAggregation =
      aggregationValue != undefined
        ? _Aggregation.fromValue(aggregationValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedSort: any[] = [];
    if (objectValue["114"] != undefined) {
      for (const item of objectValue["114"]) {
        unpackedSort.push(_Sort.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const limitValue = objectValue["120"];
    const unpackedLimit = limitValue != undefined ? Number(limitValue) : null;
    const offsetValue = objectValue["121"];
    const unpackedOffset = offsetValue != undefined ? Number(offsetValue) : null;
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
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const nodePtrValue = objectValue["101"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromValue(nodePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new QueryEvent({
      type: Number(objectValue["100"]),
      name: objectValue["102"],
      definition: _NodeDefinitionReference.fromValue(
        objectValue["103"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      join: unpackedJoin,
      select: unpackedSelect,
      subqueries: unpackedSubqueries,
      where: unpackedWhere,
      having: unpackedHaving,
      groupBy: unpackedGroupBy,
      aggregation: unpackedAggregation,
      sort: unpackedSort,
      limit: unpackedLimit,
      offset: unpackedOffset,
      parent: unpackedParentPtr,
      snapshot: unpackedSnapshotPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      node: unpackedNodePtr,
      space: unpackedSpacePtr,
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
  ): QueryEvent {
    return QueryEvent.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): QueryEventProto {
    return QueryEvent.__packProto__(this);
  }

  static __packProto__(object: QueryEvent): QueryEventProto {
    const objectProto: Partial<QueryEventProto> = { metatype: 2003 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.type = Number(object.type) as QueryTypeProto;
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    objectProto.name = object.name;
    objectProto.definition = object.definition.toProto();
    if (object.join != null) {
      objectProto.join = object.join.toProto();
    }
    if (object.select != null) {
      objectProto.select = object.select.toProto();
    }
    if (object.subqueries) {
      const packedSubqueries: any[] = [];
      for (const item of object.subqueries) {
        packedSubqueries.push(item.toProto());
      }
      objectProto.subqueries = packedSubqueries;
    }
    if (object.where != null) {
      objectProto.where = object.where.toProto();
    }
    if (object.having != null) {
      objectProto.having = object.having.toProto();
    }
    if (object.groupBy) {
      const packedGroupBy: any[] = [];
      for (const item of object.groupBy) {
        packedGroupBy.push(item.toProto());
      }
      objectProto.groupBy = packedGroupBy;
    }
    if (object.aggregation != null) {
      objectProto.aggregation = object.aggregation.toProto();
    }
    if (object.sort) {
      const packedSort: any[] = [];
      for (const item of object.sort) {
        packedSort.push(item.toProto());
      }
      objectProto.sort = packedSort;
    }
    if (object.limit != null) {
      objectProto.limit = object.limit;
    }
    if (object.offset != null) {
      objectProto.offset = object.offset;
    }
    return objectProto as QueryEventProto;
  }

  static __unpackProto__(
    objectProto: QueryEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryEvent {
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Expression = STRUCT_CLASS_BY_TYPE[StructType.EXPRESSION] as typeof Expression;
    const _Join = STRUCT_CLASS_BY_TYPE[StructType.JOIN] as typeof Join;
    const _Aggregation = STRUCT_CLASS_BY_TYPE[StructType.AGGREGATION] as typeof Aggregation;
    const _Condition = STRUCT_CLASS_BY_TYPE[StructType.CONDITION] as typeof Condition;
    const _Sort = STRUCT_CLASS_BY_TYPE[StructType.SORT] as typeof Sort;
    const _Select = STRUCT_CLASS_BY_TYPE[StructType.SELECT] as typeof Select;
    const _Query = STRUCT_CLASS_BY_TYPE[StructType.QUERY] as typeof Query;
    const unpackedSubqueries: any[] = [];
    if (objectProto.subqueries) {
      for (const item of objectProto.subqueries) {
        unpackedSubqueries.push(
          _Query.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroupBy: any[] = [];
    if (objectProto.groupBy) {
      for (const item of objectProto.groupBy) {
        unpackedGroupBy.push(
          _Expression.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSort: any[] = [];
    if (objectProto.sort) {
      for (const item of objectProto.sort) {
        unpackedSort.push(_Sort.fromProto(item!, _session, _supergraph, _graph, _connection));
      }
    }
    return new QueryEvent({
      type: Number(objectProto.type) as QueryType,
      name: objectProto.name,
      definition: _NodeDefinitionReference.fromProto(
        objectProto.definition!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      join:
        objectProto.join != undefined
          ? _Join.fromProto(objectProto.join!, _session, _supergraph, _graph, _connection)
          : null,
      select:
        objectProto.select != undefined
          ? _Select.fromProto(objectProto.select!, _session, _supergraph, _graph, _connection)
          : null,
      subqueries: unpackedSubqueries,
      where:
        objectProto.where != undefined
          ? _Condition.fromProto(objectProto.where!, _session, _supergraph, _graph, _connection)
          : null,
      having:
        objectProto.having != undefined
          ? _Condition.fromProto(objectProto.having!, _session, _supergraph, _graph, _connection)
          : null,
      groupBy: unpackedGroupBy,
      aggregation:
        objectProto.aggregation != undefined
          ? _Aggregation.fromProto(
              objectProto.aggregation!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      sort: unpackedSort,
      limit: objectProto.limit != undefined ? Number(objectProto.limit) : null,
      offset: objectProto.offset != undefined ? Number(objectProto.offset) : null,
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
      id: String(objectProto.id),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: QueryEventProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): QueryEvent {
    return QueryEvent.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): QueryEvent {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = QueryEventProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.QUERY_EVENT, QueryEvent);
/* ==== DESTACK_GENERATED_END:NODE:2003 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2004 ==== */
/**
 * An Event that represents a Measurement.
 */
export abstract class MeasurementEvent extends Event {
  static metatype: NodeType = NodeType.MEASUREMENT_EVENT;

  abstract get parent(): Space | null;
  declare readonly parentPtr: NodeReference | null;

  abstract get space(): Space | null;
  declare readonly spacePtr: NodeReference | null;

  abstract get definition(): Metric | null;
  declare readonly definitionPtr: NodeReference;

  abstract get snapshot(): Snapshot | null;
  declare readonly snapshotPtr: NodeReference | null;

  /**
   * Event.createdAt
   */
  declare readonly createdAt: Temporal.ZonedDateTime;

  abstract get createdBy(): (Node & IsSubject) | null;
  declare readonly createdByPtr: NodeReference | null;

  abstract get node(): Node | null;
  declare readonly nodePtr: NodeReference | null;

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.MEASUREMENT_EVENT, MeasurementEvent);
/* ==== DESTACK_GENERATED_END:NODE:2004 ==== */
