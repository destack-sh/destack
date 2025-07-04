import {
  EnumType,
  NodeType,
  StoreType,
  StructType,
  TraitType,
} from "@destack/language/core/builtin/common";
import type {
  CustomEntityDefinition,
  CustomTraitDefinition,
} from "@destack/language/core/builtin/entity";
import type { CustomEventDefinition } from "@destack/language/core/builtin/event";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node, isNode } from "@destack/language/core/builtin/node";
import { StructFrozen, isStruct } from "@destack/language/core/builtin/struct";
import { PropertyDefinition } from "@destack/language/core/common";
import type { CustomProperty } from "@destack/language/core/common/property";
import type { CustomStructDefinition } from "@destack/language/core/common/struct";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  NODE_CLASS_BY_TYPE,
  STRUCT_CLASS_BY_TYPE,
  TRAIT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import {
  NodeDefinitionReferenceProto,
  NodeDefinitionTypeProto,
  NodeReferenceProto,
  NodeTypeProto,
  ObjectDefinitionReferenceProto,
  ObjectDefinitionTypeProto,
  PropertyReferenceProto,
  PropertyReferenceTypeProto,
  StoreTypeProto,
  StructDefinitionReferenceProto,
  StructDefinitionTypeProto,
  StructTypeProto,
  TraitTypeProto,
} from "@destack/proto";
import { assertNever, base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:STRUCT:200 ==== */
/**
 * Reference to a Node definition.
 */
export class NodeDefinitionReference extends StructFrozen {
  static metatype: StructType = StructType.NODE_DEFINITION_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * NodeDefinitionReference.type
   */
  readonly type: NodeDefinitionType;

  /**
   * NodeDefinitionReference.nodeType
   */
  readonly nodeType: NodeType;

  /**
   * definition
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  constructor(options: {
    type: NodeDefinitionType;
    nodeType: NodeType;
    definition?: CustomEntityDefinition | CustomEventDefinition | NodeReference | null;
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
      throw new Error(`NodeDefinitionReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType;
    if (_nodeType === null) {
      throw new Error(`NodeDefinitionReference.nodeType is required`);
    }
    this.nodeType = _nodeType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;

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
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${NodeDefinitionType[this.type]}`);
      propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      if (this.definition !== null) {
        propertyReprs.push(`definition=${this.definition?.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<NodeDefinitionReference ${propertyReprs.join(" ")}>`;
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
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
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
      this._value = NodeDefinitionReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeDefinitionReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 200;
    objectValue["100"] = object.type;
    objectValue["101"] = object.nodeType;
    if (object.definitionPtr != null) {
      objectValue["105"] = object.definitionPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const definitionPtrValue = objectValue["105"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new NodeDefinitionReference({
      type: Number(objectValue["100"]),
      nodeType: Number(objectValue["101"]),
      definition: unpackedDefinitionPtr,
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
  ): NodeDefinitionReference {
    return NodeDefinitionReference.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): NodeDefinitionReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = NodeDefinitionReference.__packProto__(this);
    }
    return this._proto as NodeDefinitionReferenceProto;
  }

  static __packProto__(object: NodeDefinitionReference): NodeDefinitionReferenceProto {
    const objectProto: Partial<NodeDefinitionReferenceProto> = { metatype: 200 };
    objectProto.type = Number(object.type) as NodeDefinitionTypeProto;
    objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    return objectProto as NodeDefinitionReferenceProto;
  }

  static __unpackProto__(
    objectProto: NodeDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new NodeDefinitionReference({
      type: Number(objectProto.type) as NodeDefinitionType,
      nodeType: Number(objectProto.nodeType) as NodeType,
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
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: NodeDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): NodeDefinitionReference {
    return NodeDefinitionReference.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): NodeDefinitionReference {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NodeDefinitionReferenceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Whether this definition references multiple Node definitions. */
  get isMulti(): boolean {
    if (this.type == NodeDefinitionType.BUILTIN) {
      const nodeClass = NODE_CLASS_BY_TYPE[this.nodeType];
      const nodeDefinition = nodeClass.__definition__;
      return nodeDefinition.traits.includes(TraitType.EXTENSIBLE);
    } else if (this.type == NodeDefinitionType.CUSTOM) {
      throw new Error(`unexpected node definition reference: ${this.repr()}`);
    } else {
      assertNever(this.type);
    }
  }

  /** Resolve a Property in this definition. */
  resolveProperty(name: string): PropertyDefinition | null {
    if (this.type == NodeDefinitionType.BUILTIN) {
      const nodeClass = NODE_CLASS_BY_TYPE[this.nodeType];
      const nodeDefinition = nodeClass.__definition__;
      return nodeDefinition.resolveProperty(name);
    } else if (this.type == NodeDefinitionType.CUSTOM) {
      throw new Error(`unexpected node definition reference: ${this.repr()}`);
    } else {
      assertNever(this.type);
    }
  }

  /** Resolve a Property in this definition (error if not found). */
  resolvePropertyOrError(name: string): PropertyDefinition {
    const property = this.resolveProperty(name);
    if (property == null) {
      throw new Error(`could not find property ${name} in ${this.repr()}`);
    }
    return property;
  }

  static of(base: NodeType | NodeClass | CustomEventDefinition | CustomEntityDefinition) {
    if (typeof base == "number") {
      return new NodeDefinitionReference({ type: NodeDefinitionType.BUILTIN, nodeType: base });
    } else if (
      isNode(base, NodeType.CUSTOM_EVENT_DEFINITION) ||
      isNode(base, NodeType.CUSTOM_ENTITY_DEFINITION)
    ) {
      return new NodeDefinitionReference({
        type: NodeDefinitionType.CUSTOM,
        nodeType: base.metatype,
        definition: base,
      });
    } else {
      return new NodeDefinitionReference({
        type: NodeDefinitionType.BUILTIN,
        nodeType: base.metatype,
      });
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_DEFINITION_REFERENCE, NodeDefinitionReference);
/* ==== DESTACK_GENERATED_END:STRUCT:200 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:201 ==== */
/**
 * Reference to an object "type" (builtin, custom or trait).
 */
export class ObjectDefinitionReference extends StructFrozen {
  static metatype: StructType = StructType.OBJECT_DEFINITION_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * ObjectDefinitionReference.type
   */
  readonly type: ObjectDefinitionType;

  /**
   * ObjectDefinitionReference.nodeType
   */
  readonly nodeType: NodeType | null;

  /**
   * ObjectDefinitionReference.traitType
   */
  readonly traitType: TraitType | null;

  /**
   * ObjectDefinitionReference.structType
   */
  readonly structType: StructType | null;

  /**
   * definition
   */
  get definition():
    | CustomEntityDefinition
    | CustomEventDefinition
    | CustomTraitDefinition
    | CustomStructDefinition
    | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | CustomTraitDefinition
        | CustomStructDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  constructor(options: {
    type: ObjectDefinitionType;
    nodeType?: NodeType | null;
    traitType?: TraitType | null;
    structType?: StructType | null;
    definition?:
      | CustomEntityDefinition
      | CustomEventDefinition
      | CustomTraitDefinition
      | CustomStructDefinition
      | NodeReference
      | null;
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
      throw new Error(`ObjectDefinitionReference.type is required`);
    }
    this.type = _type;
    let _nodeType = options.nodeType ?? null;
    this.nodeType = _nodeType;
    let _traitType = options.traitType ?? null;
    this.traitType = _traitType;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;

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
    if (!(this.traitType === other.traitType)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${ObjectDefinitionType[this.type]}`);
      if (this.nodeType !== null) {
        propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      }
      if (this.traitType !== null) {
        propertyReprs.push(`traitType=${TraitType[this.traitType]}`);
      }
      if (this.structType !== null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.definition !== null) {
        propertyReprs.push(`definition=${this.definition?.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ObjectDefinitionReference ${propertyReprs.join(" ")}>`;
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
    if (this.nodeType !== null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.traitType !== null) {
      h = (h * 31 + this.traitType) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
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
      this._value = ObjectDefinitionReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: ObjectDefinitionReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 201;
    objectValue["100"] = object.type;
    if (object.nodeType != null) {
      objectValue["101"] = object.nodeType;
    }
    if (object.traitType != null) {
      objectValue["102"] = object.traitType;
    }
    if (object.structType != null) {
      objectValue["103"] = object.structType;
    }
    if (object.definitionPtr != null) {
      objectValue["105"] = object.definitionPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ObjectDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodeTypeValue = objectValue["101"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const traitTypeValue = objectValue["102"];
    const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : null;
    const structTypeValue = objectValue["103"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const definitionPtrValue = objectValue["105"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new ObjectDefinitionReference({
      type: Number(objectValue["100"]),
      nodeType: unpackedNodeType,
      traitType: unpackedTraitType,
      structType: unpackedStructType,
      definition: unpackedDefinitionPtr,
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
  ): ObjectDefinitionReference {
    return ObjectDefinitionReference.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): ObjectDefinitionReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ObjectDefinitionReference.__packProto__(this);
    }
    return this._proto as ObjectDefinitionReferenceProto;
  }

  static __packProto__(object: ObjectDefinitionReference): ObjectDefinitionReferenceProto {
    const objectProto: Partial<ObjectDefinitionReferenceProto> = { metatype: 201 };
    objectProto.type = Number(object.type) as ObjectDefinitionTypeProto;
    if (object.nodeType != null) {
      objectProto.nodeType = Number(object.nodeType) as NodeTypeProto;
    }
    if (object.traitType != null) {
      objectProto.traitType = Number(object.traitType) as TraitTypeProto;
    }
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    return objectProto as ObjectDefinitionReferenceProto;
  }

  static __unpackProto__(
    objectProto: ObjectDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ObjectDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new ObjectDefinitionReference({
      type: Number(objectProto.type) as ObjectDefinitionType,
      nodeType:
        objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      traitType:
        objectProto.traitType != undefined ? (Number(objectProto.traitType) as TraitType) : null,
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
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: ObjectDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): ObjectDefinitionReference {
    return ObjectDefinitionReference.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): ObjectDefinitionReference {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ObjectDefinitionReferenceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  static of(base: NodeType | NodeClass | CustomEventDefinition | CustomEntityDefinition) {
    if (typeof base == "number") {
      return new ObjectDefinitionReference({
        type: ObjectDefinitionType.BUILTIN_NODE,
        nodeType: base,
      });
    } else if (
      isNode(base, NodeType.CUSTOM_EVENT_DEFINITION) ||
      isNode(base, NodeType.CUSTOM_ENTITY_DEFINITION)
    ) {
      return new ObjectDefinitionReference({
        type: ObjectDefinitionType.CUSTOM_NODE,
        definition: base,
      });
    } else {
      return new ObjectDefinitionReference({
        type: ObjectDefinitionType.BUILTIN_NODE,
        nodeType: base.metatype,
      });
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.OBJECT_DEFINITION_REFERENCE, ObjectDefinitionReference);
/* ==== DESTACK_GENERATED_END:STRUCT:201 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:251 ==== */
/**
 * A reference to a builtin object's Property.
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
   * id of the builtin Property
   */
  readonly id: number | null;

  /**
   * custom Property of a custom Node or Struct
   */
  get customProperty(): CustomProperty | null {
    const nodePtr: NodeReference | null = this.customPropertyPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomProperty | null;
    }
    return null;
  }
  readonly customPropertyPtr: NodeReference | null;

  constructor(options: {
    type: PropertyReferenceType;
    nodeType?: NodeType | null;
    traitType?: TraitType | null;
    structType?: StructType | null;
    id?: number | null;
    customProperty?: CustomProperty | NodeReference | null;
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
    let _id = options.id ?? null;
    this.id = _id;
    let _customProperty = options.customProperty ?? null;
    if (_customProperty != null && _customProperty.metatype != StructType.NODE_REFERENCE) {
      _customProperty = (_customProperty as Node).toRef();
    }
    this.customPropertyPtr = _customProperty;

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
    if (!(this.traitType === other.traitType)) {
      return false;
    }
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.customPropertyPtr?.id === other.customPropertyPtr?.id)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${PropertyReferenceType[this.type]}`);
      if (this.nodeType !== null) {
        propertyReprs.push(`nodeType=${NodeType[this.nodeType]}`);
      }
      if (this.traitType !== null) {
        propertyReprs.push(`traitType=${TraitType[this.traitType]}`);
      }
      if (this.structType !== null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      if (this.id !== null) {
        propertyReprs.push(`id=${this.id}`);
      }
      if (this.customProperty !== null) {
        propertyReprs.push(`customProperty=${this.customProperty?.repr()}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<PropertyReference ${propertyReprs.join(" ")}>`;
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
    if (this.nodeType !== null) {
      h = (h * 31 + this.nodeType) & 0xffffffff;
    }
    if (this.traitType !== null) {
      h = (h * 31 + this.traitType) & 0xffffffff;
    }
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    if (this.id !== null) {
      h = (h * 31 + hashInt(this.id)) & 0xffffffff;
    }
    if (this.customPropertyPtr !== null) {
      h = (h * 31 + hashString(this.customPropertyPtr.id)) & 0xffffffff;
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
      this._value = PropertyReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: PropertyReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 251;
    objectValue["100"] = object.type;
    if (object.nodeType != null) {
      objectValue["101"] = object.nodeType;
    }
    if (object.traitType != null) {
      objectValue["102"] = object.traitType;
    }
    if (object.structType != null) {
      objectValue["103"] = object.structType;
    }
    if (object.id != null) {
      objectValue["105"] = object.id;
    }
    if (object.customPropertyPtr != null) {
      objectValue["106"] = object.customPropertyPtr.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const nodeTypeValue = objectValue["101"];
    const unpackedNodeType = nodeTypeValue != undefined ? Number(nodeTypeValue) : null;
    const traitTypeValue = objectValue["102"];
    const unpackedTraitType = traitTypeValue != undefined ? Number(traitTypeValue) : null;
    const structTypeValue = objectValue["103"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    const idValue = objectValue["105"];
    const unpackedId = idValue != undefined ? Number(idValue) : null;
    const customPropertyPtrValue = objectValue["106"];
    const unpackedCustomPropertyPtr =
      customPropertyPtrValue != undefined
        ? _NodeReference.fromValue(
            customPropertyPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
        : null;
    return new PropertyReference({
      type: Number(objectValue["100"]),
      nodeType: unpackedNodeType,
      traitType: unpackedTraitType,
      structType: unpackedStructType,
      id: unpackedId,
      customProperty: unpackedCustomPropertyPtr,
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
    return PropertyReference.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): PropertyReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = PropertyReference.__packProto__(this);
    }
    return this._proto as PropertyReferenceProto;
  }

  static __packProto__(object: PropertyReference): PropertyReferenceProto {
    const objectProto: Partial<PropertyReferenceProto> = { metatype: 251 };
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
    if (object.id != null) {
      objectProto.id = object.id;
    }
    if (object.customPropertyPtr != null) {
      objectProto.customPropertyPtr = object.customPropertyPtr.toProto();
    }
    return objectProto as PropertyReferenceProto;
  }

  static __unpackProto__(
    objectProto: PropertyReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): PropertyReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new PropertyReference({
      type: Number(objectProto.type) as PropertyReferenceType,
      nodeType:
        objectProto.nodeType != undefined ? (Number(objectProto.nodeType) as NodeType) : null,
      traitType:
        objectProto.traitType != undefined ? (Number(objectProto.traitType) as TraitType) : null,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      id: objectProto.id != undefined ? Number(objectProto.id) : null,
      customProperty:
        objectProto.customPropertyPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.customPropertyPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
    return PropertyReference.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): PropertyReference {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = PropertyReferenceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */

  /** Resolve the property reference to a PropertyDefinition or CustomProperty. */
  resolve(): PropertyDefinition | CustomProperty | null {
    if (this.type == PropertyReferenceType.BUILTIN) {
      if (this.id == null) {
        throw new Error(`no id for builtin property reference ${this.repr()}`);
      }
      if (this.nodeType) {
        const nodeClass = NODE_CLASS_BY_TYPE[this.nodeType];
        return nodeClass.__propertiesById__[this.id];
      } else if (this.traitType) {
        const traitClass = TRAIT_CLASS_BY_TYPE[this.traitType];
        return traitClass.__propertiesById__[this.id];
      } else if (this.structType) {
        const structClass = STRUCT_CLASS_BY_TYPE[this.structType];
        return structClass.__propertiesById__[this.id];
      } else {
        return Node.__propertiesById__[this.id];
      }
    } else if (this.type == PropertyReferenceType.CUSTOM) {
      return this.customProperty;
    } else {
      assertNever(this.type);
    }
  }

  /** Resolve the property reference to a PropertyDefinition or CustomProperty. */
  resolveOrError(): PropertyDefinition | CustomProperty {
    const resolved = this.resolve();
    if (resolved == null) {
      throw new Error(`could not resolve property reference ${this.repr()}`);
    }
    return resolved;
  }

  static of(attribute: CustomProperty | PropertyReference): PropertyReference {
    if (isStruct(attribute, StructType.PROPERTY_REFERENCE)) {
      return attribute;
    } else if (isNode(attribute, NodeType.CUSTOM_PROPERTY)) {
      if (attribute.parentPtr == null) {
        throw new Error(`${attribute.repr()} has no parent`);
      }
      return new PropertyReference({
        type: PropertyReferenceType.CUSTOM,
        nodeType: attribute.parentPtr.type,
        customProperty: attribute,
      });
    } else {
      assertNever(attribute);
    }
  }

  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.PROPERTY_REFERENCE, PropertyReference);
/* ==== DESTACK_GENERATED_END:STRUCT:251 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:10 ==== */
/**
 * NodeDefinitionType
 */
export enum NodeDefinitionType {
  BUILTIN = 1,
  CUSTOM = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.NODE_DEFINITION_TYPE, NodeDefinitionType);
/* ==== DESTACK_GENERATED_END:ENUM:10 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:11 ==== */
/**
 * ObjectDefinitionType
 */
export enum ObjectDefinitionType {
  BUILTIN_NODE = 1,
  CUSTOM_NODE = 2,
  BUILTIN_TRAIT = 3,
  CUSTOM_TRAIT = 4,
  BUILTIN_STRUCT = 5,
  CUSTOM_STRUCT = 6,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.OBJECT_DEFINITION_TYPE, ObjectDefinitionType);
/* ==== DESTACK_GENERATED_END:ENUM:11 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:12 ==== */
/**
 * StructDefinitionType
 */
export enum StructDefinitionType {
  BUILTIN_STRUCT = 1,
  CUSTOM_STRUCT = 2,
  BUILTIN_ENUM = 3,
  CUSTOM_ENUM = 4,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.STRUCT_DEFINITION_TYPE, StructDefinitionType);
/* ==== DESTACK_GENERATED_END:ENUM:12 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:13 ==== */
/**
 * PropertyReferenceType
 */
export enum PropertyReferenceType {
  BUILTIN = 1,
  CUSTOM = 2,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.PROPERTY_REFERENCE_TYPE, PropertyReferenceType);
/* ==== DESTACK_GENERATED_END:ENUM:13 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:202 ==== */
/**
 * Reference to a Struct definition (builtin, custom or by trait).
 */
export class StructDefinitionReference extends StructFrozen {
  static metatype: StructType = StructType.STRUCT_DEFINITION_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * StructDefinitionReference.type
   */
  readonly type: StructDefinitionType;

  /**
   * StructDefinitionReference.structType
   */
  readonly structType: StructType | null;

  /**
   * definition
   */
  get definition(): CustomStructDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as CustomStructDefinition;
    }
    return null;
  }
  readonly definitionPtr: NodeReference;

  constructor(options: {
    type: StructDefinitionType;
    structType?: StructType | null;
    definition: CustomStructDefinition | NodeReference;
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
      throw new Error(`StructDefinitionReference.type is required`);
    }
    this.type = _type;
    let _structType = options.structType ?? null;
    this.structType = _structType;
    let _definition = options.definition;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    if (_definition === null) {
      throw new Error(`StructDefinitionReference.definition is required`);
    }
    this.definitionPtr = _definition;

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
    if (!(this.structType === other.structType)) {
      return false;
    }
    if (!(this.definitionPtr.id === other.definitionPtr.id)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${StructDefinitionType[this.type]}`);
      if (this.structType !== null) {
        propertyReprs.push(`structType=${StructType[this.structType]}`);
      }
      propertyReprs.push(`definition=${this.definition?.repr()}`);
      // @ts-expect-error(readonly)
      this._repr = `<StructDefinitionReference ${propertyReprs.join(" ")}>`;
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
    if (this.structType !== null) {
      h = (h * 31 + this.structType) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;

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
      this._value = StructDefinitionReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: StructDefinitionReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 202;
    objectValue["100"] = object.type;
    if (object.structType != null) {
      objectValue["101"] = object.structType;
    }
    objectValue["105"] = object.definitionPtr.toValue();
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const structTypeValue = objectValue["101"];
    const unpackedStructType = structTypeValue != undefined ? Number(structTypeValue) : null;
    return new StructDefinitionReference({
      type: Number(objectValue["100"]),
      structType: unpackedStructType,
      definition: _NodeReference.fromValue(
        objectValue["105"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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
  ): StructDefinitionReference {
    return StructDefinitionReference.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): StructDefinitionReferenceProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = StructDefinitionReference.__packProto__(this);
    }
    return this._proto as StructDefinitionReferenceProto;
  }

  static __packProto__(object: StructDefinitionReference): StructDefinitionReferenceProto {
    const objectProto: Partial<StructDefinitionReferenceProto> = { metatype: 202 };
    objectProto.type = Number(object.type) as StructDefinitionTypeProto;
    if (object.structType != null) {
      objectProto.structType = Number(object.structType) as StructTypeProto;
    }
    objectProto.definitionPtr = object.definitionPtr.toProto();
    return objectProto as StructDefinitionReferenceProto;
  }

  static __unpackProto__(
    objectProto: StructDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinitionReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new StructDefinitionReference({
      type: Number(objectProto.type) as StructDefinitionType,
      structType:
        objectProto.structType != undefined ? (Number(objectProto.structType) as StructType) : null,
      definition: _NodeReference.fromProto(
        objectProto.definitionPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: StructDefinitionReferenceProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): StructDefinitionReference {
    return StructDefinitionReference.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): StructDefinitionReference {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = StructDefinitionReferenceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.STRUCT_DEFINITION_REFERENCE, StructDefinitionReference);
/* ==== DESTACK_GENERATED_END:STRUCT:202 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:250 ==== */
/**
 * A reference to a Node (builtin or custom).
 */
export class NodeReference extends StructFrozen {
  static metatype: StructType = StructType.NODE_REFERENCE;
  static __isFrozen__: boolean = true;

  /**
   * The type of the Node.
   */
  readonly type: NodeType;

  /**
   * The unique id of the Node.
   */
  readonly id: string;

  /**
   * The unique id of the custom Node definition.
   */
  readonly definitionId: string | null;

  /**
   * The id of the Snapshot the Node belonged to.
   */
  readonly snapshotId: string | null;

  /**
   * The id of the Space the Node belonged to.
   */
  readonly spaceId: string | null;

  /**
   * The type of the Store the Node belonged to.
   */
  readonly storeType: StoreType | null;

  constructor(options: {
    type: NodeType;
    id: string;
    definitionId?: string | null;
    snapshotId?: string | null;
    spaceId?: string | null;
    storeType?: StoreType | null;
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
      throw new Error(`NodeReference.type is required`);
    }
    this.type = _type;
    let _id = options.id;
    if (_id === null) {
      throw new Error(`NodeReference.id is required`);
    }
    this.id = _id;
    let _definitionId = options.definitionId ?? null;
    this.definitionId = _definitionId;
    let _snapshotId = options.snapshotId ?? null;
    this.snapshotId = _snapshotId;
    let _spaceId = options.spaceId ?? null;
    this.spaceId = _spaceId;
    let _storeType = options.storeType ?? null;
    this.storeType = _storeType;

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
    if (!(this.id === other.id)) {
      return false;
    }
    if (!(this.definitionId === other.definitionId)) {
      return false;
    }
    if (!(this.snapshotId === other.snapshotId)) {
      return false;
    }
    if (!(this.spaceId === other.spaceId)) {
      return false;
    }
    if (!(this.storeType === other.storeType)) {
      return false;
    }
    return true;
  }

  repr(): string {
    if (this._repr === null) {
      const propertyReprs: string[] = [];
      propertyReprs.push(`type=${NodeType[this.type]}`);
      propertyReprs.push(`id=${this.id}`);
      if (this.definitionId !== null) {
        propertyReprs.push(`definitionId=${this.definitionId}`);
      }
      if (this.snapshotId !== null) {
        propertyReprs.push(`snapshotId=${this.snapshotId}`);
      }
      if (this.spaceId !== null) {
        propertyReprs.push(`spaceId=${this.spaceId}`);
      }
      if (this.storeType !== null) {
        propertyReprs.push(`storeType=${StoreType[this.storeType]}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<NodeReference ${propertyReprs.join(" ")}>`;
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
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.definitionId !== null) {
      h = (h * 31 + hashString(this.definitionId.toString())) & 0xffffffff;
    }
    if (this.snapshotId !== null) {
      h = (h * 31 + hashString(this.snapshotId.toString())) & 0xffffffff;
    }
    if (this.spaceId !== null) {
      h = (h * 31 + hashString(this.spaceId.toString())) & 0xffffffff;
    }
    if (this.storeType !== null) {
      h = (h * 31 + this.storeType) & 0xffffffff;
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
      this._value = NodeReference.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: NodeReference): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 250;
    objectValue["100"] = object.type;
    objectValue["101"] = String(object.id);
    if (object.definitionId != null) {
      objectValue["102"] = String(object.definitionId);
    }
    if (object.snapshotId != null) {
      objectValue["103"] = String(object.snapshotId);
    }
    if (object.spaceId != null) {
      objectValue["110"] = String(object.spaceId);
    }
    if (object.storeType != null) {
      objectValue["111"] = object.storeType;
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
    const definitionIdValue = objectValue["102"];
    const unpackedDefinitionId = definitionIdValue != undefined ? String(definitionIdValue) : null;
    const snapshotIdValue = objectValue["103"];
    const unpackedSnapshotId = snapshotIdValue != undefined ? String(snapshotIdValue) : null;
    const spaceIdValue = objectValue["110"];
    const unpackedSpaceId = spaceIdValue != undefined ? String(spaceIdValue) : null;
    const storeTypeValue = objectValue["111"];
    const unpackedStoreType = storeTypeValue != undefined ? Number(storeTypeValue) : null;
    return new NodeReference({
      type: Number(objectValue["100"]),
      id: String(objectValue["101"]),
      definitionId: unpackedDefinitionId,
      snapshotId: unpackedSnapshotId,
      spaceId: unpackedSpaceId,
      storeType: unpackedStoreType,
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
    const objectProto: Partial<NodeReferenceProto> = { metatype: 250 };
    objectProto.type = Number(object.type) as NodeTypeProto;
    objectProto.id = String(object.id);
    if (object.definitionId != null) {
      objectProto.definitionId = String(object.definitionId);
    }
    if (object.snapshotId != null) {
      objectProto.snapshotId = String(object.snapshotId);
    }
    if (object.spaceId != null) {
      objectProto.spaceId = String(object.spaceId);
    }
    if (object.storeType != null) {
      objectProto.storeType = Number(object.storeType) as StoreTypeProto;
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
      type: Number(objectProto.type) as NodeType,
      id: String(objectProto.id),
      definitionId: objectProto.definitionId != undefined ? String(objectProto.definitionId) : null,
      snapshotId: objectProto.snapshotId != undefined ? String(objectProto.snapshotId) : null,
      spaceId: objectProto.spaceId != undefined ? String(objectProto.spaceId) : null,
      storeType:
        objectProto.storeType != undefined ? (Number(objectProto.storeType) as StoreType) : null,
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

  static fromProtoString(packedProtoString: string): NodeReference {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = NodeReferenceProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.NODE_REFERENCE, NodeReference);
/* ==== DESTACK_GENERATED_END:STRUCT:250 ==== */
