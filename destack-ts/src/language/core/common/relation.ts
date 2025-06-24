import { NodeClass } from "@destack/language";
import {
  CustomEntityDefinition,
  EnumType,
  Field,
  Node,
  NodeType,
  Region,
  Session,
  StructFrozen,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import {
  AttributeReferenceProto,
  AttributeTypeProto,
  NodeReferenceProto,
  NodeTypeProto,
  PropertyReferenceProto,
  PropertyReferenceTypeProto,
  RegionProto,
  RelationReferenceProto,
  RelationTypeProto,
  ScopeProto,
  StructTypeProto,
  TraitTypeProto,
} from "@destack/proto";
import { assertNever } from "@destack/utils/functools";

/* ==== DESTACK_GENERATED_START:ENUM:50010 ==== */
/**
 * RelationType
 */
export enum RelationType {
  BUILTIN_NODE = 1,
  CUSTOM_NODE = 2,
  TRAIT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.RELATION_TYPE, RelationType);
/* ==== DESTACK_GENERATED_END:ENUM:50010 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50011 ==== */
/**
 * AttributeType
 */
export enum AttributeType {
  PROPERTY = 1,
  FIELD = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ATTRIBUTE_TYPE, AttributeType);
/* ==== DESTACK_GENERATED_END:ENUM:50011 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:50012 ==== */
/**
 * PropertyReferenceType
 */
export enum PropertyReferenceType {
  NODE = 1,
  TRAIT = 2,
  STRUCT = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PROPERTY_REFERENCE_TYPE, PropertyReferenceType);
/* ==== DESTACK_GENERATED_END:ENUM:50012 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50000 ==== */
/**
 * The scope in the Space graph.
 */
export class Scope extends StructFrozen {
  static metatype: StructType = StructType.SCOPE;
  static __isFrozen__: boolean = true;

  /**
   * Scope.region
   */
  readonly region: Region | null;

  /**
   * Scope.spaceId
   */
  readonly spaceId: string | null;

  constructor(options: {
    region?: Region | null;
    spaceId?: string | null;
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
    let _region = options.region ?? null;
    this.region = _region;
    let _spaceId = options.spaceId ?? null;
    this.spaceId = _spaceId;

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
    if ((this.region == null) !== (other.region == null) || (this.region != null && !(this.region === other.region))) {
      return false;
    }
    if (
      (this.spaceId == null) !== (other.spaceId == null) ||
      (this.spaceId != null && !(this.spaceId === other.spaceId))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Scope.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Scope): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50000;
    if (object.region != null) {
      objectValue["31"] = object.region;
    }
    if (object.spaceId != null) {
      objectValue["32"] = String(object.spaceId);
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Scope {
    const regionValue = objectValue["31"];
    const unpackedRegion = regionValue != undefined ? Number(regionValue) : null;
    const spaceIdValue = objectValue["32"];
    const unpackedSpaceId = spaceIdValue != undefined ? String(spaceIdValue) : null;
    return new Scope({
      region: unpackedRegion,
      spaceId: unpackedSpaceId,
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
  ): Scope {
    return Scope.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): ScopeProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Scope.__packProto__(this);
    }
    return this._proto as ScopeProto;
  }

  static __packProto__(object: Scope): ScopeProto {
    const objectProto: Partial<ScopeProto> = { metatype: 50000 };
    if (object.region != null) {
      objectProto.region = Number(object.region) as RegionProto;
    }
    if (object.spaceId != null) {
      objectProto.spaceId = String(object.spaceId);
    }
    return objectProto as ScopeProto;
  }

  static __unpackProto__(
    objectProto: ScopeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Scope {
    return new Scope({
      region: objectProto.region != undefined ? (Number(objectProto.region) as Region) : null,
      spaceId: objectProto.spaceId != undefined ? String(objectProto.spaceId) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ScopeProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Scope {
    return Scope.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.SCOPE, Scope);
/* ==== DESTACK_GENERATED_END:STRUCT:50000 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50107 ==== */
/**
 * Reference to a Node "type" (builtin or custom, i.e. a "relation").
 */
export class RelationReference extends StructFrozen {
  static metatype: StructType = StructType.RELATION_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * RelationReference.type
   */
  readonly type: RelationType;

  /**
   * RelationReference.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * definition
   */
  get definition(): CustomEntityDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * RelationReference.traitType
   */
  readonly traitType: TraitType | null;

  constructor(options: {
    type: RelationType;
    nodeType?: NodeType | null;
    definition?: CustomEntityDefinition | NodeReference | null;
    traitType?: TraitType | null;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`RelationReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition instanceof Node) {
      _definition = _definition.toRef();
    }
    this.definitionPtr = _definition;
    let _traitType = options.traitType ?? null;
    this.traitType = _traitType;

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
    if (
      (this.nodeType == null) !== (other.nodeType == null) ||
      (this.nodeType != null && !(this.nodeType === other.nodeType))
    ) {
      return false;
    }
    if (
      (this.traitType == null) !== (other.traitType == null) ||
      (this.traitType != null && !(this.traitType === other.traitType))
    ) {
      return false;
    }
    if (
      (this.definitionPtr == null) !== (other.definitionPtr == null) ||
      (this.definitionPtr != null && !(this.definitionPtr.id === other.definitionPtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = RelationReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: RelationReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50107;
    objectValue["30"] = object.type;
    if (object.nodeType != null) {
      objectValue["31"] = object.nodeType;
    }
    if (object.definitionPtr != null) {
      objectValue["32"] = object.definitionPtr.toValue();
    }
    if (object.traitType != null) {
      objectValue["33"] = object.traitType;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RelationReference {
    const nodeTypeValue = objectValue["31"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const traitTypeValue = objectValue["33"];
    const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : null;
    const definitionValue = objectValue["32"];
    const unpackedDefinition =
      definitionValue != undefined
        ? NodeReference.fromValue(definitionValue, _session, _supergraph, _graph, _connection)
        : null;
    return new RelationReference({
      type: Number(objectValue["30"]),
      nodeType: unpackedNodeType,
      traitType: unpackedTraitType,
      definition: unpackedDefinition,
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
  ): RelationReference {
    return RelationReference.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): RelationReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = RelationReference.__packProto__(this);
    }
    return this._proto as RelationReferenceProto;
  }

  static __packProto__(object: RelationReference): RelationReferenceProto {
    const objectProto: Partial<RelationReferenceProto> = { metatype: 50107 };
    objectProto.type = Number(object.type) as RelationTypeProto;
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.traitType != null) {
      objectProto.traitType = Number(object.traitType) as TraitTypeProto;
    }
    return objectProto as RelationReferenceProto;
  }

  static __unpackProto__(
    objectProto: RelationReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RelationReference {
    return new RelationReference({
      type: Number(objectProto.type) as RelationType,
      nodeType: objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      traitType: objectProto.traitType != undefined ? (Number(objectProto.traitType) as TraitType) : null,
      definition:
        objectProto.definitionPtr != undefined
          ? NodeReference.fromProto(objectProto.definitionPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: RelationReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): RelationReference {
    return RelationReference.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  static of(base: NodeType | NodeClass | CustomEntityDefinition) {
    if (typeof base == "number") {
      return new RelationReference({ type: RelationType.BUILTIN_NODE, nodeType: base });
    } else if (base instanceof CustomEntityDefinition) {
      return new RelationReference({ type: RelationType.CUSTOM_NODE, definition: base });
    } else {
      return new RelationReference({ type: RelationType.BUILTIN_NODE, nodeType: base.metatype });
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.RELATION_REFERENCE, RelationReference);
/* ==== DESTACK_GENERATED_END:STRUCT:50107 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50108 ==== */
/**
 * Reference to a Field or Property.
 */
export class AttributeReference extends StructFrozen {
  static metatype: StructType = StructType.ATTRIBUTE_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * AttributeReference.type
   */
  readonly type: AttributeType;

  /**
   * AttributeReference.propPtr
   */
  readonly propPtr: PropertyReference | null;

  /**
   * field
   */
  get field(): Field | null {
    const nodePtr: NodeReference | null = this.fieldPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Field | null;
    }
    return null;
  }
  readonly fieldPtr: NodeReference | null;

  constructor(options: {
    type: AttributeType;
    propPtr?: PropertyReference | null;
    field?: Field | NodeReference | null;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`AttributeReference.type is required`);
    }
    this.type = _type;
    let _propPtr = options.propPtr ?? null;
    this.propPtr = _propPtr;
    let _field = options.field ?? null;
    if (_field != null && _field instanceof Node) {
      _field = _field.toRef();
    }
    this.fieldPtr = _field;

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
    if (
      (this.propPtr == null) !== (other.propPtr == null) ||
      (this.propPtr != null && !this.propPtr.equals(other.propPtr))
    ) {
      return false;
    }
    if (
      (this.fieldPtr == null) !== (other.fieldPtr == null) ||
      (this.fieldPtr != null && !(this.fieldPtr.id === other.fieldPtr.id))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = AttributeReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: AttributeReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50108;
    objectValue["30"] = object.type;
    if (object.propPtr != null) {
      objectValue["31"] = object.propPtr.toValue();
    }
    if (object.fieldPtr != null) {
      objectValue["32"] = object.fieldPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): AttributeReference {
    const propPtrValue = objectValue["31"];
    const unpackedPropPtr =
      propPtrValue != undefined
        ? PropertyReference.fromValue(propPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fieldValue = objectValue["32"];
    const unpackedField =
      fieldValue != undefined ? NodeReference.fromValue(fieldValue, _session, _supergraph, _graph, _connection) : null;
    return new AttributeReference({
      type: Number(objectValue["30"]),
      propPtr: unpackedPropPtr,
      field: unpackedField,
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
  ): AttributeReference {
    return AttributeReference.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): AttributeReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = AttributeReference.__packProto__(this);
    }
    return this._proto as AttributeReferenceProto;
  }

  static __packProto__(object: AttributeReference): AttributeReferenceProto {
    const objectProto: Partial<AttributeReferenceProto> = { metatype: 50108 };
    objectProto.type = Number(object.type) as AttributeTypeProto;
    if (object.propPtr != null) {
      objectProto.propPtr = object.propPtr.toProto();
    }
    if (object.fieldPtr != null) {
      objectProto.fieldPtr = object.fieldPtr.toProto();
    }
    return objectProto as AttributeReferenceProto;
  }

  static __unpackProto__(
    objectProto: AttributeReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): AttributeReference {
    return new AttributeReference({
      type: Number(objectProto.type) as AttributeType,
      propPtr:
        objectProto.propPtr != undefined
          ? PropertyReference.fromProto(objectProto.propPtr!, _session, _supergraph, _graph, _connection)
          : null,
      field:
        objectProto.fieldPtr != undefined
          ? NodeReference.fromProto(objectProto.fieldPtr!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: AttributeReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): AttributeReference {
    return AttributeReference.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  static of(attribute: Field | PropertyReference | AttributeReference) {
    if (attribute instanceof PropertyReference) {
      return new AttributeReference({ type: AttributeType.PROPERTY, propPtr: attribute });
    } else if (attribute instanceof Field) {
      return new AttributeReference({ type: AttributeType.FIELD, field: attribute });
    } else if (attribute instanceof AttributeReference) {
      return attribute;
    } else {
      assertNever(attribute);
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ATTRIBUTE_REFERENCE, AttributeReference);
/* ==== DESTACK_GENERATED_END:STRUCT:50108 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50003 ==== */
/**
 * A reference to a builtin object's Property.
 * If type is unset, this refers to a base property in one of the base BuiltinObject types.
 */
export class PropertyReference extends StructFrozen {
  static metatype: StructType = StructType.PROPERTY_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * PropertyReference.type
   */
  readonly type: PropertyReferenceType;

  /**
   * PropertyReference.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * PropertyReference.traitType
   */
  readonly traitType: TraitType | null;

  /**
   * PropertyReference.structType
   */
  readonly structType: StructType | null;

  /**
   * PropertyReference.id
   */
  readonly id: number;

  constructor(options: {
    type: PropertyReferenceType;
    nodeType?: NodeType | null;
    traitType?: TraitType | null;
    structType?: StructType | null;
    id: number;
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`PropertyReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _traitType = options.traitType ?? null;
    this.traitType = _traitType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _id = options.id;
    if (_id === null) {
      throw new Error(`PropertyReference.id is required`);
    }
    this.id = _id;

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
    if (
      (this.nodeType == null) !== (other.nodeType == null) ||
      (this.nodeType != null && !(this.nodeType === other.nodeType))
    ) {
      return false;
    }
    if (
      (this.traitType == null) !== (other.traitType == null) ||
      (this.traitType != null && !(this.traitType === other.traitType))
    ) {
      return false;
    }
    if (
      (this.structType == null) !== (other.structType == null) ||
      (this.structType != null && !(this.structType === other.structType))
    ) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = PropertyReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PropertyReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50003;
    objectValue["30"] = object.type;
    if (object.nodeType != null) {
      objectValue["31"] = object.nodeType;
    }
    if (object.traitType != null) {
      objectValue["32"] = object.traitType;
    }
    if (object.structType != null) {
      objectValue["33"] = object.structType;
    }
    objectValue["35"] = object.id;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyReference {
    const nodeTypeValue = objectValue["31"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const traitTypeValue = objectValue["32"];
    const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : null;
    const structTypeValue = objectValue["33"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    return new PropertyReference({
      type: Number(objectValue["30"]),
      nodeType: unpackedNodeType,
      traitType: unpackedTraitType,
      structType: unpackedStructType,
      id: Number(objectValue["35"]),
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
  ): PropertyReference {
    return PropertyReference.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): PropertyReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PropertyReference.__packProto__(this);
    }
    return this._proto as PropertyReferenceProto;
  }

  static __packProto__(object: PropertyReference): PropertyReferenceProto {
    const objectProto: Partial<PropertyReferenceProto> = { metatype: 50003 };
    objectProto.type = Number(object.type) as PropertyReferenceTypeProto;
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.traitType != null) {
      objectProto.traitType = Number(object.traitType) as TraitTypeProto;
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    objectProto.id = object.id;
    return objectProto as PropertyReferenceProto;
  }

  static __unpackProto__(
    objectProto: PropertyReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyReference {
    return new PropertyReference({
      type: Number(objectProto.type) as PropertyReferenceType,
      nodeType: objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      traitType: objectProto.traitType != undefined ? (Number(objectProto.traitType) as TraitType) : null,
      structType: objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      id: Number(objectProto.id),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: PropertyReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyReference {
    return PropertyReference.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PROPERTY_REFERENCE, PropertyReference);
/* ==== DESTACK_GENERATED_END:STRUCT:50003 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:50002 ==== */
/**
 * A reference to a Node.
 */
export class NodeReference extends StructFrozen {
  static metatype: StructType = StructType.NODE_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * NodeReference.nodeType
   */
  readonly nodeType: NodeType;

  /**
   * NodeReference.id
   */
  readonly id: string;

  /**
   * NodeReference.spaceId
   */
  readonly spaceId: string | null;

  /**
   * NodeReference.definitionId
   */
  readonly definitionId: string | null;

  constructor(options: {
    nodeType: NodeType;
    id: string;
    spaceId?: string | null;
    definitionId?: string | null;
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
    let _nodeType = options.nodeType;
    if (_nodeType === null) {
      throw new Error(`NodeReference.nodeType is required`);
    }
    this.nodeType = _nodeType;
    let _id = options.id;
    if (_id === null) {
      throw new Error(`NodeReference.id is required`);
    }
    this.id = _id;
    let _spaceId = options.spaceId ?? null;
    this.spaceId = _spaceId;
    let _definitionId = options.definitionId ?? null;
    this.definitionId = _definitionId;

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
    if (!(this.nodeType === other.nodeType)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (
      (this.spaceId == null) !== (other.spaceId == null) ||
      (this.spaceId != null && !(this.spaceId === other.spaceId))
    ) {
      return false;
    }
    if (
      (this.definitionId == null) !== (other.definitionId == null) ||
      (this.definitionId != null && !(this.definitionId === other.definitionId))
    ) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = NodeReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 50002;
    objectValue["31"] = object.nodeType;
    objectValue["32"] = String(object.id);
    if (object.spaceId != null) {
      objectValue["34"] = String(object.spaceId);
    }
    if (object.definitionId != null) {
      objectValue["35"] = String(object.definitionId);
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeReference {
    const spaceIdValue = objectValue["34"];
    const unpackedSpaceId = spaceIdValue != undefined ? String(spaceIdValue) : null;
    const definitionIdValue = objectValue["35"];
    const unpackedDefinitionId = definitionIdValue != undefined ? String(definitionIdValue) : null;
    return new NodeReference({
      nodeType: Number(objectValue["31"]),
      id: String(objectValue["32"]),
      spaceId: unpackedSpaceId,
      definitionId: unpackedDefinitionId,
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
  ): NodeReference {
    return NodeReference.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): NodeReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeReference.__packProto__(this);
    }
    return this._proto as NodeReferenceProto;
  }

  static __packProto__(object: NodeReference): NodeReferenceProto {
    const objectProto: Partial<NodeReferenceProto> = { metatype: 50002 };
    objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    objectProto.id = String(object.id);
    if (object.spaceId != null) {
      objectProto.spaceId = String(object.spaceId);
    }
    if (object.definitionId != null) {
      objectProto.definitionId = String(object.definitionId);
    }
    return objectProto as NodeReferenceProto;
  }

  static __unpackProto__(
    objectProto: NodeReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeReference {
    return new NodeReference({
      nodeType: Number(objectProto.nodeType) as NodeType,
      id: String(objectProto.id),
      spaceId: objectProto.spaceId != undefined ? String(objectProto.spaceId) : null,
      definitionId: objectProto.definitionId != undefined ? String(objectProto.definitionId) : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NodeReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeReference {
    return NodeReference.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_REFERENCE, NodeReference);
/* ==== DESTACK_GENERATED_END:STRUCT:50002 ==== */
