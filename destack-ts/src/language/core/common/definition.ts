import {
  CascadeAction,
  EdgeType,
  EnumType,
  NodeType,
  PrimitiveType,
  PropertyType,
  ScalarType,
  StoreType,
  StructType,
  TraitType,
  TypeCardinality,
  ValueFactory,
} from "@destack/language/core/builtin/common";
import type { ObjectDefinitionReference } from "@destack/language/core/builtin/relation";
import {
  ObjectDefinitionType,
  PropertyReference,
  PropertyReferenceType,
} from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { Icon } from "@destack/language/core/common/icon";
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
  TRAIT_CLASS_BY_TYPE,
  registerStructClass,
} from "@destack/language/registry";
import {
  ActionDefinitionProto,
  CascadeActionProto,
  ConstantDefinitionProto,
  EdgeTypeProto,
  EnumDefinitionProto,
  EnumTypeProto,
  MethodDefinitionProto,
  NodeDefinitionProto,
  NodeTypeProto,
  OptionDefinitionProto,
  OptionGroupDefinitionProto,
  PermissionDefinitionProto,
  PrimitiveTypeProto,
  PropertyDefinitionProto,
  PropertyGroupDefinitionProto,
  PropertyTypeProto,
  ScalarTypeProto,
  StoreTypeProto,
  StructDefinitionProto,
  StructTypeProto,
  TraitDefinitionProto,
  TraitTypeProto,
  TypeCardinalityProto,
  ValueFactoryProto,
} from "@destack/proto";
import { assertNever, base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:100 ==== */
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.BUILTIN_DEFINITION, BuiltinDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:100 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:101 ==== */
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
   * NodeDefinition.primaryStoreTypes
   */
  readonly primaryStoreTypes: Array<StoreType>;

  /**
   * NodeDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * NodeDefinition.groups
   */
  readonly groups: Array<PropertyGroupDefinition>;

  /**
   * Whether this Node is global.
   */
  readonly isGlobal: boolean;

  /**
   * Whether this Node is per Space.
   */
  readonly isSpatial: boolean;

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
   * The base type this Node extends (directly).
   */
  readonly baseType: NodeType | null;

  /**
   * Nodes that extend this Node type (directly).
   */
  readonly extendedBy: Array<NodeType>;

  /**
   * Nodes that this Node inherits (directly and indirectly).
   */
  readonly inherits: Array<NodeType>;

  /**
   * Nodes that inherit this Node type (directly and indirectly).
   */
  readonly inheritedBy: Array<NodeType>;

  /**
   * Traits directly inherited by this Node (directly).
   */
  readonly baseTraits: Array<TraitType>;

  /**
   * Traits directly and indirectly inherited by this Node (directly and indirectly).
   */
  readonly traits: Array<TraitType>;

  /**
   * The root ancestor type of this Node type (if any).
   */
  readonly rootType: NodeType | null;

  /**
   * The parent types of this Node type (directly).
   */
  readonly parentTypes: Array<NodeType>;

  /**
   * The child types of this Node type (directly).
   */
  readonly childTypes: Array<NodeType>;

  /**
   * The ancestor types of this Node type (directly and indirectly).
   */
  readonly ancestorTypes: Array<NodeType>;

  /**
   * The descendant types of this Node type (directly and indirectly).
   */
  readonly descendantTypes: Array<NodeType>;

  /**
   * The event types of this Node (directly and indirectly).
   */
  readonly eventTypes: Array<NodeType>;

  /**
   * The base event types of this Node (directly).
   */
  readonly baseEventTypes: Array<NodeType>;

  constructor(options: {
    id: number;
    type: NodeType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    primaryStoreTypes?: Array<StoreType>;
    properties?: Array<PropertyDefinition>;
    groups?: Array<PropertyGroupDefinition>;
    isGlobal: boolean;
    isSpatial: boolean;
    isAbstract: boolean;
    isExtensible: boolean;
    isFrozen: boolean;
    baseType?: NodeType | null;
    extendedBy?: Array<NodeType>;
    inherits?: Array<NodeType>;
    inheritedBy?: Array<NodeType>;
    baseTraits?: Array<TraitType>;
    traits?: Array<TraitType>;
    rootType?: NodeType | null;
    parentTypes?: Array<NodeType>;
    childTypes?: Array<NodeType>;
    ancestorTypes?: Array<NodeType>;
    descendantTypes?: Array<NodeType>;
    eventTypes?: Array<NodeType>;
    baseEventTypes?: Array<NodeType>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _primaryStoreTypes = options.primaryStoreTypes ?? null;
    if (_primaryStoreTypes === null) {
      _primaryStoreTypes = [];
    }
    this.primaryStoreTypes = _primaryStoreTypes;
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _groups = options.groups ?? null;
    if (_groups === null) {
      _groups = [];
    }
    this.groups = _groups;
    let _isGlobal = options.isGlobal;
    if (_isGlobal === null) {
      throw new Error(`NodeDefinition.isGlobal is required`);
    }
    this.isGlobal = _isGlobal;
    let _isSpatial = options.isSpatial;
    if (_isSpatial === null) {
      throw new Error(`NodeDefinition.isSpatial is required`);
    }
    this.isSpatial = _isSpatial;
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
    let _baseTraits = options.baseTraits ?? null;
    if (_baseTraits === null) {
      _baseTraits = [];
    }
    this.baseTraits = _baseTraits;
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _rootType = options.rootType ?? null;
    this.rootType = _rootType;
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
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes === null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _baseEventTypes = options.baseEventTypes ?? null;
    if (_baseEventTypes === null) {
      _baseEventTypes = [];
    }
    this.baseEventTypes = _baseEventTypes;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.primaryStoreTypes.length !== other.primaryStoreTypes.length) {
      return false;
    }
    for (let i = 0; i < this.primaryStoreTypes.length; i++) {
      if (!(this.primaryStoreTypes[i] === other.primaryStoreTypes[i])) {
        return false;
      }
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.groups.length !== other.groups.length) {
      return false;
    }
    for (let i = 0; i < this.groups.length; i++) {
      if (!this.groups[i].equals(other.groups[i])) {
        return false;
      }
    }
    if (!(this.isGlobal === other.isGlobal)) {
      return false;
    }
    if (!(this.isSpatial === other.isSpatial)) {
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
    if (!(this.baseType === other.baseType)) {
      return false;
    }
    if (this.extendedBy.length !== other.extendedBy.length) {
      return false;
    }
    for (let i = 0; i < this.extendedBy.length; i++) {
      if (!(this.extendedBy[i] === other.extendedBy[i])) {
        return false;
      }
    }
    if (this.inherits.length !== other.inherits.length) {
      return false;
    }
    for (let i = 0; i < this.inherits.length; i++) {
      if (!(this.inherits[i] === other.inherits[i])) {
        return false;
      }
    }
    if (this.inheritedBy.length !== other.inheritedBy.length) {
      return false;
    }
    for (let i = 0; i < this.inheritedBy.length; i++) {
      if (!(this.inheritedBy[i] === other.inheritedBy[i])) {
        return false;
      }
    }
    if (this.baseTraits.length !== other.baseTraits.length) {
      return false;
    }
    for (let i = 0; i < this.baseTraits.length; i++) {
      if (!(this.baseTraits[i] === other.baseTraits[i])) {
        return false;
      }
    }
    if (this.traits.length !== other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (!(this.rootType === other.rootType)) {
      return false;
    }
    if (this.parentTypes.length !== other.parentTypes.length) {
      return false;
    }
    for (let i = 0; i < this.parentTypes.length; i++) {
      if (!(this.parentTypes[i] === other.parentTypes[i])) {
        return false;
      }
    }
    if (this.childTypes.length !== other.childTypes.length) {
      return false;
    }
    for (let i = 0; i < this.childTypes.length; i++) {
      if (!(this.childTypes[i] === other.childTypes[i])) {
        return false;
      }
    }
    if (this.ancestorTypes.length !== other.ancestorTypes.length) {
      return false;
    }
    for (let i = 0; i < this.ancestorTypes.length; i++) {
      if (!(this.ancestorTypes[i] === other.ancestorTypes[i])) {
        return false;
      }
    }
    if (this.descendantTypes.length !== other.descendantTypes.length) {
      return false;
    }
    for (let i = 0; i < this.descendantTypes.length; i++) {
      if (!(this.descendantTypes[i] === other.descendantTypes[i])) {
        return false;
      }
    }
    if (this.eventTypes.length !== other.eventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.eventTypes.length; i++) {
      if (!(this.eventTypes[i] === other.eventTypes[i])) {
        return false;
      }
    }
    if (this.baseEventTypes.length !== other.baseEventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.baseEventTypes.length; i++) {
      if (!(this.baseEventTypes[i] === other.baseEventTypes[i])) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${NodeType[this.type]}`);
      propertyReprs.push(`isGlobal=${this.isGlobal}`);
      propertyReprs.push(`isSpatial=${this.isSpatial}`);
      propertyReprs.push(`isAbstract=${this.isAbstract}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      propertyReprs.push(`isFrozen=${this.isFrozen}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<NodeDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.primaryStoreTypes && this.primaryStoreTypes.length > 0) {
      for (const _item of this.primaryStoreTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.groups && this.groups.length > 0) {
      for (const _item of this.groups) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.isGlobal)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isSpatial)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;
    if (this.baseType !== null) {
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
    if (this.baseTraits && this.baseTraits.length > 0) {
      for (const _item of this.baseTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.rootType !== null) {
      h = (h * 31 + this.rootType) & 0xffffffff;
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
    if (this.eventTypes && this.eventTypes.length > 0) {
      for (const _item of this.eventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.baseEventTypes && this.baseEventTypes.length > 0) {
      for (const _item of this.baseEventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = NodeDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 101;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.primaryStoreTypes.length > 0) {
      const packedPrimaryStoreTypes: any[] = [];
      for (const item of object.primaryStoreTypes) {
        packedPrimaryStoreTypes.push(item);
      }
      objectValue["104"] = packedPrimaryStoreTypes;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    if (object.groups.length > 0) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toValue());
      }
      objectValue["106"] = packedGroups;
    }
    objectValue["110"] = object.isGlobal;
    objectValue["111"] = object.isSpatial;
    objectValue["112"] = object.isAbstract;
    objectValue["113"] = object.isExtensible;
    objectValue["114"] = object.isFrozen;
    if (object.baseType != null) {
      objectValue["120"] = object.baseType;
    }
    if (object.extendedBy.length > 0) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectValue["121"] = packedExtendedBy;
    }
    if (object.inherits.length > 0) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectValue["122"] = packedInherits;
    }
    if (object.inheritedBy.length > 0) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectValue["123"] = packedInheritedBy;
    }
    if (object.baseTraits.length > 0) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(item);
      }
      objectValue["124"] = packedBaseTraits;
    }
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["125"] = packedTraits;
    }
    if (object.rootType != null) {
      objectValue["130"] = object.rootType;
    }
    if (object.parentTypes.length > 0) {
      const packedParentTypes: any[] = [];
      for (const item of object.parentTypes) {
        packedParentTypes.push(item);
      }
      objectValue["131"] = packedParentTypes;
    }
    if (object.childTypes.length > 0) {
      const packedChildTypes: any[] = [];
      for (const item of object.childTypes) {
        packedChildTypes.push(item);
      }
      objectValue["132"] = packedChildTypes;
    }
    if (object.ancestorTypes.length > 0) {
      const packedAncestorTypes: any[] = [];
      for (const item of object.ancestorTypes) {
        packedAncestorTypes.push(item);
      }
      objectValue["133"] = packedAncestorTypes;
    }
    if (object.descendantTypes.length > 0) {
      const packedDescendantTypes: any[] = [];
      for (const item of object.descendantTypes) {
        packedDescendantTypes.push(item);
      }
      objectValue["134"] = packedDescendantTypes;
    }
    if (object.eventTypes.length > 0) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectValue["140"] = packedEventTypes;
    }
    if (object.baseEventTypes.length > 0) {
      const packedBaseEventTypes: any[] = [];
      for (const item of object.baseEventTypes) {
        packedBaseEventTypes.push(item);
      }
      objectValue["141"] = packedBaseEventTypes;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedPrimaryStoreTypes: any[] = [];
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedPrimaryStoreTypes.push(Number(item));
      }
    }
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectValue["106"] != undefined) {
      for (const item of objectValue["106"]) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const baseTypeValue = objectValue["120"];
    const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : null;
    const unpackedExtendedBy: any[] = [];
    if (objectValue["121"] != undefined) {
      for (const item of objectValue["121"]) {
        unpackedExtendedBy.push(Number(item));
      }
    }
    const unpackedInherits: any[] = [];
    if (objectValue["122"] != undefined) {
      for (const item of objectValue["122"]) {
        unpackedInherits.push(Number(item));
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectValue["123"] != undefined) {
      for (const item of objectValue["123"]) {
        unpackedInheritedBy.push(Number(item));
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectValue["124"] != undefined) {
      for (const item of objectValue["124"]) {
        unpackedBaseTraits.push(Number(item));
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["125"] != undefined) {
      for (const item of objectValue["125"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const rootTypeValue = objectValue["130"];
    const unpackedRootType = rootTypeValue != undefined ? Number(rootTypeValue) : null;
    const unpackedParentTypes: any[] = [];
    if (objectValue["131"] != undefined) {
      for (const item of objectValue["131"]) {
        unpackedParentTypes.push(Number(item));
      }
    }
    const unpackedChildTypes: any[] = [];
    if (objectValue["132"] != undefined) {
      for (const item of objectValue["132"]) {
        unpackedChildTypes.push(Number(item));
      }
    }
    const unpackedAncestorTypes: any[] = [];
    if (objectValue["133"] != undefined) {
      for (const item of objectValue["133"]) {
        unpackedAncestorTypes.push(Number(item));
      }
    }
    const unpackedDescendantTypes: any[] = [];
    if (objectValue["134"] != undefined) {
      for (const item of objectValue["134"]) {
        unpackedDescendantTypes.push(Number(item));
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectValue["140"] != undefined) {
      for (const item of objectValue["140"]) {
        unpackedEventTypes.push(Number(item));
      }
    }
    const unpackedBaseEventTypes: any[] = [];
    if (objectValue["141"] != undefined) {
      for (const item of objectValue["141"]) {
        unpackedBaseEventTypes.push(Number(item));
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new NodeDefinition({
      type: Number(objectValue["100"]),
      primaryStoreTypes: unpackedPrimaryStoreTypes,
      properties: unpackedProperties,
      groups: unpackedGroups,
      isGlobal: objectValue["110"],
      isSpatial: objectValue["111"],
      isAbstract: objectValue["112"],
      isExtensible: objectValue["113"],
      isFrozen: objectValue["114"],
      baseType: unpackedBaseType,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      baseTraits: unpackedBaseTraits,
      traits: unpackedTraits,
      rootType: unpackedRootType,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      eventTypes: unpackedEventTypes,
      baseEventTypes: unpackedBaseEventTypes,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinition {
    return NodeDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): NodeDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeDefinition.__packProto__(this);
    }
    return this._proto as NodeDefinitionProto;
  }

  static __packProto__(object: NodeDefinition): NodeDefinitionProto {
    const objectProto: Partial<NodeDefinitionProto> = { metatype: 101 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as NodeTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.primaryStoreTypes) {
      const packedPrimaryStoreTypes: any[] = [];
      for (const item of object.primaryStoreTypes) {
        packedPrimaryStoreTypes.push(Number(item) as StoreTypeProto);
      }
      objectProto.primaryStoreTypes = packedPrimaryStoreTypes;
    }
    if (object.properties) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toProto());
      }
      objectProto.properties = packedProperties;
    }
    if (object.groups) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toProto());
      }
      objectProto.groups = packedGroups;
    }
    objectProto.isGlobal = object.isGlobal;
    objectProto.isSpatial = object.isSpatial;
    objectProto.isAbstract = object.isAbstract;
    objectProto.isExtensible = object.isExtensible;
    objectProto.isFrozen = object.isFrozen;
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
    if (object.baseTraits) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.baseTraits = packedBaseTraits;
    }
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.rootType != null) {
      objectProto.rootType = Number(object.rootType) as NodeTypeProto;
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
    if (object.eventTypes) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.eventTypes = packedEventTypes;
    }
    if (object.baseEventTypes) {
      const packedBaseEventTypes: any[] = [];
      for (const item of object.baseEventTypes) {
        packedBaseEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.baseEventTypes = packedBaseEventTypes;
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
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedPrimaryStoreTypes: any[] = [];
    if (objectProto.primaryStoreTypes) {
      for (const item of objectProto.primaryStoreTypes) {
        unpackedPrimaryStoreTypes.push(Number(item) as StoreType);
      }
    }
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectProto.groups) {
      for (const item of objectProto.groups) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
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
    const unpackedBaseTraits: any[] = [];
    if (objectProto.baseTraits) {
      for (const item of objectProto.baseTraits) {
        unpackedBaseTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
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
    const unpackedEventTypes: any[] = [];
    if (objectProto.eventTypes) {
      for (const item of objectProto.eventTypes) {
        unpackedEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedBaseEventTypes: any[] = [];
    if (objectProto.baseEventTypes) {
      for (const item of objectProto.baseEventTypes) {
        unpackedBaseEventTypes.push(Number(item) as NodeType);
      }
    }
    return new NodeDefinition({
      type: Number(objectProto.type) as NodeType,
      primaryStoreTypes: unpackedPrimaryStoreTypes,
      properties: unpackedProperties,
      groups: unpackedGroups,
      isGlobal: objectProto.isGlobal,
      isSpatial: objectProto.isSpatial,
      isAbstract: objectProto.isAbstract,
      isExtensible: objectProto.isExtensible,
      isFrozen: objectProto.isFrozen,
      baseType:
        objectProto.baseType != undefined ? (Number(objectProto.baseType) as NodeType) : null,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      baseTraits: unpackedBaseTraits,
      traits: unpackedTraits,
      rootType:
        objectProto.rootType != undefined ? (Number(objectProto.rootType) as NodeType) : null,
      parentTypes: unpackedParentTypes,
      childTypes: unpackedChildTypes,
      ancestorTypes: unpackedAncestorTypes,
      descendantTypes: unpackedDescendantTypes,
      eventTypes: unpackedEventTypes,
      baseEventTypes: unpackedBaseEventTypes,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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
  resolveProperty(name: string): PropertyDefinition | null {
    const nodeClass = NODE_CLASS_BY_TYPE[this.type];
    const property = nodeClass.__properties__[name];
    if (property == null) {
      return null;
    }
    return property;
  }

  /** Resolve a Property in this definition (error if not found). */
  resolvePropertyOrError(name: string): PropertyDefinition {
    const property = this.resolveProperty(name);
    if (property == null) {
      throw new Error(`could not find property ${name} in ${this.repr()}`);
    }
    return property;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_DEFINITION, NodeDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:101 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:102 ==== */
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
   * TraitDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * TraitDefinition.groups
   */
  readonly groups: Array<PropertyGroupDefinition>;

  /**
   * TraitDefinition.alias
   */
  readonly alias: string;

  /**
   * Whether this Trait can be extended by custom Nodes and custom Traits.
   */
  readonly isExtensible: boolean;

  /**
   * Traits directly and indirectly inherited by this trait.
   */
  readonly traits: Array<TraitType>;

  /**
   * Traits directly inherited by this trait.
   */
  readonly baseTraits: Array<TraitType>;

  /**
   * The event types of this Trait (directly and indirectly).
   */
  readonly eventTypes: Array<NodeType>;

  /**
   * The base event types of this Trait (directly).
   */
  readonly baseEventTypes: Array<NodeType>;

  constructor(options: {
    id: number;
    type: TraitType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    groups?: Array<PropertyGroupDefinition>;
    alias: string;
    isExtensible: boolean;
    traits?: Array<TraitType>;
    baseTraits?: Array<TraitType>;
    eventTypes?: Array<NodeType>;
    baseEventTypes?: Array<NodeType>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _groups = options.groups ?? null;
    if (_groups === null) {
      _groups = [];
    }
    this.groups = _groups;
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
    let _traits = options.traits ?? null;
    if (_traits === null) {
      _traits = [];
    }
    this.traits = _traits;
    let _baseTraits = options.baseTraits ?? null;
    if (_baseTraits === null) {
      _baseTraits = [];
    }
    this.baseTraits = _baseTraits;
    let _eventTypes = options.eventTypes ?? null;
    if (_eventTypes === null) {
      _eventTypes = [];
    }
    this.eventTypes = _eventTypes;
    let _baseEventTypes = options.baseEventTypes ?? null;
    if (_baseEventTypes === null) {
      _baseEventTypes = [];
    }
    this.baseEventTypes = _baseEventTypes;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.groups.length !== other.groups.length) {
      return false;
    }
    for (let i = 0; i < this.groups.length; i++) {
      if (!this.groups[i].equals(other.groups[i])) {
        return false;
      }
    }
    if (!(this.alias === other.alias)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (this.traits.length !== other.traits.length) {
      return false;
    }
    for (let i = 0; i < this.traits.length; i++) {
      if (!(this.traits[i] === other.traits[i])) {
        return false;
      }
    }
    if (this.baseTraits.length !== other.baseTraits.length) {
      return false;
    }
    for (let i = 0; i < this.baseTraits.length; i++) {
      if (!(this.baseTraits[i] === other.baseTraits[i])) {
        return false;
      }
    }
    if (this.eventTypes.length !== other.eventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.eventTypes.length; i++) {
      if (!(this.eventTypes[i] === other.eventTypes[i])) {
        return false;
      }
    }
    if (this.baseEventTypes.length !== other.baseEventTypes.length) {
      return false;
    }
    for (let i = 0; i < this.baseEventTypes.length; i++) {
      if (!(this.baseEventTypes[i] === other.baseEventTypes[i])) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${TraitType[this.type]}`);
      propertyReprs.push(`alias=${this.alias}`);
      propertyReprs.push(`isExtensible=${this.isExtensible}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<TraitDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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
    if (this.groups && this.groups.length > 0) {
      for (const _item of this.groups) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashString(this.alias)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.traits && this.traits.length > 0) {
      for (const _item of this.traits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.baseTraits && this.baseTraits.length > 0) {
      for (const _item of this.baseTraits) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.eventTypes && this.eventTypes.length > 0) {
      for (const _item of this.eventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.baseEventTypes && this.baseEventTypes.length > 0) {
      for (const _item of this.baseEventTypes) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = TraitDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: TraitDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 102;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    if (object.groups.length > 0) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toValue());
      }
      objectValue["106"] = packedGroups;
    }
    objectValue["110"] = object.alias;
    objectValue["111"] = object.isExtensible;
    if (object.traits.length > 0) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(item);
      }
      objectValue["120"] = packedTraits;
    }
    if (object.baseTraits.length > 0) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(item);
      }
      objectValue["121"] = packedBaseTraits;
    }
    if (object.eventTypes.length > 0) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(item);
      }
      objectValue["140"] = packedEventTypes;
    }
    if (object.baseEventTypes.length > 0) {
      const packedBaseEventTypes: any[] = [];
      for (const item of object.baseEventTypes) {
        packedBaseEventTypes.push(item);
      }
      objectValue["141"] = packedBaseEventTypes;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectValue["106"] != undefined) {
      for (const item of objectValue["106"]) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTraits: any[] = [];
    if (objectValue["120"] != undefined) {
      for (const item of objectValue["120"]) {
        unpackedTraits.push(Number(item));
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectValue["121"] != undefined) {
      for (const item of objectValue["121"]) {
        unpackedBaseTraits.push(Number(item));
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectValue["140"] != undefined) {
      for (const item of objectValue["140"]) {
        unpackedEventTypes.push(Number(item));
      }
    }
    const unpackedBaseEventTypes: any[] = [];
    if (objectValue["141"] != undefined) {
      for (const item of objectValue["141"]) {
        unpackedBaseEventTypes.push(Number(item));
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new TraitDefinition({
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      groups: unpackedGroups,
      alias: objectValue["110"],
      isExtensible: objectValue["111"],
      traits: unpackedTraits,
      baseTraits: unpackedBaseTraits,
      eventTypes: unpackedEventTypes,
      baseEventTypes: unpackedBaseEventTypes,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TraitDefinition {
    return TraitDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TraitDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = TraitDefinition.__packProto__(this);
    }
    return this._proto as TraitDefinitionProto;
  }

  static __packProto__(object: TraitDefinition): TraitDefinitionProto {
    const objectProto: Partial<TraitDefinitionProto> = { metatype: 102 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as TraitTypeProto;
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
    if (object.groups) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toProto());
      }
      objectProto.groups = packedGroups;
    }
    objectProto.alias = object.alias;
    objectProto.isExtensible = object.isExtensible;
    if (object.traits) {
      const packedTraits: any[] = [];
      for (const item of object.traits) {
        packedTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.traits = packedTraits;
    }
    if (object.baseTraits) {
      const packedBaseTraits: any[] = [];
      for (const item of object.baseTraits) {
        packedBaseTraits.push(Number(item) as TraitTypeProto);
      }
      objectProto.baseTraits = packedBaseTraits;
    }
    if (object.eventTypes) {
      const packedEventTypes: any[] = [];
      for (const item of object.eventTypes) {
        packedEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.eventTypes = packedEventTypes;
    }
    if (object.baseEventTypes) {
      const packedBaseEventTypes: any[] = [];
      for (const item of object.baseEventTypes) {
        packedBaseEventTypes.push(Number(item) as NodeTypeProto);
      }
      objectProto.baseEventTypes = packedBaseEventTypes;
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
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectProto.groups) {
      for (const item of objectProto.groups) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedTraits: any[] = [];
    if (objectProto.traits) {
      for (const item of objectProto.traits) {
        unpackedTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedBaseTraits: any[] = [];
    if (objectProto.baseTraits) {
      for (const item of objectProto.baseTraits) {
        unpackedBaseTraits.push(Number(item) as TraitType);
      }
    }
    const unpackedEventTypes: any[] = [];
    if (objectProto.eventTypes) {
      for (const item of objectProto.eventTypes) {
        unpackedEventTypes.push(Number(item) as NodeType);
      }
    }
    const unpackedBaseEventTypes: any[] = [];
    if (objectProto.baseEventTypes) {
      for (const item of objectProto.baseEventTypes) {
        unpackedBaseEventTypes.push(Number(item) as NodeType);
      }
    }
    return new TraitDefinition({
      type: Number(objectProto.type) as TraitType,
      properties: unpackedProperties,
      groups: unpackedGroups,
      alias: objectProto.alias,
      isExtensible: objectProto.isExtensible,
      traits: unpackedTraits,
      baseTraits: unpackedBaseTraits,
      eventTypes: unpackedEventTypes,
      baseEventTypes: unpackedBaseEventTypes,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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

  /** Resolve a Property in this definition. */
  resolveProperty(name: string): PropertyDefinition | null {
    const nodeClass = TRAIT_CLASS_BY_TYPE[this.type];
    const property = nodeClass.__properties__[name];
    if (property == null) {
      return null;
    }
    return property;
  }

  /** Resolve a Property in this definition (error if not found). */
  resolvePropertyOrError(name: string): PropertyDefinition {
    const property = this.resolveProperty(name);
    if (property == null) {
      throw new Error(`could not find property ${name} in ${this.repr()}`);
    }
    return property;
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TRAIT_DEFINITION, TraitDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:102 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:103 ==== */
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
   * StructDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  /**
   * StructDefinition.groups
   */
  readonly groups: Array<PropertyGroupDefinition>;

  /**
   * Whether this Struct cannot be modified.
   */
  readonly isFrozen: boolean;

  /**
   * Whether this Struct cannot be instantiated directly.
   */
  readonly isAbstract: boolean;

  /**
   * Whether this Struct can be extended by custom Structs.
   */
  readonly isExtensible: boolean;

  /**
   * The base type this Struct extends (directly).
   */
  readonly baseType: StructType | null;

  /**
   * Structs that extend this Struct type (directly).
   */
  readonly extendedBy: Array<StructType>;

  /**
   * Structs that this Struct inherits (directly and indirectly).
   */
  readonly inherits: Array<StructType>;

  /**
   * Structs that inherit this Struct type (directly and indirectly).
   */
  readonly inheritedBy: Array<StructType>;

  constructor(options: {
    id: number;
    type: StructType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    groups?: Array<PropertyGroupDefinition>;
    isFrozen: boolean;
    isAbstract: boolean;
    isExtensible: boolean;
    baseType?: StructType | null;
    extendedBy?: Array<StructType>;
    inherits?: Array<StructType>;
    inheritedBy?: Array<StructType>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _properties = options.properties ?? null;
    if (_properties === null) {
      _properties = [];
    }
    this.properties = _properties;
    let _groups = options.groups ?? null;
    if (_groups === null) {
      _groups = [];
    }
    this.groups = _groups;
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

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
      return false;
    }
    for (let i = 0; i < this.properties.length; i++) {
      if (!this.properties[i].equals(other.properties[i])) {
        return false;
      }
    }
    if (this.groups.length !== other.groups.length) {
      return false;
    }
    for (let i = 0; i < this.groups.length; i++) {
      if (!this.groups[i].equals(other.groups[i])) {
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
    if (!(this.baseType === other.baseType)) {
      return false;
    }
    if (this.extendedBy.length !== other.extendedBy.length) {
      return false;
    }
    for (let i = 0; i < this.extendedBy.length; i++) {
      if (!(this.extendedBy[i] === other.extendedBy[i])) {
        return false;
      }
    }
    if (this.inherits.length !== other.inherits.length) {
      return false;
    }
    for (let i = 0; i < this.inherits.length; i++) {
      if (!(this.inherits[i] === other.inherits[i])) {
        return false;
      }
    }
    if (this.inheritedBy.length !== other.inheritedBy.length) {
      return false;
    }
    for (let i = 0; i < this.inheritedBy.length; i++) {
      if (!(this.inheritedBy[i] === other.inheritedBy[i])) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${StructType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<StructDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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
    if (this.groups && this.groups.length > 0) {
      for (const _item of this.groups) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isAbstract)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.baseType !== null) {
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
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = StructDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StructDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 103;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["105"] = packedProperties;
    }
    if (object.groups.length > 0) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toValue());
      }
      objectValue["106"] = packedGroups;
    }
    objectValue["110"] = object.isFrozen;
    objectValue["111"] = object.isAbstract;
    objectValue["112"] = object.isExtensible;
    if (object.baseType != null) {
      objectValue["120"] = object.baseType;
    }
    if (object.extendedBy.length > 0) {
      const packedExtendedBy: any[] = [];
      for (const item of object.extendedBy) {
        packedExtendedBy.push(item);
      }
      objectValue["121"] = packedExtendedBy;
    }
    if (object.inherits.length > 0) {
      const packedInherits: any[] = [];
      for (const item of object.inherits) {
        packedInherits.push(item);
      }
      objectValue["122"] = packedInherits;
    }
    if (object.inheritedBy.length > 0) {
      const packedInheritedBy: any[] = [];
      for (const item of object.inheritedBy) {
        packedInheritedBy.push(item);
      }
      objectValue["123"] = packedInheritedBy;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["105"] != undefined) {
      for (const item of objectValue["105"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectValue["106"] != undefined) {
      for (const item of objectValue["106"]) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const baseTypeValue = objectValue["120"];
    const unpackedBaseType = baseTypeValue != undefined ? Number(baseTypeValue) : null;
    const unpackedExtendedBy: any[] = [];
    if (objectValue["121"] != undefined) {
      for (const item of objectValue["121"]) {
        unpackedExtendedBy.push(Number(item));
      }
    }
    const unpackedInherits: any[] = [];
    if (objectValue["122"] != undefined) {
      for (const item of objectValue["122"]) {
        unpackedInherits.push(Number(item));
      }
    }
    const unpackedInheritedBy: any[] = [];
    if (objectValue["123"] != undefined) {
      for (const item of objectValue["123"]) {
        unpackedInheritedBy.push(Number(item));
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new StructDefinition({
      type: Number(objectValue["100"]),
      properties: unpackedProperties,
      groups: unpackedGroups,
      isFrozen: objectValue["110"],
      isAbstract: objectValue["111"],
      isExtensible: objectValue["112"],
      baseType: unpackedBaseType,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinition {
    return StructDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): StructDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StructDefinition.__packProto__(this);
    }
    return this._proto as StructDefinitionProto;
  }

  static __packProto__(object: StructDefinition): StructDefinitionProto {
    const objectProto: Partial<StructDefinitionProto> = { metatype: 103 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as StructTypeProto;
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
    if (object.groups) {
      const packedGroups: any[] = [];
      for (const item of object.groups) {
        packedGroups.push(item.toProto());
      }
      objectProto.groups = packedGroups;
    }
    objectProto.isFrozen = object.isFrozen;
    objectProto.isAbstract = object.isAbstract;
    objectProto.isExtensible = object.isExtensible;
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
    const _PropertyGroupDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_GROUP_DEFINITION
    ] as typeof PropertyGroupDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedGroups: any[] = [];
    if (objectProto.groups) {
      for (const item of objectProto.groups) {
        unpackedGroups.push(
          _PropertyGroupDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
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
    return new StructDefinition({
      type: Number(objectProto.type) as StructType,
      properties: unpackedProperties,
      groups: unpackedGroups,
      isFrozen: objectProto.isFrozen,
      isAbstract: objectProto.isAbstract,
      isExtensible: objectProto.isExtensible,
      baseType:
        objectProto.baseType != undefined ? (Number(objectProto.baseType) as StructType) : null,
      extendedBy: unpackedExtendedBy,
      inherits: unpackedInherits,
      inheritedBy: unpackedInheritedBy,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:103 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:104 ==== */
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
  readonly options: Array<OptionDefinition>;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    options?: Array<OptionDefinition>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (this.options.length !== other.options.length) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<EnumDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
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
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = EnumDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: EnumDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 104;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.options.length > 0) {
      const packedOptions: any[] = [];
      for (const item of object.options) {
        packedOptions.push(item.toValue());
      }
      objectValue["104"] = packedOptions;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
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
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedOptions.push(
          _OptionDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new EnumDefinition({
      type: Number(objectValue["100"]),
      options: unpackedOptions,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): EnumDefinition {
    return EnumDefinition.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): EnumDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = EnumDefinition.__packProto__(this);
    }
    return this._proto as EnumDefinitionProto;
  }

  static __packProto__(object: EnumDefinition): EnumDefinitionProto {
    const objectProto: Partial<EnumDefinitionProto> = { metatype: 104 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:104 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:110 ==== */
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
   * PropertyDefinition.groupId
   */
  readonly groupId: number | null;

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
   * PropertyDefinition.nodeIsExtensible
   */
  readonly nodeIsExtensible: boolean;

  /**
   * PropertyDefinition.nodeIsHeterogenous
   */
  readonly nodeIsHeterogenous: boolean;

  /**
   * PropertyDefinition.nodeIsSpatial
   */
  readonly nodeIsSpatial: boolean;

  /**
   * PropertyDefinition.edgeType
   */
  readonly edgeType: EdgeType | null;

  /**
   * PropertyDefinition.cascade
   */
  readonly cascade: CascadeAction | null;

  /**
   * PropertyDefinition.isRequired
   */
  readonly isRequired: boolean;

  /**
   * PropertyDefinition.isUnique
   */
  readonly isUnique: boolean;

  /**
   * PropertyDefinition.isReadonly
   */
  readonly isReadonly: boolean;

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
   * PropertyDefinition.isManaged
   */
  readonly isManaged: boolean;

  constructor(options: {
    id: number;
    type: PropertyType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    object: ObjectDefinitionReference;
    originalObject: ObjectDefinitionReference;
    groupId?: number | null;
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
    nodeIsExtensible: boolean;
    nodeIsHeterogenous: boolean;
    nodeIsSpatial: boolean;
    edgeType?: EdgeType | null;
    cascade?: CascadeAction | null;
    isRequired: boolean;
    isUnique: boolean;
    isReadonly: boolean;
    isWired: boolean;
    isStored: boolean;
    isRepr: boolean;
    isHash: boolean;
    isEq: boolean;
    isManaged: boolean;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _groupId = options.groupId ?? null;
    this.groupId = _groupId;
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
    let _nodeIsExtensible = options.nodeIsExtensible;
    if (_nodeIsExtensible === null) {
      throw new Error(`PropertyDefinition.nodeIsExtensible is required`);
    }
    this.nodeIsExtensible = _nodeIsExtensible;
    let _nodeIsHeterogenous = options.nodeIsHeterogenous;
    if (_nodeIsHeterogenous === null) {
      throw new Error(`PropertyDefinition.nodeIsHeterogenous is required`);
    }
    this.nodeIsHeterogenous = _nodeIsHeterogenous;
    let _nodeIsSpatial = options.nodeIsSpatial;
    if (_nodeIsSpatial === null) {
      throw new Error(`PropertyDefinition.nodeIsSpatial is required`);
    }
    this.nodeIsSpatial = _nodeIsSpatial;
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
    let _isManaged = options.isManaged;
    if (_isManaged === null) {
      throw new Error(`PropertyDefinition.isManaged is required`);
    }
    this.isManaged = _isManaged;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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
    if (!(this.groupId === other.groupId)) {
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
    if (!(this.nodeIsExtensible === other.nodeIsExtensible)) {
      return false;
    }
    if (!(this.nodeIsHeterogenous === other.nodeIsHeterogenous)) {
      return false;
    }
    if (!(this.nodeIsSpatial === other.nodeIsSpatial)) {
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
    if (!(this.isManaged === other.isManaged)) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
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
      if (this.keyType !== null) {
        propertyReprs.push(`keyType=${this.keyType.repr()}`);
      }
      if (this.value !== null) {
        propertyReprs.push(`value=${this.value.repr()}`);
      }
      if (this.valueFactory !== null) {
        propertyReprs.push(`valueFactory=${ValueFactory[this.valueFactory]}`);
      }
      propertyReprs.push(`isRequired=${this.isRequired}`);
      propertyReprs.push(`isUnique=${this.isUnique}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PropertyDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.object.hash()) & 0xffffffff;
    h = (h * 31 + this.originalObject.hash()) & 0xffffffff;
    if (this.groupId !== null) {
      h = (h * 31 + hashInt(this.groupId)) & 0xffffffff;
    }
    h = (h * 31 + this.cardinality) & 0xffffffff;
    h = (h * 31 + this.scalarType) & 0xffffffff;
    if (this.primitiveType !== null) {
      h = (h * 31 + this.primitiveType) & 0xffffffff;
    }
    if (this.enumType !== null) {
      h = (h * 31 + this.enumType) & 0xffffffff;
    }
    if (this.nodeType !== null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.keyType !== null) {
      h = (h * 31 + this.keyType.hash()) & 0xffffffff;
    }
    if (this.value !== null) {
      h = (h * 31 + this.value.hash()) & 0xffffffff;
    }
    if (this.valueFactory !== null) {
      h = (h * 31 + this.valueFactory) & 0xffffffff;
    }
    if (this.collectionConstraint !== null) {
      h = (h * 31 + this.collectionConstraint.hash()) & 0xffffffff;
    }
    if (this.stringConstraint !== null) {
      h = (h * 31 + this.stringConstraint.hash()) & 0xffffffff;
    }
    if (this.numberConstraint !== null) {
      h = (h * 31 + this.numberConstraint.hash()) & 0xffffffff;
    }
    if (this.nodeConstraint !== null) {
      h = (h * 31 + this.nodeConstraint.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.nodeIsExtensible)) & 0xffffffff;
    h = (h * 31 + hashBool(this.nodeIsHeterogenous)) & 0xffffffff;
    h = (h * 31 + hashBool(this.nodeIsSpatial)) & 0xffffffff;
    if (this.edgeType !== null) {
      h = (h * 31 + this.edgeType) & 0xffffffff;
    }
    if (this.cascade !== null) {
      h = (h * 31 + this.cascade) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isRequired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isUnique)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isReadonly)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isWired)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isStored)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isRepr)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isHash)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isEq)) & 0xffffffff;
    h = (h * 31 + hashBool(this.isManaged)) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PropertyDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PropertyDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 110;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    objectValue["104"] = object.object.toValue();
    objectValue["105"] = object.originalObject.toValue();
    if (object.groupId != null) {
      objectValue["106"] = object.groupId;
    }
    objectValue["110"] = object.cardinality;
    objectValue["111"] = object.scalarType;
    if (object.primitiveType != null) {
      objectValue["112"] = object.primitiveType;
    }
    if (object.enumType != null) {
      objectValue["113"] = object.enumType;
    }
    if (object.nodeType != null) {
      objectValue["114"] = object.nodeType;
    }
    if (object.structType != null) {
      objectValue["115"] = object.structType;
    }
    if (object.keyType != null) {
      objectValue["116"] = object.keyType.toValue();
    }
    if (object.value != null) {
      objectValue["120"] = object.value.toValue();
    }
    if (object.valueFactory != null) {
      objectValue["121"] = object.valueFactory;
    }
    if (object.collectionConstraint != null) {
      objectValue["130"] = object.collectionConstraint.toValue();
    }
    if (object.stringConstraint != null) {
      objectValue["131"] = object.stringConstraint.toValue();
    }
    if (object.numberConstraint != null) {
      objectValue["132"] = object.numberConstraint.toValue();
    }
    if (object.nodeConstraint != null) {
      objectValue["133"] = object.nodeConstraint.toValue();
    }
    objectValue["140"] = object.nodeIsExtensible;
    objectValue["141"] = object.nodeIsHeterogenous;
    objectValue["142"] = object.nodeIsSpatial;
    if (object.edgeType != null) {
      objectValue["144"] = object.edgeType;
    }
    if (object.cascade != null) {
      objectValue["145"] = object.cascade;
    }
    objectValue["150"] = object.isRequired;
    objectValue["151"] = object.isUnique;
    objectValue["153"] = object.isReadonly;
    objectValue["155"] = object.isWired;
    objectValue["156"] = object.isStored;
    objectValue["157"] = object.isRepr;
    objectValue["158"] = object.isHash;
    objectValue["159"] = object.isEq;
    objectValue["160"] = object.isManaged;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
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
    const groupIdValue = objectValue["106"];
    const unpackedGroupId = groupIdValue != undefined ? Number(groupIdValue) : null;
    const primitiveTypeValue = objectValue["112"];
    const unpackedPrimitiveType =
      primitiveTypeValue != undefined ? Number(primitiveTypeValue) : null;
    const enumTypeValue = objectValue["113"];
    const unpackedEnumType = enumTypeValue != undefined ? Number(enumTypeValue) : null;
    const nodeTypeValue = objectValue["114"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const structTypeValue = objectValue["115"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const keyTypeValue = objectValue["116"];
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
    const edgeTypeValue = objectValue["144"];
    const unpackedEdgeType = edgeTypeValue != undefined ? Number(edgeTypeValue) : null;
    const cascadeValue = objectValue["145"];
    const unpackedCascade = cascadeValue != undefined ? Number(cascadeValue) : null;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new PropertyDefinition({
      type: Number(objectValue["100"]),
      object: _ObjectDefinitionReference.fromValue(
        objectValue["104"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      originalObject: _ObjectDefinitionReference.fromValue(
        objectValue["105"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      groupId: unpackedGroupId,
      cardinality: Number(objectValue["110"]),
      scalarType: Number(objectValue["111"]),
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
      nodeIsExtensible: objectValue["140"],
      nodeIsHeterogenous: objectValue["141"],
      nodeIsSpatial: objectValue["142"],
      edgeType: unpackedEdgeType,
      cascade: unpackedCascade,
      isRequired: objectValue["150"],
      isUnique: objectValue["151"],
      isReadonly: objectValue["153"],
      isWired: objectValue["155"],
      isStored: objectValue["156"],
      isRepr: objectValue["157"],
      isHash: objectValue["158"],
      isEq: objectValue["159"],
      isManaged: objectValue["160"],
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyDefinition {
    return PropertyDefinition.__unpackValue__(
      objectValue,
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
    const objectProto: Partial<PropertyDefinitionProto> = { metatype: 110 };
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
    if (object.groupId != null) {
      objectProto.groupId = object.groupId;
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
    objectProto.nodeIsExtensible = object.nodeIsExtensible;
    objectProto.nodeIsHeterogenous = object.nodeIsHeterogenous;
    objectProto.nodeIsSpatial = object.nodeIsSpatial;
    if (object.edgeType != null) {
      objectProto.edgeType = Number(object.edgeType) as EdgeTypeProto;
    }
    if (object.cascade != null) {
      objectProto.cascade = Number(object.cascade) as CascadeActionProto;
    }
    objectProto.isRequired = object.isRequired;
    objectProto.isUnique = object.isUnique;
    objectProto.isReadonly = object.isReadonly;
    objectProto.isWired = object.isWired;
    objectProto.isStored = object.isStored;
    objectProto.isRepr = object.isRepr;
    objectProto.isHash = object.isHash;
    objectProto.isEq = object.isEq;
    objectProto.isManaged = object.isManaged;
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
      groupId: objectProto.groupId != undefined ? Number(objectProto.groupId) : null,
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
      nodeIsExtensible: objectProto.nodeIsExtensible,
      nodeIsHeterogenous: objectProto.nodeIsHeterogenous,
      nodeIsSpatial: objectProto.nodeIsSpatial,
      edgeType:
        objectProto.edgeType != undefined ? (Number(objectProto.edgeType) as EdgeType) : null,
      cascade:
        objectProto.cascade != undefined ? (Number(objectProto.cascade) as CascadeAction) : null,
      isRequired: objectProto.isRequired,
      isUnique: objectProto.isUnique,
      isReadonly: objectProto.isReadonly,
      isWired: objectProto.isWired,
      isStored: objectProto.isStored,
      isRepr: objectProto.isRepr,
      isHash: objectProto.isHash,
      isEq: objectProto.isEq,
      isManaged: objectProto.isManaged,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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
    if (this.object.type === ObjectDefinitionType.BUILTIN_NODE) {
      return new PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        nodeType: this.object.nodeType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_STRUCT) {
      return new PropertyReference({
        type: PropertyReferenceType.BUILTIN,
        structType: this.object.structType,
        id: this.id,
      });
    } else if (this.object.type === ObjectDefinitionType.BUILTIN_TRAIT) {
      return new PropertyReference({
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
/* ==== DESTACK_GENERATED_END:STRUCT:110 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:111 ==== */
/**
 * Definition of a builtin Property Group.
 */
export class PropertyGroupDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.PROPERTY_GROUP_DEFINITION;
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

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
      throw new Error(`PropertyGroupDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PropertyGroupDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PropertyGroupDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PropertyGroupDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PropertyGroupDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 111;
    objectValue["2"] = object.id;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyGroupDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new PropertyGroupDefinition({
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyGroupDefinition {
    return PropertyGroupDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PropertyGroupDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PropertyGroupDefinition.__packProto__(this);
    }
    return this._proto as PropertyGroupDefinitionProto;
  }

  static __packProto__(object: PropertyGroupDefinition): PropertyGroupDefinitionProto {
    const objectProto: Partial<PropertyGroupDefinitionProto> = { metatype: 111 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    return objectProto as PropertyGroupDefinitionProto;
  }

  static __unpackProto__(
    objectProto: PropertyGroupDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyGroupDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new PropertyGroupDefinition({
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PropertyGroupDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyGroupDefinition {
    return PropertyGroupDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PropertyGroupDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PropertyGroupDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PROPERTY_GROUP_DEFINITION, PropertyGroupDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:111 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:112 ==== */
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
   * OptionDefinition.groupId
   */
  readonly groupId: number | null;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    groupId?: number | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _groupId = options.groupId ?? null;
    this.groupId = _groupId;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.groupId === other.groupId)) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<OptionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.groupId !== null) {
      h = (h * 31 + hashInt(this.groupId)) & 0xffffffff;
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = OptionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: OptionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 112;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.groupId != null) {
      objectValue["105"] = object.groupId;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const groupIdValue = objectValue["105"];
    const unpackedGroupId = groupIdValue != undefined ? Number(groupIdValue) : null;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new OptionDefinition({
      type: Number(objectValue["100"]),
      groupId: unpackedGroupId,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionDefinition {
    return OptionDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): OptionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = OptionDefinition.__packProto__(this);
    }
    return this._proto as OptionDefinitionProto;
  }

  static __packProto__(object: OptionDefinition): OptionDefinitionProto {
    const objectProto: Partial<OptionDefinitionProto> = { metatype: 112 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    if (object.groupId != null) {
      objectProto.groupId = object.groupId;
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
    return new OptionDefinition({
      type: Number(objectProto.type) as EnumType,
      groupId: objectProto.groupId != undefined ? Number(objectProto.groupId) : null,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:112 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:113 ==== */
/**
 * Definition of a builtin Enum Option Group.
 */
export class OptionGroupDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.OPTION_GROUP_DEFINITION;
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

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
      throw new Error(`OptionGroupDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`OptionGroupDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<OptionGroupDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = OptionGroupDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: OptionGroupDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 113;
    objectValue["2"] = object.id;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionGroupDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new OptionGroupDefinition({
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionGroupDefinition {
    return OptionGroupDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): OptionGroupDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = OptionGroupDefinition.__packProto__(this);
    }
    return this._proto as OptionGroupDefinitionProto;
  }

  static __packProto__(object: OptionGroupDefinition): OptionGroupDefinitionProto {
    const objectProto: Partial<OptionGroupDefinitionProto> = { metatype: 113 };
    objectProto.id = object.id;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    return objectProto as OptionGroupDefinitionProto;
  }

  static __unpackProto__(
    objectProto: OptionGroupDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionGroupDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    return new OptionGroupDefinition({
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: OptionGroupDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): OptionGroupDefinition {
    return OptionGroupDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): OptionGroupDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = OptionGroupDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OPTION_GROUP_DEFINITION, OptionGroupDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:113 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:120 ==== */
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
    _value?: { [key: string]: any } | null;
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
    this._value = options._value ?? null;
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
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ConstantDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.description !== null) {
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = ConstantDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ConstantDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 120;
    objectValue["101"] = object.name;
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    objectValue["120"] = object.value.toValue();
    objectValue["130"] = object.isDeferred;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ConstantDefinition({
      name: objectValue["101"],
      description: unpackedDescription,
      value: _Value.fromValue(objectValue["120"], _session, _supergraph, _graph, _connection),
      isDeferred: objectValue["130"],
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ConstantDefinition {
    return ConstantDefinition.__unpackValue__(
      objectValue,
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
    const objectProto: Partial<ConstantDefinitionProto> = { metatype: 120 };
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
/* ==== DESTACK_GENERATED_END:STRUCT:120 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:130 ==== */
/**
 * Definition of a builtin Method.
 */
export class MethodDefinition extends BuiltinDefinition {
  static metatype: StructType = StructType.METHOD_DEFINITION;
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
   * MethodDefinition.properties
   */
  readonly properties: Array<PropertyDefinition>;

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
      throw new Error(`MethodDefinition.id is required`);
    }
    this.id = _id;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`MethodDefinition.name is required`);
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

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.properties.length !== other.properties.length) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<MethodDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = MethodDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: MethodDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 130;
    objectValue["2"] = object.id;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["104"] = packedProperties;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MethodDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new MethodDefinition({
      properties: unpackedProperties,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MethodDefinition {
    return MethodDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): MethodDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = MethodDefinition.__packProto__(this);
    }
    return this._proto as MethodDefinitionProto;
  }

  static __packProto__(object: MethodDefinition): MethodDefinitionProto {
    const objectProto: Partial<MethodDefinitionProto> = { metatype: 130 };
    objectProto.id = object.id;
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
    return objectProto as MethodDefinitionProto;
  }

  static __unpackProto__(
    objectProto: MethodDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MethodDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new MethodDefinition({
      properties: unpackedProperties,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: MethodDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): MethodDefinition {
    return MethodDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): MethodDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MethodDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.METHOD_DEFINITION, MethodDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:130 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:131 ==== */
/**
 * Definition of a builtin Action.
 */
export class ActionDefinition extends MethodDefinition {
  static metatype: StructType = StructType.ACTION_DEFINITION;
  static __isFrozen__: boolean = true;

  constructor(options: {
    id: number;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: Array<PropertyDefinition>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    if (this.properties.length !== other.properties.length) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ActionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.properties && this.properties.length > 0) {
      for (const _item of this.properties) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = ActionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ActionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 131;
    objectValue["2"] = object.id;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    if (object.properties.length > 0) {
      const packedProperties: any[] = [];
      for (const item of object.properties) {
        packedProperties.push(item.toValue());
      }
      objectValue["104"] = packedProperties;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ActionDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectValue["104"] != undefined) {
      for (const item of objectValue["104"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromValue(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ActionDefinition({
      properties: unpackedProperties,
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ActionDefinition {
    return ActionDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ActionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ActionDefinition.__packProto__(this);
    }
    return this._proto as ActionDefinitionProto;
  }

  static __packProto__(object: ActionDefinition): ActionDefinitionProto {
    const objectProto: Partial<ActionDefinitionProto> = { metatype: 131 };
    objectProto.id = object.id;
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
    return objectProto as ActionDefinitionProto;
  }

  static __unpackProto__(
    objectProto: ActionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ActionDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new ActionDefinition({
      properties: unpackedProperties,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ActionDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ActionDefinition {
    return ActionDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ActionDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ActionDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ACTION_DEFINITION, ActionDefinition);
/* ==== DESTACK_GENERATED_END:STRUCT:131 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:140 ==== */
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
   * PermissionDefinition.type
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
   * PermissionDefinition.nodeType
   */
  readonly nodeType: NodeType;

  constructor(options: {
    id: number;
    type: EnumType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    nodeType: NodeType;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`PermissionDefinition.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`PermissionDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;
    let _description = options.description ?? null;
    this.description = _description;
    let _nodeType = options.nodeType;
    if (_nodeType === null) {
      throw new Error(`PermissionDefinition.nodeType is required`);
    }
    this.nodeType = _nodeType;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.nodeType === other.nodeType)) {
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
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${EnumType[this.type]}`);
      propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      propertyReprs.push(`id=${this.id}`);
      propertyReprs.push(`name=${this.name}`);
      if (this.description !== null) {
        propertyReprs.push(`description=${this.description}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PermissionDefinition ${propertyReprs.join(" ")}>`;
    }
    return this._repr;
  }

  hash(): number {
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + this.nodeType) & 0xffffffff;
    h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.description !== null) {
      h = (h * 31 + hashString(this.description)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PermissionDefinition.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PermissionDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 140;
    objectValue["2"] = object.id;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    if (object.description != null) {
      objectValue["103"] = object.description;
    }
    objectValue["110"] = object.nodeType;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const iconValue = objectValue["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromValue(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectValue["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new PermissionDefinition({
      type: Number(objectValue["100"]),
      nodeType: Number(objectValue["110"]),
      id: Number(objectValue["2"]),
      name: objectValue["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PermissionDefinition {
    return PermissionDefinition.__unpackValue__(
      objectValue,
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
    const objectProto: Partial<PermissionDefinitionProto> = { metatype: 140 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as EnumTypeProto;
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    if (object.description != null) {
      objectProto.description = object.description;
    }
    objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
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
    return new PermissionDefinition({
      type: Number(objectProto.type) as EnumType,
      nodeType: Number(objectProto.nodeType) as NodeType,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _supergraph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
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
/* ==== DESTACK_GENERATED_END:STRUCT:140 ==== */
