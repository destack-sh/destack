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
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type {
  IsArchivable,
  IsCustomizable,
  IsDeletable,
  IsSourceable,
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
  implements IsTaggable, IsArchivable, IsDeletable, IsSourceable
{
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY;

  /**
   * CustomProperty.parent
   */
  get parent(): (Entity & IsCustomizable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
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
    if (nodePtr !== null) {
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
   * CustomProperty.name
   */
  /**
   * CustomProperty.name
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
  /**
   * CustomProperty.group
   */
  get groupPtr(): NodeReference | null {
    return this._groupPtr;
  }
  set groupPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["group"];
    this._session.updateSetProperty(this, prop, value);
    this._groupPtr = value;
  }
  _groupPtr: NodeReference | null;

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

  constructor(options: {
    id?: string;
    parent?: (Entity & IsCustomizable) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: CustomProperty | NodeReference | null;
    template?: CustomProperty | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsSubject) | NodeReference | null;
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
      if (this._session === null) {
        throw new Error(`CustomProperty has no session`);
      }
      if (this._session.spacePtr === null) {
        throw new Error(`CustomProperty has no space`);
      }
      _space = this._session.spacePtr;
    }
    if (_space === null) {
      throw new Error(`CustomProperty.space is required`);
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
    this._type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomProperty.name is required`);
    }
    this._name = _name;
    let _icon = options.icon ?? null;
    this._icon = _icon;
    let _group = options.group ?? null;
    if (_group != null && _group.metatype != StructType.NODE_REFERENCE) {
      _group = (_group as Node).toRef();
    }
    this._groupPtr = _group;
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
    if (!(this._type === other._type)) {
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
    if (!(this._groupPtr?.id === other._groupPtr?.id)) {
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
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._icon !== null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
    }
    if (this._groupPtr !== null) {
      h = (h * 31 + hashString(this._groupPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._cardinality) & 0xffffffff;
    h = (h * 31 + this._scalarType) & 0xffffffff;
    if (this._primitiveType !== null) {
      h = (h * 31 + this._primitiveType) & 0xffffffff;
    }
    if (this._enumType !== null) {
      h = (h * 31 + this._enumType) & 0xffffffff;
    }
    if (this._nodeType !== null) {
      h = (h * 31 + this._nodeType) & 0xffffffff;
    }
    if (this._structType !== null) {
      h = (h * 31 + this._structType) & 0xffffffff;
    }
    if (this._definitionPtr !== null) {
      h = (h * 31 + hashString(this._definitionPtr.id)) & 0xffffffff;
    }
    if (this._keyType !== null) {
      h = (h * 31 + this._keyType.hash()) & 0xffffffff;
    }
    if (this._value !== null) {
      h = (h * 31 + this._value.hash()) & 0xffffffff;
    }
    if (this._valueFactory !== null) {
      h = (h * 31 + this._valueFactory) & 0xffffffff;
    }
    if (this._collectionConstraint !== null) {
      h = (h * 31 + this._collectionConstraint.hash()) & 0xffffffff;
    }
    if (this._stringConstraint !== null) {
      h = (h * 31 + this._stringConstraint.hash()) & 0xffffffff;
    }
    if (this._numberConstraint !== null) {
      h = (h * 31 + this._numberConstraint.hash()) & 0xffffffff;
    }
    if (this._nodeConstraint !== null) {
      h = (h * 31 + this._nodeConstraint.hash()) & 0xffffffff;
    }
    if (this._edgeType !== null) {
      h = (h * 31 + this._edgeType) & 0xffffffff;
    }
    if (this._cascade !== null) {
      h = (h * 31 + this._cascade) & 0xffffffff;
    }
    if (this._isRequired !== null) {
      h = (h * 31 + hashBool(this._isRequired)) & 0xffffffff;
    }
    if (this._isUnique !== null) {
      h = (h * 31 + hashBool(this._isUnique)) & 0xffffffff;
    }
    if (this._isComputed !== null) {
      h = (h * 31 + hashBool(this._isComputed)) & 0xffffffff;
    }
    if (this._isReadonly !== null) {
      h = (h * 31 + hashBool(this._isReadonly)) & 0xffffffff;
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
    return `<CustomProperty "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return CustomProperty.__packValue__(this);
  }

  static __packValue__(object: CustomProperty): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 110;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
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
    objectValue["100"] = object._type;
    objectValue["101"] = object._name;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    if (object._groupPtr != null) {
      objectValue["105"] = object._groupPtr.toValue();
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
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
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
      archivedAt: unpackedArchivedAt,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      orderKey: objectValue["27"],
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
    const objectProto: Partial<CustomPropertyProto> = { metatype: 110 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
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
    objectProto.type = Number(object._type) as PropertyTypeProto;
    objectProto.name = object._name;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
    }
    if (object._groupPtr != null) {
      objectProto.groupPtr = object._groupPtr.toProto();
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
export class CustomPropertyGroup extends Entity implements IsArchivable, IsDeletable, IsSourceable {
  static metatype: NodeType = NodeType.CUSTOM_PROPERTY_GROUP;

  /**
   * CustomPropertyGroup.parent
   */
  get parent(): (Entity & IsCustomizable) | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
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
    if (nodePtr !== null) {
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
  /**
   * CustomPropertyGroup.name
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
   * CustomPropertyGroup.icon
   */
  /**
   * CustomPropertyGroup.icon
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

  constructor(options: {
    id?: string;
    parent?: (Entity & IsCustomizable) | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: CustomPropertyGroup | NodeReference | null;
    template?: CustomPropertyGroup | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsSubject) | NodeReference | null;
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
      if (this._session === null) {
        throw new Error(`CustomPropertyGroup has no session`);
      }
      if (this._session.spacePtr === null) {
        throw new Error(`CustomPropertyGroup has no space`);
      }
      _space = this._session.spacePtr;
    }
    if (_space === null) {
      throw new Error(`CustomPropertyGroup.space is required`);
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
    this._name = _name;
    let _icon = options.icon ?? null;
    this._icon = _icon;

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
    if (!(this._name === other._name)) {
      return false;
    }
    if (
      (this._icon == null) !== (other._icon == null) ||
      (this._icon != null && !this._icon.equals(other._icon))
    ) {
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
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._icon !== null) {
      h = (h * 31 + this._icon.hash()) & 0xffffffff;
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
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

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
    return `<CustomPropertyGroup "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return CustomPropertyGroup.__packValue__(this);
  }

  static __packValue__(object: CustomPropertyGroup): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 111;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
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
    objectValue["101"] = object._name;
    if (object._icon != null) {
      objectValue["102"] = object._icon.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
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
      archivedAt: unpackedArchivedAt,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      template: unpackedTemplatePtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      id: String(objectValue["2"]),
      orderKey: objectValue["27"],
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
    objectProto.spacePtr = object.spacePtr.toProto();
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
    objectProto.name = object._name;
    if (object._icon != null) {
      objectProto.icon = object._icon.toProto();
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
