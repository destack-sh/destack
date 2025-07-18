import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  IsJoinable,
  IsScriptable,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  Event,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Organization } from "@destack/language/universe/organization";
import { MaterializationProto, TeamProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:122100 ==== */
/**
 * An Team with Users and Teams.
 */
export class Team extends Entity implements IsActor, IsJoinable, IsScriptable {
  static metatype: NodeType = NodeType.TEAM;

  /**
   * Team.parent
   */
  get parent(): Organization | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Organization | null;
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
   * The definition this CustomEntity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
   * The previous Entity this Entity is based on (from the base Snapshot).
   */
  get precededBy(): Team | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Team | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instantiationRoot(): Entity | null {
    const nodePtr: NodeReference | null = this.instantiationRootPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instantiationRootPtr: NodeReference | null;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
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
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
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
   * The main / root Script of this Node.
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
   * Team.slug
   */
  /**
   * Team.slug
   */
  get slug(): string {
    return this._slug;
  }
  set slug(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["slug"];
    this._session.updateSetProperty(this, prop, value);
    this._slug = value;
  }
  _slug: string;

  constructor(options: {
    id?: string;
    parent?: Organization | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: Team | NodeReference | null;
    instantiationRoot?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    name?: string;
    script?: Script | NodeReference | null;
    slug: string;
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
        throw new Error(`no active Space for Team`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`Team.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`Team.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for Team`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`Team.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for Team`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`Team.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instantiationRoot = options.instantiationRoot ?? null;
    if (_instantiationRoot != null && _instantiationRoot.metatype != StructType.NODE_REFERENCE) {
      _instantiationRoot = (_instantiationRoot as Node).toRef();
    }
    this.instantiationRootPtr = _instantiationRoot;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "Team";
    }
    if (_name === null) {
      throw new Error(`Team.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _slug = options.slug;
    if (_slug === null) {
      throw new Error(`Team.slug is required`);
    }
    this._slug = _slug;

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
        throw new Error(`Team.createdAt and Team.updatedAt are required for existing Nodes`);
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
    if (!(this._slug === other._slug)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
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
    h = (h * 31 + hashString(this._slug)) & 0xffffffff;
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
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
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.TEAM,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.slug ?? this.name;
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
    propertyReprs.push(`slug=${`"${this.slug}"`}`);
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<Team "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return Team.__packValue__(this);
  }

  static __packValue__(object: Team): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 122100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    objectValue["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectValue["11"] = object.definitionPtr.toValue();
    }
    objectValue["12"] = object.branchPtr.toValue();
    objectValue["13"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["14"] = object.precededByPtr.toValue();
    }
    if (object.instantiationRootPtr != null) {
      objectValue["15"] = object.instantiationRootPtr.toValue();
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
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["30"] = packedCustomValues;
    }
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["102"] = object._slug;
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Team {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instantiationRootPtrValue = objectValue["15"];
    const unpackedInstantiationRootPtr =
      instantiationRootPtrValue != undefined
        ? _NodeReference.fromValue(
            instantiationRootPtrValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
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
    const unpackedCustomValues = {} as any;
    if (objectValue["30"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new Team({
      parent: unpackedParentPtr,
      slug: objectValue["102"],
      script: unpackedScriptPtr,
      materialization: Number(objectValue["10"]),
      definition: unpackedDefinitionPtr,
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
      instantiationRoot: unpackedInstantiationRootPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      id: String(objectValue["2"]),
      customValues: unpackedCustomValues,
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
  ): Team {
    return Team.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): TeamProto {
    return Team.__packProto__(this);
  }

  static __packProto__(object: Team): TeamProto {
    const objectProto: Partial<TeamProto> = { metatype: 122100 };
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
    if (object.instantiationRootPtr != null) {
      objectProto.instantiationRootPtr = object.instantiationRootPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.slug = object._slug;
    return objectProto as TeamProto;
  }

  static __unpackProto__(
    objectProto: TeamProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Team {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new Team({
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
      slug: objectProto.slug,
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
      instantiationRoot:
        objectProto.instantiationRootPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instantiationRootPtr!,
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
      id: String(objectProto.id),
      customValues: unpackedCustomValues,
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
    objectProto: TeamProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Team {
    return Team.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Team {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TeamProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.TEAM, Team);
/* ==== DESTACK_GENERATED_END:NODE:122100 ==== */
