import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  CascadeAction,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  PropertyType,
  ScalarType,
  StructType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type {
  CustomEntityDefinition,
  CustomTraitDefinition,
  Snapshot,
} from "@destack/language/core/builtin/entity";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import type { CustomEventDefinition } from "@destack/language/core/builtin/event";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsArchivable,
  IsCustomizable,
  IsDeletable,
  IsSourceable,
  IsSpatial,
  IsSubject,
  IsTaggable,
} from "@destack/language/core/builtin/trait";
import type { CustomEnumDefinition } from "@destack/language/core/common/enum";
import type { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type { CustomStructDefinition } from "@destack/language/core/common/struct";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import {
  CascadeActionProto,
  CustomPropertyGroupProto,
  CustomPropertyProto,
  EdgeTypeProto,
  EnumTypeProto,
  MaterializationProto,
  NodeTypeProto,
  PrimitiveTypeProto,
  PropertyTypeProto,
  ScalarTypeProto,
  StructTypeProto,
  TypeCardinalityProto,
  ValueFactoryProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:110 ==== */
/**
 * A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
 */
export class CustomProperty
  extends Entity
  implements IsSpatial, IsTaggable, IsArchivable, IsDeletable, IsSourceable
{
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * CustomProperty.parent
   */
  get parent(): (Node & IsCustomizable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsCustomizable) | null;
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
  get predecessor(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
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
   * IsArchivable.archivedAt
   */
  readonly archivedAt: Temporal.ZonedDateTime | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * CustomProperty.type
   */
  get type(): PropertyType {
    return this.#type;
  }
  set type(value: PropertyType) {
    const oldValue = this.#type;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["type"] === undefined) {
      this._dirty["type"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#type = value;
  }
  #type: PropertyType;

  /**
   * CustomProperty.name
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
   * CustomProperty.icon
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

  /**
   * CustomProperty.group
   */
  get group(): CustomPropertyGroup | null {
    const nodePtr: NodeReference | null = this.groupPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomPropertyGroup | null;
    }
    return null;
  }
  set group(node: CustomPropertyGroup | null) {
    if (node === null) {
      this.groupPtr = null;
    } else {
      this.groupPtr = node.toRef();
    }
  }
  get groupPtr(): NodeReference | null {
    return this.#groupPtr;
  }
  set groupPtr(value: NodeReference | null) {
    const oldValue = this.#groupPtr;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["groupPtr"] === undefined) {
      this._dirty["groupPtr"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#groupPtr = value;
  }
  #groupPtr: NodeReference | null;

  /**
   * CustomProperty.cardinality
   */
  get cardinality(): TypeCardinality {
    return this.#cardinality;
  }
  set cardinality(value: TypeCardinality) {
    const oldValue = this.#cardinality;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["cardinality"] === undefined) {
      this._dirty["cardinality"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#cardinality = value;
  }
  #cardinality: TypeCardinality;

  /**
   * CustomProperty.scalarType
   */
  get scalarType(): ScalarType {
    return this.#scalarType;
  }
  set scalarType(value: ScalarType) {
    const oldValue = this.#scalarType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["scalarType"] === undefined) {
      this._dirty["scalarType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#scalarType = value;
  }
  #scalarType: ScalarType;

  /**
   * CustomProperty.primitiveType
   */
  get primitiveType(): PrimitiveType | null {
    return this.#primitiveType;
  }
  set primitiveType(value: PrimitiveType | null) {
    const oldValue = this.#primitiveType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["primitiveType"] === undefined) {
      this._dirty["primitiveType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#primitiveType = value;
  }
  #primitiveType: PrimitiveType | null;

  /**
   * CustomProperty.enumType
   */
  get enumType(): EnumType | null {
    return this.#enumType;
  }
  set enumType(value: EnumType | null) {
    const oldValue = this.#enumType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["enumType"] === undefined) {
      this._dirty["enumType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#enumType = value;
  }
  #enumType: EnumType | null;

  /**
   * CustomProperty.nodeType
   */
  get nodeType(): NodeType | null {
    return this.#nodeType;
  }
  set nodeType(value: NodeType | null) {
    const oldValue = this.#nodeType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["nodeType"] === undefined) {
      this._dirty["nodeType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#nodeType = value;
  }
  #nodeType: NodeType | null;

  /**
   * CustomProperty.structType
   */
  get structType(): StructType | null {
    return this.#structType;
  }
  set structType(value: StructType | null) {
    const oldValue = this.#structType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["structType"] === undefined) {
      this._dirty["structType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#structType = value;
  }
  #structType: StructType | null;

  /**
   * CustomProperty.definition
   */
  get definition():
    | CustomEntityDefinition
    | CustomEventDefinition
    | CustomEnumDefinition
    | CustomStructDefinition
    | CustomTraitDefinition
    | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | CustomEnumDefinition
        | CustomStructDefinition
        | CustomTraitDefinition
        | null;
    }
    return null;
  }
  set definition(
    node:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomTraitDefinition
      | null,
  ) {
    if (node === null) {
      this.definitionPtr = null;
    } else {
      this.definitionPtr = node.toRef();
    }
  }
  get definitionPtr(): NodeReference | null {
    return this.#definitionPtr;
  }
  set definitionPtr(value: NodeReference | null) {
    const oldValue = this.#definitionPtr;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["definitionPtr"] === undefined) {
      this._dirty["definitionPtr"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#definitionPtr = value;
  }
  #definitionPtr: NodeReference | null;

  /**
   * CustomProperty.keyType
   */
  get keyType(): Type | null {
    return this.#keyType;
  }
  set keyType(value: Type | null) {
    const oldValue = this.#keyType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["keyType"] === undefined) {
      this._dirty["keyType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#keyType = value;
  }
  #keyType: Type | null;

  /**
   * CustomProperty.value
   */
  get value(): Value | null {
    return this.#value;
  }
  set value(value: Value | null) {
    const oldValue = this.#value;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["value"] === undefined) {
      this._dirty["value"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#value = value;
  }
  #value: Value | null;

  /**
   * CustomProperty.valueFactory
   */
  get valueFactory(): ValueFactory | null {
    return this.#valueFactory;
  }
  set valueFactory(value: ValueFactory | null) {
    const oldValue = this.#valueFactory;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["valueFactory"] === undefined) {
      this._dirty["valueFactory"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#valueFactory = value;
  }
  #valueFactory: ValueFactory | null;

  /**
   * CustomProperty.collectionConstraint
   */
  get collectionConstraint(): CollectionConstraint | null {
    return this.#collectionConstraint;
  }
  set collectionConstraint(value: CollectionConstraint | null) {
    const oldValue = this.#collectionConstraint;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["collectionConstraint"] === undefined) {
      this._dirty["collectionConstraint"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#collectionConstraint = value;
  }
  #collectionConstraint: CollectionConstraint | null;

  /**
   * CustomProperty.stringConstraint
   */
  get stringConstraint(): StringConstraint | null {
    return this.#stringConstraint;
  }
  set stringConstraint(value: StringConstraint | null) {
    const oldValue = this.#stringConstraint;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["stringConstraint"] === undefined) {
      this._dirty["stringConstraint"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#stringConstraint = value;
  }
  #stringConstraint: StringConstraint | null;

  /**
   * CustomProperty.numberConstraint
   */
  get numberConstraint(): NumberConstraint | null {
    return this.#numberConstraint;
  }
  set numberConstraint(value: NumberConstraint | null) {
    const oldValue = this.#numberConstraint;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["numberConstraint"] === undefined) {
      this._dirty["numberConstraint"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#numberConstraint = value;
  }
  #numberConstraint: NumberConstraint | null;

  /**
   * CustomProperty.nodeConstraint
   */
  get nodeConstraint(): NodeConstraint | null {
    return this.#nodeConstraint;
  }
  set nodeConstraint(value: NodeConstraint | null) {
    const oldValue = this.#nodeConstraint;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["nodeConstraint"] === undefined) {
      this._dirty["nodeConstraint"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#nodeConstraint = value;
  }
  #nodeConstraint: NodeConstraint | null;

  /**
   * CustomProperty.edgeType
   */
  get edgeType(): EdgeType | null {
    return this.#edgeType;
  }
  set edgeType(value: EdgeType | null) {
    const oldValue = this.#edgeType;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["edgeType"] === undefined) {
      this._dirty["edgeType"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#edgeType = value;
  }
  #edgeType: EdgeType | null;

  /**
   * CustomProperty.cascade
   */
  get cascade(): CascadeAction | null {
    return this.#cascade;
  }
  set cascade(value: CascadeAction | null) {
    const oldValue = this.#cascade;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["cascade"] === undefined) {
      this._dirty["cascade"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#cascade = value;
  }
  #cascade: CascadeAction | null;

  /**
   * CustomProperty.isRequired
   */
  get isRequired(): boolean | null {
    return this.#isRequired;
  }
  set isRequired(value: boolean | null) {
    const oldValue = this.#isRequired;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isRequired"] === undefined) {
      this._dirty["isRequired"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#isRequired = value;
  }
  #isRequired: boolean | null;

  /**
   * CustomProperty.isUnique
   */
  get isUnique(): boolean | null {
    return this.#isUnique;
  }
  set isUnique(value: boolean | null) {
    const oldValue = this.#isUnique;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isUnique"] === undefined) {
      this._dirty["isUnique"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#isUnique = value;
  }
  #isUnique: boolean | null;

  /**
   * CustomProperty.isComputed
   */
  get isComputed(): boolean | null {
    return this.#isComputed;
  }
  set isComputed(value: boolean | null) {
    const oldValue = this.#isComputed;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isComputed"] === undefined) {
      this._dirty["isComputed"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#isComputed = value;
  }
  #isComputed: boolean | null;

  /**
   * CustomProperty.isReadonly
   */
  get isReadonly(): boolean | null {
    return this.#isReadonly;
  }
  set isReadonly(value: boolean | null) {
    const oldValue = this.#isReadonly;
    if (this._dirty == null) {
      this._dirty = {};
    }
    if (this._dirty["isReadonly"] === undefined) {
      this._dirty["isReadonly"] = oldValue;
    }
    if (!this._session.dirty[this.id]) {
      this._session.dirty[this.id] = this;
    }
    this.#isReadonly = value;
  }
  #isReadonly: boolean | null;

  constructor(options: {
    id?: string;
    parent?: (Node & IsCustomizable) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: CustomProperty | NodeReference | null;
    template?: CustomProperty | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    archivedAt?: Temporal.ZonedDateTime | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    source?: Script | NodeReference | null;
    type?: PropertyType;
    name: string;
    icon?: Icon | null;
    group?: CustomPropertyGroup | NodeReference | null;
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    definition?:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomEnumDefinition
      | CustomStructDefinition
      | CustomTraitDefinition
      | NodeReference
      | null;
    keyType?: Type | null;
    value?: Value | null;
    valueFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isRequired?: boolean | null;
    isUnique?: boolean | null;
    isComputed?: boolean | null;
    isReadonly?: boolean | null;
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
      throw new Error(`CustomProperty.materialization is required`);
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
    let _archivedAt = options.archivedAt ?? null;
    this.archivedAt = _archivedAt;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomProperty.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 1 /* PropertyType.MEMBER */;
    }
    if (_type === null) {
      throw new Error(`CustomProperty.type is required`);
    }
    this.#type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomProperty.name is required`);
    }
    this.#name = _name;
    let _icon = options.icon ?? null;
    this.#icon = _icon;
    let _group = options.group ?? null;
    if (_group != null && _group.metatype != StructType.NODE_REFERENCE) {
      _group = (_group as Node).toRef();
    }
    this.#groupPtr = _group;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality === null) {
      throw new Error(`CustomProperty.cardinality is required`);
    }
    this.#cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`CustomProperty.scalarType is required`);
    }
    this.#scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.#primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.#enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this.#nodeType = _nodeType;
    let _structType = options.structType ?? null;
    this.#structType = _structType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.#definitionPtr = _definition;
    let _keyType = options.keyType ?? null;
    this.#keyType = _keyType;
    let _value = options.value ?? null;
    this.#value = _value;
    let _valueFactory = options.valueFactory ?? null;
    this.#valueFactory = _valueFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.#collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.#stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.#numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.#nodeConstraint = _nodeConstraint;
    let _edgeType = options.edgeType ?? null;
    this.#edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.#cascade = _cascade;
    let _isRequired = options.isRequired ?? null;
    this.#isRequired = _isRequired;
    let _isUnique = options.isUnique ?? null;
    this.#isUnique = _isUnique;
    let _isComputed = options.isComputed ?? null;
    this.#isComputed = _isComputed;
    let _isReadonly = options.isReadonly ?? null;
    this.#isReadonly = _isReadonly;

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
          `CustomProperty.createdAt and CustomProperty.updatedAt are required for existing Nodes`,
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
    if (!(this.#type === other.#type)) {
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
    if (!(this.#groupPtr?.id === other.#groupPtr?.id)) {
      return false;
    }
    if (!(this.#cardinality === other.#cardinality)) {
      return false;
    }
    if (!(this.#scalarType === other.#scalarType)) {
      return false;
    }
    if (!(this.#primitiveType === other.#primitiveType)) {
      return false;
    }
    if (!(this.#enumType === other.#enumType)) {
      return false;
    }
    if (!(this.#nodeType === other.#nodeType)) {
      return false;
    }
    if (!(this.#structType === other.#structType)) {
      return false;
    }
    if (!(this.#definitionPtr?.id === other.#definitionPtr?.id)) {
      return false;
    }
    if (
      (this.#keyType == null) !== (other.#keyType == null) ||
      (this.#keyType != null && !this.#keyType.equals(other.#keyType))
    ) {
      return false;
    }
    if (
      (this.#value == null) !== (other.#value == null) ||
      (this.#value != null && !this.#value.equals(other.#value))
    ) {
      return false;
    }
    if (!(this.#valueFactory === other.#valueFactory)) {
      return false;
    }
    if (
      (this.#collectionConstraint == null) !== (other.#collectionConstraint == null) ||
      (this.#collectionConstraint != null &&
        !this.#collectionConstraint.equals(other.#collectionConstraint))
    ) {
      return false;
    }
    if (
      (this.#stringConstraint == null) !== (other.#stringConstraint == null) ||
      (this.#stringConstraint != null && !this.#stringConstraint.equals(other.#stringConstraint))
    ) {
      return false;
    }
    if (
      (this.#numberConstraint == null) !== (other.#numberConstraint == null) ||
      (this.#numberConstraint != null && !this.#numberConstraint.equals(other.#numberConstraint))
    ) {
      return false;
    }
    if (
      (this.#nodeConstraint == null) !== (other.#nodeConstraint == null) ||
      (this.#nodeConstraint != null && !this.#nodeConstraint.equals(other.#nodeConstraint))
    ) {
      return false;
    }
    if (!(this.#edgeType === other.#edgeType)) {
      return false;
    }
    if (!(this.#cascade === other.#cascade)) {
      return false;
    }
    if (!(this.#isRequired === other.#isRequired)) {
      return false;
    }
    if (!(this.#isUnique === other.#isUnique)) {
      return false;
    }
    if (!(this.#isComputed === other.#isComputed)) {
      return false;
    }
    if (!(this.#isReadonly === other.#isReadonly)) {
      return false;
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
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
    h = (h * 31 + this.#type) & 0xffffffff;
    h = (h * 31 + hashString(this.#name)) & 0xffffffff;
    if (this.#icon !== null) {
      h = (h * 31 + this.#icon.hash()) & 0xffffffff;
    }
    if (this.#groupPtr !== null) {
      h = (h * 31 + hashString(this.#groupPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.#cardinality) & 0xffffffff;
    h = (h * 31 + this.#scalarType) & 0xffffffff;
    if (this.#primitiveType !== null) {
      h = (h * 31 + this.#primitiveType) & 0xffffffff;
    }
    if (this.#enumType !== null) {
      h = (h * 31 + this.#enumType) & 0xffffffff;
    }
    if (this.#nodeType !== null) {
      h = (h * 31 + this.#nodeType) & 0xffffffff;
    }
    if (this.#structType !== null) {
      h = (h * 31 + this.#structType) & 0xffffffff;
    }
    if (this.#definitionPtr !== null) {
      h = (h * 31 + hashString(this.#definitionPtr.id)) & 0xffffffff;
    }
    if (this.#keyType !== null) {
      h = (h * 31 + this.#keyType.hash()) & 0xffffffff;
    }
    if (this.#value !== null) {
      h = (h * 31 + this.#value.hash()) & 0xffffffff;
    }
    if (this.#valueFactory !== null) {
      h = (h * 31 + this.#valueFactory) & 0xffffffff;
    }
    if (this.#collectionConstraint !== null) {
      h = (h * 31 + this.#collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.#stringConstraint !== null) {
      h = (h * 31 + this.#stringConstraint.hash()) & 0xffffffff;
    }
    if (this.#numberConstraint !== null) {
      h = (h * 31 + this.#numberConstraint.hash()) & 0xffffffff;
    }
    if (this.#nodeConstraint !== null) {
      h = (h * 31 + this.#nodeConstraint.hash()) & 0xffffffff;
    }
    if (this.#edgeType !== null) {
      h = (h * 31 + this.#edgeType) & 0xffffffff;
    }
    if (this.#cascade !== null) {
      h = (h * 31 + this.#cascade) & 0xffffffff;
    }
    if (this.#isRequired !== null) {
      h = (h * 31 + hashBool(this.#isRequired)) & 0xffffffff;
    }
    if (this.#isUnique !== null) {
      h = (h * 31 + hashBool(this.#isUnique)) & 0xffffffff;
    }
    if (this.#isComputed !== null) {
      h = (h * 31 + hashBool(this.#isComputed)) & 0xffffffff;
    }
    if (this.#isReadonly !== null) {
      h = (h * 31 + hashBool(this.#isReadonly)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.archivedAt !== null) {
      h = (h * 31 + hashString(this.archivedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.CUSTOM_PROPERTY,
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
    propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
    propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
    if (this.primitiveType !== null) {
      propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
    }
    if (this.enumType !== null) {
      propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
    }
    if (this.nodeType !== null) {
      propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
    }
    if (this.structType !== null) {
      propertyReprs.push(`structType=${StructType[this.structType]}`);
    }
    if (this.definition !== null) {
      propertyReprs.push(`definition=${this.definition?.repr()}`);
    }
    if (this.keyType !== null) {
      propertyReprs.push(`keyType=${this.keyType.repr()}`);
    }
    return `<CustomProperty '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomProperty.__packValue__(this);
  }

  static __packValue__(object: CustomProperty): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 110;
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
    if (object.archivedAt != null) {
      objectValue["24"] = object.archivedAt.toString({ timeZoneName: "never" });
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["27"] = object.orderKey;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    objectValue["100"] = object.#type;
    objectValue["101"] = object.#name;
    if (object.#icon != null) {
      objectValue["102"] = object.#icon.toValue();
    }
    if (object.#groupPtr != null) {
      objectValue["105"] = object.#groupPtr.toValue();
    }
    objectValue["110"] = object.#cardinality;
    objectValue["111"] = object.#scalarType;
    if (object.#primitiveType != null) {
      objectValue["112"] = object.#primitiveType;
    }
    if (object.#enumType != null) {
      objectValue["113"] = object.#enumType;
    }
    if (object.#nodeType != null) {
      objectValue["114"] = object.#nodeType;
    }
    if (object.#structType != null) {
      objectValue["115"] = object.#structType;
    }
    if (object.#definitionPtr != null) {
      objectValue["116"] = object.#definitionPtr.toValue();
    }
    if (object.#keyType != null) {
      objectValue["117"] = object.#keyType.toValue();
    }
    if (object.#value != null) {
      objectValue["120"] = object.#value.toValue();
    }
    if (object.#valueFactory != null) {
      objectValue["121"] = object.#valueFactory;
    }
    if (object.#collectionConstraint != null) {
      objectValue["130"] = object.#collectionConstraint.toValue();
    }
    if (object.#stringConstraint != null) {
      objectValue["131"] = object.#stringConstraint.toValue();
    }
    if (object.#numberConstraint != null) {
      objectValue["132"] = object.#numberConstraint.toValue();
    }
    if (object.#nodeConstraint != null) {
      objectValue["133"] = object.#nodeConstraint.toValue();
    }
    if (object.#edgeType != null) {
      objectValue["140"] = object.#edgeType;
    }
    if (object.#cascade != null) {
      objectValue["141"] = object.#cascade;
    }
    if (object.#isRequired != null) {
      objectValue["150"] = object.#isRequired;
    }
    if (object.#isUnique != null) {
      objectValue["151"] = object.#isUnique;
    }
    if (object.#isComputed != null) {
      objectValue["152"] = object.#isComputed;
    }
    if (object.#isReadonly != null) {
      objectValue["153"] = object.#isReadonly;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const groupPtrValue = objectValue["105"];
    const unpackedGroupPtr =
      groupPtrValue != undefined
        ? _NodeReference.fromValue(groupPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const primitiveTypeValue = objectValue["112"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["113"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["114"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["115"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const definitionPtrValue = objectValue["116"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyTypeValue = objectValue["117"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? _Type.fromValue(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectValue["120"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromValue(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueFactoryValue = objectValue["121"];
    const unpackedValueFactory = valueFactoryValue != undefined ? Number(valueFactoryValue) : null;
    const collectionConstraintValue = objectValue["130"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? _CollectionConstraint.fromValue(
            collectionConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const stringConstraintValue = objectValue["131"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? _StringConstraint.fromValue(
            stringConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const numberConstraintValue = objectValue["132"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? _NumberConstraint.fromValue(
            numberConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const nodeConstraintValue = objectValue["133"];
    const unpackedNodeConstraint =
      nodeConstraintValue != undefined
        ? _NodeConstraint.fromValue(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const edgeTypeValue = objectValue["140"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["141"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    const isRequiredValue = objectValue["150"];
    const unpackedIsRequired = isRequiredValue != undefined ? isRequiredValue : null;
    const isUniqueValue = objectValue["151"];
    const unpackedIsUnique = isUniqueValue != undefined ? isUniqueValue : null;
    const isComputedValue = objectValue["152"];
    const unpackedIsComputed = isComputedValue != undefined ? isComputedValue : null;
    const isReadonlyValue = objectValue["153"];
    const unpackedIsReadonly = isReadonlyValue != undefined ? isReadonlyValue : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const archivedAtValue = objectValue["24"];
    const unpackedArchivedAt =
      archivedAtValue != undefined
        ? Temporal.Instant.from(archivedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
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
    return new CustomProperty({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      group: unpackedGroupPtr,
      cardinality: Number(objectValue["110"]),
      scalarType: Number(objectValue["111"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      definition: unpackedDefinitionPtr,
      keyType: unpackedKeyType,
      value: unpackedValue,
      valueFactory: unpackedValueFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isRequired: unpackedIsRequired,
      isUnique: unpackedIsUnique,
      isComputed: unpackedIsComputed,
      isReadonly: unpackedIsReadonly,
      space: unpackedSpacePtr,
      archivedAt: unpackedArchivedAt,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
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
  ): CustomProperty {
    return CustomProperty.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CustomPropertyProto {
    return CustomProperty.__packProto__(this);
  }

  static __packProto__(object: CustomProperty): CustomPropertyProto {
    const objectProto: Partial<CustomPropertyProto> = { metatype: 110 };
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
    if (object.archivedAt != null) {
      objectProto.archivedAt = packProtoTimestamp(object.archivedAt);
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.orderKey = object.orderKey;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    objectProto.type = Number(object.#type) as PropertyTypeProto;
    objectProto.name = object.#name;
    if (object.#icon != null) {
      objectProto.icon = object.#icon.toProto();
    }
    if (object.#groupPtr != null) {
      objectProto.groupPtr = object.#groupPtr.toProto();
    }
    objectProto.cardinality = Number(object.#cardinality) as TypeCardinalityProto;
    objectProto.scalarType = Number(object.#scalarType) as ScalarTypeProto;
    if (object.#primitiveType != null) {
      objectProto.primitiveType = Number(object.#primitiveType) as PrimitiveTypeProto;
    }
    if (object.#enumType != null) {
      objectProto.enumType = Number(object.#enumType) as EnumTypeProto;
    }
    if (object.#nodeType != null) {
      objectProto.nodeType = Number(object.#nodeType) as NodeTypeProto;
    }
    if (object.#structType != null) {
      objectProto.structType = Number(object.#structType) as StructTypeProto;
    }
    if (object.#definitionPtr != null) {
      objectProto.definitionPtr = object.#definitionPtr.toProto();
    }
    if (object.#keyType != null) {
      objectProto.keyType = object.#keyType.toProto();
    }
    if (object.#value != null) {
      objectProto.value = object.#value.toProto();
    }
    if (object.#valueFactory != null) {
      objectProto.valueFactory = Number(object.#valueFactory) as ValueFactoryProto;
    }
    if (object.#collectionConstraint != null) {
      objectProto.collectionConstraint = object.#collectionConstraint.toProto();
    }
    if (object.#stringConstraint != null) {
      objectProto.stringConstraint = object.#stringConstraint.toProto();
    }
    if (object.#numberConstraint != null) {
      objectProto.numberConstraint = object.#numberConstraint.toProto();
    }
    if (object.#nodeConstraint != null) {
      objectProto.nodeConstraint = object.#nodeConstraint.toProto();
    }
    if (object.#edgeType != null) {
      objectProto.edgeType = Number(object.#edgeType) as EdgeTypeProto;
    }
    if (object.#cascade != null) {
      objectProto.cascade = Number(object.#cascade) as CascadeActionProto;
    }
    if (object.#isRequired != null) {
      objectProto.isRequired = object.#isRequired;
    }
    if (object.#isUnique != null) {
      objectProto.isUnique = object.#isUnique;
    }
    if (object.#isComputed != null) {
      objectProto.isComputed = object.#isComputed;
    }
    if (object.#isReadonly != null) {
      objectProto.isReadonly = object.#isReadonly;
    }
    return objectProto as CustomPropertyProto;
  }

  static __unpackProto__(
    objectProto: CustomPropertyProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    const _NumberConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NUMBER_CONSTRAINT
    ] as typeof NumberConstraint;
    const _StringConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.STRING_CONSTRAINT
    ] as typeof StringConstraint;
    const _CollectionConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.COLLECTION_CONSTRAINT
    ] as typeof CollectionConstraint;
    const _NodeConstraint = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_CONSTRAINT
    ] as typeof NodeConstraint;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new CustomProperty({
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
      type: Number(objectProto.type) as PropertyType,
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      group:
        objectProto.groupPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.groupPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      cardinality: Number(objectProto.cardinality) as TypeCardinality,
      scalarType: Number(objectProto.scalarType) as ScalarType,
      primitiveType:
        objectProto.primitiveType != undefined
          ? (Number(objectProto.primitiveType) as PrimitiveType)
          : null,
      enumType:
        objectProto.enumType != undefined ? (Number(objectProto.enumType) as EnumType) : null,
      nodeType:
        objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      keyType:
        objectProto.keyType != undefined
          ? _Type.fromProto(objectProto.keyType!, _session, _supergraph, _graph, _connection)
          : null,
      value:
        objectProto.value != undefined
          ? _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection)
          : null,
      valueFactory:
        objectProto.valueFactory != undefined
          ? (Number(objectProto.valueFactory) as ValueFactory)
          : null,
      collectionConstraint:
        objectProto.collectionConstraint != undefined
          ? _CollectionConstraint.fromProto(
              objectProto.collectionConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      stringConstraint:
        objectProto.stringConstraint != undefined
          ? _StringConstraint.fromProto(
              objectProto.stringConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      numberConstraint:
        objectProto.numberConstraint != undefined
          ? _NumberConstraint.fromProto(
              objectProto.numberConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      nodeConstraint:
        objectProto.nodeConstraint != undefined
          ? _NodeConstraint.fromProto(
              objectProto.nodeConstraint!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      edgeType:
        objectProto.edgeType != undefined ? (Number(objectProto.edgeType) as EdgeType) : null,
      cascade:
        objectProto.cascade != undefined ? (Number(objectProto.cascade) as CascadeAction) : null,
      isRequired: objectProto.isRequired != undefined ? objectProto.isRequired : null,
      isUnique: objectProto.isUnique != undefined ? objectProto.isUnique : null,
      isComputed: objectProto.isComputed != undefined ? objectProto.isComputed : null,
      isReadonly: objectProto.isReadonly != undefined ? objectProto.isReadonly : null,
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
      archivedAt:
        objectProto.archivedAt != undefined ? unpackProtoTimestamp(objectProto.archivedAt!) : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      orderKey: objectProto.orderKey,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomPropertyProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
    return CustomProperty.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CustomProperty {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomPropertyProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  eq(value: any): Condition {
    if (value === null) {
      return Condition.of(this, ConditionalType.NOT_EXISTS);
    }
    return Condition.of(this, ConditionalType.EQUALS, value);
  }

  neq(value: any): Condition {
    if (value === null) {
      return Condition.of(this, ConditionalType.EXISTS);
    }
    return Condition.of(this, ConditionalType.NOT_EQUALS, value);
  }

  gt(value: any): Condition {
    return Condition.of(this, ConditionalType.GREATER_THAN, value);
  }

  gte(value: any): Condition {
    return Condition.of(this, ConditionalType.GREATER_THAN_OR_EQUALS, value);
  }

  lt(value: any): Condition {
    return Condition.of(this, ConditionalType.LESS_THAN, value);
  }

  lte(value: any): Condition {
    return Condition.of(this, ConditionalType.LESS_THAN_OR_EQUALS, value);
  }

  startsWith(value: string): Condition {
    return Condition.of(this, ConditionalType.STARTS_WITH, value);
  }

  endsWith(value: string): Condition {
    return Condition.of(this, ConditionalType.ENDS_WITH, value);
  }

  in(...values: any[]): Condition {
    return Condition.of(this, ConditionalType.IN, values);
  }

  notIn(...values: any[]): Condition {
    return Condition.of(this, ConditionalType.NOT_IN, values);
  }

  exists(): Condition {
    return Condition.of(this, ConditionalType.EXISTS);
  }

  isNotNone(): Condition {
    return Condition.of(this, ConditionalType.EXISTS);
  }

  notExists(): Condition {
    return Condition.of(this, ConditionalType.NOT_EXISTS);
  }

  isNone(): Condition {
    return Condition.of(this, ConditionalType.NOT_EXISTS);
  }

  asc(): Sort {
    return Sort.of(this, SortType.ASCENDING);
  }

  desc(): Sort {
    return Sort.of(this, SortType.DESCENDING);
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_PROPERTY, CustomProperty);
/* ==== DESTACK_GENERATED_END:NODE:110 ==== */

/* ==== DESTACK_GENERATED_START:NODE:111 ==== */
/**
 * A CustomPropertyGroup is a group of CustomProperties.
 */
export class CustomPropertyGroup
  extends Entity
  implements IsSpatial, IsArchivable, IsDeletable, IsSourceable
{
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY_GROUP;

  /**
   * CustomPropertyGroup.parent
   */
  get parent(): (Node & IsCustomizable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsCustomizable) | null;
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
  get predecessor(): CustomPropertyGroup | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomPropertyGroup | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The template this Entity instance is based on (from the template tree).
   */
  get template(): CustomPropertyGroup | null {
    const nodePtr: NodeReference | null = this.templatePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as CustomPropertyGroup | null;
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
   * IsArchivable.archivedAt
   */
  readonly archivedAt: Temporal.ZonedDateTime | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * CustomPropertyGroup.name
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
   * CustomPropertyGroup.icon
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
    parent?: (Node & IsCustomizable) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: CustomPropertyGroup | NodeReference | null;
    template?: CustomPropertyGroup | NodeReference | null;
    instanceRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    archivedAt?: Temporal.ZonedDateTime | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
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
      throw new Error(`CustomPropertyGroup.materialization is required`);
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
    let _archivedAt = options.archivedAt ?? null;
    this.archivedAt = _archivedAt;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomPropertyGroup.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomPropertyGroup.name is required`);
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
          `CustomPropertyGroup.createdAt and CustomPropertyGroup.updatedAt are required for existing Nodes`,
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
    h = (h * 31 + hashString(this.#name)) & 0xffffffff;
    if (this.#icon !== null) {
      h = (h * 31 + this.#icon.hash()) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.archivedAt !== null) {
      h = (h * 31 + hashString(this.archivedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.CUSTOM_PROPERTY_GROUP,
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
    return `<CustomPropertyGroup '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomPropertyGroup.__packValue__(this);
  }

  static __packValue__(object: CustomPropertyGroup): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 111;
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
    if (object.archivedAt != null) {
      objectValue["24"] = object.archivedAt.toString({ timeZoneName: "never" });
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["27"] = object.orderKey;
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
  ): CustomPropertyGroup {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
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
    const archivedAtValue = objectValue["24"];
    const unpackedArchivedAt =
      archivedAtValue != undefined
        ? Temporal.Instant.from(archivedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
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
    return new CustomPropertyGroup({
      parent: unpackedParentPtr,
      name: objectValue["101"],
      icon: unpackedIcon,
      space: unpackedSpacePtr,
      archivedAt: unpackedArchivedAt,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
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
  ): CustomPropertyGroup {
    return CustomPropertyGroup.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): CustomPropertyGroupProto {
    return CustomPropertyGroup.__packProto__(this);
  }

  static __packProto__(object: CustomPropertyGroup): CustomPropertyGroupProto {
    const objectProto: Partial<CustomPropertyGroupProto> = { metatype: 111 };
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
    if (object.archivedAt != null) {
      objectProto.archivedAt = packProtoTimestamp(object.archivedAt);
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.orderKey = object.orderKey;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    objectProto.name = object.#name;
    if (object.#icon != null) {
      objectProto.icon = object.#icon.toProto();
    }
    return objectProto as CustomPropertyGroupProto;
  }

  static __unpackProto__(
    objectProto: CustomPropertyGroupProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomPropertyGroup {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new CustomPropertyGroup({
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
      archivedAt:
        objectProto.archivedAt != undefined ? unpackProtoTimestamp(objectProto.archivedAt!) : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      orderKey: objectProto.orderKey,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: CustomPropertyGroupProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomPropertyGroup {
    return CustomPropertyGroup.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): CustomPropertyGroup {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomPropertyGroupProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_PROPERTY_GROUP, CustomPropertyGroup);
/* ==== DESTACK_GENERATED_END:NODE:111 ==== */
