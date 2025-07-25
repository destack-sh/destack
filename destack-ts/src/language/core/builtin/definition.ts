import { EnumType, NodeType, StructType, TraitType } from "@destack/language/core/builtin/builtin";
import {
  type CascadeAction,
  type EdgeType,
  type GraphDomain,
  PrimitiveType,
  type PropertyType,
  ScalarType,
  TypeCardinality,
  type ValueFactory,
} from "@destack/language/core/builtin/common";
import { ConstraintType, IndexType } from "@destack/language/core/builtin/meta";
import type { PackedCache } from "@destack/language/core/builtin/object";
import type {
  ObjectDefinitionReference,
  PropertyReference,
} from "@destack/language/core/builtin/relation";
import {
  ObjectDefinitionType,
  PropertyReferenceType,
} from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import {
  type CollectionConstraint,
  type NumberConstraint,
  type StringConstraint,
  Type,
} from "@destack/language/core/builtin/type";
import type { UInt8, UInt32 } from "@destack/language/core/builtin/types";
import type { Value } from "@destack/language/core/builtin/value";
import type { ActionDefinition } from "@destack/language/core/common/action";
import type { Icon } from "@destack/language/core/common/icon";
import type { MethodDefinition } from "@destack/language/core/common/method";
import { Condition, ConditionalType, Sort, SortType } from "@destack/language/core/common/query";
import type { Session } from "@destack/language/core/runtime/session";
import {
  NODE_CLASS_BY_TYPE,
  registerStructClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
import { assertNever } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:12 ==== */
/**
 * Definition of a builtin Node.
 */
export class NodeDefinition extends StructFrozen {
  static metatype: StructType = StructType.NODE_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * NodeDefinition.id
   */
  readonly id: UInt32;

  /**
   * NodeDefinition.type
   */
  readonly type: NodeType;

  /**
   * NodeDefinition.name
   */
  readonly name: string;

  /**
   * NodeDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * NodeDefinition.description
   */
  readonly description: string | null;

  /**
   * NodeDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

  /**
   * Whether this Node cannot be instantiated directly.
   */
  readonly isAbstract: boolean;

  /**
   * Whether this Node can be extended by custom Nodes.
   */
  readonly isExtensible: boolean;

  /**
   * Whether this Node cannot be extended by custom Nodes.
   */
  readonly isFinal: boolean;

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
   * All constants of this Node (including inherited).
   */
  readonly constants: readonly ConstantDefinition[];

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
   * Traits implemented by this Node.
   */
  readonly traits: readonly TraitType[];

  /**
   * Traits declared by this Node (directly).
   */
  readonly selfTraits: readonly TraitType[];

  /**
   * The event types related to this Node.
   */
  readonly eventTypes: readonly NodeType[];

  /**
   * The event types declared by this Node (directly).
   */
  readonly selfEventTypes: readonly NodeType[];

  /**
   * The enum types related to this Node.
   */
  readonly enumTypes: readonly EnumType[];

  /**
   * The enum types declared by this Node (directly).
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
   * NodeDefinition.domain
   */
  readonly domain: GraphDomain | null;

  constructor(options: {
    id: UInt32;
    type: NodeType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly UInt8[];
    isAbstract: boolean;
    isExtensible: boolean;
    isFinal: boolean;
    isFrozen: boolean;
    properties?: readonly PropertyDefinition[];
    indexes?: readonly IndexDefinition[];
    constraints?: readonly ConstraintDefinition[];
    permissions?: readonly PermissionDefinition[];
    methods?: readonly MethodDefinition[];
    actions?: readonly ActionDefinition[];
    constants?: readonly ConstantDefinition[];
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
    domain?: GraphDomain | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`NodeDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`NodeDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`NodeDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _isAbstract = options.isAbstract;
    if (_isAbstract == null) {
      throw new Error(`NodeDefinition.isAbstract is required`);
    }
    this.isAbstract = _isAbstract;
    let _isExtensible = options.isExtensible;
    if (_isExtensible == null) {
      throw new Error(`NodeDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _isFinal = options.isFinal;
    if (_isFinal == null) {
      throw new Error(`NodeDefinition.isFinal is required`);
    }
    this.isFinal = _isFinal;
    let _isFrozen = options.isFrozen;
    if (_isFrozen == null) {
      throw new Error(`NodeDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;
    let _properties = options.properties ?? null;
    if (_properties == null) {
      _properties = [];
    }
    this.properties = _properties;
    let _indexes = options.indexes ?? null;
    if (_indexes == null) {
      _indexes = [];
    }
    this.indexes = _indexes;
    let _constraints = options.constraints ?? null;
    if (_constraints == null) {
      _constraints = [];
    }
    this.constraints = _constraints;
    let _permissions = options.permissions ?? null;
    if (_permissions == null) {
      _permissions = [];
    }
    this.permissions = _permissions;
    let _methods = options.methods ?? null;
    if (_methods == null) {
      _methods = [];
    }
    this.methods = _methods;
    let _actions = options.actions ?? null;
    if (_actions == null) {
      _actions = [];
    }
    this.actions = _actions;
    let _constants = options.constants ?? null;
    if (_constants == null) {
      _constants = [];
    }
    this.constants = _constants;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _extendedBy = options.extendedBy ?? null;
    if (_extendedBy == null) {
      _extendedBy = [];
    }
    this.extendedBy = _extendedBy;
    let _inherits = options.inherits ?? null;
    if (_inherits == null) {
      _inherits = [];
    }
    this.inherits = _inherits;
    let _inheritedBy = options.inheritedBy ?? null;
    if (_inheritedBy == null) {
      _inheritedBy = [];
    }
    this.inheritedBy = _inheritedBy;
    let _traits = options.traits ?? null;
    if (_traits == null) {
      _traits = [];
    }
    this.traits = _traits;
    let _selfTraits = options.selfTraits ?? null;
    if (_selfTraits == null) {
      _selfTraits = [];
    }
    this.selfTraits = _selfTraits;
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes == null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _selfEventTypes = options.selfEventTypes ?? null;
    if (_selfEventTypes == null) {
      _selfEventTypes = [];
    }
    this.selfEventTypes = _selfEventTypes;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes == null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes == null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;
    let _parentTypes = options.parentTypes ?? null;
    if (_parentTypes == null) {
      _parentTypes = [];
    }
    this.parentTypes = _parentTypes;
    let _childTypes = options.childTypes ?? null;
    if (_childTypes == null) {
      _childTypes = [];
    }
    this.childTypes = _childTypes;
    let _ancestorTypes = options.ancestorTypes ?? null;
    if (_ancestorTypes == null) {
      _ancestorTypes = [];
    }
    this.ancestorTypes = _ancestorTypes;
    let _descendantTypes = options.descendantTypes ?? null;
    if (_descendantTypes == null) {
      _descendantTypes = [];
    }
    this.descendantTypes = _descendantTypes;
    let _expectedParentTypes = options.expectedParentTypes ?? null;
    if (_expectedParentTypes == null) {
      _expectedParentTypes = [];
    }
    this.expectedParentTypes = _expectedParentTypes;
    let _expectedChildTypes = options.expectedChildTypes ?? null;
    if (_expectedChildTypes == null) {
      _expectedChildTypes = [];
    }
    this.expectedChildTypes = _expectedChildTypes;
    let _expectedAncestorTypes = options.expectedAncestorTypes ?? null;
    if (_expectedAncestorTypes == null) {
      _expectedAncestorTypes = [];
    }
    this.expectedAncestorTypes = _expectedAncestorTypes;
    let _expectedDescendantTypes = options.expectedDescendantTypes ?? null;
    if (_expectedDescendantTypes == null) {
      _expectedDescendantTypes = [];
    }
    this.expectedDescendantTypes = _expectedDescendantTypes;
    let _domain = options.domain ?? null;
    this.domain = _domain;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

    if (!(this.isAbstract === other.isAbstract)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.isFinal === other.isFinal)) {
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

    if (this.constants.length != other.constants.length) {
      return false;
    }
    for (let i = 0; i < this.constants.length; i++) {
      if (!this.constants[i].equals(other.constants[i])) {
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

    if (!(this.domain === other.domain)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${NodeType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      propertyReprs.push(`isAbstract=${this.isAbstract}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      propertyReprs.push(`isFinal=${this.isFinal}`);
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
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isFinal)) & 0xffffffff;
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
    if (this.constants && this.constants.length > 0) {
      for (const _item of this.constants) {
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
    if (this.domain != null) {
      h = (h * 31 + this.domain) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class TraitDefinition extends StructFrozen {
  static metatype: StructType = StructType.TRAIT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * TraitDefinition.id
   */
  readonly id: UInt32;

  /**
   * TraitDefinition.type
   */
  readonly type: TraitType;

  /**
   * TraitDefinition.name
   */
  readonly name: string;

  /**
   * TraitDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * TraitDefinition.description
   */
  readonly description: string | null;

  /**
   * TraitDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

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
    id: UInt32;
    type: TraitType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly UInt8[];
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
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`TraitDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`TraitDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`TraitDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _alias = options.alias;
    if (_alias == null) {
      throw new Error(`TraitDefinition.alias is required`);
    }
    this.alias = _alias;
    let _isExtensible = options.isExtensible;
    if (_isExtensible == null) {
      throw new Error(`TraitDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _permissions = options.permissions ?? null;
    if (_permissions == null) {
      _permissions = [];
    }
    this.permissions = _permissions;
    let _selfTraits = options.selfTraits ?? null;
    if (_selfTraits == null) {
      _selfTraits = [];
    }
    this.selfTraits = _selfTraits;
    let _traits = options.traits ?? null;
    if (_traits == null) {
      _traits = [];
    }
    this.traits = _traits;
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes == null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _selfEventTypes = options.selfEventTypes ?? null;
    if (_selfEventTypes == null) {
      _selfEventTypes = [];
    }
    this.selfEventTypes = _selfEventTypes;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes == null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes == null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${TraitType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      propertyReprs.push(`alias=${`"${this.alias}"`}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      // @ts-expect-error(readonly) */
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
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class StructDefinition extends StructFrozen {
  static metatype: StructType = StructType.STRUCT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * StructDefinition.id
   */
  readonly id: UInt32;

  /**
   * StructDefinition.type
   */
  readonly type: StructType;

  /**
   * StructDefinition.name
   */
  readonly name: string;

  /**
   * StructDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * StructDefinition.description
   */
  readonly description: string | null;

  /**
   * StructDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

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
   * StructDefinition.constants
   */
  readonly constants: readonly ConstantDefinition[];

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
    id: UInt32;
    type: StructType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly UInt8[];
    isFrozen: boolean;
    isAbstract: boolean;
    isExtensible: boolean;
    properties?: readonly PropertyDefinition[];
    methods?: readonly MethodDefinition[];
    actions?: readonly ActionDefinition[];
    constants?: readonly ConstantDefinition[];
    tags?: readonly TagDefinition[];
    baseType?: StructType | null;
    extendedBy?: readonly StructType[];
    inherits?: readonly StructType[];
    inheritedBy?: readonly StructType[];
    enumTypes?: readonly EnumType[];
    selfEnumTypes?: readonly EnumType[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`StructDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`StructDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`StructDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _isFrozen = options.isFrozen;
    if (_isFrozen == null) {
      throw new Error(`StructDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;
    let _isAbstract = options.isAbstract;
    if (_isAbstract == null) {
      throw new Error(`StructDefinition.isAbstract is required`);
    }
    this.isAbstract = _isAbstract;
    let _isExtensible = options.isExtensible;
    if (_isExtensible == null) {
      throw new Error(`StructDefinition.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _properties = options.properties ?? null;
    if (_properties == null) {
      _properties = [];
    }
    this.properties = _properties;
    let _methods = options.methods ?? null;
    if (_methods == null) {
      _methods = [];
    }
    this.methods = _methods;
    let _actions = options.actions ?? null;
    if (_actions == null) {
      _actions = [];
    }
    this.actions = _actions;
    let _constants = options.constants ?? null;
    if (_constants == null) {
      _constants = [];
    }
    this.constants = _constants;
    let _tags = options.tags ?? null;
    if (_tags == null) {
      _tags = [];
    }
    this.tags = _tags;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _extendedBy = options.extendedBy ?? null;
    if (_extendedBy == null) {
      _extendedBy = [];
    }
    this.extendedBy = _extendedBy;
    let _inherits = options.inherits ?? null;
    if (_inherits == null) {
      _inherits = [];
    }
    this.inherits = _inherits;
    let _inheritedBy = options.inheritedBy ?? null;
    if (_inheritedBy == null) {
      _inheritedBy = [];
    }
    this.inheritedBy = _inheritedBy;
    let _enumTypes = options.enumTypes ?? null;
    if (_enumTypes == null) {
      _enumTypes = [];
    }
    this.enumTypes = _enumTypes;
    let _selfEnumTypes = options.selfEnumTypes ?? null;
    if (_selfEnumTypes == null) {
      _selfEnumTypes = [];
    }
    this.selfEnumTypes = _selfEnumTypes;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

    if (this.constants.length != other.constants.length) {
      return false;
    }
    for (let i = 0; i < this.constants.length; i++) {
      if (!this.constants[i].equals(other.constants[i])) {
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
      // @ts-expect-error(readonly) */
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
    if (this.constants && this.constants.length > 0) {
      for (const _item of this.constants) {
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
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class EnumDefinition extends StructFrozen {
  static metatype: StructType = StructType.ENUM_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * EnumDefinition.id
   */
  readonly id: UInt32;

  /**
   * EnumDefinition.type
   */
  readonly type: EnumType;

  /**
   * EnumDefinition.name
   */
  readonly name: string;

  /**
   * EnumDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * EnumDefinition.description
   */
  readonly description: string | null;

  /**
   * EnumDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

  /**
   * EnumDefinition.options
   */
  readonly options: readonly OptionDefinition[];

  constructor(options: {
    id: UInt32;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly UInt8[];
    options?: readonly OptionDefinition[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`EnumDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`EnumDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`EnumDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _options = options.options ?? null;
    if (_options == null) {
      _options = [];
    }
    this.options = _options;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

    if (this.options.length != other.options.length) {
      return false;
    }
    for (let i = 0; i < this.options.length; i++) {
      if (!this.options[i].equals(other.options[i])) {
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
      // @ts-expect-error(readonly) */
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
    if (this.options && this.options.length > 0) {
      for (const _item of this.options) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class PropertyDefinition extends Type {
  static metatype: StructType = StructType.PROPERTY_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * PropertyDefinition.id
   */
  readonly id: UInt8;

  /**
   * PropertyDefinition.type
   */
  readonly type: PropertyType;

  /**
   * The name of this Type when it was used.
   */
  readonly name: string | null;

  /**
   * PropertyDefinition.description
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
   * PropertyDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

  /**
   * PropertyDefinition.edgeType
   */
  readonly edgeType: EdgeType | null;

  /**
   * PropertyDefinition.cascade
   */
  readonly cascade: CascadeAction | null;

  /**
   * Whether this Property is part of the object's identity.
   *  (And thus is always required, in every instance including partials; only for Nodes.)
   */
  readonly isIdentity: boolean;

  /**
   * Whether this Property must have a unique value.
   */
  readonly isUnique: boolean;

  /**
   * Whether this Property is read-only.
   */
  readonly isReadonly: boolean;

  /**
   * Whether this Property is the main property of the object.
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
    id: UInt8;
    type: PropertyType;
    name?: string | null;
    description?: string | null;
    object: ObjectDefinitionReference;
    originalObject: ObjectDefinitionReference;
    taggings?: readonly UInt8[];
    cardinality?: TypeCardinality;
    scalarType: ScalarType;
    primitiveType?: PrimitiveType | null;
    enumType?: EnumType | null;
    nodeTypes?: readonly NodeType[];
    structType?: StructType | null;
    keyType?: Type | null;
    literalValue?: Value | null;
    defaultValue?: Value | null;
    defaultFactory?: ValueFactory | null;
    collectionConstraint?: CollectionConstraint | null;
    stringConstraint?: StringConstraint | null;
    numberConstraint?: NumberConstraint | null;
    isRequired?: boolean | null;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isIdentity: boolean;
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
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(options);

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`PropertyDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`PropertyDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name ?? null;
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;
    let _object = options.object;
    if (_object == null) {
      throw new Error(`PropertyDefinition.object is required`);
    }
    this.object = _object;
    let _originalObject = options.originalObject;
    if (_originalObject == null) {
      throw new Error(`PropertyDefinition.originalObject is required`);
    }
    this.originalObject = _originalObject;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _edgeType = options.edgeType ?? null;
    this.edgeType = _edgeType;
    let _cascade = options.cascade ?? null;
    this.cascade = _cascade;
    let _isIdentity = options.isIdentity;
    if (_isIdentity == null) {
      throw new Error(`PropertyDefinition.isIdentity is required`);
    }
    this.isIdentity = _isIdentity;
    let _isUnique = options.isUnique;
    if (_isUnique == null) {
      throw new Error(`PropertyDefinition.isUnique is required`);
    }
    this.isUnique = _isUnique;
    let _isReadonly = options.isReadonly;
    if (_isReadonly == null) {
      throw new Error(`PropertyDefinition.isReadonly is required`);
    }
    this.isReadonly = _isReadonly;
    let _isMain = options.isMain;
    if (_isMain == null) {
      throw new Error(`PropertyDefinition.isMain is required`);
    }
    this.isMain = _isMain;
    let _isWired = options.isWired;
    if (_isWired == null) {
      throw new Error(`PropertyDefinition.isWired is required`);
    }
    this.isWired = _isWired;
    let _isStored = options.isStored;
    if (_isStored == null) {
      throw new Error(`PropertyDefinition.isStored is required`);
    }
    this.isStored = _isStored;
    let _isRepr = options.isRepr;
    if (_isRepr == null) {
      throw new Error(`PropertyDefinition.isRepr is required`);
    }
    this.isRepr = _isRepr;
    let _isHash = options.isHash;
    if (_isHash == null) {
      throw new Error(`PropertyDefinition.isHash is required`);
    }
    this.isHash = _isHash;
    let _isEq = options.isEq;
    if (_isEq == null) {
      throw new Error(`PropertyDefinition.isEq is required`);
    }
    this.isEq = _isEq;
    let _isInternal = options.isInternal;
    if (_isInternal == null) {
      throw new Error(`PropertyDefinition.isInternal is required`);
    }
    this.isInternal = _isInternal;

    /* identity */
    /* ... (already set in parent) */
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
    if (!(this.description === other.description)) {
      return false;
    }
    if (!this.object.equals(other.object)) {
      return false;
    }
    if (!this.originalObject.equals(other.originalObject)) {
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

    if (!(this.edgeType === other.edgeType)) {
      return false;
    }
    if (!(this.cascade === other.cascade)) {
      return false;
    }
    if (!(this.isIdentity === other.isIdentity)) {
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
    if (
      (this.defaultValue == null) !== (other.defaultValue == null) ||
      (this.defaultValue != null && !this.defaultValue.equals(other.defaultValue))
    ) {
      return false;
    }
    if (!(this.defaultFactory === other.defaultFactory)) {
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
    if (!(this.isRequired === other.isRequired)) {
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
    if (this.nodeTypes.length != other.nodeTypes.length) {
      return false;
    }
    for (let i = 0; i < this.nodeTypes.length; i++) {
      if (!(this.nodeTypes[i] === other.nodeTypes[i])) {
        return false;
      }
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
      (this.literalValue == null) !== (other.literalValue == null) ||
      (this.literalValue != null && !this.literalValue.equals(other.literalValue))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      if (this.name != null) {
        propertyReprs.push(`name=${`"${this.name}"`}`);
      }
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      propertyReprs.push(`isIdentity=${this.isIdentity}`);
      propertyReprs.push(`isUnique=${this.isUnique}`);
      propertyReprs.push(`isReadonly=${this.isReadonly}`);
      propertyReprs.push(`isMain=${this.isMain}`);
      propertyReprs.push(`cardinality=${TypeCardinality[this.cardinality]}`);
      propertyReprs.push(`scalarType=${ScalarType[this.scalarType]}`);
      if (this.primitiveType != null) {
        propertyReprs.push(`primitiveType=${PrimitiveType[this.primitiveType]}`);
      }
      if (this.enumType != null) {
        propertyReprs.push(`enumType=${EnumType[this.enumType]}`);
      }
      if (this.nodeTypes.length > 0) {
        propertyReprs.push(
          `nodeTypes=${this.nodeTypes.map((_item) => NodeType[_item]).join(", ")}`,
        );
      }
      if (this.structType != null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.keyType != null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.literalValue != null) {
        propertyReprs.push(`literalValue=${this.literalValue.repr()}`);
      }
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    if (this.name != null) {
      h = (h * 31 + hashString(this.name)) & 0xffffffff;
    }
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    h = (h * 31 + this.object.hash()) & 0xffffffff;
    h = (h * 31 + this.originalObject.hash()) & 0xffffffff;
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    if (this.edgeType != null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade != null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isIdentity)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isUnique)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isReadonly)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isMain)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isWired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isStored)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepr)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isHash)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isEq)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isInternal)) & 0xffffffff;
    if (this.defaultValue != null) {
      h = (h * 31 + this.defaultValue.hash()) & 0xffffffff;
    }
    if (this.defaultFactory != null) {
      h = (h * 31 + this.defaultFactory) & 0xffffffff;
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
    if (this.isRequired != null) {
      h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    }
    h = (h * 31 + this.cardinality) & 0xffffffff;
    h = (h * 31 + this.scalarType) & 0xffffffff;
    if (this.primitiveType != null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType != null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeTypes && this.nodeTypes.length > 0) {
      for (const _item of this.nodeTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.structType != null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.keyType != null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.literalValue != null) {
      h = (h * 31 + this.literalValue.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
        nodeTypes: this.nodeTypes,
        structType: this.structType,
        keyType: this.keyType,
        isRequired: this.isRequired,
        literalValue: this.literalValue,
        defaultValue: this.defaultValue,
        defaultFactory: this.defaultFactory,
        collectionConstraint: this.collectionConstraint,
        stringConstraint: this.stringConstraint,
        numberConstraint: this.numberConstraint,
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
export class OptionDefinition extends StructFrozen {
  static metatype: StructType = StructType.OPTION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * OptionDefinition.id
   */
  readonly id: UInt8;

  /**
   * OptionDefinition.type
   */
  readonly type: EnumType;

  /**
   * OptionDefinition.name
   */
  readonly name: string;

  /**
   * OptionDefinition.icon
   */
  readonly icon: Icon | null;

  /**
   * OptionDefinition.description
   */
  readonly description: string | null;

  /**
   * OptionDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

  constructor(options: {
    id: UInt8;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    taggings?: readonly UInt8[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`OptionDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`OptionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`OptionDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
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
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
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
   * ConstantDefinition.id
   */
  readonly id: UInt8;

  /**
   * ConstantDefinition.name
   */
  readonly name: string;

  /**
   * ConstantDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstantDefinition.taggings
   */
  readonly taggings: readonly UInt8[];

  /**
   * ConstantDefinition.value
   */
  readonly value: Value;

  constructor(options: {
    id: UInt8;
    name: string;
    description?: string | null;
    taggings?: readonly UInt8[];
    value: Value;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`ConstantDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`ConstantDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;
    let _taggings = options.taggings ?? null;
    if (_taggings == null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _value = options.value;
    if (_value == null) {
      throw new Error(`ConstantDefinition.value is required`);
    }
    this.value = _value;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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

    if (!this.value.equals(other.value)) {
      return false;
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
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.taggings && this.taggings.length > 0) {
      for (const _item of this.taggings) {
        h = (h * 31 + hashInt(_item)) & 0xffffffff;
      }
    }
    h = (h * 31 + this.value.hash()) & 0xffffffff;
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class TagDefinition extends StructFrozen {
  static metatype: StructType = StructType.TAG_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * TagDefinition.id
   */
  readonly id: UInt8;

  /**
   * TagDefinition.name
   */
  readonly name: string;

  /**
   * TagDefinition.description
   */
  readonly description: string | null;

  constructor(options: {
    id: UInt8;
    name: string;
    description?: string | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`TagDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`TagDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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
    if (!(this.description === other.description)) {
      return false;
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
      // @ts-expect-error(readonly) */
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
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class IndexDefinition extends StructFrozen {
  static metatype: StructType = StructType.INDEX_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * IndexDefinition.id
   */
  readonly id: UInt8;

  /**
   * IndexDefinition.type
   */
  readonly type: IndexType;

  /**
   * IndexDefinition.name
   */
  readonly name: string;

  /**
   * IndexDefinition.description
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

  constructor(options: {
    id: UInt8;
    type: IndexType;
    name: string;
    description?: string | null;
    properties?: readonly PropertyReference[];
    cover?: readonly PropertyReference[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`IndexDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`IndexDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`IndexDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties == null) {
      _properties = [];
    }
    this.properties = _properties;
    let _cover = options.cover ?? null;
    if (_cover == null) {
      _cover = [];
    }
    this.cover = _cover;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.description === other.description)) {
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

    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${IndexType[this.type]}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
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
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class ConstraintDefinition extends StructFrozen {
  static metatype: StructType = StructType.CONSTRAINT_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * ConstraintDefinition.id
   */
  readonly id: UInt8;

  /**
   * ConstraintDefinition.type
   */
  readonly type: ConstraintType;

  /**
   * ConstraintDefinition.name
   */
  readonly name: string;

  /**
   * ConstraintDefinition.description
   */
  readonly description: string | null;

  /**
   * ConstraintDefinition.properties
   */
  readonly properties: readonly PropertyReference[];

  constructor(options: {
    id: UInt8;
    type: ConstraintType;
    name: string;
    description?: string | null;
    properties?: readonly PropertyReference[];
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`ConstraintDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type == null) {
      throw new Error(`ConstraintDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`ConstraintDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;
    let _properties = options.properties ?? null;
    if (_properties == null) {
      _properties = [];
    }
    this.properties = _properties;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    if (!(this.description === other.description)) {
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

    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`type=${ConstraintType[this.type]}`);
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly) */
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
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
export class PermissionDefinition extends StructFrozen {
  static metatype: StructType = StructType.PERMISSION_DEFINITION;
  static __isFrozen__: boolean = true;

  /**
   * PermissionDefinition.id
   */
  readonly id: UInt8;

  /**
   * PermissionDefinition.name
   */
  readonly name: string;

  /**
   * PermissionDefinition.description
   */
  readonly description: string | null;

  constructor(options: {
    id: UInt8;
    name: string;
    description?: string | null;
    _session?: Session | null;
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _id = options.id;
    if (_id == null) {
      throw new Error(`PermissionDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name == null) {
      throw new Error(`PermissionDefinition.name is required`);
    }
    this.name = _name;
    let _description = options.description ?? null;
    this.description = _description;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
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
    if (!(this.description === other.description)) {
      return false;
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
      // @ts-expect-error(readonly) */
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
    if (this.description != null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PERMISSION_DEFINITION, PermissionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:50000 ==== */
