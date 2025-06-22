import { PlaneShape, Agent, IsTaggable, IsScriptable, BuiltinObject, ACTIVE_SESSION, IsVisual, Session, Scene, View, StructFrozen, IsOrdered, Graph, Struct, AnnotationShape, activeSession, StructType, NodeType, Window, Canvas, FrameView, Dimension, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Position, CustomView, Spatial, LabelView, InputView, Space, CustomViewDefinition, IsDeletable, Entity, SplitView, EnumType, Script, Layer, Node, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:10400 ==== */
export class NumberInputView extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable, IsScriptable, IsVisual, View, InputView {
  readonly id: string;
  get parent(): Window | Scene | Layer | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Window | Scene | Layer | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | AnnotationShape | Canvas | PlaneShape | Layer | Scene | null | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  get space(): Space | null | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference | null
  readonly materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
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
  isVisible: boolean | null;
  opacity: number | null;
  value: string | null;
  placeholder: string | null;
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
  ;
  scriptPtr: NodeReference | null

  constructor(options: {
    name: string,
    position?: Position | null,
    width?: Dimension | null,
    height?: Dimension | null,
    minWidth?: Dimension | null,
    minHeight?: Dimension | null,
    maxWidth?: Dimension | null,
    maxHeight?: Dimension | null,
    isVisible?: boolean | null,
    opacity?: number | null,
    value?: string | null,
    placeholder?: string | null,
    script?: Script | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
    this.position = options.position ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
    this.minWidth = options.minWidth ?? null;
    this.minHeight = options.minHeight ?? null;
    this.maxWidth = options.maxWidth ?? null;
    this.maxHeight = options.maxHeight ?? null;
    this.isVisible = options.isVisible ?? null;
    this.opacity = options.opacity ?? null;
    this.value = options.value ?? null;
    this.placeholder = options.placeholder ?? null;
    this.scriptPtr = options.script != null ? (options.script.metatype == StructType.NODE_REFERENCE ? (options.script as NodeReference) : (options.script as Node).toRef()) : null;
  }

  equals(other: any): boolean {
    throw new Error("Not implemented");
  }

  hash(): number {
    throw new Error("Not implemented");
  }

  validate(): void {
    throw new Error("Not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference(NodeType.NUMBER_INPUT_VIEW, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:10400 ==== */