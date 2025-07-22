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
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT, ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import { Event } from "@destack/language/core/builtin/event";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { IsActor } from "@destack/language/core/builtin/trait";
import type { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type { Space } from "@destack/language/core/common/space";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { GraphConnection } from "@destack/language/core/runtime/connection";
import type { Graph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:20300 ==== */
/**
 * A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
 */
export class CustomProperty extends Entity {
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
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
  get precededBy(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as CustomProperty | null;
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
   * CustomProperty.type
   */
  /**
   * CustomProperty.type
   */
  get type(): PropertyType {
    return this._type;
  }
  set type(value: PropertyType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: PropertyType;

  /**
   * CustomProperty.icon
   */
  /**
   * CustomProperty.icon
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
   * CustomProperty.cardinality
   */
  /**
   * CustomProperty.cardinality
   */
  get cardinality(): TypeCardinality {
    return this._cardinality;
  }
  set cardinality(value: TypeCardinality) {
    const prop = (this.constructor as NodeClass).__properties__["cardinality"];
    this._session.updateSetProperty(this, prop, value);
    this._cardinality = value;
  }
  _cardinality: TypeCardinality;

  /**
   * CustomProperty.scalarType
   */
  /**
   * CustomProperty.scalarType
   */
  get scalarType(): ScalarType {
    return this._scalarType;
  }
  set scalarType(value: ScalarType) {
    const prop = (this.constructor as NodeClass).__properties__["scalar_type"];
    this._session.updateSetProperty(this, prop, value);
    this._scalarType = value;
  }
  _scalarType: ScalarType;

  /**
   * CustomProperty.primitiveType
   */
  /**
   * CustomProperty.primitiveType
   */
  get primitiveType(): PrimitiveType | null {
    return this._primitiveType;
  }
  set primitiveType(value: PrimitiveType | null) {
    const prop = (this.constructor as NodeClass).__properties__["primitive_type"];
    this._session.updateSetProperty(this, prop, value);
    this._primitiveType = value;
  }
  _primitiveType: PrimitiveType | null;

  /**
   * CustomProperty.enumType
   */
  /**
   * CustomProperty.enumType
   */
  get enumType(): EnumType | null {
    return this._enumType;
  }
  set enumType(value: EnumType | null) {
    const prop = (this.constructor as NodeClass).__properties__["enum_type"];
    this._session.updateSetProperty(this, prop, value);
    this._enumType = value;
  }
  _enumType: EnumType | null;

  /**
   * CustomProperty.nodeType
   */
  /**
   * CustomProperty.nodeType
   */
  get nodeType(): NodeType | null {
    return this._nodeType;
  }
  set nodeType(value: NodeType | null) {
    const prop = (this.constructor as NodeClass).__properties__["node_type"];
    this._session.updateSetProperty(this, prop, value);
    this._nodeType = value;
  }
  _nodeType: NodeType | null;

  /**
   * CustomProperty.structType
   */
  /**
   * CustomProperty.structType
   */
  get structType(): StructType | null {
    return this._structType;
  }
  set structType(value: StructType | null) {
    const prop = (this.constructor as NodeClass).__properties__["struct_type"];
    this._session.updateSetProperty(this, prop, value);
    this._structType = value;
  }
  _structType: StructType | null;

  /**
   * CustomProperty.customDefinition
   */
  get customDefinition(): Entity | null {
    const nodePtr: NodeReference | null = this.customDefinitionPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  set customDefinition(node: Entity | null) {
    if (node === null) {
      this.customDefinitionPtr = null;
    } else {
      this.customDefinitionPtr = node.toRef();
    }
  }
  /**
   * CustomProperty.customDefinition
   */
  get customDefinitionPtr(): NodeReference | null {
    return this._customDefinitionPtr;
  }
  set customDefinitionPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["custom_definition"];
    this._session.updateSetProperty(this, prop, value);
    this._customDefinitionPtr = value;
  }
  _customDefinitionPtr: NodeReference | null;

  /**
   * CustomProperty.keyType
   */
  /**
   * CustomProperty.keyType
   */
  get keyType(): Type | null {
    return this._keyType;
  }
  set keyType(value: Type | null) {
    const prop = (this.constructor as NodeClass).__properties__["key_type"];
    this._session.updateSetProperty(this, prop, value);
    this._keyType = value;
  }
  _keyType: Type | null;

  /**
   * CustomProperty.value
   */
  /**
   * CustomProperty.value
   */
  get value(): Value | null {
    return this._value;
  }
  set value(value: Value | null) {
    const prop = (this.constructor as NodeClass).__properties__["value"];
    this._session.updateSetProperty(this, prop, value);
    this._value = value;
  }
  _value: Value | null;

  /**
   * CustomProperty.valueFactory
   */
  /**
   * CustomProperty.valueFactory
   */
  get valueFactory(): ValueFactory | null {
    return this._valueFactory;
  }
  set valueFactory(value: ValueFactory | null) {
    const prop = (this.constructor as NodeClass).__properties__["value_factory"];
    this._session.updateSetProperty(this, prop, value);
    this._valueFactory = value;
  }
  _valueFactory: ValueFactory | null;

  /**
   * CustomProperty.collectionConstraint
   */
  /**
   * CustomProperty.collectionConstraint
   */
  get collectionConstraint(): CollectionConstraint | null {
    return this._collectionConstraint;
  }
  set collectionConstraint(value: CollectionConstraint | null) {
    const prop = (this.constructor as NodeClass).__properties__["collection_constraint"];
    this._session.updateSetProperty(this, prop, value);
    this._collectionConstraint = value;
  }
  _collectionConstraint: CollectionConstraint | null;

  /**
   * CustomProperty.stringConstraint
   */
  /**
   * CustomProperty.stringConstraint
   */
  get stringConstraint(): StringConstraint | null {
    return this._stringConstraint;
  }
  set stringConstraint(value: StringConstraint | null) {
    const prop = (this.constructor as NodeClass).__properties__["string_constraint"];
    this._session.updateSetProperty(this, prop, value);
    this._stringConstraint = value;
  }
  _stringConstraint: StringConstraint | null;

  /**
   * CustomProperty.numberConstraint
   */
  /**
   * CustomProperty.numberConstraint
   */
  get numberConstraint(): NumberConstraint | null {
    return this._numberConstraint;
  }
  set numberConstraint(value: NumberConstraint | null) {
    const prop = (this.constructor as NodeClass).__properties__["number_constraint"];
    this._session.updateSetProperty(this, prop, value);
    this._numberConstraint = value;
  }
  _numberConstraint: NumberConstraint | null;

  /**
   * CustomProperty.nodeConstraint
   */
  /**
   * CustomProperty.nodeConstraint
   */
  get nodeConstraint(): NodeConstraint | null {
    return this._nodeConstraint;
  }
  set nodeConstraint(value: NodeConstraint | null) {
    const prop = (this.constructor as NodeClass).__properties__["node_constraint"];
    this._session.updateSetProperty(this, prop, value);
    this._nodeConstraint = value;
  }
  _nodeConstraint: NodeConstraint | null;

  /**
   * CustomProperty.edgeType
   */
  /**
   * CustomProperty.edgeType
   */
  get edgeType(): EdgeType | null {
    return this._edgeType;
  }
  set edgeType(value: EdgeType | null) {
    const prop = (this.constructor as NodeClass).__properties__["edge_type"];
    this._session.updateSetProperty(this, prop, value);
    this._edgeType = value;
  }
  _edgeType: EdgeType | null;

  /**
   * CustomProperty.cascade
   */
  /**
   * CustomProperty.cascade
   */
  get cascade(): CascadeAction | null {
    return this._cascade;
  }
  set cascade(value: CascadeAction | null) {
    const prop = (this.constructor as NodeClass).__properties__["cascade"];
    this._session.updateSetProperty(this, prop, value);
    this._cascade = value;
  }
  _cascade: CascadeAction | null;

  /**
   * Whether this property must be set.
   */
  /**
   * Whether this property must be set.
   */
  get isRequired(): boolean | null {
    return this._isRequired;
  }
  set isRequired(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_required"];
    this._session.updateSetProperty(this, prop, value);
    this._isRequired = value;
  }
  _isRequired: boolean | null;

  /**
   * Whether this property must have a unique value.
   */
  /**
   * Whether this property must have a unique value.
   */
  get isUnique(): boolean | null {
    return this._isUnique;
  }
  set isUnique(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_unique"];
    this._session.updateSetProperty(this, prop, value);
    this._isUnique = value;
  }
  _isUnique: boolean | null;

  /**
   * Whether this property is computed.
   */
  /**
   * Whether this property is computed.
   */
  get isComputed(): boolean | null {
    return this._isComputed;
  }
  set isComputed(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_computed"];
    this._session.updateSetProperty(this, prop, value);
    this._isComputed = value;
  }
  _isComputed: boolean | null;

  /**
   * Whether this property is read-only.
   */
  /**
   * Whether this property is read-only.
   */
  get isReadonly(): boolean | null {
    return this._isReadonly;
  }
  set isReadonly(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_readonly"];
    this._session.updateSetProperty(this, prop, value);
    this._isReadonly = value;
  }
  _isReadonly: boolean | null;

  /**
   * Whether this property is the main property of the object.
   */
  /**
   * Whether this property is the main property of the object.
   */
  get isMain(): boolean | null {
    return this._isMain;
  }
  set isMain(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_main"];
    this._session.updateSetProperty(this, prop, value);
    this._isMain = value;
  }
  _isMain: boolean | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: CustomProperty | NodeReference | null;
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
    type?: PropertyType;
    icon?: Icon | null;
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    customDefinition?: Entity | NodeReference | null;
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
    isMain?: boolean | null;
    _session?: Session | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null
        ? options.parent.constructor.name == "NodeReference"
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      /* session */
      options._session ?? null,
      /* graph */
      options._graph ?? null,
      /* connection */
      options._connection ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
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
        throw new Error(`no active Space for CustomProperty`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`CustomProperty.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`CustomProperty.materialization is required`);
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
        throw new Error(`no active Branch for CustomProperty`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`CustomProperty.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for CustomProperty`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`CustomProperty.snapshot is required`);
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
      _name = "CustomProperty";
    }
    if (_name === null) {
      throw new Error(`CustomProperty.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomProperty.orderKey is required`);
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 1 /* PropertyType.MEMBER */;
    }
    if (_type === null) {
      throw new Error(`CustomProperty.type is required`);
    }
    this._type = _type;
    let _icon = options.icon ?? null;
    this._icon = _icon;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality === null) {
      throw new Error(`CustomProperty.cardinality is required`);
    }
    this._cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`CustomProperty.scalarType is required`);
    }
    this._scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this._primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this._enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this._nodeType = _nodeType;
    let _structType = options.structType ?? null;
    this._structType = _structType;
    let _customDefinition = options.customDefinition ?? null;
    if (_customDefinition != null && _customDefinition.constructor.name != "NodeReference") {
      _customDefinition = (_customDefinition as Node).toRef();
    }
    this._customDefinitionPtr = _customDefinition as NodeReference | null;
    let _keyType = options.keyType ?? null;
    this._keyType = _keyType;
    let _value = options.value ?? null;
    this._value = _value;
    let _valueFactory = options.valueFactory ?? null;
    this._valueFactory = _valueFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this._collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this._stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this._numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this._nodeConstraint = _nodeConstraint;
    let _edgeType = options.edgeType ?? null;
    this._edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this._cascade = _cascade;
    let _isRequired = options.isRequired ?? null;
    this._isRequired = _isRequired;
    let _isUnique = options.isUnique ?? null;
    this._isUnique = _isUnique;
    let _isComputed = options.isComputed ?? null;
    this._isComputed = _isComputed;
    let _isReadonly = options.isReadonly ?? null;
    this._isReadonly = _isReadonly;
    let _isMain = options.isMain ?? null;
    this._isMain = _isMain;

    /* identity */
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
          `CustomProperty.createdAt and CustomProperty.updatedAt are required for existing Nodes`,
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (
      (this._icon == null) !== (other._icon == null) ||
      (this._icon != null && !this._icon.equals(other._icon))
    ) {
      return false;
    }
    if (!(this._cardinality === other._cardinality)) {
      return false;
    }
    if (!(this._scalarType === other._scalarType)) {
      return false;
    }
    if (!(this._primitiveType === other._primitiveType)) {
      return false;
    }
    if (!(this._enumType === other._enumType)) {
      return false;
    }
    if (!(this._nodeType === other._nodeType)) {
      return false;
    }
    if (!(this._structType === other._structType)) {
      return false;
    }
    if (!(this._customDefinitionPtr?.id === other._customDefinitionPtr?.id)) {
      return false;
    }
    if (
      (this._keyType == null) !== (other._keyType == null) ||
      (this._keyType != null && !this._keyType.equals(other._keyType))
    ) {
      return false;
    }
    if (
      (this._value == null) !== (other._value == null) ||
      (this._value != null && !this._value.equals(other._value))
    ) {
      return false;
    }
    if (!(this._valueFactory === other._valueFactory)) {
      return false;
    }
    if (
      (this._collectionConstraint == null) !== (other._collectionConstraint == null) ||
      (this._collectionConstraint != null &&
        !this._collectionConstraint.equals(other._collectionConstraint))
    ) {
      return false;
    }
    if (
      (this._stringConstraint == null) !== (other._stringConstraint == null) ||
      (this._stringConstraint != null && !this._stringConstraint.equals(other._stringConstraint))
    ) {
      return false;
    }
    if (
      (this._numberConstraint == null) !== (other._numberConstraint == null) ||
      (this._numberConstraint != null && !this._numberConstraint.equals(other._numberConstraint))
    ) {
      return false;
    }
    if (
      (this._nodeConstraint == null) !== (other._nodeConstraint == null) ||
      (this._nodeConstraint != null && !this._nodeConstraint.equals(other._nodeConstraint))
    ) {
      return false;
    }
    if (!(this._edgeType === other._edgeType)) {
      return false;
    }
    if (!(this._cascade === other._cascade)) {
      return false;
    }
    if (!(this._isRequired === other._isRequired)) {
      return false;
    }
    if (!(this._isUnique === other._isUnique)) {
      return false;
    }
    if (!(this._isComputed === other._isComputed)) {
      return false;
    }
    if (!(this._isReadonly === other._isReadonly)) {
      return false;
    }
    if (!(this._isMain === other._isMain)) {
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
    h = (h * 31 + this._type) & 0xffffffff;
    if (this._icon != null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
    }
    h = (h * 31 + this._cardinality) & 0xffffffff;
    h = (h * 31 + this._scalarType) & 0xffffffff;
    if (this._primitiveType != null) {
      h = (h * 31 + this._primitiveType) & 0xffffffff;
    }
    if (this._enumType != null) {
      h = (h * 31 + this._enumType) & 0xffffffff;
    }
    if (this._nodeType != null) {
      h = (h * 31 + this._nodeType) & 0xffffffff;
    }
    if (this._structType != null) {
      h = (h * 31 + this._structType) & 0xffffffff;
    }
    if (this._customDefinitionPtr != null) {
      h = (h * 31 + hashString(this._customDefinitionPtr.id)) & 0xffffffff;
    }
    if (this._keyType != null) {
      h = (h * 31 + this._keyType.hash()) & 0xffffffff;
    }
    if (this._value != null) {
      h = (h * 31 + this._value.hash()) & 0xffffffff;
    }
    if (this._valueFactory != null) {
      h = (h * 31 + this._valueFactory) & 0xffffffff;
    }
    if (this._collectionConstraint != null) {
      h = (h * 31 + this._collectionConstraint.hash()) & 0xffffffff;
    }
    if (this._stringConstraint != null) {
      h = (h * 31 + this._stringConstraint.hash()) & 0xffffffff;
    }
    if (this._numberConstraint != null) {
      h = (h * 31 + this._numberConstraint.hash()) & 0xffffffff;
    }
    if (this._nodeConstraint != null) {
      h = (h * 31 + this._nodeConstraint.hash()) & 0xffffffff;
    }
    if (this._edgeType != null) {
      h = (h * 31 + this._edgeType) & 0xffffffff;
    }
    if (this._cascade != null) {
      h = (h * 31 + this._cascade) & 0xffffffff;
    }
    if (this._isRequired != null) {
      h = (h * 31 + hashBool(this._isRequired)) & 0xffffffff;
    }
    if (this._isUnique != null) {
      h = (h * 31 + hashBool(this._isUnique)) & 0xffffffff;
    }
    if (this._isComputed != null) {
      h = (h * 31 + hashBool(this._isComputed)) & 0xffffffff;
    }
    if (this._isReadonly != null) {
      h = (h * 31 + hashBool(this._isReadonly)) & 0xffffffff;
    }
    if (this._isMain != null) {
      h = (h * 31 + hashBool(this._isMain)) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
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
      type: NodeType.CUSTOM_PROPERTY,
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
    propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
    propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
    if (this.primitiveType != null) {
      propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
    }
    if (this.enumType != null) {
      propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
    }
    if (this.nodeType != null) {
      propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
    }
    if (this.structType != null) {
      propertyReprs.push(`structType=${StructType[this.structType]}`);
    }
    if (this.customDefinition != null) {
      propertyReprs.push(`customDefinition=${this.customDefinition?.repr()}`);
    }
    if (this.keyType != null) {
      propertyReprs.push(`keyType=${this.keyType.repr()}`);
    }
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<CustomProperty "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  toType(): Type {
    const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
    return new _Type({
      cardinality: this.cardinality,
      scalarType: this.scalarType,
      primitiveType: this.primitiveType,
      enumType: this.enumType,
      nodeType: this.nodeType,
      structType: this.structType,
      keyType: this.keyType,
      isRequired: this.isRequired,
      value: this.value,
      valueFactory: this.valueFactory,
      collectionConstraint: this.collectionConstraint,
      stringConstraint: this.stringConstraint,
      numberConstraint: this.numberConstraint,
      nodeConstraint: this.nodeConstraint,
    });
  }

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
/* ==== DESTACK_GENERATED_END:NODE:20300 ==== */
