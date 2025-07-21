import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  NodeType,
  PlatformType,
  RuntimeLanguage,
  StructType,
} from "@destack/language/core/builtin/common";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import { Event } from "@destack/language/core/builtin/event";
import { MethodCardinality, MethodType } from "@destack/language/core/builtin/meta";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { IsActor, IsRunnable } from "@destack/language/core/builtin/trait";
import type { PropertyDefinition } from "@destack/language/core/common/definition";
import type { Icon } from "@destack/language/core/common/icon";
import { Method, MethodDefinition } from "@destack/language/core/common/method";
import type { Space } from "@destack/language/core/common/space";
import type { Text } from "@destack/language/core/common/text";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
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
import {
  ActionDefinitionProto,
  ActionProto,
  MaterializationProto,
  MethodCardinalityProto,
  MethodTypeProto,
  PlatformTypeProto,
  RuntimeLanguageProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:40100 ==== */
/**
 * Definition of a builtin Action.
 */
export class ActionDefinition extends MethodDefinition {
  static metatype: StructType = StructType.ACTION_DEFINITION;
  static __isFrozen__: boolean = true;

  constructor(options: {
    id: number;
    type: MethodType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyDefinition[];
    cardinality?: MethodCardinality;
    platforms?: readonly PlatformType[];
    languages?: readonly RuntimeLanguage[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
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
    if (!(this.cardinality === other.cardinality)) {
      return false;
    }
    if (this.platforms.length != other.platforms.length) {
      return false;
    }
    for (let i = 0; i < this.platforms.length; i++) {
      if (!(this.platforms[i] === other.platforms[i])) {
        return false;
      }
    }
    if (this.languages.length != other.languages.length) {
      return false;
    }
    for (let i = 0; i < this.languages.length; i++) {
      if (!(this.languages[i] === other.languages[i])) {
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
      propertyReprs.push(`name=${`"${this.name}"`}`);
      if (this.description != null) {
        propertyReprs.push(`description=${`"${this.description}"`}`);
      }
      // @ts-expect-error(readonly)
      this._repr = `<ActionDefinition ${propertyReprs.join(" ")}>`;
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
    h = (h * 31 + this.cardinality) & 0xffffffff;
    if (this.platforms && this.platforms.length > 0) {
      for (const _item of this.platforms) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this.languages && this.languages.length > 0) {
      for (const _item of this.languages) {
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
      this._cson = ActionDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: ActionDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 40100;
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
      objectCson["104"] = packedProperties;
    }
    objectCson["110"] = object.cardinality;
    if (object.platforms.length > 0) {
      const packedPlatforms: any[] = [];
      for (const item of object.platforms) {
        packedPlatforms.push(item);
      }
      objectCson["120"] = packedPlatforms;
    }
    if (object.languages.length > 0) {
      const packedLanguages: any[] = [];
      for (const item of object.languages) {
        packedLanguages.push(item);
      }
      objectCson["121"] = packedLanguages;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
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
    if (objectCson["104"] != undefined) {
      for (const item of objectCson["104"]) {
        unpackedProperties.push(
          _PropertyDefinition.fromCson(item, _session, _supergraph, _graph, _connection),
        );
      }
    }
    const unpackedPlatforms: any[] = [];
    if (objectCson["120"] != undefined) {
      for (const item of objectCson["120"]) {
        unpackedPlatforms.push(Number(item));
      }
    }
    const unpackedLanguages: any[] = [];
    if (objectCson["121"] != undefined) {
      for (const item of objectCson["121"]) {
        unpackedLanguages.push(Number(item));
      }
    }
    const iconValue = objectCson["102"];
    const unpackedIcon =
      iconValue != undefined
        ? _Icon.fromCson(iconValue, _session, _supergraph, _graph, _connection)
        : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    return new ActionDefinition({
      type: Number(objectCson["100"]),
      properties: unpackedProperties,
      cardinality: Number(objectCson["110"]),
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
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
  ): ActionDefinition {
    return ActionDefinition.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): ActionDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = ActionDefinition.__packProto__(this);
    }
    return this._proto as ActionDefinitionProto;
  }

  static __packProto__(object: ActionDefinition): ActionDefinitionProto {
    const objectProto: Partial<ActionDefinitionProto> = { metatype: 40100 };
    objectProto.id = object.id;
    objectProto.type = Number(object.type) as MethodTypeProto;
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
    objectProto.cardinality = Number(object.cardinality) as MethodCardinalityProto;
    if (object.platforms) {
      const packedPlatforms: any[] = [];
      for (const item of object.platforms) {
        packedPlatforms.push(Number(item) as PlatformTypeProto);
      }
      objectProto.platforms = packedPlatforms;
    }
    if (object.languages) {
      const packedLanguages: any[] = [];
      for (const item of object.languages) {
        packedLanguages.push(Number(item) as RuntimeLanguageProto);
      }
      objectProto.languages = packedLanguages;
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
    const unpackedPlatforms: any[] = [];
    if (objectProto.platforms) {
      for (const item of objectProto.platforms) {
        unpackedPlatforms.push(Number(item) as PlatformType);
      }
    }
    const unpackedLanguages: any[] = [];
    if (objectProto.languages) {
      for (const item of objectProto.languages) {
        unpackedLanguages.push(Number(item) as RuntimeLanguage);
      }
    }
    return new ActionDefinition({
      type: Number(objectProto.type) as MethodType,
      properties: unpackedProperties,
      cardinality: Number(objectProto.cardinality) as MethodCardinality,
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
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
/* ==== DESTACK_GENERATED_END:STRUCT:40100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40100 ==== */
/**
 * An implementation of a unit of work, usually expressed with Code or some tool.
 * May defer to a builtin or some other service in a separate system.
 */
export class Action extends Method implements IsRunnable {
  static metatype: NodeType = NodeType.ACTION;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Action | NodeReference | null;
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
    type: MethodType;
    text?: Text | null;
    cardinality?: MethodCardinality;
    platforms?: readonly PlatformType[];
    languages?: readonly RuntimeLanguage[];
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
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
    if (!(this._type === other._type)) {
      return false;
    }
    if (
      (this._text == null) !== (other._text == null) ||
      (this._text != null && !this._text.equals(other._text))
    ) {
      return false;
    }
    if (!(this._cardinality === other._cardinality)) {
      return false;
    }
    if (this._platforms.length != other._platforms.length) {
      return false;
    }
    for (let i = 0; i < this._platforms.length; i++) {
      if (!(this._platforms[i] === other._platforms[i])) {
        return false;
      }
    }
    if (this._languages.length != other._languages.length) {
      return false;
    }
    for (let i = 0; i < this._languages.length; i++) {
      if (!(this._languages[i] === other._languages[i])) {
        return false;
      }
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
    if (this._text != null) {
      h = (h * 31 + this._text.hash()) & 0xffffffff;
    }
    h = (h * 31 + this._cardinality) & 0xffffffff;
    if (this._platforms && this._platforms.length > 0) {
      for (const _item of this._platforms) {
        h = (h * 31 + _item) & 0xffffffff;
      }
    }
    if (this._languages && this._languages.length > 0) {
      for (const _item of this._languages) {
        h = (h * 31 + _item) & 0xffffffff;
      }
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
      type: NodeType.ACTION,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Action "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return Action.__packCson__(this);
  }

  static __packCson__(object: Action): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 40100;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    objectCson["100"] = object._type;
    if (object._text != null) {
      objectCson["104"] = object._text.toCson();
    }
    objectCson["110"] = object._cardinality;
    if (object._platforms.length > 0) {
      const packedPlatforms: any[] = [];
      for (const item of object._platforms) {
        packedPlatforms.push(item);
      }
      objectCson["120"] = packedPlatforms;
    }
    if (object._languages.length > 0) {
      const packedLanguages: any[] = [];
      for (const item of object._languages) {
        packedLanguages.push(item);
      }
      objectCson["121"] = packedLanguages;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Action {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Text = STRUCT_CLASS_BY_TYPE[StructType.TEXT] as typeof Text;
    const textValue = objectCson["104"];
    const unpackedText =
      textValue != undefined
        ? _Text.fromCson(textValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedPlatforms: any[] = [];
    if (objectCson["120"] != undefined) {
      for (const item of objectCson["120"]) {
        unpackedPlatforms.push(Number(item));
      }
    }
    const unpackedLanguages: any[] = [];
    if (objectCson["121"] != undefined) {
      for (const item of objectCson["121"]) {
        unpackedLanguages.push(Number(item));
      }
    }
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new Action({
      type: Number(objectCson["100"]),
      text: unpackedText,
      cardinality: Number(objectCson["110"]),
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Action {
    return Action.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): ActionProto {
    return Action.__packProto__(this);
  }

  static __packProto__(object: Action): ActionProto {
    const objectProto: Partial<ActionProto> = { metatype: 40100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
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
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.type = Number(object._type) as MethodTypeProto;
    if (object._text != null) {
      objectProto.text = object._text.toProto();
    }
    objectProto.cardinality = Number(object._cardinality) as MethodCardinalityProto;
    if (object._platforms) {
      const packedPlatforms: any[] = [];
      for (const item of object._platforms) {
        packedPlatforms.push(Number(item) as PlatformTypeProto);
      }
      objectProto.platforms = packedPlatforms;
    }
    if (object._languages) {
      const packedLanguages: any[] = [];
      for (const item of object._languages) {
        packedLanguages.push(Number(item) as RuntimeLanguageProto);
      }
      objectProto.languages = packedLanguages;
    }
    return objectProto as ActionProto;
  }

  static __unpackProto__(
    objectProto: ActionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Action {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Text = STRUCT_CLASS_BY_TYPE[StructType.TEXT] as typeof Text;
    const unpackedPlatforms: any[] = [];
    if (objectProto.platforms) {
      for (const item of objectProto.platforms) {
        unpackedPlatforms.push(Number(item) as PlatformType);
      }
    }
    const unpackedLanguages: any[] = [];
    if (objectProto.languages) {
      for (const item of objectProto.languages) {
        unpackedLanguages.push(Number(item) as RuntimeLanguage);
      }
    }
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Action({
      type: Number(objectProto.type) as MethodType,
      text:
        objectProto.text != undefined
          ? _Text.fromProto(objectProto.text!, _session, _supergraph, _graph, _connection)
          : null,
      cardinality: Number(objectProto.cardinality) as MethodCardinality,
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
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
      materialization: Number(objectProto.materialization) as Materialization,
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
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
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
    objectProto: ActionProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Action {
    return Action.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Action {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = ActionProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.ACTION, Action);
/* ==== DESTACK_GENERATED_END:NODE:40100 ==== */
