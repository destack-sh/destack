import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
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
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  Event,
  Materialization,
  Node,
  NodeType,
  StructType,
} from "@destack/language/core";
import type {
  Axis2,
  Corner2,
  Grid2,
  GridSpan2,
  Inset2,
  Length,
  Offset2,
  Vector2,
} from "@destack/language/geometry";
import { Align, Anchor, Direction, Distribute, Layout } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Border, Fill, Shadow } from "@destack/language/style";
import { LayoutView } from "@destack/language/view/layout";
import {
  AlignProto,
  AnchorProto,
  DirectionProto,
  DistributeProto,
  LayoutProto,
  MaterializationProto,
  SplitViewProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1800400 ==== */
/**
 * A split container View.
 */
export class SplitView extends LayoutView {
  static metatype: NodeType = NodeType.SPLIT_VIEW;

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
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
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
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): SplitView | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as SplitView | null;
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
      return this._supergraph.get(nodePtr.id) as Entity | null;
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
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
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
   * Entity2D.position
   */
  /**
   * Entity2D.position
   */
  get position(): Vector2 | null {
    return this._position;
  }
  set position(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["position"];
    this._session.updateSetProperty(this, prop, value);
    this._position = value;
  }
  _position: Vector2 | null;

  /**
   * Entity2D.offset
   */
  /**
   * Entity2D.offset
   */
  get offset(): Offset2 | null {
    return this._offset;
  }
  set offset(value: Offset2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["offset"];
    this._session.updateSetProperty(this, prop, value);
    this._offset = value;
  }
  _offset: Offset2 | null;

  /**
   * Entity2D.scale
   */
  /**
   * Entity2D.scale
   */
  get scale(): Vector2 | null {
    return this._scale;
  }
  set scale(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["scale"];
    this._session.updateSetProperty(this, prop, value);
    this._scale = value;
  }
  _scale: Vector2 | null;

  /**
   * Entity2D.rotation
   */
  /**
   * Entity2D.rotation
   */
  get rotation(): Vector2 | null {
    return this._rotation;
  }
  set rotation(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["rotation"];
    this._session.updateSetProperty(this, prop, value);
    this._rotation = value;
  }
  _rotation: Vector2 | null;

  /**
   * Entity2D.skew
   */
  /**
   * Entity2D.skew
   */
  get skew(): Vector2 | null {
    return this._skew;
  }
  set skew(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["skew"];
    this._session.updateSetProperty(this, prop, value);
    this._skew = value;
  }
  _skew: Vector2 | null;

  /**
   * Entity2D.origin
   */
  /**
   * Entity2D.origin
   */
  get origin(): Vector2 | null {
    return this._origin;
  }
  set origin(value: Vector2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["origin"];
    this._session.updateSetProperty(this, prop, value);
    this._origin = value;
  }
  _origin: Vector2 | null;

  /**
   * Entity2D.anchor
   */
  /**
   * Entity2D.anchor
   */
  get anchor(): Anchor | null {
    return this._anchor;
  }
  set anchor(value: Anchor | null) {
    const prop = (this.constructor as NodeClass).__properties__["anchor"];
    this._session.updateSetProperty(this, prop, value);
    this._anchor = value;
  }
  _anchor: Anchor | null;

  /**
   * View.width
   */
  /**
   * View.width
   */
  get width(): Length | null {
    return this._width;
  }
  set width(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: Length | null;

  /**
   * View.height
   */
  /**
   * View.height
   */
  get height(): Length | null {
    return this._height;
  }
  set height(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: Length | null;

  /**
   * View.minWidth
   */
  /**
   * View.minWidth
   */
  get minWidth(): Length | null {
    return this._minWidth;
  }
  set minWidth(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_width"];
    this._session.updateSetProperty(this, prop, value);
    this._minWidth = value;
  }
  _minWidth: Length | null;

  /**
   * View.minHeight
   */
  /**
   * View.minHeight
   */
  get minHeight(): Length | null {
    return this._minHeight;
  }
  set minHeight(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["min_height"];
    this._session.updateSetProperty(this, prop, value);
    this._minHeight = value;
  }
  _minHeight: Length | null;

  /**
   * View.maxWidth
   */
  /**
   * View.maxWidth
   */
  get maxWidth(): Length | null {
    return this._maxWidth;
  }
  set maxWidth(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_width"];
    this._session.updateSetProperty(this, prop, value);
    this._maxWidth = value;
  }
  _maxWidth: Length | null;

  /**
   * View.maxHeight
   */
  /**
   * View.maxHeight
   */
  get maxHeight(): Length | null {
    return this._maxHeight;
  }
  set maxHeight(value: Length | null) {
    const prop = (this.constructor as NodeClass).__properties__["max_height"];
    this._session.updateSetProperty(this, prop, value);
    this._maxHeight = value;
  }
  _maxHeight: Length | null;

  /**
   * View.isVisible
   */
  /**
   * View.isVisible
   */
  get isVisible(): boolean | null {
    return this._isVisible;
  }
  set isVisible(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_visible"];
    this._session.updateSetProperty(this, prop, value);
    this._isVisible = value;
  }
  _isVisible: boolean | null;

  /**
   * View.opacity
   */
  /**
   * View.opacity
   */
  get opacity(): number | null {
    return this._opacity;
  }
  set opacity(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["opacity"];
    this._session.updateSetProperty(this, prop, value);
    this._opacity = value;
  }
  _opacity: number | null;

  /**
   * View.fill
   */
  /**
   * View.fill
   */
  get fill(): Fill | null {
    return this._fill;
  }
  set fill(value: Fill | null) {
    const prop = (this.constructor as NodeClass).__properties__["fill"];
    this._session.updateSetProperty(this, prop, value);
    this._fill = value;
  }
  _fill: Fill | null;

  /**
   * View.shadow
   */
  /**
   * View.shadow
   */
  get shadow(): Shadow | null {
    return this._shadow;
  }
  set shadow(value: Shadow | null) {
    const prop = (this.constructor as NodeClass).__properties__["shadow"];
    this._session.updateSetProperty(this, prop, value);
    this._shadow = value;
  }
  _shadow: Shadow | null;

  /**
   * View.border
   */
  /**
   * View.border
   */
  get border(): Border | null {
    return this._border;
  }
  set border(value: Border | null) {
    const prop = (this.constructor as NodeClass).__properties__["border"];
    this._session.updateSetProperty(this, prop, value);
    this._border = value;
  }
  _border: Border | null;

  /**
   * View.radius
   */
  /**
   * View.radius
   */
  get radius(): Corner2 | null {
    return this._radius;
  }
  set radius(value: Corner2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["radius"];
    this._session.updateSetProperty(this, prop, value);
    this._radius = value;
  }
  _radius: Corner2 | null;

  /**
   * LayoutView.layout
   */
  /**
   * LayoutView.layout
   */
  get layout(): Layout | null {
    return this._layout;
  }
  set layout(value: Layout | null) {
    const prop = (this.constructor as NodeClass).__properties__["layout"];
    this._session.updateSetProperty(this, prop, value);
    this._layout = value;
  }
  _layout: Layout | null;

  /**
   * LayoutView.direction
   */
  /**
   * LayoutView.direction
   */
  get direction(): Direction | null {
    return this._direction;
  }
  set direction(value: Direction | null) {
    const prop = (this.constructor as NodeClass).__properties__["direction"];
    this._session.updateSetProperty(this, prop, value);
    this._direction = value;
  }
  _direction: Direction | null;

  /**
   * LayoutView.distribute
   */
  /**
   * LayoutView.distribute
   */
  get distribute(): Distribute | null {
    return this._distribute;
  }
  set distribute(value: Distribute | null) {
    const prop = (this.constructor as NodeClass).__properties__["distribute"];
    this._session.updateSetProperty(this, prop, value);
    this._distribute = value;
  }
  _distribute: Distribute | null;

  /**
   * LayoutView.align
   */
  /**
   * LayoutView.align
   */
  get align(): Align | null {
    return this._align;
  }
  set align(value: Align | null) {
    const prop = (this.constructor as NodeClass).__properties__["align"];
    this._session.updateSetProperty(this, prop, value);
    this._align = value;
  }
  _align: Align | null;

  /**
   * LayoutView.gap
   */
  /**
   * LayoutView.gap
   */
  get gap(): Axis2 | null {
    return this._gap;
  }
  set gap(value: Axis2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["gap"];
    this._session.updateSetProperty(this, prop, value);
    this._gap = value;
  }
  _gap: Axis2 | null;

  /**
   * LayoutView.padding
   */
  /**
   * LayoutView.padding
   */
  get padding(): Inset2 | null {
    return this._padding;
  }
  set padding(value: Inset2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["padding"];
    this._session.updateSetProperty(this, prop, value);
    this._padding = value;
  }
  _padding: Inset2 | null;

  /**
   * LayoutView.grid
   */
  /**
   * LayoutView.grid
   */
  get grid(): Grid2 | null {
    return this._grid;
  }
  set grid(value: Grid2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid"];
    this._session.updateSetProperty(this, prop, value);
    this._grid = value;
  }
  _grid: Grid2 | null;

  /**
   * LayoutView.gridSpan
   */
  /**
   * LayoutView.gridSpan
   */
  get gridSpan(): GridSpan2 | null {
    return this._gridSpan;
  }
  set gridSpan(value: GridSpan2 | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid_span"];
    this._session.updateSetProperty(this, prop, value);
    this._gridSpan = value;
  }
  _gridSpan: GridSpan2 | null;

  /**
   * LayoutView.aspectRatio
   */
  /**
   * LayoutView.aspectRatio
   */
  get aspectRatio(): number | null {
    return this._aspectRatio;
  }
  set aspectRatio(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["aspect_ratio"];
    this._session.updateSetProperty(this, prop, value);
    this._aspectRatio = value;
  }
  _aspectRatio: number | null;

  /**
   * LayoutView.isWrap
   */
  /**
   * LayoutView.isWrap
   */
  get isWrap(): boolean | null {
    return this._isWrap;
  }
  set isWrap(value: boolean | null) {
    const prop = (this.constructor as NodeClass).__properties__["is_wrap"];
    this._session.updateSetProperty(this, prop, value);
    this._isWrap = value;
  }
  _isWrap: boolean | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: SplitView | NodeReference | null;
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
    position?: Vector2 | null;
    offset?: Offset2 | null;
    scale?: Vector2 | null;
    rotation?: Vector2 | null;
    skew?: Vector2 | null;
    origin?: Vector2 | null;
    anchor?: Anchor | null;
    width?: Length | null;
    height?: Length | null;
    minWidth?: Length | null;
    minHeight?: Length | null;
    maxWidth?: Length | null;
    maxHeight?: Length | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    fill?: Fill | null;
    shadow?: Shadow | null;
    border?: Border | null;
    radius?: Corner2 | null;
    layout?: Layout | null;
    direction?: Direction | null;
    distribute?: Distribute | null;
    align?: Align | null;
    gap?: Axis2 | null;
    padding?: Inset2 | null;
    grid?: Grid2 | null;
    gridSpan?: GridSpan2 | null;
    aspectRatio?: number | null;
    isWrap?: boolean | null;
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
        throw new Error(`no active Space for SplitView`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`SplitView.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`SplitView.materialization is required`);
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
        throw new Error(`no active Branch for SplitView`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`SplitView.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for SplitView`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`SplitView.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.metatype != StructType.NODE_REFERENCE) {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "SplitView";
    }
    if (_name === null) {
      throw new Error(`SplitView.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`SplitView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _key = options.key ?? null;
    this._key = _key;
    let _position = options.position ?? null;
    this._position = _position;
    let _offset = options.offset ?? null;
    this._offset = _offset;
    let _scale = options.scale ?? null;
    this._scale = _scale;
    let _rotation = options.rotation ?? null;
    this._rotation = _rotation;
    let _skew = options.skew ?? null;
    this._skew = _skew;
    let _origin = options.origin ?? null;
    this._origin = _origin;
    let _anchor = options.anchor ?? null;
    this._anchor = _anchor;
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
    let _isVisible = options.isVisible ?? null;
    this._isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this._opacity = _opacity;
    let _fill = options.fill ?? null;
    this._fill = _fill;
    let _shadow = options.shadow ?? null;
    this._shadow = _shadow;
    let _border = options.border ?? null;
    this._border = _border;
    let _radius = options.radius ?? null;
    this._radius = _radius;
    let _layout = options.layout ?? null;
    this._layout = _layout;
    let _direction = options.direction ?? null;
    this._direction = _direction;
    let _distribute = options.distribute ?? null;
    this._distribute = _distribute;
    let _align = options.align ?? null;
    this._align = _align;
    let _gap = options.gap ?? null;
    this._gap = _gap;
    let _padding = options.padding ?? null;
    this._padding = _padding;
    let _grid = options.grid ?? null;
    this._grid = _grid;
    let _gridSpan = options.gridSpan ?? null;
    this._gridSpan = _gridSpan;
    let _aspectRatio = options.aspectRatio ?? null;
    this._aspectRatio = _aspectRatio;
    let _isWrap = options.isWrap ?? null;
    this._isWrap = _isWrap;

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
          `SplitView.createdAt and SplitView.updatedAt are required for existing Nodes`,
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
    if (!(this._layout === other._layout)) {
      return false;
    }
    if (!(this._direction === other._direction)) {
      return false;
    }
    if (!(this._distribute === other._distribute)) {
      return false;
    }
    if (!(this._align === other._align)) {
      return false;
    }
    if (
      (this._gap == null) !== (other._gap == null) ||
      (this._gap != null && !this._gap.equals(other._gap))
    ) {
      return false;
    }
    if (
      (this._padding == null) !== (other._padding == null) ||
      (this._padding != null && !this._padding.equals(other._padding))
    ) {
      return false;
    }
    if (
      (this._grid == null) !== (other._grid == null) ||
      (this._grid != null && !this._grid.equals(other._grid))
    ) {
      return false;
    }
    if (
      (this._gridSpan == null) !== (other._gridSpan == null) ||
      (this._gridSpan != null && !this._gridSpan.equals(other._gridSpan))
    ) {
      return false;
    }
    if (
      (this._aspectRatio == null) !== (other._aspectRatio == null) ||
      (this._aspectRatio != null &&
        !(
          this._aspectRatio === other._aspectRatio ||
          Math.abs(this._aspectRatio - other._aspectRatio) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this._isWrap === other._isWrap)) {
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
    if (!(this._isVisible === other._isVisible)) {
      return false;
    }
    if (
      (this._opacity == null) !== (other._opacity == null) ||
      (this._opacity != null &&
        !(this._opacity === other._opacity || Math.abs(this._opacity - other._opacity) < 1e-10))
    ) {
      return false;
    }
    if (
      (this._fill == null) !== (other._fill == null) ||
      (this._fill != null && !this._fill.equals(other._fill))
    ) {
      return false;
    }
    if (
      (this._shadow == null) !== (other._shadow == null) ||
      (this._shadow != null && !this._shadow.equals(other._shadow))
    ) {
      return false;
    }
    if (
      (this._border == null) !== (other._border == null) ||
      (this._border != null && !this._border.equals(other._border))
    ) {
      return false;
    }
    if (
      (this._radius == null) !== (other._radius == null) ||
      (this._radius != null && !this._radius.equals(other._radius))
    ) {
      return false;
    }
    if (
      (this._position == null) !== (other._position == null) ||
      (this._position != null && !this._position.equals(other._position))
    ) {
      return false;
    }
    if (
      (this._offset == null) !== (other._offset == null) ||
      (this._offset != null && !this._offset.equals(other._offset))
    ) {
      return false;
    }
    if (
      (this._scale == null) !== (other._scale == null) ||
      (this._scale != null && !this._scale.equals(other._scale))
    ) {
      return false;
    }
    if (
      (this._rotation == null) !== (other._rotation == null) ||
      (this._rotation != null && !this._rotation.equals(other._rotation))
    ) {
      return false;
    }
    if (
      (this._skew == null) !== (other._skew == null) ||
      (this._skew != null && !this._skew.equals(other._skew))
    ) {
      return false;
    }
    if (
      (this._origin == null) !== (other._origin == null) ||
      (this._origin != null && !this._origin.equals(other._origin))
    ) {
      return false;
    }
    if (!(this._anchor === other._anchor)) {
      return false;
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
    if (this._layout != null) {
      h = (h * 31 + this._layout) & 0xffffffff;
    }
    if (this._direction != null) {
      h = (h * 31 + this._direction) & 0xffffffff;
    }
    if (this._distribute != null) {
      h = (h * 31 + this._distribute) & 0xffffffff;
    }
    if (this._align != null) {
      h = (h * 31 + this._align) & 0xffffffff;
    }
    if (this._gap != null) {
      h = (h * 31 + this._gap.hash()) & 0xffffffff;
    }
    if (this._padding != null) {
      h = (h * 31 + this._padding.hash()) & 0xffffffff;
    }
    if (this._grid != null) {
      h = (h * 31 + this._grid.hash()) & 0xffffffff;
    }
    if (this._gridSpan != null) {
      h = (h * 31 + this._gridSpan.hash()) & 0xffffffff;
    }
    if (this._aspectRatio != null) {
      h = (h * 31 + hashFloat(this._aspectRatio)) & 0xffffffff;
    }
    if (this._isWrap != null) {
      h = (h * 31 + hashBool(this._isWrap)) & 0xffffffff;
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
    if (this._isVisible != null) {
      h = (h * 31 + hashBool(this._isVisible)) & 0xffffffff;
    }
    if (this._opacity != null) {
      h = (h * 31 + hashFloat(this._opacity)) & 0xffffffff;
    }
    if (this._fill != null) {
      h = (h * 31 + this._fill.hash()) & 0xffffffff;
    }
    if (this._shadow != null) {
      h = (h * 31 + this._shadow.hash()) & 0xffffffff;
    }
    if (this._border != null) {
      h = (h * 31 + this._border.hash()) & 0xffffffff;
    }
    if (this._radius != null) {
      h = (h * 31 + this._radius.hash()) & 0xffffffff;
    }
    if (this._position != null) {
      h = (h * 31 + this._position.hash()) & 0xffffffff;
    }
    if (this._offset != null) {
      h = (h * 31 + this._offset.hash()) & 0xffffffff;
    }
    if (this._scale != null) {
      h = (h * 31 + this._scale.hash()) & 0xffffffff;
    }
    if (this._rotation != null) {
      h = (h * 31 + this._rotation.hash()) & 0xffffffff;
    }
    if (this._skew != null) {
      h = (h * 31 + this._skew.hash()) & 0xffffffff;
    }
    if (this._origin != null) {
      h = (h * 31 + this._origin.hash()) & 0xffffffff;
    }
    if (this._anchor != null) {
      h = (h * 31 + this._anchor) & 0xffffffff;
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
      type: NodeType.SPLIT_VIEW,
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
    return `<SplitView "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return SplitView.__packCson__(this);
  }

  static __packCson__(object: SplitView): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1800400;
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
    if (object._position != null) {
      objectCson["110"] = object._position.toCson();
    }
    if (object._offset != null) {
      objectCson["111"] = object._offset.toCson();
    }
    if (object._scale != null) {
      objectCson["112"] = object._scale.toCson();
    }
    if (object._rotation != null) {
      objectCson["113"] = object._rotation.toCson();
    }
    if (object._skew != null) {
      objectCson["114"] = object._skew.toCson();
    }
    if (object._origin != null) {
      objectCson["115"] = object._origin.toCson();
    }
    if (object._anchor != null) {
      objectCson["116"] = object._anchor;
    }
    if (object._width != null) {
      objectCson["120"] = object._width.toCson();
    }
    if (object._height != null) {
      objectCson["121"] = object._height.toCson();
    }
    if (object._minWidth != null) {
      objectCson["122"] = object._minWidth.toCson();
    }
    if (object._minHeight != null) {
      objectCson["123"] = object._minHeight.toCson();
    }
    if (object._maxWidth != null) {
      objectCson["124"] = object._maxWidth.toCson();
    }
    if (object._maxHeight != null) {
      objectCson["125"] = object._maxHeight.toCson();
    }
    if (object._isVisible != null) {
      objectCson["130"] = object._isVisible;
    }
    if (object._opacity != null) {
      objectCson["131"] = object._opacity;
    }
    if (object._fill != null) {
      objectCson["140"] = object._fill.toCson();
    }
    if (object._shadow != null) {
      objectCson["141"] = object._shadow.toCson();
    }
    if (object._border != null) {
      objectCson["142"] = object._border.toCson();
    }
    if (object._radius != null) {
      objectCson["143"] = object._radius.toCson();
    }
    if (object._layout != null) {
      objectCson["150"] = object._layout;
    }
    if (object._direction != null) {
      objectCson["151"] = object._direction;
    }
    if (object._distribute != null) {
      objectCson["152"] = object._distribute;
    }
    if (object._align != null) {
      objectCson["153"] = object._align;
    }
    if (object._gap != null) {
      objectCson["154"] = object._gap.toCson();
    }
    if (object._padding != null) {
      objectCson["155"] = object._padding.toCson();
    }
    if (object._grid != null) {
      objectCson["156"] = object._grid.toCson();
    }
    if (object._gridSpan != null) {
      objectCson["157"] = object._gridSpan.toCson();
    }
    if (object._aspectRatio != null) {
      objectCson["158"] = object._aspectRatio;
    }
    if (object._isWrap != null) {
      objectCson["159"] = object._isWrap;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): SplitView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Offset2 = STRUCT_CLASS_BY_TYPE[StructType.OFFSET2] as typeof Offset2;
    const _Grid2 = STRUCT_CLASS_BY_TYPE[StructType.GRID2] as typeof Grid2;
    const _GridSpan2 = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN2] as typeof GridSpan2;
    const _Inset2 = STRUCT_CLASS_BY_TYPE[StructType.INSET2] as typeof Inset2;
    const _Corner2 = STRUCT_CLASS_BY_TYPE[StructType.CORNER2] as typeof Corner2;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const layoutValue = objectCson["150"];
    const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : null;
    const directionValue = objectCson["151"];
    const unpackedDirection = directionValue != undefined ? Number(directionValue) : null;
    const distributeValue = objectCson["152"];
    const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : null;
    const alignValue = objectCson["153"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const gapValue = objectCson["154"];
    const unpackedGap =
      gapValue != undefined
        ? _Axis2.fromCson(gapValue, _session, _supergraph, _graph, _connection)
        : null;
    const paddingValue = objectCson["155"];
    const unpackedPadding =
      paddingValue != undefined
        ? _Inset2.fromCson(paddingValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridValue = objectCson["156"];
    const unpackedGrid =
      gridValue != undefined
        ? _Grid2.fromCson(gridValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridSpanValue = objectCson["157"];
    const unpackedGridSpan =
      gridSpanValue != undefined
        ? _GridSpan2.fromCson(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectCson["158"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const isWrapValue = objectCson["159"];
    const unpackedIsWrap = isWrapValue != undefined ? isWrapValue : null;
    const widthValue = objectCson["120"];
    const unpackedWidth =
      widthValue != undefined
        ? _Length.fromCson(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectCson["121"];
    const unpackedHeight =
      heightValue != undefined
        ? _Length.fromCson(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectCson["122"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? _Length.fromCson(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectCson["123"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? _Length.fromCson(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectCson["124"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? _Length.fromCson(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectCson["125"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? _Length.fromCson(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const isVisibleValue = objectCson["130"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectCson["131"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const fillValue = objectCson["140"];
    const unpackedFill =
      fillValue != undefined
        ? _Fill.fromCson(fillValue, _session, _supergraph, _graph, _connection)
        : null;
    const shadowValue = objectCson["141"];
    const unpackedShadow =
      shadowValue != undefined
        ? _Shadow.fromCson(shadowValue, _session, _supergraph, _graph, _connection)
        : null;
    const borderValue = objectCson["142"];
    const unpackedBorder =
      borderValue != undefined
        ? _Border.fromCson(borderValue, _session, _supergraph, _graph, _connection)
        : null;
    const radiusValue = objectCson["143"];
    const unpackedRadius =
      radiusValue != undefined
        ? _Corner2.fromCson(radiusValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectCson["110"];
    const unpackedPosition =
      positionValue != undefined
        ? _Vector2.fromCson(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const offsetValue = objectCson["111"];
    const unpackedOffset =
      offsetValue != undefined
        ? _Offset2.fromCson(offsetValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectCson["112"];
    const unpackedScale =
      scaleValue != undefined
        ? _Vector2.fromCson(scaleValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectCson["113"];
    const unpackedRotation =
      rotationValue != undefined
        ? _Vector2.fromCson(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectCson["114"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2.fromCson(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const originValue = objectCson["115"];
    const unpackedOrigin =
      originValue != undefined
        ? _Vector2.fromCson(originValue, _session, _supergraph, _graph, _connection)
        : null;
    const anchorValue = objectCson["116"];
    const unpackedAnchor = anchorValue != undefined ? Number(anchorValue) : null;
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
    return new SplitView({
      layout: unpackedLayout,
      direction: unpackedDirection,
      distribute: unpackedDistribute,
      align: unpackedAlign,
      gap: unpackedGap,
      padding: unpackedPadding,
      grid: unpackedGrid,
      gridSpan: unpackedGridSpan,
      aspectRatio: unpackedAspectRatio,
      isWrap: unpackedIsWrap,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      isVisible: unpackedIsVisible,
      opacity: unpackedOpacity,
      fill: unpackedFill,
      shadow: unpackedShadow,
      border: unpackedBorder,
      radius: unpackedRadius,
      position: unpackedPosition,
      offset: unpackedOffset,
      scale: unpackedScale,
      rotation: unpackedRotation,
      skew: unpackedSkew,
      origin: unpackedOrigin,
      anchor: unpackedAnchor,
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
  ): SplitView {
    return SplitView.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): SplitViewProto {
    return SplitView.__packProto__(this);
  }

  static __packProto__(object: SplitView): SplitViewProto {
    const objectProto: Partial<SplitViewProto> = { metatype: 1800400 };
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
    if (object._position != null) {
      objectProto.position = object._position.toProto();
    }
    if (object._offset != null) {
      objectProto.offset = object._offset.toProto();
    }
    if (object._scale != null) {
      objectProto.scale = object._scale.toProto();
    }
    if (object._rotation != null) {
      objectProto.rotation = object._rotation.toProto();
    }
    if (object._skew != null) {
      objectProto.skew = object._skew.toProto();
    }
    if (object._origin != null) {
      objectProto.origin = object._origin.toProto();
    }
    if (object._anchor != null) {
      objectProto.anchor = Number(object._anchor) as AnchorProto;
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
    if (object._isVisible != null) {
      objectProto.isVisible = object._isVisible;
    }
    if (object._opacity != null) {
      objectProto.opacity = object._opacity;
    }
    if (object._fill != null) {
      objectProto.fill = object._fill.toProto();
    }
    if (object._shadow != null) {
      objectProto.shadow = object._shadow.toProto();
    }
    if (object._border != null) {
      objectProto.border = object._border.toProto();
    }
    if (object._radius != null) {
      objectProto.radius = object._radius.toProto();
    }
    if (object._layout != null) {
      objectProto.layout = Number(object._layout) as LayoutProto;
    }
    if (object._direction != null) {
      objectProto.direction = Number(object._direction) as DirectionProto;
    }
    if (object._distribute != null) {
      objectProto.distribute = Number(object._distribute) as DistributeProto;
    }
    if (object._align != null) {
      objectProto.align = Number(object._align) as AlignProto;
    }
    if (object._gap != null) {
      objectProto.gap = object._gap.toProto();
    }
    if (object._padding != null) {
      objectProto.padding = object._padding.toProto();
    }
    if (object._grid != null) {
      objectProto.grid = object._grid.toProto();
    }
    if (object._gridSpan != null) {
      objectProto.gridSpan = object._gridSpan.toProto();
    }
    if (object._aspectRatio != null) {
      objectProto.aspectRatio = object._aspectRatio;
    }
    if (object._isWrap != null) {
      objectProto.isWrap = object._isWrap;
    }
    return objectProto as SplitViewProto;
  }

  static __unpackProto__(
    objectProto: SplitViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): SplitView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Length = STRUCT_CLASS_BY_TYPE[StructType.LENGTH] as typeof Length;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Vector2 = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2] as typeof Vector2;
    const _Offset2 = STRUCT_CLASS_BY_TYPE[StructType.OFFSET2] as typeof Offset2;
    const _Grid2 = STRUCT_CLASS_BY_TYPE[StructType.GRID2] as typeof Grid2;
    const _GridSpan2 = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN2] as typeof GridSpan2;
    const _Inset2 = STRUCT_CLASS_BY_TYPE[StructType.INSET2] as typeof Inset2;
    const _Corner2 = STRUCT_CLASS_BY_TYPE[StructType.CORNER2] as typeof Corner2;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new SplitView({
      layout: objectProto.layout != undefined ? (Number(objectProto.layout) as Layout) : null,
      direction:
        objectProto.direction != undefined ? (Number(objectProto.direction) as Direction) : null,
      distribute:
        objectProto.distribute != undefined ? (Number(objectProto.distribute) as Distribute) : null,
      align: objectProto.align != undefined ? (Number(objectProto.align) as Align) : null,
      gap:
        objectProto.gap != undefined
          ? _Axis2.fromProto(objectProto.gap!, _session, _supergraph, _graph, _connection)
          : null,
      padding:
        objectProto.padding != undefined
          ? _Inset2.fromProto(objectProto.padding!, _session, _supergraph, _graph, _connection)
          : null,
      grid:
        objectProto.grid != undefined
          ? _Grid2.fromProto(objectProto.grid!, _session, _supergraph, _graph, _connection)
          : null,
      gridSpan:
        objectProto.gridSpan != undefined
          ? _GridSpan2.fromProto(objectProto.gridSpan!, _session, _supergraph, _graph, _connection)
          : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
      width:
        objectProto.width != undefined
          ? _Length.fromProto(objectProto.width!, _session, _supergraph, _graph, _connection)
          : null,
      height:
        objectProto.height != undefined
          ? _Length.fromProto(objectProto.height!, _session, _supergraph, _graph, _connection)
          : null,
      minWidth:
        objectProto.minWidth != undefined
          ? _Length.fromProto(objectProto.minWidth!, _session, _supergraph, _graph, _connection)
          : null,
      minHeight:
        objectProto.minHeight != undefined
          ? _Length.fromProto(objectProto.minHeight!, _session, _supergraph, _graph, _connection)
          : null,
      maxWidth:
        objectProto.maxWidth != undefined
          ? _Length.fromProto(objectProto.maxWidth!, _session, _supergraph, _graph, _connection)
          : null,
      maxHeight:
        objectProto.maxHeight != undefined
          ? _Length.fromProto(objectProto.maxHeight!, _session, _supergraph, _graph, _connection)
          : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      fill:
        objectProto.fill != undefined
          ? _Fill.fromProto(objectProto.fill!, _session, _supergraph, _graph, _connection)
          : null,
      shadow:
        objectProto.shadow != undefined
          ? _Shadow.fromProto(objectProto.shadow!, _session, _supergraph, _graph, _connection)
          : null,
      border:
        objectProto.border != undefined
          ? _Border.fromProto(objectProto.border!, _session, _supergraph, _graph, _connection)
          : null,
      radius:
        objectProto.radius != undefined
          ? _Corner2.fromProto(objectProto.radius!, _session, _supergraph, _graph, _connection)
          : null,
      position:
        objectProto.position != undefined
          ? _Vector2.fromProto(objectProto.position!, _session, _supergraph, _graph, _connection)
          : null,
      offset:
        objectProto.offset != undefined
          ? _Offset2.fromProto(objectProto.offset!, _session, _supergraph, _graph, _connection)
          : null,
      scale:
        objectProto.scale != undefined
          ? _Vector2.fromProto(objectProto.scale!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? _Vector2.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      origin:
        objectProto.origin != undefined
          ? _Vector2.fromProto(objectProto.origin!, _session, _supergraph, _graph, _connection)
          : null,
      anchor: objectProto.anchor != undefined ? (Number(objectProto.anchor) as Anchor) : null,
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
    objectProto: SplitViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): SplitView {
    return SplitView.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): SplitView {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = SplitViewProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.SPLIT_VIEW, SplitView);
/* ==== DESTACK_GENERATED_END:NODE:1800400 ==== */
