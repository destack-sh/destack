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
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT, ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import type { CustomEvent } from "@destack/language/core/builtin/event";
import { Event } from "@destack/language/core/builtin/event";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsActor,
  IsCustomizable,
  IsExtensible,
  IsSourceable,
} from "@destack/language/core/builtin/trait";
import type { CustomEnum } from "@destack/language/core/common/enum";
import type { Icon } from "@destack/language/core/common/icon";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type { Space } from "@destack/language/core/common/space";
import type { CustomStruct } from "@destack/language/core/common/struct";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
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
import {
  CascadeActionProto,
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

/* ==== DESTACK_GENERATED_START:NODE:20300 ==== */
/**
 * A CustomProperty is a custom attribute of an IsCustomizable or IsExtensible.
 */
export class CustomProperty extends Entity implements IsSourceable {
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * CustomProperty.parent
   */
  get parent(): (Entity & IsCustomizable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsCustomizable) | null;
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
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
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
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
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
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
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
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
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
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
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
   * CustomProperty.definition
   */
  get definition(): (Entity & IsExtensible) | CustomEvent | CustomEnum | CustomStruct | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as
        | (Entity & IsExtensible)
        | CustomEvent
        | CustomEnum
        | CustomStruct
        | null;
    }
    return null;
  }
  set definition(node: (Entity & IsExtensible) | CustomEvent | CustomEnum | CustomStruct | null) {
    if (node === null) {
      this.definitionPtr = null;
    } else {
      this.definitionPtr = node.toRef();
    }
  }
  /**
   * CustomProperty.definition
   */
  get definitionPtr(): NodeReference | null {
    return this._definitionPtr;
  }
  set definitionPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["definition"];
    this._session.updateSetProperty(this, prop, value);
    this._definitionPtr = value;
  }
  _definitionPtr: NodeReference | null;

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
   * CustomProperty.isRequired
   */
  /**
   * CustomProperty.isRequired
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
   * CustomProperty.isUnique
   */
  /**
   * CustomProperty.isUnique
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
   * CustomProperty.isComputed
   */
  /**
   * CustomProperty.isComputed
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
   * CustomProperty.isReadonly
   */
  /**
   * CustomProperty.isReadonly
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
   * CustomProperty.isMain
   */
  /**
   * CustomProperty.isMain
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
    parent?: (Entity & IsCustomizable) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
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
    orderKey?: string;
    name?: string;
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
    definition?:
      | (Entity & IsExtensible)
      | CustomEvent
      | CustomEnum
      | CustomStruct
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
    isMain?: boolean | null;
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
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`CustomProperty.materialization is required`);
    }
    this.materialization = _materialization;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
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
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
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
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.metatype != StructType.NODE_REFERENCE) {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance;
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
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "CustomProperty";
    }
    if (_name === null) {
      throw new Error(`CustomProperty.name is required`);
    }
    this._name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
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
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this._definitionPtr = _definition;
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
          `CustomProperty.createdAt and CustomProperty.updatedAt are required for existing Nodes`,
        );
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (!(this._definitionPtr?.id === other._definitionPtr?.id)) {
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
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this._name === other._name)) {
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
    if (this._definitionPtr != null) {
      h = (h * 31 + hashString(this._definitionPtr.id)) & 0xffffffff;
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
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
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
      branchId: this.branchPtr?.id ?? null,
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
    if (this.definition != null) {
      propertyReprs.push(`definition=${this.definition?.repr()}`);
    }
    if (this.keyType != null) {
      propertyReprs.push(`keyType=${this.keyType.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<CustomProperty "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return CustomProperty.__packValue__(this);
  }

  static __packValue__(object: CustomProperty): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 20300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    objectValue["12"] = object.branchPtr.toValue();
    objectValue["13"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["14"] = object.precededByPtr.toValue();
    }
    if (object.instancePtr != null) {
      objectValue["15"] = object.instancePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectValue["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectValue["22"] = object.createdByPtr.toValue();
    }
    objectValue["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectValue["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectValue["25"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["31"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    if (object._key != null) {
      objectValue["70"] = object._key;
    }
    objectValue["100"] = object._type;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    objectValue["110"] = object._cardinality;
    objectValue["111"] = object._scalarType;
    if (object._primitiveType != null) {
      objectValue["112"] = object._primitiveType;
    }
    if (object._enumType != null) {
      objectValue["113"] = object._enumType;
    }
    if (object._nodeType != null) {
      objectValue["114"] = object._nodeType;
    }
    if (object._structType != null) {
      objectValue["115"] = object._structType;
    }
    if (object._definitionPtr != null) {
      objectValue["116"] = object._definitionPtr.toValue();
    }
    if (object._keyType != null) {
      objectValue["117"] = object._keyType.toValue();
    }
    if (object._value != null) {
      objectValue["120"] = object._value.toValue();
    }
    if (object._valueFactory != null) {
      objectValue["121"] = object._valueFactory;
    }
    if (object._collectionConstraint != null) {
      objectValue["130"] = object._collectionConstraint.toValue();
    }
    if (object._stringConstraint != null) {
      objectValue["131"] = object._stringConstraint.toValue();
    }
    if (object._numberConstraint != null) {
      objectValue["132"] = object._numberConstraint.toValue();
    }
    if (object._nodeConstraint != null) {
      objectValue["133"] = object._nodeConstraint.toValue();
    }
    if (object._edgeType != null) {
      objectValue["140"] = object._edgeType;
    }
    if (object._cascade != null) {
      objectValue["141"] = object._cascade;
    }
    if (object._isRequired != null) {
      objectValue["150"] = object._isRequired;
    }
    if (object._isUnique != null) {
      objectValue["151"] = object._isUnique;
    }
    if (object._isComputed != null) {
      objectValue["152"] = object._isComputed;
    }
    if (object._isReadonly != null) {
      objectValue["153"] = object._isReadonly;
    }
    if (object._isMain != null) {
      objectValue["154"] = object._isMain;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomProperty {
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
    const isMainValue = objectValue["154"];
    const unpackedIsMain = isMainValue != undefined ? isMainValue : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["70"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    const precededByPtrValue = objectValue["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectValue["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromValue(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    return new CustomProperty({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      icon: unpackedIcon,
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
      isMain: unpackedIsMain,
      source: unpackedSourcePtr,
      key: unpackedKey,
      materialization: Number(objectValue["10"]),
      branch: _NodeReference.fromValue(
        objectValue["12"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromValue(
        objectValue["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      orderKey: objectValue["31"],
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
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
  ): CustomProperty {
    return CustomProperty.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CustomPropertyProto {
    return CustomProperty.__packProto__(this);
  }

  static __packProto__(object: CustomProperty): CustomPropertyProto {
    const objectProto: Partial<CustomPropertyProto> = { metatype: 20300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
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
    objectProto.orderKey = object.orderKey;
    objectProto.name = object._name;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.type = Number(object._type) as PropertyTypeProto;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
    }
    objectProto.cardinality = Number(object._cardinality) as TypeCardinalityProto;
    objectProto.scalarType = Number(object._scalarType) as ScalarTypeProto;
    if (object._primitiveType != null) {
      objectProto.primitiveType = Number(object._primitiveType) as PrimitiveTypeProto;
    }
    if (object._enumType != null) {
      objectProto.enumType = Number(object._enumType) as EnumTypeProto;
    }
    if (object._nodeType != null) {
      objectProto.nodeType = Number(object._nodeType) as NodeTypeProto;
    }
    if (object._structType != null) {
      objectProto.structType = Number(object._structType) as StructTypeProto;
    }
    if (object._definitionPtr != null) {
      objectProto.definitionPtr = object._definitionPtr.toProto();
    }
    if (object._keyType != null) {
      objectProto.keyType = object._keyType.toProto();
    }
    if (object._value != null) {
      objectProto.value = object._value.toProto();
    }
    if (object._valueFactory != null) {
      objectProto.valueFactory = Number(object._valueFactory) as ValueFactoryProto;
    }
    if (object._collectionConstraint != null) {
      objectProto.collectionConstraint = object._collectionConstraint.toProto();
    }
    if (object._stringConstraint != null) {
      objectProto.stringConstraint = object._stringConstraint.toProto();
    }
    if (object._numberConstraint != null) {
      objectProto.numberConstraint = object._numberConstraint.toProto();
    }
    if (object._nodeConstraint != null) {
      objectProto.nodeConstraint = object._nodeConstraint.toProto();
    }
    if (object._edgeType != null) {
      objectProto.edgeType = Number(object._edgeType) as EdgeTypeProto;
    }
    if (object._cascade != null) {
      objectProto.cascade = Number(object._cascade) as CascadeActionProto;
    }
    if (object._isRequired != null) {
      objectProto.isRequired = object._isRequired;
    }
    if (object._isUnique != null) {
      objectProto.isUnique = object._isUnique;
    }
    if (object._isComputed != null) {
      objectProto.isComputed = object._isComputed;
    }
    if (object._isReadonly != null) {
      objectProto.isReadonly = object._isReadonly;
    }
    if (object._isMain != null) {
      objectProto.isMain = object._isMain;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
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
      isMain: objectProto.isMain != undefined ? objectProto.isMain : null,
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
      key: objectProto.key != undefined ? objectProto.key : null,
      materialization: Number(objectProto.materialization) as Materialization,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _supergraph,
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
              _supergraph,
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
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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

  toType(): Type {
    return new Type({
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
