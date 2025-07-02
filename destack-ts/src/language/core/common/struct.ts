import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import { NodeType, StructType } from "@destack/language/core/builtin/common";
import { Entity } from "@destack/language/core/builtin/entity";
import { Node } from "@destack/language/core/builtin/node";
import type {
  NodeReference,
  StructDefinitionReference,
} from "@destack/language/core/builtin/relation";
import { Struct } from "@destack/language/core/builtin/struct";
import type {
  IsCustomizable,
  IsDeletable,
  IsSourceable,
  IsSpatial,
  IsSubject,
  IsTaggable,
} from "@destack/language/core/builtin/trait";
import type { Icon } from "@destack/language/core/common/icon";
import type { Value } from "@destack/language/core/common/value";
import type { QueryConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import { CustomStructDefinitionProto, CustomStructProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:153 ==== */
/**
 * A CustomStruct is an instance of a CustomStructDefinition.
 */
export class CustomStruct extends Struct {
  static metatype: StructType = StructType.CUSTOM_STRUCT;
  static __isFrozen__: boolean = false;

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
  set definition(value: CustomStructDefinition) {
    this.definitionPtr = value.toRef();
  }
  definitionPtr: NodeReference;

  /**
   * CustomStruct.customValues
   */
  customValues: Map<string, Value>;

  constructor(options: {
    definition: CustomStructDefinition | NodeReference;
    customValues?: Map<string, Value>;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _definition = options.definition;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    if (_definition === null) {
      throw new Error(`CustomStruct.definition is required`);
    }
    this.definitionPtr = _definition;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;

    // identity
    // ...
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.definitionPtr.id === other.definitionPtr.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues.get(key)!.equals(other.customValues.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  repr(): string {
    return `<CustomStruct>`;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    return CustomStruct.__packValue__(this);
  }

  static __packValue__(object: CustomStruct): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 153;
    objectValue["6"] = object.definitionPtr.toValue();
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStruct {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomStruct({
      definition: _NodeReference.fromValue(
        objectValue["6"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStruct {
    return CustomStruct.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): CustomStructProto {
    return CustomStruct.__packProto__(this);
  }

  static __packProto__(object: CustomStruct): CustomStructProto {
    const objectProto: Partial<CustomStructProto> = { metatype: 153 };
    objectProto.definitionPtr = object.definitionPtr.toProto();
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    return objectProto as CustomStructProto;
  }

  static __unpackProto__(
    objectProto: CustomStructProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStruct {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomStruct({
      definition: _NodeReference.fromProto(
        objectProto.definitionPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: CustomStructProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStruct {
    return CustomStruct.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): CustomStruct {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomStructProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.CUSTOM_STRUCT, CustomStruct);
/* ==== DESTACK_GENERATED_END:STRUCT:153 ==== */

/* ==== DESTACK_GENERATED_START:NODE:300 ==== */
/**
 * A CustomStructDefinition describes a custom Struct with custom Properties.
 */
export class CustomStructDefinition
  extends Entity
  implements IsSpatial, IsTaggable, IsDeletable, IsSourceable, IsCustomizable
{
  static metatype: NodeType = NodeType.CUSTOM_STRUCT_DEFINITION;

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
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /**
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

  /**
   * A custom Struct's prototype is the default template new CustomStruct instances are based on.
   */
  prototype: CustomStruct | null;

  /**
   * CustomStructDefinition.baseType
   */
  baseType: StructDefinitionReference | null;

  /**
   * CustomStructDefinition.isFrozen
   */
  isFrozen: boolean;

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
   * CustomStructDefinition.name
   */
  name: string;

  /**
   * CustomStructDefinition.icon
   */
  icon: Icon | null;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: Map<string, Value>;
    orderKey?: string;
    prototype?: CustomStruct | null;
    baseType?: StructDefinitionReference | null;
    isFrozen?: boolean;
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`CustomStructDefinition.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _prototype = options.prototype ?? null;
    this.prototype = _prototype;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _isFrozen = options.isFrozen ?? null;
    if (_isFrozen === null) {
      _isFrozen = false;
    }
    if (_isFrozen === null) {
      throw new Error(`CustomStructDefinition.isFrozen is required`);
    }
    this.isFrozen = _isFrozen;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`CustomStructDefinition.name is required`);
    }
    this.name = _name;
    let _icon = options.icon ?? null;
    this.icon = _icon;

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
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
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
      (this.prototype == null) !== (other.prototype == null) ||
      (this.prototype != null && !this.prototype.equals(other.prototype))
    ) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
      return false;
    }
    if (!(this.isFrozen === other.isFrozen)) {
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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues.get(key)!.equals(other.customValues.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.prototype !== null) {
      h = (h * 31 + this.prototype.hash()) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isFrozen)) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    if (this.icon !== null) {
      h = (h * 31 + this.icon.hash()) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.sourcePtr !== null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
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
      type: NodeType.CUSTOM_STRUCT_DEFINITION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
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
    return `<CustomStructDefinition '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return CustomStructDefinition.__packValue__(this);
  }

  static __packValue__(object: CustomStructDefinition): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 300;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["27"] = object.orderKey;
    if (object.prototype != null) {
      objectValue["40"] = object.prototype.toValue();
    }
    if (object.baseType != null) {
      objectValue["41"] = object.baseType.toValue();
    }
    objectValue["42"] = object.isFrozen;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    objectValue["101"] = object.name;
    if (object.icon != null) {
      objectValue["102"] = object.icon.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStructDefinition {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StructDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.STRUCT_DEFINITION_REFERENCE
    ] as typeof StructDefinitionReference;
    const _CustomStruct = STRUCT_CLASS_BY_TYPE[StructType.CUSTOM_STRUCT] as typeof CustomStruct;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const prototypeValue = objectValue["40"];
    const unpackedPrototype =
      prototypeValue != undefined
        ? _CustomStruct.fromValue(prototypeValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["41"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _StructDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
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
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
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
    return new CustomStructDefinition({
      prototype: unpackedPrototype,
      baseType: unpackedBaseType,
      isFrozen: objectValue["42"],
      name: objectValue["101"],
      icon: unpackedIcon,
      space: unpackedSpacePtr,
      deletedAt: unpackedDeletedAt,
      source: unpackedSourcePtr,
      customValues: unpackedCustomValues,
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
  ): CustomStructDefinition {
    return CustomStructDefinition.__unpackValue__(
      objectValue,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  toProto(): CustomStructDefinitionProto {
    return CustomStructDefinition.__packProto__(this);
  }

  static __packProto__(object: CustomStructDefinition): CustomStructDefinitionProto {
    const objectProto: Partial<CustomStructDefinitionProto> = { metatype: 300 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.orderKey = object.orderKey;
    if (object.prototype != null) {
      objectProto.prototype = object.prototype.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
    }
    objectProto.isFrozen = object.isFrozen;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    objectProto.name = object.name;
    if (object.icon != null) {
      objectProto.icon = object.icon.toProto();
    }
    return objectProto as CustomStructDefinitionProto;
  }

  static __unpackProto__(
    objectProto: CustomStructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStructDefinition {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _StructDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.STRUCT_DEFINITION_REFERENCE
    ] as typeof StructDefinitionReference;
    const _CustomStruct = STRUCT_CLASS_BY_TYPE[StructType.CUSTOM_STRUCT] as typeof CustomStruct;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new CustomStructDefinition({
      prototype:
        objectProto.prototype != undefined
          ? _CustomStruct.fromProto(
              objectProto.prototype!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      baseType:
        objectProto.baseType != undefined
          ? _StructDefinitionReference.fromProto(
              objectProto.baseType!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isFrozen: objectProto.isFrozen,
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
      customValues: unpackedCustomValues,
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
    objectProto: CustomStructDefinitionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): CustomStructDefinition {
    return CustomStructDefinition.__unpackProto__(
      objectProto,
      _session,
      _supergraph,
      _graph,
      _connection,
    );
  }

  static fromProtoString(packedProtoString: string): CustomStructDefinition {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = CustomStructDefinitionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.CUSTOM_STRUCT_DEFINITION, CustomStructDefinition);
/* ==== DESTACK_GENERATED_END:NODE:300 ==== */
