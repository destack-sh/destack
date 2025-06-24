import { IsShape } from "@destack/language/canvas";
import {
  Align,
  Dimension,
  Entity,
  Graph,
  IsDeletable,
  IsOrdered,
  IsScriptable,
  IsSubject,
  IsTaggable,
  IsTracked,
  IsVisual,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  Position,
  QueryConnection,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
  Vector2,
} from "@destack/language/core";
import { Script } from "@destack/language/logic";
import { Layer, Scene, Window } from "@destack/language/scene";
import { Space } from "@destack/language/space";
import { ContainerView, ContentView, View } from "@destack/language/view";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:11012 ==== */
export enum ArrowHeadType {
  ARROW = 1,
  TRIANGLE = 2,
  DOT = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:11012 ==== */

/* ==== DESTACK_GENERATED_START:NODE:11012 ==== */
export class ArrowShape
  extends Node
  implements
    Spatial,
    Entity,
    IsTracked,
    IsDeletable,
    IsOrdered,
    IsTaggable,
    IsScriptable,
    IsVisual,
    View,
    ContentView,
    IsShape
{
  static metatype: NodeType = NodeType.ARROW_SHAPE;
  static __traits__: TraitType[] = [
    TraitType.SCRIPTABLE,
    TraitType.SPATIAL,
    TraitType.VISUAL,
    TraitType.VIEW,
    TraitType.ENTITY,
    TraitType.TAGGABLE,
    TraitType.SHAPE,
    TraitType.TRACKED,
    TraitType.DELETABLE,
    TraitType.CONTENT_VIEW,
    TraitType.ORDERED,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.CANVAS,
    NodeType.SPLIT_VIEW,
    NodeType.LAYER,
  ];
  static __childTypes__: NodeType[] = [
    NodeType.TAGGING,
    NodeType.SCRIPT,
    NodeType.COLOR_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.SHADOW_STYLE,
  ];
  static __ancestorTypes__: NodeType[] = [
    NodeType.SPACE,
    NodeType.PLANE_SHAPE,
    NodeType.FRAME_VIEW,
    NodeType.ANNOTATION_SHAPE,
    NodeType.WINDOW,
    NodeType.FOLDER,
    NodeType.LABEL_VIEW,
    NodeType.CUSTOM_VIEW_DEFINITION,
    NodeType.CUSTOM_VIEW,
    NodeType.SCENE,
    NodeType.SPLIT_VIEW,
    NodeType.CANVAS,
    NodeType.LAYER,
  ];
  static __descendantTypes__: NodeType[] = [
    NodeType.OPTION,
    NodeType.FIELD,
    NodeType.TAGGING,
    NodeType.COLOR_STYLE,
    NodeType.FILL_STYLE,
    NodeType.FONT_STYLE,
    NodeType.BORDER_STYLE,
    NodeType.SHADOW_STYLE,
    NodeType.GRADIENT_STYLE,
    NodeType.TRANSITION_STYLE,
    NodeType.EFFECT_STYLE,
    NodeType.SCRIPT,
  ];

  get parent(): Window | Scene | Layer | (Node & ContainerView) | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Window | Scene | Layer | (Node & ContainerView) | null | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;
  get space(): Space | null | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): (Node & IsSubject) | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  readonly orderKey: string;
  name: string;
  position: Position | null;
  width: Dimension | null;
  height: Dimension | null;
  minWidth: Dimension | null;
  minHeight: Dimension | null;
  maxWidth: Dimension | null;
  maxHeight: Dimension | null;
  align: Align | null;
  isVisible: boolean | null;
  opacity: number | null;
  startType: ArrowHeadType;
  start: Vector2;
  endType: ArrowHeadType;
  end: Vector2;
  get script(): Script | null | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Script | null | null;
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
  scriptPtr: NodeReference | null;

  constructor(options: {
    id?: string;
    parent?: Window | Scene | Layer | (Node & ContainerView) | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    orderKey?: string;
    name: string;
    position?: Position | null;
    width?: Dimension | null;
    height?: Dimension | null;
    minWidth?: Dimension | null;
    minHeight?: Dimension | null;
    maxWidth?: Dimension | null;
    maxHeight?: Dimension | null;
    align?: Align | null;
    isVisible?: boolean | null;
    opacity?: number | null;
    startType: ArrowHeadType;
    start: Vector2;
    endType: ArrowHeadType;
    end: Vector2;
    script?: Script | NodeReference | null;
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
    if (_parent != null && _parent instanceof Node) {
      _parent = _parent.toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space instanceof Node) {
      _space = _space.toRef();
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = MaterializationType.FULL_GRAPH;
    }
    if (_materialization === null) {
      throw new Error(`ArrowShape.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`ArrowShape.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`ArrowShape.name is required`);
    }
    this.name = _name;
    let _position = options.position ?? null;
    this.position = _position;
    let _width = options.width ?? null;
    this.width = _width;
    let _height = options.height ?? null;
    this.height = _height;
    let _minWidth = options.minWidth ?? null;
    this.minWidth = _minWidth;
    let _minHeight = options.minHeight ?? null;
    this.minHeight = _minHeight;
    let _maxWidth = options.maxWidth ?? null;
    this.maxWidth = _maxWidth;
    let _maxHeight = options.maxHeight ?? null;
    this.maxHeight = _maxHeight;
    let _align = options.align ?? null;
    this.align = _align;
    let _isVisible = options.isVisible ?? null;
    this.isVisible = _isVisible;
    let _opacity = options.opacity ?? null;
    this.opacity = _opacity;
    let _startType = options.startType;
    if (_startType === null) {
      throw new Error(`ArrowShape.startType is required`);
    }
    this.startType = _startType;
    let _start = options.start;
    if (_start === null) {
      throw new Error(`ArrowShape.start is required`);
    }
    this.start = _start;
    let _endType = options.endType;
    if (_endType === null) {
      throw new Error(`ArrowShape.endType is required`);
    }
    this.endType = _endType;
    let _end = options.end;
    if (_end === null) {
      throw new Error(`ArrowShape.end is required`);
    }
    this.end = _end;
    let _script = options.script ?? null;
    if (_script != null && _script instanceof Node) {
      _script = _script.toRef();
    }
    this.scriptPtr = _script;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO();
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy instanceof Node
            ? options.createdBy.toRef()
            : options.createdBy
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedByPtr =
        options.updatedBy != null
          ? options.updatedBy instanceof Node
            ? options.updatedBy.toRef()
            : options.updatedBy
          : null;
    }
  }

  equals(other: any): boolean {
    throw new Error("not implemented");
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.ARROW_SHAPE,
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
}
/* ==== DESTACK_GENERATED_END:NODE:11012 ==== */
