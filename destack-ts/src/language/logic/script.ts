import {
  Agent,
  AnnotationShape,
  ArrowShape,
  Canvas,
  CustomEntityDefinition,
  CustomView,
  CustomViewDefinition,
  Entity,
  Folder,
  FrameView,
  Graph,
  IsDeletable,
  IsExtensible,
  IsOrdered,
  IsRunnable,
  IsTracked,
  LabelView,
  Layer,
  LineShape,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  NumberInputView,
  PlaneShape,
  QueryConnection,
  Scene,
  Service,
  Session,
  SliderInputView,
  Space,
  Spatial,
  SplitView,
  StructType,
  Supergraph,
  TextView,
  ThreadView,
  User,
  Value,
  WizardView,
} from "@/language";

/* ==== DESTACK_GENERATED_START:NODE:3000 ==== */
export class Script
  extends Node
  implements Spatial, Entity, IsTracked, IsDeletable, IsExtensible, IsOrdered, IsRunnable
{
  readonly id: string;
  get parent():
    | Folder
    | CustomEntityDefinition
    | CustomViewDefinition
    | CustomView
    | FrameView
    | LabelView
    | SplitView
    | TextView
    | NumberInputView
    | SliderInputView
    | WizardView
    | ThreadView
    | AnnotationShape
    | ArrowShape
    | Canvas
    | LineShape
    | PlaneShape
    | Service
    | Layer
    | Scene
    | Agent
    | Script
    | null
    | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | Folder
        | CustomEntityDefinition
        | CustomViewDefinition
        | CustomView
        | FrameView
        | LabelView
        | SplitView
        | TextView
        | NumberInputView
        | SliderInputView
        | WizardView
        | ThreadView
        | AnnotationShape
        | ArrowShape
        | Canvas
        | LineShape
        | PlaneShape
        | Service
        | Layer
        | Scene
        | Agent
        | Script
        | null
        | null;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
  readonly deletedAt: Temporal.ZonedDateTime | null;
  value: Map<string, Value>;
  readonly orderKey: string;
  name: string;
  code: string | null;

  constructor(options: {
    id: string;
    parent?:
      | Folder
      | CustomEntityDefinition
      | CustomViewDefinition
      | CustomView
      | FrameView
      | LabelView
      | SplitView
      | TextView
      | NumberInputView
      | SliderInputView
      | WizardView
      | ThreadView
      | AnnotationShape
      | ArrowShape
      | Canvas
      | LineShape
      | PlaneShape
      | Service
      | Layer
      | Scene
      | Agent
      | Script
      | NodeReference
      | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    value?: Map<string, Value>;
    orderKey?: string;
    name: string;
    code?: string | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
    _graph?: Graph | null;
    _connection?: QueryConnection | null;
  }) {
    super(
      // id
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.deletedAt = options.deletedAt ?? null;
    this.value = options.value ?? new Map();
    this.orderKey = options.orderKey ?? "a0";
    this.name = options.name;
    this.code = options.code ?? null;
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
      nodeType: NodeType.SCRIPT,
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
/* ==== DESTACK_GENERATED_END:NODE:3000 ==== */
