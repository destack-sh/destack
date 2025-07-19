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
import type { Vector2f } from "@destack/language/geometry";
import type { Script } from "@destack/language/logic";
import { STRUCT_CLASS_BY_TYPE, registerNodeClass } from "@destack/language/registry";
import type { Border, Fill, Shadow } from "@destack/language/style";
import type {
  Axis2,
  Axis3,
  Corners,
  Dimension,
  Grid,
  GridSpan,
  Insets,
  Position,
} from "@destack/language/view/common";
import { Align, Direction, Distribute, Layout } from "@destack/language/view/common";
import { ContainerView } from "@destack/language/view/container";
import {
  AlignProto,
  DirectionProto,
  DistributeProto,
  FrameViewProto,
  LayoutProto,
  MaterializationProto,
} from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashFloat, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:NODE:1800200 ==== */
/**
 * A frame View is a bare ContainerView.
 */
export class FrameView extends ContainerView {
  static metatype: NodeType = NodeType.FRAME_VIEW;

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
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): FrameView | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as FrameView | null;
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

  /**
   * ContainerView.layout
   */
  /**
   * ContainerView.layout
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
   * ContainerView.direction
   */
  /**
   * ContainerView.direction
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
   * ContainerView.distribute
   */
  /**
   * ContainerView.distribute
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
   * ContainerView.align
   */
  /**
   * ContainerView.align
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
   * ContainerView.gap
   */
  /**
   * ContainerView.gap
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
   * ContainerView.padding
   */
  /**
   * ContainerView.padding
   */
  get padding(): Insets | null {
    return this._padding;
  }
  set padding(value: Insets | null) {
    const prop = (this.constructor as NodeClass).__properties__["padding"];
    this._session.updateSetProperty(this, prop, value);
    this._padding = value;
  }
  _padding: Insets | null;

  /**
   * ContainerView.grid
   */
  /**
   * ContainerView.grid
   */
  get grid(): Grid | null {
    return this._grid;
  }
  set grid(value: Grid | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid"];
    this._session.updateSetProperty(this, prop, value);
    this._grid = value;
  }
  _grid: Grid | null;

  /**
   * ContainerView.gridSpan
   */
  /**
   * ContainerView.gridSpan
   */
  get gridSpan(): GridSpan | null {
    return this._gridSpan;
  }
  set gridSpan(value: GridSpan | null) {
    const prop = (this.constructor as NodeClass).__properties__["grid_span"];
    this._session.updateSetProperty(this, prop, value);
    this._gridSpan = value;
  }
  _gridSpan: GridSpan | null;

  /**
   * ContainerView.aspectRatio
   */
  /**
   * ContainerView.aspectRatio
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
   * ContainerView.isWrap
   */
  /**
   * ContainerView.isWrap
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

  /**
   * ContainerView.isVisible
   */
  /**
   * ContainerView.isVisible
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
   * ContainerView.opacity
   */
  /**
   * ContainerView.opacity
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
   * ContainerView.fill
   */
  /**
   * ContainerView.fill
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
   * ContainerView.rotation
   */
  /**
   * ContainerView.rotation
   */
  get rotation(): Axis3 | null {
    return this._rotation;
  }
  set rotation(value: Axis3 | null) {
    const prop = (this.constructor as NodeClass).__properties__["rotation"];
    this._session.updateSetProperty(this, prop, value);
    this._rotation = value;
  }
  _rotation: Axis3 | null;

  /**
   * ContainerView.skew
   */
  /**
   * ContainerView.skew
   */
  get skew(): Vector2f | null {
    return this._skew;
  }
  set skew(value: Vector2f | null) {
    const prop = (this.constructor as NodeClass).__properties__["skew"];
    this._session.updateSetProperty(this, prop, value);
    this._skew = value;
  }
  _skew: Vector2f | null;

  /**
   * ContainerView.scale
   */
  /**
   * ContainerView.scale
   */
  get scale(): number | null {
    return this._scale;
  }
  set scale(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["scale"];
    this._session.updateSetProperty(this, prop, value);
    this._scale = value;
  }
  _scale: number | null;

  /**
   * ContainerView.shadow
   */
  /**
   * ContainerView.shadow
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
   * ContainerView.border
   */
  /**
   * ContainerView.border
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
   * ContainerView.radius
   */
  /**
   * ContainerView.radius
   */
  get radius(): Corners | null {
    return this._radius;
  }
  set radius(value: Corners | null) {
    const prop = (this.constructor as NodeClass).__properties__["radius"];
    this._session.updateSetProperty(this, prop, value);
    this._radius = value;
  }
  _radius: Corners | null;

  constructor(options: {
    id?: string;
    parent?: Entity | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: FrameView | NodeReference | null;
    instance?: Entity | NodeReference | null;
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
    layout?: Layout | null;
    direction?: Direction | null;
    distribute?: Distribute | null;
    align?: Align | null;
    gap?: Axis2 | null;
    padding?: Insets | null;
    grid?: Grid | null;
    gridSpan?: GridSpan | null;
    aspectRatio?: number | null;
    isWrap?: boolean | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    fill?: Fill | null;
    rotation?: Axis3 | null;
    skew?: Vector2f | null;
    scale?: number | null;
    shadow?: Shadow | null;
    border?: Border | null;
    radius?: Corners | null;
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
        throw new Error(`no active Space for FrameView`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`FrameView.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`FrameView.materialization is required`);
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
        throw new Error(`no active Branch for FrameView`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`FrameView.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for FrameView`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`FrameView.snapshot is required`);
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
      throw new Error(`FrameView.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "FrameView";
    }
    if (_name === null) {
      throw new Error(`FrameView.name is required`);
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
      throw new Error(`FrameView.isExtensible is required`);
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
    let _isVisible = options.isVisible ?? null;
    this._isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this._opacity = _opacity;
    let _fill = options.fill ?? null;
    this._fill = _fill;
    let _rotation = options.rotation ?? null;
    this._rotation = _rotation;
    let _skew = options.skew ?? null;
    this._skew = _skew;
    let _scale = options.scale ?? null;
    this._scale = _scale;
    let _shadow = options.shadow ?? null;
    this._shadow = _shadow;
    let _border = options.border ?? null;
    this._border = _border;
    let _radius = options.radius ?? null;
    this._radius = _radius;

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
          `FrameView.createdAt and FrameView.updatedAt are required for existing Nodes`,
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
      (this._scale == null) !== (other._scale == null) ||
      (this._scale != null &&
        !(this._scale === other._scale || Math.abs(this._scale - other._scale) < 1e-10))
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
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
    if (this._isVisible != null) {
      h = (h * 31 + hashBool(this._isVisible)) & 0xffffffff;
    }
    if (this._opacity != null) {
      h = (h * 31 + hashFloat(this._opacity)) & 0xffffffff;
    }
    if (this._fill != null) {
      h = (h * 31 + this._fill.hash()) & 0xffffffff;
    }
    if (this._rotation != null) {
      h = (h * 31 + this._rotation.hash()) & 0xffffffff;
    }
    if (this._skew != null) {
      h = (h * 31 + this._skew.hash()) & 0xffffffff;
    }
    if (this._scale != null) {
      h = (h * 31 + hashFloat(this._scale)) & 0xffffffff;
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
      type: NodeType.FRAME_VIEW,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
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
    return `<FrameView "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return FrameView.__packCson__(this);
  }

  static __packCson__(object: FrameView): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 1800200;
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
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["30"] = packedCustomValues;
    }
    objectCson["31"] = object.orderKey;
    objectCson["50"] = object._name;
    if (object.sourcePtr != null) {
      objectCson["60"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["70"] = object._key;
    }
    if (object._scriptPtr != null) {
      objectCson["80"] = object._scriptPtr.toCson();
    }
    objectCson["90"] = object.isExtensible;
    if (object._position != null) {
      objectCson["110"] = object._position.toCson();
    }
    if (object._width != null) {
      objectCson["111"] = object._width.toCson();
    }
    if (object._height != null) {
      objectCson["112"] = object._height.toCson();
    }
    if (object._minWidth != null) {
      objectCson["113"] = object._minWidth.toCson();
    }
    if (object._minHeight != null) {
      objectCson["114"] = object._minHeight.toCson();
    }
    if (object._maxWidth != null) {
      objectCson["115"] = object._maxWidth.toCson();
    }
    if (object._maxHeight != null) {
      objectCson["116"] = object._maxHeight.toCson();
    }
    if (object._layout != null) {
      objectCson["120"] = object._layout;
    }
    if (object._direction != null) {
      objectCson["121"] = object._direction;
    }
    if (object._distribute != null) {
      objectCson["122"] = object._distribute;
    }
    if (object._align != null) {
      objectCson["123"] = object._align;
    }
    if (object._gap != null) {
      objectCson["124"] = object._gap.toCson();
    }
    if (object._padding != null) {
      objectCson["125"] = object._padding.toCson();
    }
    if (object._grid != null) {
      objectCson["126"] = object._grid.toCson();
    }
    if (object._gridSpan != null) {
      objectCson["127"] = object._gridSpan.toCson();
    }
    if (object._aspectRatio != null) {
      objectCson["128"] = object._aspectRatio;
    }
    if (object._isWrap != null) {
      objectCson["129"] = object._isWrap;
    }
    if (object._isVisible != null) {
      objectCson["140"] = object._isVisible;
    }
    if (object._opacity != null) {
      objectCson["141"] = object._opacity;
    }
    if (object._fill != null) {
      objectCson["142"] = object._fill.toCson();
    }
    if (object._rotation != null) {
      objectCson["143"] = object._rotation.toCson();
    }
    if (object._skew != null) {
      objectCson["144"] = object._skew.toCson();
    }
    if (object._scale != null) {
      objectCson["145"] = object._scale;
    }
    if (object._shadow != null) {
      objectCson["146"] = object._shadow.toCson();
    }
    if (object._border != null) {
      objectCson["147"] = object._border.toCson();
    }
    if (object._radius != null) {
      objectCson["148"] = object._radius.toCson();
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FrameView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const _Grid = STRUCT_CLASS_BY_TYPE[StructType.GRID] as typeof Grid;
    const _GridSpan = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN] as typeof GridSpan;
    const _Insets = STRUCT_CLASS_BY_TYPE[StructType.INSETS] as typeof Insets;
    const _Corners = STRUCT_CLASS_BY_TYPE[StructType.CORNERS] as typeof Corners;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    const layoutValue = objectCson["120"];
    const unpackedLayout = layoutValue != undefined ? Number(layoutValue) : null;
    const directionValue = objectCson["121"];
    const unpackedDirection = directionValue != undefined ? Number(directionValue) : null;
    const distributeValue = objectCson["122"];
    const unpackedDistribute = distributeValue != undefined ? Number(distributeValue) : null;
    const alignValue = objectCson["123"];
    const unpackedAlign = alignValue != undefined ? Number(alignValue) : null;
    const gapValue = objectCson["124"];
    const unpackedGap =
      gapValue != undefined
        ? _Axis2.fromCson(gapValue, _session, _supergraph, _graph, _connection)
        : null;
    const paddingValue = objectCson["125"];
    const unpackedPadding =
      paddingValue != undefined
        ? _Insets.fromCson(paddingValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridValue = objectCson["126"];
    const unpackedGrid =
      gridValue != undefined
        ? _Grid.fromCson(gridValue, _session, _supergraph, _graph, _connection)
        : null;
    const gridSpanValue = objectCson["127"];
    const unpackedGridSpan =
      gridSpanValue != undefined
        ? _GridSpan.fromCson(gridSpanValue, _session, _supergraph, _graph, _connection)
        : null;
    const aspectRatioValue = objectCson["128"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const isWrapValue = objectCson["129"];
    const unpackedIsWrap = isWrapValue != undefined ? isWrapValue : null;
    const isVisibleValue = objectCson["140"];
    const unpackedIsVisible = isVisibleValue != undefined ? isVisibleValue : null;
    const opacityValue = objectCson["141"];
    const unpackedOpacity = opacityValue != undefined ? opacityValue : null;
    const fillValue = objectCson["142"];
    const unpackedFill =
      fillValue != undefined
        ? _Fill.fromCson(fillValue, _session, _supergraph, _graph, _connection)
        : null;
    const rotationValue = objectCson["143"];
    const unpackedRotation =
      rotationValue != undefined
        ? _Axis3.fromCson(rotationValue, _session, _supergraph, _graph, _connection)
        : null;
    const skewValue = objectCson["144"];
    const unpackedSkew =
      skewValue != undefined
        ? _Vector2f.fromCson(skewValue, _session, _supergraph, _graph, _connection)
        : null;
    const scaleValue = objectCson["145"];
    const unpackedScale = scaleValue != undefined ? scaleValue : null;
    const shadowValue = objectCson["146"];
    const unpackedShadow =
      shadowValue != undefined
        ? _Shadow.fromCson(shadowValue, _session, _supergraph, _graph, _connection)
        : null;
    const borderValue = objectCson["147"];
    const unpackedBorder =
      borderValue != undefined
        ? _Border.fromCson(borderValue, _session, _supergraph, _graph, _connection)
        : null;
    const radiusValue = objectCson["148"];
    const unpackedRadius =
      radiusValue != undefined
        ? _Corners.fromCson(radiusValue, _session, _supergraph, _graph, _connection)
        : null;
    const positionValue = objectCson["110"];
    const unpackedPosition =
      positionValue != undefined
        ? _Position.fromCson(positionValue, _session, _supergraph, _graph, _connection)
        : null;
    const widthValue = objectCson["111"];
    const unpackedWidth =
      widthValue != undefined
        ? _Dimension.fromCson(widthValue, _session, _supergraph, _graph, _connection)
        : null;
    const heightValue = objectCson["112"];
    const unpackedHeight =
      heightValue != undefined
        ? _Dimension.fromCson(heightValue, _session, _supergraph, _graph, _connection)
        : null;
    const minWidthValue = objectCson["113"];
    const unpackedMinWidth =
      minWidthValue != undefined
        ? _Dimension.fromCson(minWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const minHeightValue = objectCson["114"];
    const unpackedMinHeight =
      minHeightValue != undefined
        ? _Dimension.fromCson(minHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxWidthValue = objectCson["115"];
    const unpackedMaxWidth =
      maxWidthValue != undefined
        ? _Dimension.fromCson(maxWidthValue, _session, _supergraph, _graph, _connection)
        : null;
    const maxHeightValue = objectCson["116"];
    const unpackedMaxHeight =
      maxHeightValue != undefined
        ? _Dimension.fromCson(maxHeightValue, _session, _supergraph, _graph, _connection)
        : null;
    const sourcePtrValue = objectCson["60"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["70"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
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
    const scriptPtrValue = objectCson["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["30"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["30"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    return new FrameView({
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
      isVisible: unpackedIsVisible,
      opacity: unpackedOpacity,
      fill: unpackedFill,
      rotation: unpackedRotation,
      skew: unpackedSkew,
      scale: unpackedScale,
      shadow: unpackedShadow,
      border: unpackedBorder,
      radius: unpackedRadius,
      position: unpackedPosition,
      width: unpackedWidth,
      height: unpackedHeight,
      minWidth: unpackedMinWidth,
      minHeight: unpackedMinHeight,
      maxWidth: unpackedMaxWidth,
      maxHeight: unpackedMaxHeight,
      isExtensible: objectCson["90"],
      source: unpackedSourcePtr,
      key: unpackedKey,
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
      name: objectCson["50"],
      id: String(objectCson["2"]),
      script: unpackedScriptPtr,
      orderKey: objectCson["31"],
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      customValues: unpackedCustomValues,
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
  ): FrameView {
    return FrameView.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
  }

  toProto(): FrameViewProto {
    return FrameView.__packProto__(this);
  }

  static __packProto__(object: FrameView): FrameViewProto {
    const objectProto: Partial<FrameViewProto> = { metatype: 1800200 };
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
    if (object._isVisible != null) {
      objectProto.isVisible = object._isVisible;
    }
    if (object._opacity != null) {
      objectProto.opacity = object._opacity;
    }
    if (object._fill != null) {
      objectProto.fill = object._fill.toProto();
    }
    if (object._rotation != null) {
      objectProto.rotation = object._rotation.toProto();
    }
    if (object._skew != null) {
      objectProto.skew = object._skew.toProto();
    }
    if (object._scale != null) {
      objectProto.scale = object._scale;
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
    return objectProto as FrameViewProto;
  }

  static __unpackProto__(
    objectProto: FrameViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FrameView {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Position = STRUCT_CLASS_BY_TYPE[StructType.POSITION] as typeof Position;
    const _Dimension = STRUCT_CLASS_BY_TYPE[StructType.DIMENSION] as typeof Dimension;
    const _Grid = STRUCT_CLASS_BY_TYPE[StructType.GRID] as typeof Grid;
    const _GridSpan = STRUCT_CLASS_BY_TYPE[StructType.GRID_SPAN] as typeof GridSpan;
    const _Insets = STRUCT_CLASS_BY_TYPE[StructType.INSETS] as typeof Insets;
    const _Corners = STRUCT_CLASS_BY_TYPE[StructType.CORNERS] as typeof Corners;
    const _Axis2 = STRUCT_CLASS_BY_TYPE[StructType.AXIS2] as typeof Axis2;
    const _Axis3 = STRUCT_CLASS_BY_TYPE[StructType.AXIS3] as typeof Axis3;
    const _Fill = STRUCT_CLASS_BY_TYPE[StructType.FILL] as typeof Fill;
    const _Border = STRUCT_CLASS_BY_TYPE[StructType.BORDER] as typeof Border;
    const _Shadow = STRUCT_CLASS_BY_TYPE[StructType.SHADOW] as typeof Shadow;
    const _Vector2f = STRUCT_CLASS_BY_TYPE[StructType.VECTOR2F] as typeof Vector2f;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new FrameView({
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
          ? _Insets.fromProto(objectProto.padding!, _session, _supergraph, _graph, _connection)
          : null,
      grid:
        objectProto.grid != undefined
          ? _Grid.fromProto(objectProto.grid!, _session, _supergraph, _graph, _connection)
          : null,
      gridSpan:
        objectProto.gridSpan != undefined
          ? _GridSpan.fromProto(objectProto.gridSpan!, _session, _supergraph, _graph, _connection)
          : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      isWrap: objectProto.isWrap != undefined ? objectProto.isWrap : null,
      isVisible: objectProto.isVisible != undefined ? objectProto.isVisible : null,
      opacity: objectProto.opacity != undefined ? objectProto.opacity : null,
      fill:
        objectProto.fill != undefined
          ? _Fill.fromProto(objectProto.fill!, _session, _supergraph, _graph, _connection)
          : null,
      rotation:
        objectProto.rotation != undefined
          ? _Axis3.fromProto(objectProto.rotation!, _session, _supergraph, _graph, _connection)
          : null,
      skew:
        objectProto.skew != undefined
          ? _Vector2f.fromProto(objectProto.skew!, _session, _supergraph, _graph, _connection)
          : null,
      scale: objectProto.scale != undefined ? objectProto.scale : null,
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
          ? _Corners.fromProto(objectProto.radius!, _session, _supergraph, _graph, _connection)
          : null,
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
    objectProto: FrameViewProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): FrameView {
    return FrameView.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): FrameView {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FrameViewProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FRAME_VIEW, FrameView);
/* ==== DESTACK_GENERATED_END:NODE:1800200 ==== */
