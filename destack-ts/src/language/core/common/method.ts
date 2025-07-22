import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import {
  NodeType,
  PlatformType,
  RuntimeLanguage,
  StructType,
} from "@destack/language/core/builtin/common";
import { ACTIVE_BRANCH, ACTIVE_SNAPSHOT, ACTIVE_SPACE } from "@destack/language/core/builtin/const";
import type { PropertyDefinition } from "@destack/language/core/builtin/definition";
import { BuiltinDefinition } from "@destack/language/core/builtin/definition";
import { Entity, Materialization } from "@destack/language/core/builtin/entity";
import { Event } from "@destack/language/core/builtin/event";
import { MethodCardinality, MethodType } from "@destack/language/core/builtin/meta";
import type { NodeClass } from "@destack/language/core/builtin/node";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import type { IsActor } from "@destack/language/core/builtin/trait";
import type { Icon } from "@destack/language/core/common/icon";
import type { Space } from "@destack/language/core/common/space";
import type { Text } from "@destack/language/core/common/text";
import type { Branch, Snapshot } from "@destack/language/core/common/time";
import type { Value } from "@destack/language/core/common/value";
import type { GraphConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerNodeClass,
  registerStructClass,
} from "@destack/language/registry";
import {
  MaterializationProto,
  MethodCardinalityProto,
  MethodDefinitionProto,
  MethodProto,
  MethodTypeProto,
  PlatformTypeProto,
  RuntimeLanguageProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:STRUCT:40000 ==== */
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
   * MethodDefinition.type
   */
  readonly type: MethodType;

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
  readonly properties: readonly PropertyDefinition[];

  /**
   * BuiltinDefinition.taggings
   */
  readonly taggings: readonly number[];

  /**
   * MethodDefinition.cardinality
   */
  readonly cardinality: MethodCardinality;

  /**
   * The platforms this Method is available on (all if empty).
   */
  readonly platforms: readonly PlatformType[];

  /**
   * The languages this Method is available in (all if empty).
   */
  readonly languages: readonly RuntimeLanguage[];

  constructor(options: {
    id: number;
    type: MethodType;
    name: string;
    icon?: Icon | null;
    description?: string | null;
    properties?: readonly PropertyDefinition[];
    taggings?: readonly number[];
    cardinality?: MethodCardinality;
    platforms?: readonly PlatformType[];
    languages?: readonly RuntimeLanguage[];
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
    );

    // properties
    let _id = options.id;
    if (_id === null) {
      throw new Error(`MethodDefinition.id is required`);
    }
    this.id = _id;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`MethodDefinition.type is required`);
    }
    this.type = _type;
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
    let _taggings = options.taggings ?? null;
    if (_taggings === null) {
      _taggings = [];
    }
    this.taggings = _taggings;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* MethodCardinality.UNARY */;
    }
    if (_cardinality === null) {
      throw new Error(`MethodDefinition.cardinality is required`);
    }
    this.cardinality = _cardinality;
    let _platforms = options.platforms ?? null;
    if (_platforms === null) {
      _platforms = [];
    }
    this.platforms = _platforms;
    let _languages = options.languages ?? null;
    if (_languages === null) {
      _languages = [];
    }
    this.languages = _languages;

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
      this._repr = `<MethodDefinition ${propertyReprs.join(" ")}>`;
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
      this._cson = MethodDefinition.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: MethodDefinition): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 40000;
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
    if (object.taggings.length > 0) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectCson["109"] = packedTaggings;
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
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): MethodDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectCson["104"] != undefined) {
      for (const item of objectCson["104"]) {
        unpackedProperties.push(_PropertyDefinition.fromCson(item, _session, _graph, _connection));
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
      iconValue != undefined ? _Icon.fromCson(iconValue, _session, _graph, _connection) : null;
    const descriptionValue = objectCson["103"];
    const unpackedDescription = descriptionValue != undefined ? descriptionValue : null;
    const unpackedTaggings: any[] = [];
    if (objectCson["109"] != undefined) {
      for (const item of objectCson["109"]) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new MethodDefinition({
      type: Number(objectCson["100"]),
      properties: unpackedProperties,
      cardinality: Number(objectCson["110"]),
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      id: Number(objectCson["2"]),
      name: objectCson["101"],
      icon: unpackedIcon,
      description: unpackedDescription,
      taggings: unpackedTaggings,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): MethodDefinition {
    return MethodDefinition.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): MethodDefinitionProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = MethodDefinition.__packProto__(this);
    }
    return this._proto as MethodDefinitionProto;
  }

  static __packProto__(object: MethodDefinition): MethodDefinitionProto {
    const objectProto: Partial<MethodDefinitionProto> = { metatype: 40000 };
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
    if (object.taggings) {
      const packedTaggings: any[] = [];
      for (const item of object.taggings) {
        packedTaggings.push(item);
      }
      objectProto.taggings = packedTaggings;
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
    return objectProto as MethodDefinitionProto;
  }

  static __unpackProto__(
    objectProto: MethodDefinitionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): MethodDefinition {
    const _PropertyDefinition = STRUCT_CLASS_BY_TYPE[
      StructType.PROPERTY_DEFINITION
    ] as typeof PropertyDefinition;
    const _Icon = STRUCT_CLASS_BY_TYPE[StructType.ICON] as typeof Icon;
    const unpackedProperties: any[] = [];
    if (objectProto.properties) {
      for (const item of objectProto.properties) {
        unpackedProperties.push(
          _PropertyDefinition.fromProto(item!, _session, _graph, _graph, _connection),
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
    const unpackedTaggings: any[] = [];
    if (objectProto.taggings) {
      for (const item of objectProto.taggings) {
        unpackedTaggings.push(Number(item));
      }
    }
    return new MethodDefinition({
      type: Number(objectProto.type) as MethodType,
      properties: unpackedProperties,
      cardinality: Number(objectProto.cardinality) as MethodCardinality,
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      id: Number(objectProto.id),
      name: objectProto.name,
      icon:
        objectProto.icon != undefined
          ? _Icon.fromProto(objectProto.icon!, _session, _graph, _graph, _connection)
          : null,
      description: objectProto.description != undefined ? objectProto.description : null,
      taggings: unpackedTaggings,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: MethodDefinitionProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): MethodDefinition {
    return MethodDefinition.__unpackProto__(objectProto, _session, _graph, _connection);
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
/* ==== DESTACK_GENERATED_END:STRUCT:40000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40000 ==== */
/**
 * A Method is a small piece of logic.
 */
export class Method extends Entity {
  static metatype: NodeType = NodeType.METHOD;

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
  get precededBy(): Method | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._graph.get(nodePtr.id) as Method | null;
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
   * Method.type
   */
  /**
   * Method.type
   */
  get type(): MethodType {
    return this._type;
  }
  set type(value: MethodType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: MethodType;

  /**
   * Method.text
   */
  /**
   * Method.text
   */
  get text(): Text | null {
    return this._text;
  }
  set text(value: Text | null) {
    const prop = (this.constructor as NodeClass).__properties__["text"];
    this._session.updateSetProperty(this, prop, value);
    this._text = value;
  }
  _text: Text | null;

  /**
   * Method.cardinality
   */
  /**
   * Method.cardinality
   */
  get cardinality(): MethodCardinality {
    return this._cardinality;
  }
  set cardinality(value: MethodCardinality) {
    const prop = (this.constructor as NodeClass).__properties__["cardinality"];
    this._session.updateSetProperty(this, prop, value);
    this._cardinality = value;
  }
  _cardinality: MethodCardinality;

  /**
   * The platforms this Method is available on (all if empty).
   */
  /**
   * The platforms this Method is available on (all if empty).
   */
  get platforms(): readonly PlatformType[] {
    return this._platforms;
  }
  set platforms(value: readonly PlatformType[]) {
    const prop = (this.constructor as NodeClass).__properties__["platforms"];
    this._session.updateSetProperty(this, prop, value);
    this._platforms = value;
  }
  _platforms: readonly PlatformType[];

  /**
   * The languages this Method is available in (all if empty).
   */
  /**
   * The languages this Method is available in (all if empty).
   */
  get languages(): readonly RuntimeLanguage[] {
    return this._languages;
  }
  set languages(value: readonly RuntimeLanguage[]) {
    const prop = (this.constructor as NodeClass).__properties__["languages"];
    this._session.updateSetProperty(this, prop, value);
    this._languages = value;
  }
  _languages: readonly RuntimeLanguage[];

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Method | NodeReference | null;
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
    _graph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: GraphConnection | null;
  }) {
    super(
      // id
      options.id ?? null,
      // parent
      options.parent != null
        ? options.parent.constructor.name == "NodeReference"
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null,
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
      // graph
      options._graph ?? null,
      // connection
      options._connection ?? null,
      // _isNew
      options.id == null,
    );

    // properties
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
        throw new Error(`no active Space for Method`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Method.space is required`);
    }
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Method.materialization is required`);
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
        throw new Error(`no active Branch for Method`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Method.branch is required`);
    }
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name != "NodeReference") {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Method`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Method.snapshot is required`);
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
      _name = "Method";
    }
    if (_name === null) {
      throw new Error(`Method.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`Method.orderKey is required`);
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
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Method.type is required`);
    }
    this._type = _type;
    let _text = options.text ?? null;
    this._text = _text;
    let _cardinality = options.cardinality ?? null;
    if (_cardinality === null) {
      _cardinality = 1 /* MethodCardinality.UNARY */;
    }
    if (_cardinality === null) {
      throw new Error(`Method.cardinality is required`);
    }
    this._cardinality = _cardinality;
    let _platforms = options.platforms ?? null;
    if (_platforms === null) {
      _platforms = [];
    }
    this._platforms = _platforms;
    let _languages = options.languages ?? null;
    if (_languages === null) {
      _languages = [];
    }
    this._languages = _languages;

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
        throw new Error(`Method.createdAt and Method.updatedAt are required for existing Nodes`);
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
      type: NodeType.METHOD,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Method "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return Method.__packCson__(this);
  }

  static __packCson__(object: Method): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 40000;
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
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Method {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Text = STRUCT_CLASS_BY_TYPE[StructType.TEXT] as typeof Text;
    const textValue = objectCson["104"];
    const unpackedText =
      textValue != undefined ? _Text.fromCson(textValue, _session, _graph, _connection) : null;
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
        ? _NodeReference.fromCson(parentPtrValue, _session, _graph, _connection)
        : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new Method({
      type: Number(objectCson["100"]),
      text: unpackedText,
      cardinality: Number(objectCson["110"]),
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      parent: unpackedParentPtr,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _graph, _connection),
      snapshot: _NodeReference.fromCson(objectCson["13"], _session, _graph, _connection),
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
      space: _NodeReference.fromCson(objectCson["5"], _session, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Method {
    return Method.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): MethodProto {
    return Method.__packProto__(this);
  }

  static __packProto__(object: Method): MethodProto {
    const objectProto: Partial<MethodProto> = { metatype: 40000 };
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
    return objectProto as MethodProto;
  }

  static __unpackProto__(
    objectProto: MethodProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Method {
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
          _Value.fromProto((value as any)!, _session, _graph, _graph, _connection),
        );
      }
    }
    return new Method({
      type: Number(objectProto.type) as MethodType,
      text:
        objectProto.text != undefined
          ? _Text.fromProto(objectProto.text!, _session, _graph, _graph, _connection)
          : null,
      cardinality: Number(objectProto.cardinality) as MethodCardinality,
      platforms: unpackedPlatforms,
      languages: unpackedLanguages,
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(objectProto.parentPtr!, _session, _graph, _graph, _connection)
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _graph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _graph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _graph,
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
              _graph,
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
              _graph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(objectProto.ownedByPtr!, _session, _graph, _graph, _connection)
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(objectProto.scriptPtr!, _session, _graph, _graph, _connection)
          : null,
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(objectProto.sourcePtr!, _session, _graph, _graph, _connection)
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(objectProto.spacePtr!, _session, _graph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: MethodProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Method {
    return Method.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Method {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = MethodProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.METHOD, Method);
/* ==== DESTACK_GENERATED_END:NODE:40000 ==== */
