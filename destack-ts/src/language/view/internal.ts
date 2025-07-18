import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Dimension,
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  Position,
  QueryConnection,
  Session,
  Snapshot,
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
import type { Space } from "@destack/language/universe";
import { View } from "@destack/language/view/view";
import { InternalViewProto, MaterializationProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1815000 ==== */
/**
 * A content View.
 */
export class InternalView extends View {
  static metatype: NodeType = NodeType.INTERNAL_VIEW;

  /**
   * The parent of this Entity. Most Entities can be attached to any other Entity.
   */
  get parent(): Entity | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
   * Entity.materialization
   */
  readonly materialization: Materialization;

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
  get precededBy(): InternalView | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as InternalView | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

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
   * The absolute order key of this Node in its parent.
   */
  readonly orderKey: string;

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
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
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
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * View.position
   */
  /**
   * View.position
   */
  get position(): Position | null {
    return this._position;
  }
  set position(value: Position | null) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: Position | null;

  /**
   * View.width
   */
  /**
   * View.width
   */
  get width(): Dimension | null {
    return this._width;
  }
  set width(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: Dimension | null;

  /**
   * View.height
   */
  /**
   * View.height
   */
  get height(): Dimension | null {
    return this._height;
  }
  set height(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: Dimension | null;

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  get minWidth(): Dimension | null {
    return this._minWidth;
  }
  set minWidth(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_width"];
    this._session.updateSetProperty(this, prop, value);
    this._minWidth = value;
  }
  _minWidth: Dimension | null;

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  get minHeight(): Dimension | null {
    return this._minHeight;
  }
  set minHeight(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_height"];
    this._session.updateSetProperty(this, prop, value);
    this._minHeight = value;
  }
  _minHeight: Dimension | null;

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  get maxWidth(): Dimension | null {
    return this._maxWidth;
  }
  set maxWidth(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_width"];
    this._session.updateSetProperty(this, prop, value);
    this._maxWidth = value;
  }
  _maxWidth: Dimension | null;

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  get maxHeight(): Dimension | null {
    return this._maxHeight;
  }
  set maxHeight(value: Dimension | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_height"];
    this._session.updateSetProperty(this, prop, value);
    this._maxHeight = value;
  }
  _maxHeight: Dimension | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    definition?: Entity | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference;
    precededBy?: InternalView | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    orderKey?: string;
    name?: string;
    source?: Script | NodeReference | null;
    key?: string | null;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    position?: Position | null;
    width?: Dimension | null;
    height?: Dimension | null;
    minWidth?: Dimension | null;
    minHeight?: Dimension | null;
    maxWidth?: Dimension | null;
    maxHeight?: Dimension | null;
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
        throw new Error(`no active Space for InternalView`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`InternalView.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`InternalView.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for InternalView`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`InternalView.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`InternalView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "InternalView";
    }
    if (_name === null) {
      throw new Error(`InternalView.name is required`);
    }
    this._name = _name;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _key = options.key ?? null;
    this._key = _key;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`InternalView.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _position = options.position ?? null;
    this._position = _position;
    let _width = options.width ?? null;
    this._width = _width;
    let _height = options.height ?? null;
    this._height = _height;
    let _minWidth = options.minWidth ?? null;
    this._minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this._minHeight = _minHeight;
    let _maxWidth = options.maxWidth ?? null;
    this._maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this._maxHeight = _maxHeight;

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
        throw new Error(
          `InternalView.createdAt and InternalView.updatedAt are required for existing Nodes`,
        );
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
    if (
      (this._position == null) !== (other._position == null) ||
      (this._position != null && !this._position.equals(other._position))
    ) {
      return false;
    }
    if (
      (this._width == null) !== (other._width == null) ||
      (this._width != null && !this._width.equals(other._width))
    ) {
      return false;
    }
    if (
      (this._height == null) !== (other._height == null) ||
      (this._height != null && !this._height.equals(other._height))
    ) {
      return false;
    }
    if (
      (this._minWidth == null) !== (other._minWidth == null) ||
      (this._minWidth != null && !this._minWidth.equals(other._minWidth))
    ) {
      return false;
    }
    if (
      (this._minHeight == null) !== (other._minHeight == null) ||
      (this._minHeight != null && !this._minHeight.equals(other._minHeight))
    ) {
      return false;
    }
    if (
      (this._maxWidth == null) !== (other._maxWidth == null) ||
      (this._maxWidth != null && !this._maxWidth.equals(other._maxWidth))
    ) {
      return false;
    }
    if (
      (this._maxHeight == null) !== (other._maxHeight == null) ||
      (this._maxHeight != null && !this._maxHeight.equals(other._maxHeight))
    ) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
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
    if (!(this.snapshotPtr.id === other.snapshotPtr.id)) {
      return false;
    }
    if (!(this.precededByPtr?.id === other.precededByPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
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
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this._position != null) {
      h = (h * 31 + this._position.hash()) & 0xffffffff;
    }
    if (this._width != null) {
      h = (h * 31 + this._width.hash()) & 0xffffffff;
    }
    if (this._height != null) {
      h = (h * 31 + this._height.hash()) & 0xffffffff;
    }
    if (this._minWidth != null) {
      h = (h * 31 + this._minWidth.hash()) & 0xffffffff;
    }
    if (this._minHeight != null) {
      h = (h * 31 + this._minHeight.hash()) & 0xffffffff;
    }
    if (this._maxWidth != null) {
      h = (h * 31 + this._maxWidth.hash()) & 0xffffffff;
    }
    if (this._maxHeight != null) {
      h = (h * 31 + this._maxHeight.hash()) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
    }
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    if (this.precededByPtr != null) {
      h = (h * 31 + hashString(this.precededByPtr.id)) & 0xffffffff;
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
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.INTERNAL_VIEW,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
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
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<InternalView "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return InternalView.__packValue__(this);
  }

  static __packValue__(object: InternalView): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 1815000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    objectValue["11"] = object.snapshotPtr.toValue();
    if (object.precededByPtr != null) {
      objectValue["12"] = object.precededByPtr.toValue();
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
    objectValue["31"] = object.orderKey;
    objectValue["50"] = object._name;
    if (object.sourcePtr != null) {
      objectValue["60"] = object.sourcePtr.toValue();
    }
    if (object._key != null) {
      objectValue["70"] = object._key;
    }
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    if (object._position != null) {
      objectValue["110"] = object._position.toValue();
    }
    if (object._width != null) {
      objectValue["111"] = object._width.toValue();
    }
    if (object._height != null) {
      objectValue["112"] = object._height.toValue();
    }
    if (object._minWidth != null) {
      objectValue["113"] = object._minWidth.toValue();
    }
    if (object._minHeight != null) {
      objectValue["114"] = object._minHeight.toValue();
    }
    if (object._maxWidth != null) {
      objectValue["115"] = object._maxWidth.toValue();
    }
    if (object._maxHeight != null) {
      objectValue["116"] = object._maxHeight.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InternalView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const positionValue = objectValue["110"];
    const unpackedPosition =
      positionValue != undefined
        ? _Position.fromValue(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectValue["111"];
    const unpackedWidth =
      widthValue != undefined
        ? _Dimension.fromValue(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectValue["112"];
    const unpackedHeight =
      heightValue != undefined
        ? _Dimension.fromValue(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectValue["113"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? _Dimension.fromValue(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectValue["114"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? _Dimension.fromValue(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectValue["115"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? _Dimension.fromValue(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectValue["116"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? _Dimension.fromValue(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectValue["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromValue(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectValue["70"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectValue["12"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromValue(precededByPtrValue, _session, _supergraph, _graph, _connection)
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
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
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
    return new InternalView({
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      source: unpackedSourcePtr,
      key: unpackedKey,
      parent: unpackedParentPtr,
      materialization: Number(objectValue["10"]),
      snapshot: _NodeReference.fromValue(
        objectValue["11"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectValue["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectValue["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      name: objectValue["50"],
      id: String(objectValue["2"]),
      script: unpackedScriptPtr,
      orderKey: objectValue["31"],
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
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
  ): InternalView {
    return InternalView.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): InternalViewProto {
    return InternalView.__packProto__(this);
  }

  static __packProto__(object: InternalView): InternalViewProto {
    const objectProto: Partial<InternalViewProto> = { metatype: 1815000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
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
    objectProto.orderKey = object.orderKey;
    objectProto.name = object._name;
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    if (object._position != null) {
      objectProto.position = object._position.toProto();
    }
    if (object._width != null) {
      objectProto.width = object._width.toProto();
    }
    if (object._height != null) {
      objectProto.height = object._height.toProto();
    }
    if (object._minWidth != null) {
      objectProto.minWidth = object._minWidth.toProto();
    }
    if (object._minHeight != null) {
      objectProto.minHeight = object._minHeight.toProto();
    }
    if (object._maxWidth != null) {
      objectProto.maxWidth = object._maxWidth.toProto();
    }
    if (object._maxHeight != null) {
      objectProto.maxHeight = object._maxHeight.toProto();
    }
    return objectProto as InternalViewProto;
  }

  static __unpackProto__(
    objectProto: InternalViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InternalView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new InternalView({
      position:
        objectProto.position != undefined
          ? _Position.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      width:
        objectProto.width != undefined
          ? _Dimension.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? _Dimension.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? _Dimension.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? _Dimension.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? _Dimension.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? _Dimension.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
          : null,
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
      isExtensible: objectProto.isExtensible,
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
      orderKey: objectProto.orderKey,
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      customValues: unpackedCustomValues,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: InternalViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): InternalView {
    return InternalView.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): InternalView {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = InternalViewProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.INTERNAL_VIEW, InternalView);
/* ==== DESTACK_GENERATED_END:NODE:1815000 ==== */
