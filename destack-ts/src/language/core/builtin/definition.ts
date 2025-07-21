import {
  CascadeAction,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  PropertyType,
  ScalarType,
  StoreDomain,
  StoreKey,
  StructType,
  TraitType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import { ConstraintType, IndexType } from "@destack/language/core/builtin/meta";
import type {
  ObjectDefinitionReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import {
  ObjectDefinitionType,
  PropertyReferenceType,
} from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { ActionDefinition } from "@destack/language/core/common/action";
import type { Icon } from "@destack/language/core/common/icon";
import type { MethodDefinition } from "@destack/language/core/common/method";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type {
  CollectionConstraint,
  NodeConstraint,
  NumberConstraint,
  StringConstraint,
  Type,
} from "@destack/language/core/common/type";
import type { Value } from "@destack/language/core/common/value";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  NODE_CLASS_BY_TYPE,
  STRUCT_CLASS_BY_TYPE,
  registerStructClass,
} from "@destack/language/registry";
import {
  CascadeActionProto,
  ConstantDefinitionProto,
  ConstraintDefinitionProto,
  ConstraintTypeProto,
  EdgeTypeProto,
  EnumDefinitionProto,
  EnumTypeProto,
  IndexDefinitionProto,
  IndexTypeProto,
  NodeDefinitionProto,
  NodeTypeProto,
  OptionDefinitionProto,
  PermissionDefinitionProto,
  PrimitiveTypeProto,
  PropertyDefinitionProto,
  PropertyTypeProto,
  ScalarTypeProto,
  StoreDomainProto,
  StoreKeyProto,
  StructDefinitionProto,
  StructTypeProto,
  TagDefinitionProto,
  TraitDefinitionProto,
  TraitTypeProto,
  TypeCardinalityProto,
  ValueFactoryProto,
} from "@destack/proto";
import { assertNever, base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:10 ==== */
/**
 * Definition of a builtin object.
 */
export abstract class BuiltinDefinition extends StructFrozen {
  static metatype: StructType = StructType.BUILTIN_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  declare readonly id: number;

  /**
   * BuiltinDefinition.name
   */
  declare readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  declare readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  declare readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  declare readonly taggings: readonly number[];

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.BUILTIN_DEFINITION, BuiltinDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:10 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:12 ==== */
/**
 * Definition of a builtin Node.
 */
export class NodeDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.NODE_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * NodeDefinition.type
   */
  readonly type: NodeType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  /**
   * Whether this Node cannot be instantiated directly.
   */
  readonly isAbstract: boolean;

  /**
   * Whether this Node can be extended by custom Nodes.
   */
  readonly isExtensible: boolean;

  /**
   * Whether this Node cannot be modified.
   */
  readonly isFrozen: boolean;

  /**
   * All properties of this Node (including inherited).
   */
  readonly properties: readonly PropertyDefinition[];

  /**
   * All indexes of this Node (including inherited).
   */
  readonly indexes: readonly IndexDefinition[];

  /**
   * All constraints of this Node (including inherited).
   */
  readonly constraints: readonly ConstraintDefinition[];

  /**
   * All permissions of this Node (including inherited).
   */
  readonly permissions: readonly PermissionDefinition[];

  /**
   * All methods of this Node (including inherited, excluding actions).
   */
  readonly methods: readonly MethodDefinition[];

  /**
   * All actions of this Node (including inherited).
   */
  readonly actions: readonly ActionDefinition[];

  /**
   * The base type this Node extends (directly).
   */
  readonly baseType: NodeType | null;

  /**
   * Nodes that extend this Node type (directly).
   */
  readonly extendedBy: readonly NodeType[];

  /**
   * Nodes that this Node inherits.
   */
  readonly inherits: readonly NodeType[];

  /**
   * Nodes that inherit this Node type.
   */
  readonly inheritedBy: readonly NodeType[];

  /**
   * Traits directly and indirectly inherited by this Node.
   */
  readonly traits: readonly TraitType[];

  /**
   * Traits directly inherited by this Node (directly).
   */
  readonly selfTraits: readonly TraitType[];

  /**
   * The event types related to this Node.
   */
  readonly eventTypes: readonly NodeType[];

  /**
   * The base event types related to this Node (directly).
   */
  readonly selfEventTypes: readonly NodeType[];

  /**
   * The enum types related to this Node.
   */
  readonly enumTypes: readonly EnumType[];

  /**
   * The base enum types related to this Node (directly).
   */
  readonly selfEnumTypes: readonly EnumType[];

  /**
   * The parent types of this Node type (directly).
   */
  readonly parentTypes: readonly NodeType[];

  /**
   * The child types of this Node type (directly).
   */
  readonly childTypes: readonly NodeType[];

  /**
   * The ancestor types of this Node type.
   */
  readonly ancestorTypes: readonly NodeType[];

  /**
   * The descendant types of this Node type.
   */
  readonly descendantTypes: readonly NodeType[];

  /**
   * The parent types expected for this Node type (any of).
   */
  readonly expectedParentTypes: readonly NodeType[];

  /**
   * The child types expected for this Node type (any of).
   */
  readonly expectedChildTypes: readonly NodeType[];

  /**
   * The ancestor types expected for this Node type (any of).
   */
  readonly expectedAncestorTypes: readonly NodeType[];

  /**
   * The descendant types expected for this Node type (any of).
   */
  readonly expectedDescendantTypes: readonly NodeType[];

  /**
   * NodeDefinition.primaryStoreKeys
   */
  readonly primaryStoreKeys: readonly StoreKey[];

  /**
   * NodeDefinition.storeDomain
   */
  readonly storeDomain: StoreDomain | null;

  constructor(options: {
    id: number;
    type: NodeType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    isAbstract: boolean;
    isExtensible: boolean;
    isFrozen: boolean;
    properties?: readonly PropertyDefinition[];
    indexes?: readonly IndexDefinition[];
    constraints?: readonly ConstraintDefinition[];
    permissions?: readonly PermissionDefinition[];
    methods?: readonly MethodDefinition[];
    actions?: readonly ActionDefinition[];
    baseType?: NodeType | null;
    extendedBy?: readonly NodeType[];
    inherits?: readonly NodeType[];
    inheritedBy?: readonly NodeType[];
    traits?: readonly TraitType[];
    selfTraits?: readonly TraitType[];
    eventTypes?: readonly NodeType[];
    selfEventTypes?: readonly NodeType[];
    enumTypes?: readonly EnumType[];
    selfEnumTypes?: readonly EnumType[];
    parentTypes?: readonly NodeType[];
    childTypes?: readonly NodeType[];
    ancestorTypes?: readonly NodeType[];
    descendantTypes?: readonly NodeType[];
    expectedParentTypes?: readonly NodeType[];
    expectedChildTypes?: readonly NodeType[];
    expectedAncestorTypes?: readonly NodeType[];
    expectedDescendantTypes?: readonly NodeType[];
    primaryStoreKeys?: readonly StoreKey[];
    storeDomain?: StoreDomain | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`NodeDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`NodeDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`NodeDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _isAbstract = options.isAbstract;
    if (_isAbstract === null) {
      throw new Error(`NodeDefinition.isAbstract is required`);
    }
    this.isAbstract = _isAbstract;
    let _isExtensible = options.isExtensible;
    if (_isExtensible === null) {
      throw new Error(`NodeDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _isFrozen = options.isFrozen;
    if (_isFrozen === null) {
      throw new Error(`NodeDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _indexes = options.indexes ?? null;
    if (_indexes === null) {
      _indexes = [];
    }
    this.indexes = _indexes;
    let _constraints = options.constraints ?? null;
    if (_constraints === null) {
      _constraints = [];
    }
    this.constraints = _constraints;
    let _permissions = options.permissions ?? null;
    if (_permissions === null) {
      _permissions = [];
    }
    this.permissions = _permissions;
    let _methods = options.methods ?? null;
    if (_methods === null) {
      _methods = [];
    }
    this.methods = _methods;
    let _actions = options.actions ?? null;
    if (_actions === null) {
      _actions = [];
    }
    this.actions = _actions;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _extendedBy = options.extendedBy ?? null;
    if (_extendedBy === null) {
      _extendedBy = [];
    }
    this.extendedBy = _extendedBy;
    let _inherits = options.inherits ?? null;
    if (_inherits === null) {
      _inherits = [];
    }
    this.inherits = _inherits;
    let _inheritedBy = options.inheritedBy ?? null;
    if (_inheritedBy === null) {
      _inheritedBy = [];
    }
    this.inheritedBy = _inheritedBy;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _selfTraits = options.selfTraits ?? null;
    if (_selfTraits === null) {
      _selfTraits = [];
    }
    this.selfTraits = _selfTraits;
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes === null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _selfEventTypes = options.selfEventTypes ?? null;
    if (_selfEventTypes === null) {
      _selfEventTypes = [];
    }
    this.selfEventTypes = _selfEventTypes;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes === null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes === null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;
    let _parentTypes = options.parentTypes ?? null;
    if (_parentTypes === null) {
      _parentTypes = [];
    }
    this.parentTypes = _parentTypes;
    let _childTypes = options.childTypes ?? null;
    if (_childTypes === null) {
      _childTypes = [];
    }
    this.childTypes = _childTypes;
    let _ancestorTypes = options.ancestorTypes ?? null;
    if (_ancestorTypes === null) {
      _ancestorTypes = [];
    }
    this.ancestorTypes = _ancestorTypes;
    let _descendantTypes = options.descendantTypes ?? null;
    if (_descendantTypes === null) {
      _descendantTypes = [];
    }
    this.descendantTypes = _descendantTypes;
    let _expectedParentTypes = options.expectedParentTypes ?? null;
    if (_expectedParentTypes === null) {
      _expectedParentTypes = [];
    }
    this.expectedParentTypes = _expectedParentTypes;
    let _expectedChildTypes = options.expectedChildTypes ?? null;
    if (_expectedChildTypes === null) {
      _expectedChildTypes = [];
    }
    this.expectedChildTypes = _expectedChildTypes;
    let _expectedAncestorTypes = options.expectedAncestorTypes ?? null;
    if (_expectedAncestorTypes === null) {
      _expectedAncestorTypes = [];
    }
    this.expectedAncestorTypes = _expectedAncestorTypes;
    let _expectedDescendantTypes = options.expectedDescendantTypes ?? null;
    if (_expectedDescendantTypes === null) {
      _expectedDescendantTypes = [];
    }
    this.expectedDescendantTypes = _expectedDescendantTypes;
    let _primaryStoreKeys = options.primaryStoreKeys ?? null;
    if (_primaryStoreKeys === null) {
      _primaryStoreKeys = [];
    }
    this.primaryStoreKeys = _primaryStoreKeys;
    let _storeDomain = options.storeDomain ?? null;
    this.storeDomain = _storeDomain;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.isAbstract === other.isAbstract)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.isFrozen === other.isFrozen)) {
      return false;
    }
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.indexes.length != other.indexes.length) {
      return false;
    }
    for (let i = 0; i < this.indexes.length; i++) {
      if (!this.indexes[i].equals(other.indexes[i])) {
        return false;
      }
    }
    if (this.constraints.length != other.constraints.length) {
      return false;
    }
    for (let i = 0; i < this.constraints.length; i++) {
      if (!this.constraints[i].equals(other.constraints[i])) {
        return false;
      }
    }
    if (this.permissions.length != other.permissions.length) {
      return false;
    }
    for (let i = 0; i < this.permissions.length; i++) {
      if (!this.permissions[i].equals(other.permissions[i])) {
        return false;
      }
    }
    if (this.methods.length != other.methods.length) {
      return false;
    }
    for (let i = 0; i < this.methods.length; i++) {
      if (!this.methods[i].equals(other.methods[i])) {
        return false;
      }
    }
    if (this.actions.length != other.actions.length) {
      return false;
    }
    for (let i = 0; i < this.actions.length; i++) {
      if (!this.actions[i].equals(other.actions[i])) {
        return false;
      }
    }
    if (!(this.baseType === other.baseType)) {
      return false;
    }
    if (this.extendedBy.length != other.extendedBy.length) {
      return false;
    }
    for (let i = 0; i < this.extendedBy.length; i++) {
      if (!(this.extendedBy[i] === other.extendedBy[i])) {
        return false;
      }
    }
    if (this.inherits.length != other.inherits.length) {
      return false;
    }
    for (let i = 0; i < this.inherits.length; i++) {
      if (!(this.inherits[i] === other.inherits[i])) {
        return false;
      }
    }
    if (this.inheritedBy.length != other.inheritedBy.length) {
      return false;
    }
    for (let i = 0; i < this.inheritedBy.length; i++) {
      if (!(this.inheritedBy[i] === other.inheritedBy[i])) {
        return false;
      }
    }
    if (this.traits.length != other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (this.selfTraits.length != other.selfTraits.length) {
      return false;
    }
    for (let i = 0; i < this.selfTraits.length; i++) {
      if (!(this.selfTraits[i] === other.selfTraits[i])) {
        return false;
      }
    }
    if (this.eventTypes.length != other.eventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.eventTypes.length; i++) {
      if (!(this.eventTypes[i] === other.eventTypes[i])) {
        return false;
      }
    }
    if (this.selfEventTypes.length != other.selfEventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.selfEventTypes.length; i++) {
      if (!(this.selfEventTypes[i] === other.selfEventTypes[i])) {
        return false;
      }
    }
    if (this.enumTypes.length != other.enumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.enumTypes.length; i++) {
      if (!(this.enumTypes[i] === other.enumTypes[i])) {
        return false;
      }
    }
    if (this.selfEnumTypes.length != other.selfEnumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.selfEnumTypes.length; i++) {
      if (!(this.selfEnumTypes[i] === other.selfEnumTypes[i])) {
        return false;
      }
    }
    if (this.parentTypes.length != other.parentTypes.length) {
      return false;
    }
    for (let i = 0; i < this.parentTypes.length; i++) {
      if (!(this.parentTypes[i] === other.parentTypes[i])) {
        return false;
      }
    }
    if (this.childTypes.length != other.childTypes.length) {
      return false;
    }
    for (let i = 0; i < this.childTypes.length; i++) {
      if (!(this.childTypes[i] === other.childTypes[i])) {
        return false;
      }
    }
    if (this.ancestorTypes.length != other.ancestorTypes.length) {
      return false;
    }
    for (let i = 0; i < this.ancestorTypes.length; i++) {
      if (!(this.ancestorTypes[i] === other.ancestorTypes[i])) {
        return false;
      }
    }
    if (this.descendantTypes.length != other.descendantTypes.length) {
      return false;
    }
    for (let i = 0; i < this.descendantTypes.length; i++) {
      if (!(this.descendantTypes[i] === other.descendantTypes[i])) {
        return false;
      }
    }
    if (this.expectedParentTypes.length != other.expectedParentTypes.length) {
      return false;
    }
    for (let i = 0; i < this.expectedParentTypes.length; i++) {
      if (!(this.expectedParentTypes[i] === other.expectedParentTypes[i])) {
        return false;
      }
    }
    if (this.expectedChildTypes.length != other.expectedChildTypes.length) {
      return false;
    }
    for (let i = 0; i < this.expectedChildTypes.length; i++) {
      if (!(this.expectedChildTypes[i] === other.expectedChildTypes[i])) {
        return false;
      }
    }
    if (this.expectedAncestorTypes.length != other.expectedAncestorTypes.length) {
      return false;
    }
    for (let i = 0; i < this.expectedAncestorTypes.length; i++) {
      if (!(this.expectedAncestorTypes[i] === other.expectedAncestorTypes[i])) {
        return false;
      }
    }
    if (this.expectedDescendantTypes.length != other.expectedDescendantTypes.length) {
      return false;
    }
    for (let i = 0; i < this.expectedDescendantTypes.length; i++) {
      if (!(this.expectedDescendantTypes[i] === other.expectedDescendantTypes[i])) {
        return false;
      }
    }
    if (this.primaryStoreKeys.length != other.primaryStoreKeys.length) {
      return false;
    }
    for (let i = 0; i < this.primaryStoreKeys.length; i++) {
      if (!(this.primaryStoreKeys[i] === other.primaryStoreKeys[i])) {
        return false;
      }
    }
    if (!(this.storeDomain === other.storeDomain)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${NodeType[this.type]}`);
      propertyReprs.push(`isAbstract=${this.isAbstract}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      propertyReprs.push(`isFrozen=${this.isFrozen}`);
      if (this.baseType != null) {
        propertyReprs.push(`baseType=${NodeType[this.baseType]}`);
      }
      if (this.parentTypes.length > 0) {
        propertyReprs.push(
          `parentTypes=${this.parentTypes.map((_item) => NodeType[_item]).join(", ")}`,
        );
      }
      if (this.childTypes.length > 0) {
        propertyReprs.push(
          `childTypes=${this.childTypes.map((_item) => NodeType[_item]).join(", ")}`,
        );
      }
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<NodeDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.indexes && this.indexes.length > 0) {
      for (const _item of this.indexes) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.constraints && this.constraints.length > 0) {
      for (const _item of this.constraints) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.permissions && this.permissions.length > 0) {
      for (const _item of this.permissions) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.methods && this.methods.length > 0) {
      for (const _item of this.methods) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.actions && this.actions.length > 0) {
      for (const _item of this.actions) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.baseType != null) {
      h = (h * 31 + this.baseType) & 0xffffffff;
    }
    if (this.extendedBy && this.extendedBy.length > 0) {
      for (const _item of this.extendedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.inherits && this.inherits.length > 0) {
      for (const _item of this.inherits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.inheritedBy && this.inheritedBy.length > 0) {
      for (const _item of this.inheritedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfTraits && this.selfTraits.length > 0) {
      for (const _item of this.selfTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.eventTypes && this.eventTypes.length > 0) {
      for (const _item of this.eventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfEventTypes && this.selfEventTypes.length > 0) {
      for (const _item of this.selfEventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.enumTypes && this.enumTypes.length > 0) {
      for (const _item of this.enumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfEnumTypes && this.selfEnumTypes.length > 0) {
      for (const _item of this.selfEnumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.parentTypes && this.parentTypes.length > 0) {
      for (const _item of this.parentTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.childTypes && this.childTypes.length > 0) {
      for (const _item of this.childTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.ancestorTypes && this.ancestorTypes.length > 0) {
      for (const _item of this.ancestorTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.descendantTypes && this.descendantTypes.length > 0) {
      for (const _item of this.descendantTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.expectedParentTypes && this.expectedParentTypes.length > 0) {
      for (const _item of this.expectedParentTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.expectedChildTypes && this.expectedChildTypes.length > 0) {
      for (const _item of this.expectedChildTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.expectedAncestorTypes && this.expectedAncestorTypes.length > 0) {
      for (const _item of this.expectedAncestorTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.expectedDescendantTypes && this.expectedDescendantTypes.length > 0) {
      for (const _item of this.expectedDescendantTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.primaryStoreKeys && this.primaryStoreKeys.length > 0) {
      for (const _item of this.primaryStoreKeys) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.storeDomain != null) {
      h = (h * 31 + this.storeDomain) & 0xffffffff;
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = NodeDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: NodeDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 12;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    objectCson["110"] = object.isAbstract;
    objectCson["111"] = object.isExtensible;
    objectCson["112"] = object.isFrozen;
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toCson());
      }
      objectCson["120"] = packedProperties;
    }
    if (object.indexes.length > 0) {
      const packedIndexes: any[] = [];
      for (const item of object.indexes) {
        packedIndexes.push(item.toCson());
      }
      objectCson["121"] = packedIndexes;
    }
    if (object.constraints.length > 0) {
      const packedConstraints: any[] = [];
      for (const item of object.constraints) {
        packedConstraints.push(item.toCson());
      }
      objectCson["122"] = packedConstraints;
    }
    if (object.permissions.length > 0) {
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.toCson());
      }
      objectCson["123"] = packedPermissions;
    }
    if (object.methods.length > 0) {
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.toCson());
      }
      objectCson["125"] = packedMethods;
    }
    if (object.actions.length > 0) {
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.toCson());
      }
      objectCson["126"] = packedActions;
    }
    if (object.baseType != null) {
      objectCson["130"] = object.baseType;
    }
    if (object.extendedBy.length > 0) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectCson["131"] = packedExtendedBy;
    }
    if (object.inherits.length > 0) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectCson["132"] = packedInherits;
    }
    if (object.inheritedBy.length > 0) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectCson["133"] = packedInheritedBy;
    }
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectCson["134"] = packedTraits;
    }
    if (object.selfTraits.length > 0) {
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(item);
      }
      objectCson["135"] = packedSelfTraits;
    }
    if (object.eventTypes.length > 0) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectCson["140"] = packedEventTypes;
    }
    if (object.selfEventTypes.length > 0) {
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(item);
      }
      objectCson["141"] = packedSelfEventTypes;
    }
    if (object.enumTypes.length > 0) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
    }
    if (object.selfEnumTypes.length > 0) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
    }
    if (object.parentTypes.length > 0) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(item);
      }
      objectCson["160"] = packedParentTypes;
    }
    if (object.childTypes.length > 0) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(item);
      }
      objectCson["161"] = packedChildTypes;
    }
    if (object.ancestorTypes.length > 0) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(item);
      }
      objectCson["162"] = packedAncestorTypes;
    }
    if (object.descendantTypes.length > 0) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(item);
      }
      objectCson["163"] = packedDescendantTypes;
    }
    if (object.expectedParentTypes.length > 0) {
      const packedExpectedParentTypes: any[] = [];
      for (const item of object.expectedParentTypes) {
        packedExpectedParentTypes.push(item);
      }
      objectCson["170"] = packedExpectedParentTypes;
    }
    if (object.expectedChildTypes.length > 0) {
      const packedExpectedChildTypes: any[] = [];
      for (const item of object.expectedChildTypes) {
        packedExpectedChildTypes.push(item);
      }
      objectCson["171"] = packedExpectedChildTypes;
    }
    if (object.expectedAncestorTypes.length > 0) {
      const packedExpectedAncestorTypes: any[] = [];
      for (const item of object.expectedAncestorTypes) {
        packedExpectedAncestorTypes.push(item);
      }
      objectCson["172"] = packedExpectedAncestorTypes;
    }
    if (object.expectedDescendantTypes.length > 0) {
      const packedExpectedDescendantTypes: any[] = [];
      for (const item of object.expectedDescendantTypes) {
        packedExpectedDescendantTypes.push(item);
      }
      objectCson["173"] = packedExpectedDescendantTypes;
    }
    if (object.primaryStoreKeys.length > 0) {
      const packedPrimaryStoreKeys: any[] = [];
      for (const item of object.primaryStoreKeys) {
        packedPrimaryStoreKeys.push(item);
      }
      objectCson["200"] = packedPrimaryStoreKeys;
    }
    if (object.storeDomain != null) {
      objectCson["201"] = object.storeDomain;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _IndexDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.INDEX_DEFINITION
    ] as typeof IndexDefinition;
    const _ConstraintDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.CONSTRAINT_DEFINITION
    ] as typeof ConstraintDefinition;
    const _MethodDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.METHOD_DEFINITION
    ] as typeof MethodDefinition;
    const _ActionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.ACTION_DEFINITION
    ] as typeof ActionDefinition;
    const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PERMISSION_DEFINITION
    ] as typeof PermissionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectCson["120"] != undefined) {
      for (const item of objectCson["120"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedIndexes: any[] = [];
    if (objectCson["121"] != undefined) {
      for (const item of objectCson["121"]) {
        unpackedIndexes.push(
          _IndexDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedConstraints: any[] = [];
    if (objectCson["122"] != undefined) {
      for (const item of objectCson["122"]) {
        unpackedConstraints.push(
          _ConstraintDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedPermissions: any[] = [];
    if (objectCson["123"] != undefined) {
      for (const item of objectCson["123"]) {
        unpackedPermissions.push(
          _PermissionDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedMethods: any[] = [];
    if (objectCson["125"] != undefined) {
      for (const item of objectCson["125"]) {
        unpackedMethods.push(
          _MethodDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedActions: any[] = [];
    if (objectCson["126"] != undefined) {
      for (const item of objectCson["126"]) {
        unpackedActions.push(
          _ActionDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const baseTypeValue = objectCson["130"];
    const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : null;
    const unpackedExtendedBy: any[] = [];
    if (objectCson["131"] != undefined) {
      for (const item of objectCson["131"]) {
        unpackedExtendedBy.push(Number(item));
      }
    }
    const unpackedInherits: any[] = [];
    if (objectCson["132"] != undefined) {
      for (const item of objectCson["132"]) {
        unpackedInherits.push(Number(item));
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectCson["133"] != undefined) {
      for (const item of objectCson["133"]) {
        unpackedInheritedBy.push(Number(item));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectCson["134"] != undefined) {
      for (const item of objectCson["134"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const unpackedSelfTraits: any[] = [];
    if (objectCson["135"] != undefined) {
      for (const item of objectCson["135"]) {
        unpackedSelfTraits.push(Number(item));
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectCson["140"] != undefined) {
      for (const item of objectCson["140"]) {
        unpackedEventTypes.push(Number(item));
      }
    }
    const unpackedSelfEventTypes: any[] = [];
    if (objectCson["141"] != undefined) {
      for (const item of objectCson["141"]) {
        unpackedSelfEventTypes.push(Number(item));
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectCson["150"] != undefined) {
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectCson["151"] != undefined) {
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
    }
    const unpackedParentTypes: any[] = [];
    if (objectCson["160"] != undefined) {
      for (const item of objectCson["160"]) {
        unpackedParentTypes.push(Number(item));
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectCson["161"] != undefined) {
      for (const item of objectCson["161"]) {
        unpackedChildTypes.push(Number(item));
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectCson["162"] != undefined) {
      for (const item of objectCson["162"]) {
        unpackedAncestorTypes.push(Number(item));
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectCson["163"] != undefined) {
      for (const item of objectCson["163"]) {
        unpackedDescendantTypes.push(Number(item));
      }
    }
    const unpackedExpectedParentTypes: any[] = [];
    if (objectCson["170"] != undefined) {
      for (const item of objectCson["170"]) {
        unpackedExpectedParentTypes.push(Number(item));
      }
    }
    const unpackedExpectedChildTypes: any[] = [];
    if (objectCson["171"] != undefined) {
      for (const item of objectCson["171"]) {
        unpackedExpectedChildTypes.push(Number(item));
      }
    }
    const unpackedExpectedAncestorTypes: any[] = [];
    if (objectCson["172"] != undefined) {
      for (const item of objectCson["172"]) {
        unpackedExpectedAncestorTypes.push(Number(item));
      }
    }
    const unpackedExpectedDescendantTypes: any[] = [];
    if (objectCson["173"] != undefined) {
      for (const item of objectCson["173"]) {
        unpackedExpectedDescendantTypes.push(Number(item));
      }
    }
    const unpackedPrimaryStoreKeys: any[] = [];
    if (objectCson["200"] != undefined) {
      for (const item of objectCson["200"]) {
        unpackedPrimaryStoreKeys.push(Number(item));
      }
    }
    const storeDomainValue = objectCson["201"];
    const unpackedStoreDomain = storeDomainValue != undefined ? Number(storeDomainValue) : null;
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new NodeDefinition({
      type: Number(objectCson["100"]),
      isAbstract: objectCson["110"],
      isExtensible: objectCson["111"],
      isFrozen: objectCson["112"],
      properties: unpackedProperties,
      indexes: unpackedIndexes,
      constraints: unpackedConstraints,
      permissions: unpackedPermissions,
      methods: unpackedMethods,
      actions: unpackedActions,
      baseType: unpackedBaseType,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      traits: unpackedTraits,
      selfTraits: unpackedSelfTraits,
      eventTypes: unpackedEventTypes,
      selfEventTypes: unpackedSelfEventTypes,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      expectedParentTypes: unpackedExpectedParentTypes,
      expectedChildTypes: unpackedExpectedChildTypes,
      expectedAncestorTypes: unpackedExpectedAncestorTypes,
      expectedDescendantTypes: unpackedExpectedDescendantTypes,
      primaryStoreKeys: unpackedPrimaryStoreKeys,
      storeDomain: unpackedStoreDomain,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    return NodeDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): NodeDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeDefinition.__packProto__(this);
    }
    return this._proto as NodeDefinitionProto;
  }

  static __packProto__(object: NodeDefinition): NodeDefinitionProto {
    const objectProto: Partial<NodeDefinitionProto> = { metatype: 12 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as NodeTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    objectProto.isAbstract = object.isAbstract;
    objectProto.isExtensible = object.isExtensible;
    objectProto.isFrozen = object.isFrozen;
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.indexes) {
      const packedIndexes: any[] = [];
      for (const item of object.indexes) {
        packedIndexes.push(item.toProto());
      }
      objectProto.indexes = packedIndexes;
    }
    if (object.constraints) {
      const packedConstraints: any[] = [];
      for (const item of object.constraints) {
        packedConstraints.push(item.toProto());
      }
      objectProto.constraints = packedConstraints;
    }
    if (object.permissions) {
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.toProto());
      }
      objectProto.permissions = packedPermissions;
    }
    if (object.methods) {
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.toProto());
      }
      objectProto.methods = packedMethods;
    }
    if (object.actions) {
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.toProto());
      }
      objectProto.actions = packedActions;
    }
    if (object.baseType != null) {
      objectProto.baseType = Number(object.baseType) as NodeTypeProto;
    }
    if (object.extendedBy) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(Number(item) as NodeTypeProto);
      }
      objectProto.extendedBy = packedExtendedBy;
    }
    if (object.inherits) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(Number(item) as NodeTypeProto);
      }
      objectProto.inherits = packedInherits;
    }
    if (object.inheritedBy) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(Number(item) as NodeTypeProto);
      }
      objectProto.inheritedBy = packedInheritedBy;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.selfTraits) {
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.selfTraits = packedSelfTraits;
    }
    if (object.eventTypes) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.eventTypes = packedEventTypes;
    }
    if (object.selfEventTypes) {
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.selfEventTypes = packedSelfEventTypes;
    }
    if (object.enumTypes) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.enumTypes = packedEnumTypes;
    }
    if (object.selfEnumTypes) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.selfEnumTypes = packedSelfEnumTypes;
    }
    if (object.parentTypes) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.parentTypes = packedParentTypes;
    }
    if (object.childTypes) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.childTypes = packedChildTypes;
    }
    if (object.ancestorTypes) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.ancestorTypes = packedAncestorTypes;
    }
    if (object.descendantTypes) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.descendantTypes = packedDescendantTypes;
    }
    if (object.expectedParentTypes) {
      const packedExpectedParentTypes: any[] = [];
      for (const item of object.expectedParentTypes) {
        packedExpectedParentTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.expectedParentTypes = packedExpectedParentTypes;
    }
    if (object.expectedChildTypes) {
      const packedExpectedChildTypes: any[] = [];
      for (const item of object.expectedChildTypes) {
        packedExpectedChildTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.expectedChildTypes = packedExpectedChildTypes;
    }
    if (object.expectedAncestorTypes) {
      const packedExpectedAncestorTypes: any[] = [];
      for (const item of object.expectedAncestorTypes) {
        packedExpectedAncestorTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.expectedAncestorTypes = packedExpectedAncestorTypes;
    }
    if (object.expectedDescendantTypes) {
      const packedExpectedDescendantTypes: any[] = [];
      for (const item of object.expectedDescendantTypes) {
        packedExpectedDescendantTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.expectedDescendantTypes = packedExpectedDescendantTypes;
    }
    if (object.primaryStoreKeys) {
      const packedPrimaryStoreKeys: any[] = [];
      for (const item of object.primaryStoreKeys) {
        packedPrimaryStoreKeys.push(Number(item) as StoreKeyProto);
      }
      objectProto.primaryStoreKeys = packedPrimaryStoreKeys;
    }
    if (object.storeDomain != null) {
      objectProto.storeDomain = Number(object.storeDomain) as StoreDomainProto;
    }
    return objectProto as NodeDefinitionProto;
  }

  static __unpackProto__(
    objectProto: NodeDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _IndexDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.INDEX_DEFINITION
    ] as typeof IndexDefinition;
    const _ConstraintDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.CONSTRAINT_DEFINITION
    ] as typeof ConstraintDefinition;
    const _MethodDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.METHOD_DEFINITION
    ] as typeof MethodDefinition;
    const _ActionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.ACTION_DEFINITION
    ] as typeof ActionDefinition;
    const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PERMISSION_DEFINITION
    ] as typeof PermissionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedIndexes: any[] = [];
    if (objectProto.indexes) {
      for (const item of objectProto.indexes) {
        unpackedIndexes.push(
          _IndexDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedConstraints: any[] = [];
    if (objectProto.constraints) {
      for (const item of objectProto.constraints) {
        unpackedConstraints.push(
          _ConstraintDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedPermissions: any[] = [];
    if (objectProto.permissions) {
      for (const item of objectProto.permissions) {
        unpackedPermissions.push(
          _PermissionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedMethods: any[] = [];
    if (objectProto.methods) {
      for (const item of objectProto.methods) {
        unpackedMethods.push(
          _MethodDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedActions: any[] = [];
    if (objectProto.actions) {
      for (const item of objectProto.actions) {
        unpackedActions.push(
          _ActionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedExtendedBy: any[] = [];
    if (objectProto.extendedBy) {
      for (const item of objectProto.extendedBy) {
        unpackedExtendedBy.push(Number(item) as NodeType);
      }
    }
    const unpackedInherits: any[] = [];
    if (objectProto.inherits) {
      for (const item of objectProto.inherits) {
        unpackedInherits.push(Number(item) as NodeType);
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectProto.inheritedBy) {
      for (const item of objectProto.inheritedBy) {
        unpackedInheritedBy.push(Number(item) as NodeType);
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedSelfTraits: any[] = [];
    if (objectProto.selfTraits) {
      for (const item of objectProto.selfTraits) {
        unpackedSelfTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectProto.eventTypes) {
      for (const item of objectProto.eventTypes) {
        unpackedEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedSelfEventTypes: any[] = [];
    if (objectProto.selfEventTypes) {
      for (const item of objectProto.selfEventTypes) {
        unpackedSelfEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectProto.enumTypes) {
      for (const item of objectProto.enumTypes) {
        unpackedEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectProto.selfEnumTypes) {
      for (const item of objectProto.selfEnumTypes) {
        unpackedSelfEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedParentTypes: any[] = [];
    if (objectProto.parentTypes) {
      for (const item of objectProto.parentTypes) {
        unpackedParentTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectProto.childTypes) {
      for (const item of objectProto.childTypes) {
        unpackedChildTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectProto.ancestorTypes) {
      for (const item of objectProto.ancestorTypes) {
        unpackedAncestorTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectProto.descendantTypes) {
      for (const item of objectProto.descendantTypes) {
        unpackedDescendantTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedExpectedParentTypes: any[] = [];
    if (objectProto.expectedParentTypes) {
      for (const item of objectProto.expectedParentTypes) {
        unpackedExpectedParentTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedExpectedChildTypes: any[] = [];
    if (objectProto.expectedChildTypes) {
      for (const item of objectProto.expectedChildTypes) {
        unpackedExpectedChildTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedExpectedAncestorTypes: any[] = [];
    if (objectProto.expectedAncestorTypes) {
      for (const item of objectProto.expectedAncestorTypes) {
        unpackedExpectedAncestorTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedExpectedDescendantTypes: any[] = [];
    if (objectProto.expectedDescendantTypes) {
      for (const item of objectProto.expectedDescendantTypes) {
        unpackedExpectedDescendantTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedPrimaryStoreKeys: any[] = [];
    if (objectProto.primaryStoreKeys) {
      for (const item of objectProto.primaryStoreKeys) {
        unpackedPrimaryStoreKeys.push(Number(item) as StoreKey);
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new NodeDefinition({
      type: Number(objectProto.type) as NodeType,
      isAbstract: objectProto.isAbstract,
      isExtensible: objectProto.isExtensible,
      isFrozen: objectProto.isFrozen,
      properties: unpackedProperties,
      indexes: unpackedIndexes,
      constraints: unpackedConstraints,
      permissions: unpackedPermissions,
      methods: unpackedMethods,
      actions: unpackedActions,
      baseType:
        objectProto.baseType != undefined ? (Number(objectProto.baseType) as NodeType) : null,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      traits: unpackedTraits,
      selfTraits: unpackedSelfTraits,
      eventTypes: unpackedEventTypes,
      selfEventTypes: unpackedSelfEventTypes,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      expectedParentTypes: unpackedExpectedParentTypes,
      expectedChildTypes: unpackedExpectedChildTypes,
      expectedAncestorTypes: unpackedExpectedAncestorTypes,
      expectedDescendantTypes: unpackedExpectedDescendantTypes,
      primaryStoreKeys: unpackedPrimaryStoreKeys,
      storeDomain:
        objectProto.storeDomain != undefined
          ? (Number(objectProto.storeDomain) as StoreDomain)
          : null,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NodeDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    return NodeDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): NodeDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NodeDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Resolve a Property in this definition. */
  resolvePropertyMaybe(key: string | number): PropertyDefinition | null {
    const nodeClass = NODE_CLASS_BY_TYPE[this.type];
    if (typeof key == "string") {
      return nodeClass.__propertiesByAlias__[key];
    } else if (typeof key == "number") {
      return nodeClass.__propertiesById__[key];
    } else {
      assertNever(key);
    }
  }

  /** Resolve a Property in this definition (error if not found). */
  resolveProperty(key: string | number): PropertyDefinition {
    const property = this.resolvePropertyMaybe(key);
    if (property == null) {
      throw new Error(`could not find property ${key} in ${this.repr()}`);
    }
    return property;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_DEFINITION, NodeDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:12 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:14 ==== */
/**
 * Definition of a builtin Trait.
 */
export class TraitDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.TRAIT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * TraitDefinition.type
   */
  readonly type: TraitType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  /**
   * TraitDefinition.alias
   */
  readonly alias: string;

  /**
   * Whether this Trait can be extended by custom Nodes and custom Traits.
   */
  readonly isExtensible: boolean;

  /**
   * All permissions of this Trait.
   */
  readonly permissions: readonly PermissionDefinition[];

  /**
   * Traits directly inherited by this Trait (directly).
   */
  readonly selfTraits: readonly TraitType[];

  /**
   * Traits directly and indirectly inherited by this Trait.
   */
  readonly traits: readonly TraitType[];

  /**
   * The event types related to this Trait.
   */
  readonly eventTypes: readonly NodeType[];

  /**
   * The base event types related to this Trait (directly).
   */
  readonly selfEventTypes: readonly NodeType[];

  /**
   * The enum types related to this Trait.
   */
  readonly enumTypes: readonly EnumType[];

  /**
   * The base enum types related to this Trait (directly).
   */
  readonly selfEnumTypes: readonly EnumType[];

  constructor(options: {
    id: number;
    type: TraitType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    alias: string;
    isExtensible: boolean;
    permissions?: readonly PermissionDefinition[];
    selfTraits?: readonly TraitType[];
    traits?: readonly TraitType[];
    eventTypes?: readonly NodeType[];
    selfEventTypes?: readonly NodeType[];
    enumTypes?: readonly EnumType[];
    selfEnumTypes?: readonly EnumType[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`TraitDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`TraitDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`TraitDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _alias = options.alias;
    if (_alias === null) {
      throw new Error(`TraitDefinition.alias is required`);
    }
    this.alias = _alias;
    let _isExtensible = options.isExtensible;
    if (_isExtensible === null) {
      throw new Error(`TraitDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _permissions = options.permissions ?? null;
    if (_permissions === null) {
      _permissions = [];
    }
    this.permissions = _permissions;
    let _selfTraits = options.selfTraits ?? null;
    if (_selfTraits === null) {
      _selfTraits = [];
    }
    this.selfTraits = _selfTraits;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes === null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _selfEventTypes = options.selfEventTypes ?? null;
    if (_selfEventTypes === null) {
      _selfEventTypes = [];
    }
    this.selfEventTypes = _selfEventTypes;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes === null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes === null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.alias === other.alias)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (this.permissions.length != other.permissions.length) {
      return false;
    }
    for (let i = 0; i < this.permissions.length; i++) {
      if (!this.permissions[i].equals(other.permissions[i])) {
        return false;
      }
    }
    if (this.selfTraits.length != other.selfTraits.length) {
      return false;
    }
    for (let i = 0; i < this.selfTraits.length; i++) {
      if (!(this.selfTraits[i] === other.selfTraits[i])) {
        return false;
      }
    }
    if (this.traits.length != other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (this.eventTypes.length != other.eventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.eventTypes.length; i++) {
      if (!(this.eventTypes[i] === other.eventTypes[i])) {
        return false;
      }
    }
    if (this.selfEventTypes.length != other.selfEventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.selfEventTypes.length; i++) {
      if (!(this.selfEventTypes[i] === other.selfEventTypes[i])) {
        return false;
      }
    }
    if (this.enumTypes.length != other.enumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.enumTypes.length; i++) {
      if (!(this.enumTypes[i] === other.enumTypes[i])) {
        return false;
      }
    }
    if (this.selfEnumTypes.length != other.selfEnumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.selfEnumTypes.length; i++) {
      if (!(this.selfEnumTypes[i] === other.selfEnumTypes[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${TraitType[this.type]}`);
      propertyReprs.push(`alias=${`"${this.alias}"`}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<TraitDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.alias)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.permissions && this.permissions.length > 0) {
      for (const _item of this.permissions) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.selfTraits && this.selfTraits.length > 0) {
      for (const _item of this.selfTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.eventTypes && this.eventTypes.length > 0) {
      for (const _item of this.eventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfEventTypes && this.selfEventTypes.length > 0) {
      for (const _item of this.selfEventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.enumTypes && this.enumTypes.length > 0) {
      for (const _item of this.enumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfEnumTypes && this.selfEnumTypes.length > 0) {
      for (const _item of this.selfEnumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = TraitDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: TraitDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 14;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    objectCson["110"] = object.alias;
    objectCson["111"] = object.isExtensible;
    if (object.permissions.length > 0) {
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.toCson());
      }
      objectCson["123"] = packedPermissions;
    }
    if (object.selfTraits.length > 0) {
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(item);
      }
      objectCson["130"] = packedSelfTraits;
    }
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectCson["131"] = packedTraits;
    }
    if (object.eventTypes.length > 0) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectCson["140"] = packedEventTypes;
    }
    if (object.selfEventTypes.length > 0) {
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(item);
      }
      objectCson["141"] = packedSelfEventTypes;
    }
    if (object.enumTypes.length > 0) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
    }
    if (object.selfEnumTypes.length > 0) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PERMISSION_DEFINITION
    ] as typeof PermissionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedPermissions: any[] = [];
    if (objectCson["123"] != undefined) {
      for (const item of objectCson["123"]) {
        unpackedPermissions.push(
          _PermissionDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSelfTraits: any[] = [];
    if (objectCson["130"] != undefined) {
      for (const item of objectCson["130"]) {
        unpackedSelfTraits.push(Number(item));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectCson["131"] != undefined) {
      for (const item of objectCson["131"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectCson["140"] != undefined) {
      for (const item of objectCson["140"]) {
        unpackedEventTypes.push(Number(item));
      }
    }
    const unpackedSelfEventTypes: any[] = [];
    if (objectCson["141"] != undefined) {
      for (const item of objectCson["141"]) {
        unpackedSelfEventTypes.push(Number(item));
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectCson["150"] != undefined) {
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectCson["151"] != undefined) {
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new TraitDefinition({
      type: Number(objectCson["100"]),
      alias: objectCson["110"],
      isExtensible: objectCson["111"],
      permissions: unpackedPermissions,
      selfTraits: unpackedSelfTraits,
      traits: unpackedTraits,
      eventTypes: unpackedEventTypes,
      selfEventTypes: unpackedSelfEventTypes,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    return TraitDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): TraitDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = TraitDefinition.__packProto__(this);
    }
    return this._proto as TraitDefinitionProto;
  }

  static __packProto__(object: TraitDefinition): TraitDefinitionProto {
    const objectProto: Partial<TraitDefinitionProto> = { metatype: 14 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as TraitTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    objectProto.alias = object.alias;
    objectProto.isExtensible = object.isExtensible;
    if (object.permissions) {
      const packedPermissions: any[] = [];
      for (const item of object.permissions) {
        packedPermissions.push(item.toProto());
      }
      objectProto.permissions = packedPermissions;
    }
    if (object.selfTraits) {
      const packedSelfTraits: any[] = [];
      for (const item of object.selfTraits) {
        packedSelfTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.selfTraits = packedSelfTraits;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.eventTypes) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.eventTypes = packedEventTypes;
    }
    if (object.selfEventTypes) {
      const packedSelfEventTypes: any[] = [];
      for (const item of object.selfEventTypes) {
        packedSelfEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.selfEventTypes = packedSelfEventTypes;
    }
    if (object.enumTypes) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.enumTypes = packedEnumTypes;
    }
    if (object.selfEnumTypes) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.selfEnumTypes = packedSelfEnumTypes;
    }
    return objectProto as TraitDefinitionProto;
  }

  static __unpackProto__(
    objectProto: TraitDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    const _PermissionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PERMISSION_DEFINITION
    ] as typeof PermissionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedPermissions: any[] = [];
    if (objectProto.permissions) {
      for (const item of objectProto.permissions) {
        unpackedPermissions.push(
          _PermissionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedSelfTraits: any[] = [];
    if (objectProto.selfTraits) {
      for (const item of objectProto.selfTraits) {
        unpackedSelfTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectProto.eventTypes) {
      for (const item of objectProto.eventTypes) {
        unpackedEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedSelfEventTypes: any[] = [];
    if (objectProto.selfEventTypes) {
      for (const item of objectProto.selfEventTypes) {
        unpackedSelfEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectProto.enumTypes) {
      for (const item of objectProto.enumTypes) {
        unpackedEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectProto.selfEnumTypes) {
      for (const item of objectProto.selfEnumTypes) {
        unpackedSelfEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new TraitDefinition({
      type: Number(objectProto.type) as TraitType,
      alias: objectProto.alias,
      isExtensible: objectProto.isExtensible,
      permissions: unpackedPermissions,
      selfTraits: unpackedSelfTraits,
      traits: unpackedTraits,
      eventTypes: unpackedEventTypes,
      selfEventTypes: unpackedSelfEventTypes,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: TraitDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    return TraitDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): TraitDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TraitDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TRAIT_DEFINITION, TraitDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:14 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:15 ==== */
/**
 * Definition of a builtin Struct.
 */
export class StructDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.STRUCT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * StructDefinition.type
   */
  readonly type: StructType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  /**
   * Whether this Struct is read-only (cannot be modified).
   */
  readonly isFrozen: boolean;

  /**
   * Whether this Struct is abstract (cannot be instantiated directly).
   */
  readonly isAbstract: boolean;

  /**
   * Whether this Struct can be extended by custom Structs.
   */
  readonly isExtensible: boolean;

  /**
   * All properties of this Struct.
   */
  readonly properties: readonly PropertyDefinition[];

  /**
   * All methods of this Struct (excluding actions).
   */
  readonly methods: readonly MethodDefinition[];

  /**
   * All actions of this Struct.
   */
  readonly actions: readonly ActionDefinition[];

  /**
   * StructDefinition.tags
   */
  readonly tags: readonly TagDefinition[];

  /**
   * The base type this Struct extends (directly).
   */
  readonly baseType: StructType | null;

  /**
   * Structs that extend this Struct type (directly).
   */
  readonly extendedBy: readonly StructType[];

  /**
   * Structs that this Struct inherits.
   */
  readonly inherits: readonly StructType[];

  /**
   * Structs that inherit this Struct type.
   */
  readonly inheritedBy: readonly StructType[];

  /**
   * The enum types related to this Node.
   */
  readonly enumTypes: readonly EnumType[];

  /**
   * The base enum types related to this Node (directly).
   */
  readonly selfEnumTypes: readonly EnumType[];

  constructor(options: {
    id: number;
    type: StructType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    isFrozen: boolean;
    isAbstract: boolean;
    isExtensible: boolean;
    properties?: readonly PropertyDefinition[];
    methods?: readonly MethodDefinition[];
    actions?: readonly ActionDefinition[];
    tags?: readonly TagDefinition[];
    baseType?: StructType | null;
    extendedBy?: readonly StructType[];
    inherits?: readonly StructType[];
    inheritedBy?: readonly StructType[];
    enumTypes?: readonly EnumType[];
    selfEnumTypes?: readonly EnumType[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`StructDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`StructDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`StructDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _isFrozen = options.isFrozen;
    if (_isFrozen === null) {
      throw new Error(`StructDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;
    let _isAbstract = options.isAbstract;
    if (_isAbstract === null) {
      throw new Error(`StructDefinition.isAbstract is required`);
    }
    this.isAbstract = _isAbstract;
    let _isExtensible = options.isExtensible;
    if (_isExtensible === null) {
      throw new Error(`StructDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _methods = options.methods ?? null;
    if (_methods === null) {
      _methods = [];
    }
    this.methods = _methods;
    let _actions = options.actions ?? null;
    if (_actions === null) {
      _actions = [];
    }
    this.actions = _actions;
    let _tags = options.tags ?? null;
    if (_tags === null) {
      _tags = [];
    }
    this.tags = _tags;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _extendedBy = options.extendedBy ?? null;
    if (_extendedBy === null) {
      _extendedBy = [];
    }
    this.extendedBy = _extendedBy;
    let _inherits = options.inherits ?? null;
    if (_inherits === null) {
      _inherits = [];
    }
    this.inherits = _inherits;
    let _inheritedBy = options.inheritedBy ?? null;
    if (_inheritedBy === null) {
      _inheritedBy = [];
    }
    this.inheritedBy = _inheritedBy;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes === null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes === null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.isFrozen === other.isFrozen)) {
      return false;
    }
    if (!(this.isAbstract === other.isAbstract)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.methods.length != other.methods.length) {
      return false;
    }
    for (let i = 0; i < this.methods.length; i++) {
      if (!this.methods[i].equals(other.methods[i])) {
        return false;
      }
    }
    if (this.actions.length != other.actions.length) {
      return false;
    }
    for (let i = 0; i < this.actions.length; i++) {
      if (!this.actions[i].equals(other.actions[i])) {
        return false;
      }
    }
    if (this.tags.length != other.tags.length) {
      return false;
    }
    for (let i = 0; i < this.tags.length; i++) {
      if (!this.tags[i].equals(other.tags[i])) {
        return false;
      }
    }
    if (!(this.baseType === other.baseType)) {
      return false;
    }
    if (this.extendedBy.length != other.extendedBy.length) {
      return false;
    }
    for (let i = 0; i < this.extendedBy.length; i++) {
      if (!(this.extendedBy[i] === other.extendedBy[i])) {
        return false;
      }
    }
    if (this.inherits.length != other.inherits.length) {
      return false;
    }
    for (let i = 0; i < this.inherits.length; i++) {
      if (!(this.inherits[i] === other.inherits[i])) {
        return false;
      }
    }
    if (this.inheritedBy.length != other.inheritedBy.length) {
      return false;
    }
    for (let i = 0; i < this.inheritedBy.length; i++) {
      if (!(this.inheritedBy[i] === other.inheritedBy[i])) {
        return false;
      }
    }
    if (this.enumTypes.length != other.enumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.enumTypes.length; i++) {
      if (!(this.enumTypes[i] === other.enumTypes[i])) {
        return false;
      }
    }
    if (this.selfEnumTypes.length != other.selfEnumTypes.length) {
      return false;
    }
    for (let i = 0; i < this.selfEnumTypes.length; i++) {
      if (!(this.selfEnumTypes[i] === other.selfEnumTypes[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${StructType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<StructDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.methods && this.methods.length > 0) {
      for (const _item of this.methods) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.actions && this.actions.length > 0) {
      for (const _item of this.actions) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.tags && this.tags.length > 0) {
      for (const _item of this.tags) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.baseType != null) {
      h = (h * 31 + this.baseType) & 0xffffffff;
    }
    if (this.extendedBy && this.extendedBy.length > 0) {
      for (const _item of this.extendedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.inherits && this.inherits.length > 0) {
      for (const _item of this.inherits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.inheritedBy && this.inheritedBy.length > 0) {
      for (const _item of this.inheritedBy) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.enumTypes && this.enumTypes.length > 0) {
      for (const _item of this.enumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.selfEnumTypes && this.selfEnumTypes.length > 0) {
      for (const _item of this.selfEnumTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = StructDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: StructDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 15;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    objectCson["110"] = object.isFrozen;
    objectCson["111"] = object.isAbstract;
    objectCson["112"] = object.isExtensible;
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toCson());
      }
      objectCson["120"] = packedProperties;
    }
    if (object.methods.length > 0) {
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.toCson());
      }
      objectCson["125"] = packedMethods;
    }
    if (object.actions.length > 0) {
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.toCson());
      }
      objectCson["126"] = packedActions;
    }
    if (object.tags.length > 0) {
      const packedTags: any[] = [];
      for (const item of object.tags) {
        packedTags.push(item.toCson());
      }
      objectCson["129"] = packedTags;
    }
    if (object.baseType != null) {
      objectCson["130"] = object.baseType;
    }
    if (object.extendedBy.length > 0) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectCson["131"] = packedExtendedBy;
    }
    if (object.inherits.length > 0) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectCson["132"] = packedInherits;
    }
    if (object.inheritedBy.length > 0) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectCson["133"] = packedInheritedBy;
    }
    if (object.enumTypes.length > 0) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(item);
      }
      objectCson["150"] = packedEnumTypes;
    }
    if (object.selfEnumTypes.length > 0) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(item);
      }
      objectCson["151"] = packedSelfEnumTypes;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _TagDefinition = STRUCT_CLASS_BY_TYPE[StructType.TAG_DEFINITION] as typeof TagDefinition;
    const _MethodDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.METHOD_DEFINITION
    ] as typeof MethodDefinition;
    const _ActionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.ACTION_DEFINITION
    ] as typeof ActionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectCson["120"] != undefined) {
      for (const item of objectCson["120"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedMethods: any[] = [];
    if (objectCson["125"] != undefined) {
      for (const item of objectCson["125"]) {
        unpackedMethods.push(
          _MethodDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedActions: any[] = [];
    if (objectCson["126"] != undefined) {
      for (const item of objectCson["126"]) {
        unpackedActions.push(
          _ActionDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTags: any[] = [];
    if (objectCson["129"] != undefined) {
      for (const item of objectCson["129"]) {
        unpackedTags.push(
          _TagDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const baseTypeValue = objectCson["130"];
    const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : null;
    const unpackedExtendedBy: any[] = [];
    if (objectCson["131"] != undefined) {
      for (const item of objectCson["131"]) {
        unpackedExtendedBy.push(Number(item));
      }
    }
    const unpackedInherits: any[] = [];
    if (objectCson["132"] != undefined) {
      for (const item of objectCson["132"]) {
        unpackedInherits.push(Number(item));
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectCson["133"] != undefined) {
      for (const item of objectCson["133"]) {
        unpackedInheritedBy.push(Number(item));
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectCson["150"] != undefined) {
      for (const item of objectCson["150"]) {
        unpackedEnumTypes.push(Number(item));
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectCson["151"] != undefined) {
      for (const item of objectCson["151"]) {
        unpackedSelfEnumTypes.push(Number(item));
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new StructDefinition({
      type: Number(objectCson["100"]),
      isFrozen: objectCson["110"],
      isAbstract: objectCson["111"],
      isExtensible: objectCson["112"],
      properties: unpackedProperties,
      methods: unpackedMethods,
      actions: unpackedActions,
      tags: unpackedTags,
      baseType: unpackedBaseType,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    return StructDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): StructDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StructDefinition.__packProto__(this);
    }
    return this._proto as StructDefinitionProto;
  }

  static __packProto__(object: StructDefinition): StructDefinitionProto {
    const objectProto: Partial<StructDefinitionProto> = { metatype: 15 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as StructTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    objectProto.isFrozen = object.isFrozen;
    objectProto.isAbstract = object.isAbstract;
    objectProto.isExtensible = object.isExtensible;
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.methods) {
      const packedMethods: any[] = [];
      for (const item of object.methods) {
        packedMethods.push(item.toProto());
      }
      objectProto.methods = packedMethods;
    }
    if (object.actions) {
      const packedActions: any[] = [];
      for (const item of object.actions) {
        packedActions.push(item.toProto());
      }
      objectProto.actions = packedActions;
    }
    if (object.tags) {
      const packedTags: any[] = [];
      for (const item of object.tags) {
        packedTags.push(item.toProto());
      }
      objectProto.tags = packedTags;
    }
    if (object.baseType != null) {
      objectProto.baseType = Number(object.baseType) as StructTypeProto;
    }
    if (object.extendedBy) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(Number(item) as StructTypeProto);
      }
      objectProto.extendedBy = packedExtendedBy;
    }
    if (object.inherits) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(Number(item) as StructTypeProto);
      }
      objectProto.inherits = packedInherits;
    }
    if (object.inheritedBy) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(Number(item) as StructTypeProto);
      }
      objectProto.inheritedBy = packedInheritedBy;
    }
    if (object.enumTypes) {
      const packedEnumTypes: any[] = [];
      for (const item of object.enumTypes) {
        packedEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.enumTypes = packedEnumTypes;
    }
    if (object.selfEnumTypes) {
      const packedSelfEnumTypes: any[] = [];
      for (const item of object.selfEnumTypes) {
        packedSelfEnumTypes.push(Number(item) as EnumTypeProto);
      }
      objectProto.selfEnumTypes = packedSelfEnumTypes;
    }
    return objectProto as StructDefinitionProto;
  }

  static __unpackProto__(
    objectProto: StructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _TagDefinition = STRUCT_CLASS_BY_TYPE[StructType.TAG_DEFINITION] as typeof TagDefinition;
    const _MethodDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.METHOD_DEFINITION
    ] as typeof MethodDefinition;
    const _ActionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.ACTION_DEFINITION
    ] as typeof ActionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedMethods: any[] = [];
    if (objectProto.methods) {
      for (const item of objectProto.methods) {
        unpackedMethods.push(
          _MethodDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedActions: any[] = [];
    if (objectProto.actions) {
      for (const item of objectProto.actions) {
        unpackedActions.push(
          _ActionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTags: any[] = [];
    if (objectProto.tags) {
      for (const item of objectProto.tags) {
        unpackedTags.push(
          _TagDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedExtendedBy: any[] = [];
    if (objectProto.extendedBy) {
      for (const item of objectProto.extendedBy) {
        unpackedExtendedBy.push(Number(item) as StructType);
      }
    }
    const unpackedInherits: any[] = [];
    if (objectProto.inherits) {
      for (const item of objectProto.inherits) {
        unpackedInherits.push(Number(item) as StructType);
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectProto.inheritedBy) {
      for (const item of objectProto.inheritedBy) {
        unpackedInheritedBy.push(Number(item) as StructType);
      }
    }
    const unpackedEnumTypes: any[] = [];
    if (objectProto.enumTypes) {
      for (const item of objectProto.enumTypes) {
        unpackedEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedSelfEnumTypes: any[] = [];
    if (objectProto.selfEnumTypes) {
      for (const item of objectProto.selfEnumTypes) {
        unpackedSelfEnumTypes.push(Number(item) as EnumType);
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new StructDefinition({
      type: Number(objectProto.type) as StructType,
      isFrozen: objectProto.isFrozen,
      isAbstract: objectProto.isAbstract,
      isExtensible: objectProto.isExtensible,
      properties: unpackedProperties,
      methods: unpackedMethods,
      actions: unpackedActions,
      tags: unpackedTags,
      baseType:
        objectProto.baseType != undefined ? (Number(objectProto.baseType) as StructType) : null,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      enumTypes: unpackedEnumTypes,
      selfEnumTypes: unpackedSelfEnumTypes,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    return StructDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): StructDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StructDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRUCT_DEFINITION, StructDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:15 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:17 ==== */
/**
 * Definition of a builtin Enum.
 */
export class EnumDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.ENUM_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * EnumDefinition.type
   */
  readonly type: EnumType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * EnumDefinition.options
   */
  readonly options: readonly OptionDefinition[];

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    options?: readonly OptionDefinition[];
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`EnumDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`EnumDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`EnumDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _options = options.options ?? null;
    if (_options === null) {
      _options = [];
    }
    this.options = _options;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.options.length != other.options.length) {
      return false;
    }
    for (let i = 0; i < this.options.length; i++) {
      if (!this.options[i].equals(other.options[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<EnumDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.options && this.options.length > 0) {
      for (const _item of this.options) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = EnumDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: EnumDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 17;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.options.length > 0) {
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.toCson());
      }
      objectCson["104"] = packedOptions;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    const _OptionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.OPTION_DEFINITION
    ] as typeof OptionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedOptions: any[] = [];
    if (objectCson["104"] != undefined) {
      for (const item of objectCson["104"]) {
        unpackedOptions.push(
          _OptionDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new EnumDefinition({
      type: Number(objectCson["100"]),
      options: unpackedOptions,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    return EnumDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): EnumDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = EnumDefinition.__packProto__(this);
    }
    return this._proto as EnumDefinitionProto;
  }

  static __packProto__(object: EnumDefinition): EnumDefinitionProto {
    const objectProto: Partial<EnumDefinitionProto> = { metatype: 17 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.options) {
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.toProto());
      }
      objectProto.options = packedOptions;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as EnumDefinitionProto;
  }

  static __unpackProto__(
    objectProto: EnumDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    const _OptionDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.OPTION_DEFINITION
    ] as typeof OptionDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedOptions: any[] = [];
    if (objectProto.options) {
      for (const item of objectProto.options) {
        unpackedOptions.push(
          _OptionDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new EnumDefinition({
      type: Number(objectProto.type) as EnumType,
      options: unpackedOptions,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: EnumDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    return EnumDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): EnumDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = EnumDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ENUM_DEFINITION, EnumDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:17 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:18 ==== */
/**
 * Definition of a builtin Property.
 */
export class PropertyDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.PROPERTY_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * PropertyDefinition.type
   */
  readonly type: PropertyType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * The object that this property is defined on.
   */
  readonly object: ObjectDefinitionReference;

  /**
   * The original object that this property was defined on.
   */
  readonly originalObject: ObjectDefinitionReference;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  /**
   * PropertyDefinition.cardinality
   */
  readonly cardinality: TypeCardinality;

  /**
   * PropertyDefinition.scalarType
   */
  readonly scalarType: ScalarType;

  /**
   * PropertyDefinition.primitiveType
   */
  readonly primitiveType: PrimitiveType | null;

  /**
   * PropertyDefinition.enumType
   */
  readonly enumType: EnumType | null;

  /**
   * PropertyDefinition.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * PropertyDefinition.structType
   */
  readonly structType: StructType | null;

  /**
   * PropertyDefinition.keyType
   */
  readonly keyType: Type | null;

  /**
   * PropertyDefinition.value
   */
  readonly value: Value | null;

  /**
   * PropertyDefinition.valueFactory
   */
  readonly valueFactory: ValueFactory | null;

  /**
   * PropertyDefinition.collectionConstraint
   */
  readonly collectionConstraint: CollectionConstraint | null;

  /**
   * PropertyDefinition.stringConstraint
   */
  readonly stringConstraint: StringConstraint | null;

  /**
   * PropertyDefinition.numberConstraint
   */
  readonly numberConstraint: NumberConstraint | null;

  /**
   * PropertyDefinition.nodeConstraint
   */
  readonly nodeConstraint: NodeConstraint | null;

  /**
   * PropertyDefinition.edgeType
   */
  readonly edgeType: EdgeType | null;

  /**
   * PropertyDefinition.cascade
   */
  readonly cascade: CascadeAction | null;

  /**
   * Whether this property must be set.
   */
  readonly isRequired: boolean;

  /**
   * Whether this property must have a unique value.
   */
  readonly isUnique: boolean;

  /**
   * Whether this property is read-only.
   */
  readonly isReadonly: boolean;

  /**
   * PropertyDefinition.isMain
   */
  readonly isMain: boolean;

  /**
   * PropertyDefinition.isWired
   */
  readonly isWired: boolean;

  /**
   * PropertyDefinition.isStored
   */
  readonly isStored: boolean;

  /**
   * PropertyDefinition.isRepr
   */
  readonly isRepr: boolean;

  /**
   * PropertyDefinition.isHash
   */
  readonly isHash: boolean;

  /**
   * PropertyDefinition.isEq
   */
  readonly isEq: boolean;

  /**
   * PropertyDefinition.isInternal
   */
  readonly isInternal: boolean;

  constructor(options: {
    id: number;
    type: PropertyType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    object: ObjectDefinitionReference;
    originalObject: ObjectDefinitionReference;
    taggings?: readonly number[];
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeType?: NodeType | null;
    structType?: StructType | null;
    keyType?: Type | null;
    value?: Value | null;
    valueFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    nodeConstraint?: NodeConstraint | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isRequired: boolean;
    isUnique: boolean;
    isReadonly: boolean;
    isMain: boolean;
    isWired: boolean;
    isStored: boolean;
    isRepr: boolean;
    isHash: boolean;
    isEq: boolean;
    isInternal: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PropertyDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`PropertyDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PropertyDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _object = options.object;
    if (_object === null) {
      throw new Error(`PropertyDefinition.object is required`);
    }
    this.object = _object;
    let _originalObject = options.originalObject;
    if (_originalObject === null) {
      throw new Error(`PropertyDefinition.originalObject is required`);
    }
    this.originalObject = _originalObject;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* TypeCardinality.SCALAR */;
    }
    if (_cardinality === null) {
      throw new Error(`PropertyDefinition.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _scalarType = options.scalarType;
    if (_scalarType === null) {
      throw new Error(`PropertyDefinition.scalarType is required`);
    }
    this.scalarType = _scalarType;
    let _primitiveType = options.primitiveType ?? null;
    this.primitiveType = _primitiveType;
    let _enumType = options.enumType ?? null;
    this.enumType = _enumType;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _keyType = options.keyType ?? null;
    this.keyType = _keyType;
    let _value = options.value ?? null;
    this.value = _value;
    let _valueFactory = options.valueFactory ?? null;
    this.valueFactory = _valueFactory;
    let _collectionConstraint = options.collectionConstraint ?? null;
    this.collectionConstraint = _collectionConstraint;
    let _stringConstraint = options.stringConstraint ?? null;
    this.stringConstraint = _stringConstraint;
    let _numberConstraint = options.numberConstraint ?? null;
    this.numberConstraint = _numberConstraint;
    let _nodeConstraint = options.nodeConstraint ?? null;
    this.nodeConstraint = _nodeConstraint;
    let _edgeType = options.edgeType ?? null;
    this.edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.cascade = _cascade;
    let _isRequired = options.isRequired;
    if (_isRequired === null) {
      throw new Error(`PropertyDefinition.isRequired is required`);
    }
    this.isRequired = _isRequired;
    let _isUnique = options.isUnique;
    if (_isUnique === null) {
      throw new Error(`PropertyDefinition.isUnique is required`);
    }
    this.isUnique = _isUnique;
    let _isReadonly = options.isReadonly;
    if (_isReadonly === null) {
      throw new Error(`PropertyDefinition.isReadonly is required`);
    }
    this.isReadonly = _isReadonly;
    let _isMain = options.isMain;
    if (_isMain === null) {
      throw new Error(`PropertyDefinition.isMain is required`);
    }
    this.isMain = _isMain;
    let _isWired = options.isWired;
    if (_isWired === null) {
      throw new Error(`PropertyDefinition.isWired is required`);
    }
    this.isWired = _isWired;
    let _isStored = options.isStored;
    if (_isStored === null) {
      throw new Error(`PropertyDefinition.isStored is required`);
    }
    this.isStored = _isStored;
    let _isRepr = options.isRepr;
    if (_isRepr === null) {
      throw new Error(`PropertyDefinition.isRepr is required`);
    }
    this.isRepr = _isRepr;
    let _isHash = options.isHash;
    if (_isHash === null) {
      throw new Error(`PropertyDefinition.isHash is required`);
    }
    this.isHash = _isHash;
    let _isEq = options.isEq;
    if (_isEq === null) {
      throw new Error(`PropertyDefinition.isEq is required`);
    }
    this.isEq = _isEq;
    let _isInternal = options.isInternal;
    if (_isInternal === null) {
      throw new Error(`PropertyDefinition.isInternal is required`);
    }
    this.isInternal = _isInternal;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!this.object.equals(other.object)) {
      return false;
    }
    if (!this.originalObject.equals(other.originalObject)) {
      return false;
    }
    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (!(this.scalarType === other.scalarType)) {
      return false;
    }
    if (!(this.primitiveType === other.primitiveType)) {
      return false;
    }
    if (!(this.enumType === other.enumType)) {
      return false;
    }
    if (!(this.nodeType === other.nodeType)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (
      (this.keyType == null) !== (other.keyType == null) ||
      (this.keyType != null && !this.keyType.equals(other.keyType))
    ) {
      return false;
    }
    if (
      (this.value == null) !== (other.value == null) ||
      (this.value != null && !this.value.equals(other.value))
    ) {
      return false;
    }
    if (!(this.valueFactory === other.valueFactory)) {
      return false;
    }
    if (
      (this.collectionConstraint == null) !== (other.collectionConstraint == null) ||
      (this.collectionConstraint != null &&
        !this.collectionConstraint.equals(other.collectionConstraint))
    ) {
      return false;
    }
    if (
      (this.stringConstraint == null) !== (other.stringConstraint == null) ||
      (this.stringConstraint != null && !this.stringConstraint.equals(other.stringConstraint))
    ) {
      return false;
    }
    if (
      (this.numberConstraint == null) !== (other.numberConstraint == null) ||
      (this.numberConstraint != null && !this.numberConstraint.equals(other.numberConstraint))
    ) {
      return false;
    }
    if (
      (this.nodeConstraint == null) !== (other.nodeConstraint == null) ||
      (this.nodeConstraint != null && !this.nodeConstraint.equals(other.nodeConstraint))
    ) {
      return false;
    }
    if (!(this.edgeType === other.edgeType)) {
      return false;
    }
    if (!(this.cascade === other.cascade)) {
      return false;
    }
    if (!(this.isRequired === other.isRequired)) {
      return false;
    }
    if (!(this.isUnique === other.isUnique)) {
      return false;
    }
    if (!(this.isReadonly === other.isReadonly)) {
      return false;
    }
    if (!(this.isMain === other.isMain)) {
      return false;
    }
    if (!(this.isWired === other.isWired)) {
      return false;
    }
    if (!(this.isStored === other.isStored)) {
      return false;
    }
    if (!(this.isRepr === other.isRepr)) {
      return false;
    }
    if (!(this.isHash === other.isHash)) {
      return false;
    }
    if (!(this.isEq === other.isEq)) {
      return false;
    }
    if (!(this.isInternal === other.isInternal)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
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
      if (this.keyType != null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.value != null) {
        propertyReprs.push(`value=${this.value.repr()}`);
      }
      if (this.valueFactory != null) {
        propertyReprs.push(`valueFactory=${ValueFactory[this.valueFactory]}`);
      }
      propertyReprs.push(`isRequired=${this.isRequired}`);
      propertyReprs.push(`isUnique=${this.isUnique}`);
      propertyReprs.push(`isReadonly=${this.isReadonly}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PropertyDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.object.hash()) & 0xffffffff;
    h = (h * 31 + this.originalObject.hash()) & 0xffffffff;
    h = (h * 31 + this.cardinality) & 0xffffffff;
    h = (h * 31 + this.scalarType) & 0xffffffff;
    if (this.primitiveType != null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType != null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeType != null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.structType != null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.keyType != null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.value != null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.valueFactory != null) {
      h = (h * 31 + this.valueFactory) & 0xffffffff;
    }
    if (this.collectionConstraint != null) {
      h = (h * 31 + this.collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.stringConstraint != null) {
      h = (h * 31 + this.stringConstraint.hash()) & 0xffffffff;
    }
    if (this.numberConstraint != null) {
      h = (h * 31 + this.numberConstraint.hash()) & 0xffffffff;
    }
    if (this.nodeConstraint != null) {
      h = (h * 31 + this.nodeConstraint.hash()) & 0xffffffff;
    }
    if (this.edgeType != null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade != null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isUnique)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isReadonly)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isMain)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isWired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isStored)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepr)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isHash)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isEq)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isInternal)) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = PropertyDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: PropertyDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 18;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    objectCson["104"] = object.object.toCson();
    objectCson["105"] = object.originalObject.toCson();
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    objectCson["110"] = object.cardinality;
    objectCson["111"] = object.scalarType;
    if (object.primitiveType != null) {
      objectCson["112"] = object.primitiveType;
    }
    if (object.enumType != null) {
      objectCson["113"] = object.enumType;
    }
    if (object.nodeType != null) {
      objectCson["114"] = object.nodeType;
    }
    if (object.structType != null) {
      objectCson["115"] = object.structType;
    }
    if (object.keyType != null) {
      objectCson["116"] = object.keyType.toCson();
    }
    if (object.value != null) {
      objectCson["120"] = object.value.toCson();
    }
    if (object.valueFactory != null) {
      objectCson["121"] = object.valueFactory;
    }
    if (object.collectionConstraint != null) {
      objectCson["130"] = object.collectionConstraint.toCson();
    }
    if (object.stringConstraint != null) {
      objectCson["131"] = object.stringConstraint.toCson();
    }
    if (object.numberConstraint != null) {
      objectCson["132"] = object.numberConstraint.toCson();
    }
    if (object.nodeConstraint != null) {
      objectCson["133"] = object.nodeConstraint.toCson();
    }
    if (object.edgeType != null) {
      objectCson["140"] = object.edgeType;
    }
    if (object.cascade != null) {
      objectCson["141"] = object.cascade;
    }
    objectCson["150"] = object.isRequired;
    objectCson["151"] = object.isUnique;
    objectCson["153"] = object.isReadonly;
    objectCson["154"] = object.isMain;
    objectCson["160"] = object.isWired;
    objectCson["161"] = object.isStored;
    objectCson["162"] = object.isRepr;
    objectCson["163"] = object.isHash;
    objectCson["164"] = object.isEq;
    objectCson["165"] = object.isInternal;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    const _ObjectDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.OBJECT_DEFINITION_REFERENCE
    ] as typeof ObjectDefinitionReference;
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
    const primitiveTypeValue = objectCson["112"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectCson["113"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectCson["114"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectCson["115"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const keyTypeValue = objectCson["116"];
    const unpackedKeyType =
      keyTypeValue != undefined
        ? _Type.fromCson(keyTypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueValue = objectCson["120"];
    const unpackedValue =
      valueValue != undefined
        ? _Value.fromCson(valueValue, _session, _supergraph, _graph, _connection)
        : null;
    const valueFactoryValue = objectCson["121"];
    const unpackedValueFactory = valueFactoryValue != undefined ? Number(valueFactoryValue) : null;
    const collectionConstraintValue = objectCson["130"];
    const unpackedCollectionConstraint =
      collectionConstraintValue != undefined
        ? _CollectionConstraint.fromCson(
            collectionConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const stringConstraintValue = objectCson["131"];
    const unpackedStringConstraint =
      stringConstraintValue != undefined
        ? _StringConstraint.fromCson(
            stringConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const numberConstraintValue = objectCson["132"];
    const unpackedNumberConstraint =
      numberConstraintValue != undefined
        ? _NumberConstraint.fromCson(
            numberConstraintValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    const nodeConstraintValue = objectCson["133"];
    const unpackedNodeConstraint =
      nodeConstraintValue != undefined
        ? _NodeConstraint.fromCson(nodeConstraintValue, _session, _supergraph, _graph, _connection)
        : null;
    const edgeTypeValue = objectCson["140"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectCson["141"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new PropertyDefinition({
      type: Number(objectCson["100"]),
      object: _ObjectDefinitionReference.fromCson(
        objectCson["104"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      originalObject: _ObjectDefinitionReference.fromCson(
        objectCson["105"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      cardinality: Number(objectCson["110"]),
      scalarType: Number(objectCson["111"]),
      primitiveType: unpackedPrimitiveType,
      enumType: unpackedEnumType,
      nodeType: unpackedNodeType,
      structType: unpackedStructType,
      keyType: unpackedKeyType,
      value: unpackedValue,
      valueFactory: unpackedValueFactory,
      collectionConstraint: unpackedCollectionConstraint,
      stringConstraint: unpackedStringConstraint,
      numberConstraint: unpackedNumberConstraint,
      nodeConstraint: unpackedNodeConstraint,
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isRequired: objectCson["150"],
      isUnique: objectCson["151"],
      isReadonly: objectCson["153"],
      isMain: objectCson["154"],
      isWired: objectCson["160"],
      isStored: objectCson["161"],
      isRepr: objectCson["162"],
      isHash: objectCson["163"],
      isEq: objectCson["164"],
      isInternal: objectCson["165"],
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    return PropertyDefinition.__unpackCson__(
      objectCson,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PropertyDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PropertyDefinition.__packProto__(this);
    }
    return this._proto as PropertyDefinitionProto;
  }

  static __packProto__(object: PropertyDefinition): PropertyDefinitionProto {
    const objectProto: Partial<PropertyDefinitionProto> = { metatype: 18 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as PropertyTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.object = object.object.toProto();
    objectProto.originalObject = object.originalObject.toProto();
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    objectProto.cardinality = Number(object.cardinality) as TypeCardinalityProto;
    objectProto.scalarType = Number(object.scalarType) as ScalarTypeProto;
    if (object.primitiveType != null) {
      objectProto.primitiveType = Number(object.primitiveType) as PrimitiveTypeProto;
    }
    if (object.enumType != null) {
      objectProto.enumType = Number(object.enumType) as EnumTypeProto;
    }
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.keyType != null) {
      objectProto.keyType = object.keyType.toProto();
    }
    if (object.value != null) {
      objectProto.value = object.value.toProto();
    }
    if (object.valueFactory != null) {
      objectProto.valueFactory = Number(object.valueFactory) as ValueFactoryProto;
    }
    if (object.collectionConstraint != null) {
      objectProto.collectionConstraint = object.collectionConstraint.toProto();
    }
    if (object.stringConstraint != null) {
      objectProto.stringConstraint = object.stringConstraint.toProto();
    }
    if (object.numberConstraint != null) {
      objectProto.numberConstraint = object.numberConstraint.toProto();
    }
    if (object.nodeConstraint != null) {
      objectProto.nodeConstraint = object.nodeConstraint.toProto();
    }
    if (object.edgeType != null) {
      objectProto.edgeType = Number(object.edgeType) as EdgeTypeProto;
    }
    if (object.cascade != null) {
      objectProto.cascade = Number(object.cascade) as CascadeActionProto;
    }
    objectProto.isRequired = object.isRequired;
    objectProto.isUnique = object.isUnique;
    objectProto.isReadonly = object.isReadonly;
    objectProto.isMain = object.isMain;
    objectProto.isWired = object.isWired;
    objectProto.isStored = object.isStored;
    objectProto.isRepr = object.isRepr;
    objectProto.isHash = object.isHash;
    objectProto.isEq = object.isEq;
    objectProto.isInternal = object.isInternal;
    return objectProto as PropertyDefinitionProto;
  }

  static __unpackProto__(
    objectProto: PropertyDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    const _ObjectDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.OBJECT_DEFINITION_REFERENCE
    ] as typeof ObjectDefinitionReference;
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
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new PropertyDefinition({
      type: Number(objectProto.type) as PropertyType,
      object: _ObjectDefinitionReference.fromProto(
        objectProto.object!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      originalObject: _ObjectDefinitionReference.fromProto(
        objectProto.originalObject!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
      isRequired: objectProto.isRequired,
      isUnique: objectProto.isUnique,
      isReadonly: objectProto.isReadonly,
      isMain: objectProto.isMain,
      isWired: objectProto.isWired,
      isStored: objectProto.isStored,
      isRepr: objectProto.isRepr,
      isHash: objectProto.isHash,
      isEq: objectProto.isEq,
      isInternal: objectProto.isInternal,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PropertyDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    return PropertyDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PropertyDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PropertyDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  toRef(): PropertyReference {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    if (this.object.type === ObjectDefinitionType.BUILTIN_NODE) {
      return new _PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        nodeType: this.object.nodeType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_STRUCT) {
      return new _PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        structType: this.object.structType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_TRAIT) {
      return new _PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        traitType: this.object.traitType,
        id: this.id,
      });
    } else if (
      this.object.type === ObjectDefinitionType.CUSTOM_NODE ||
      this.object.type === ObjectDefinitionType.CUSTOM_STRUCT ||
      this.object.type === ObjectDefinitionType.CUSTOM_TRAIT
    ) {
      throw new Error(`${this.repr()} cannot be associated with a custom object`);
    } else {
      assertNever(this.object.type);
    }
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

  isNull(): Condition {
    return Condition.of(this, ConditionalType.NOT_EXISTS);
  }

  isNotNull(): Condition {
    return Condition.of(this, ConditionalType.EXISTS);
  }

  asc(): Sort {
    return Sort.of(this, SortType.ASCENDING);
  }

  desc(): Sort {
    return Sort.of(this, SortType.DESCENDING);
  }

  _type: Type | null = null;

  toType(): Type {
    if (this._type === null) {
      const _Type = STRUCT_CLASS_BY_TYPE[StructType.TYPE] as typeof Type;
      this._type = new _Type({
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
    return this._type;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PROPERTY_DEFINITION, PropertyDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:18 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:20 ==== */
/**
 * Definition of a builtin Enum Option.
 */
export class OptionDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.OPTION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * OptionDefinition.type
   */
  readonly type: EnumType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`OptionDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`OptionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`OptionDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<OptionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = OptionDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: OptionDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 20;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new OptionDefinition({
      type: Number(objectCson["100"]),
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    return OptionDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): OptionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = OptionDefinition.__packProto__(this);
    }
    return this._proto as OptionDefinitionProto;
  }

  static __packProto__(object: OptionDefinition): OptionDefinitionProto {
    const objectProto: Partial<OptionDefinitionProto> = { metatype: 20 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as OptionDefinitionProto;
  }

  static __unpackProto__(
    objectProto: OptionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new OptionDefinition({
      type: Number(objectProto.type) as EnumType,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: OptionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    return OptionDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): OptionDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = OptionDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OPTION_DEFINITION, OptionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:20 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:19 ==== */
/**
 * Definition of a builtin Constant.
 */
export class ConstantDefinition extends StructFrozen {
  static metatype: StructType = StructType.CONSTANT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * ConstantDefinition.name
   */
  readonly name: string;

  /**
   * ConstantDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstantDefinition.value
   */
  readonly value: Value;

  /**
   * ConstantDefinition.isDeferred
   */
  readonly isDeferred: boolean;

  constructor(options: {
    name: string;
    description?: string | null;
    value: Value;
    isDeferred: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ConstantDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;
    let _value = options.value;
    if (_value === null) {
      throw new Error(`ConstantDefinition.value is required`);
    }
    this.value = _value;
    let _isDeferred = options.isDeferred;
    if (_isDeferred === null) {
      throw new Error(`ConstantDefinition.isDeferred is required`);
    }
    this.isDeferred = _isDeferred;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (!this.value.equals(other.value)) {
      return false;
    }
    if (!(this.isDeferred === other.isDeferred)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ConstantDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    h = (h * 31 + this.value.hash()) & 0xffffffff;
    h = (h * 31 + hashBool(this.isDeferred)) & 0xffffffff;

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = ConstantDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: ConstantDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 19;
    objectCson["101"] = object.name;
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    objectCson["120"] = object.value.toCson();
    objectCson["130"] = object.isDeferred;
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ConstantDefinition({
      name: objectCson["101"],
      description: unpackedDescription,
      value: _Value.fromCson(objectCson["120"], _session, _supergraph, _graph, _connection),
      isDeferred: objectCson["130"],
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return ConstantDefinition.__unpackCson__(
      objectCson,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ConstantDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ConstantDefinition.__packProto__(this);
    }
    return this._proto as ConstantDefinitionProto;
  }

  static __packProto__(object: ConstantDefinition): ConstantDefinitionProto {
    const objectProto: Partial<ConstantDefinitionProto> = { metatype: 19 };
    objectProto.name = object.name;
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.value = object.value.toProto();
    objectProto.isDeferred = object.isDeferred;
    return objectProto as ConstantDefinitionProto;
  }

  static __unpackProto__(
    objectProto: ConstantDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    return new ConstantDefinition({
      name: objectProto.name,
      description: objectProto.description != undefined ? objectProto.description : null,
      value: _Value.fromProto(objectProto.value!, _session, _supergraph, _graph, _connection),
      isDeferred: objectProto.isDeferred,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ConstantDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return ConstantDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ConstantDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConstantDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONSTANT_DEFINITION, ConstantDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:19 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:21 ==== */
/**
 * Definition of a builtin Tag to associate builtin definitions to.
 */
export class TagDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.TAG_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`TagDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`TagDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<TagDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = TagDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: TagDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 21;
    objectCson["2"] = object.id;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TagDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new TagDefinition({
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TagDefinition {
    return TagDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): TagDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = TagDefinition.__packProto__(this);
    }
    return this._proto as TagDefinitionProto;
  }

  static __packProto__(object: TagDefinition): TagDefinitionProto {
    const objectProto: Partial<TagDefinitionProto> = { metatype: 21 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as TagDefinitionProto;
  }

  static __unpackProto__(
    objectProto: TagDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TagDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new TagDefinition({
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: TagDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TagDefinition {
    return TagDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): TagDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TagDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TAG_DEFINITION, TagDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:21 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:30100 ==== */
/**
 * Definition of a builtin Index.
 */
export class IndexDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.INDEX_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * IndexDefinition.type
   */
  readonly type: IndexType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * IndexDefinition.properties
   */
  readonly properties: readonly PropertyReference[];

  /**
   * IndexDefinition.cover
   */
  readonly cover: readonly PropertyReference[];

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    type: IndexType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyReference[];
    cover?: readonly PropertyReference[];
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`IndexDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`IndexDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`IndexDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _cover = options.cover ?? null;
    if (_cover === null) {
      _cover = [];
    }
    this.cover = _cover;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.cover.length != other.cover.length) {
      return false;
    }
    for (let i = 0; i < this.cover.length; i++) {
      if (!this.cover[i].equals(other.cover[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${IndexType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<IndexDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.cover && this.cover.length > 0) {
      for (const _item of this.cover) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = IndexDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: IndexDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 30100;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toCson());
      }
      objectCson["105"] = packedProperties;
    }
    if (object.cover.length > 0) {
      const packedCover: any[] = [];
      for (const item of object.cover) {
        packedCover.push(item.toCson());
      }
      objectCson["106"] = packedCover;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectCson["105"] != undefined) {
      for (const item of objectCson["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedCover: any[] = [];
    if (objectCson["106"] != undefined) {
      for (const item of objectCson["106"]) {
        unpackedCover.push(
          _PropertyReference.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new IndexDefinition({
      type: Number(objectCson["100"]),
      properties: unpackedProperties,
      cover: unpackedCover,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    return IndexDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): IndexDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = IndexDefinition.__packProto__(this);
    }
    return this._proto as IndexDefinitionProto;
  }

  static __packProto__(object: IndexDefinition): IndexDefinitionProto {
    const objectProto: Partial<IndexDefinitionProto> = { metatype: 30100 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as IndexTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.cover) {
      const packedCover: any[] = [];
      for (const item of object.cover) {
        packedCover.push(item.toProto());
      }
      objectProto.cover = packedCover;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as IndexDefinitionProto;
  }

  static __unpackProto__(
    objectProto: IndexDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedCover: any[] = [];
    if (objectProto.cover) {
      for (const item of objectProto.cover) {
        unpackedCover.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new IndexDefinition({
      type: Number(objectProto.type) as IndexType,
      properties: unpackedProperties,
      cover: unpackedCover,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: IndexDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): IndexDefinition {
    return IndexDefinition.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): IndexDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = IndexDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.INDEX_DEFINITION, IndexDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:30100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:30200 ==== */
/**
 * Definition of a builtin Constraint.
 */
export class ConstraintDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.CONSTRAINT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * ConstraintDefinition.type
   */
  readonly type: ConstraintType;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstraintDefinition.properties
   */
  readonly properties: readonly PropertyReference[];

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    type: ConstraintType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyReference[];
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`ConstraintDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`ConstraintDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ConstraintDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.properties.length != other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ConstraintType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ConstraintDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = ConstraintDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: ConstraintDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 30200;
    objectCson["2"] = object.id;
    objectCson["100"] = object.type;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toCson());
      }
      objectCson["105"] = packedProperties;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectCson["105"] != undefined) {
      for (const item of objectCson["105"]) {
        unpackedProperties.push(
          _PropertyReference.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new ConstraintDefinition({
      type: Number(objectCson["100"]),
      properties: unpackedProperties,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    return ConstraintDefinition.__unpackCson__(
      objectCson,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ConstraintDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ConstraintDefinition.__packProto__(this);
    }
    return this._proto as ConstraintDefinitionProto;
  }

  static __packProto__(object: ConstraintDefinition): ConstraintDefinitionProto {
    const objectProto: Partial<ConstraintDefinitionProto> = { metatype: 30200 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as ConstraintTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as ConstraintDefinitionProto;
  }

  static __unpackProto__(
    objectProto: ConstraintDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    const _PropertyReference = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_REFERENCE
    ] as typeof PropertyReference;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyReference.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new ConstraintDefinition({
      type: Number(objectProto.type) as ConstraintType,
      properties: unpackedProperties,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ConstraintDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstraintDefinition {
    return ConstraintDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ConstraintDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ConstraintDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CONSTRAINT_DEFINITION, ConstraintDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:30200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50000 ==== */
/**
 * Definition of a builtin Permission for a builtin Node.
 */
export class PermissionDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.PERMISSION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * BuiltinDefinition.id
   */
  readonly id: number;

  /**
   * BuiltinDefinition.name
   */
  readonly name: string;

  /**
   * BuiltinDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * BuiltinDefinition.description
   */
  readonly description: string | null;

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly number[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PermissionDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PermissionDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (
      (this.icon == null) !== (other.icon == null) ||
      (this.icon != null && !this.icon.equals(other.icon))
    ) {
      return false;
    }
    if (!(this.description === other.description)) {
      return false;
    }
    if (this.taggings.length != other.taggings.length) {
      return false;
    }
    for (let i = 0; i < this.taggings.length; i++) {
      if (!(this.taggings[i] === other.taggings[i])) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PermissionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon != null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = PermissionDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: PermissionDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 50000;
    objectCson["2"] = object.id;
    objectCson["101"] = object.name;
    if (object.icon != null) {
      objectCson["102"] = object.icon.toCson();
    }
    if (object.description != null) {
      objectCson["103"] = object.description;
    }
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new PermissionDefinition({
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _supergraph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    return PermissionDefinition.__unpackCson__(
      objectCson,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PermissionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PermissionDefinition.__packProto__(this);
    }
    return this._proto as PermissionDefinitionProto;
  }

  static __packProto__(object: PermissionDefinition): PermissionDefinitionProto {
    const objectProto: Partial<PermissionDefinitionProto> = { metatype: 50000 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
    }
    return objectProto as PermissionDefinitionProto;
  }

  static __unpackProto__(
    objectProto: PermissionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new PermissionDefinition({
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PermissionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    return PermissionDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PermissionDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PermissionDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PERMISSION_DEFINITION, PermissionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:50000 ==== */
