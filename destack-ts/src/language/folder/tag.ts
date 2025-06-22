import { PlaneShape, CustomEntityDefinition, Agent, IsTaggable, BuiltinObject, ACTIVE_SESSION, Folder, Session, Scene, StructFrozen, LineShape, IsOrdered, GradientStyle, Graph, Service, Struct, Option, AnnotationShape, ShadowStyle, activeSession, StructType, NodeType, Action, Canvas, EffectStyle, FrameView, Icon, Palette, Supergraph, QueryConnection, SliderInputView, NodeReference, MaterializationType, CustomEnumDefinition, WizardView, Thread, FontStyle, User, CustomView, Spatial, LabelView, ThreadView, EditEvent, Message, TransitionStyle, Space, CustomStructDefinition, CustomViewDefinition, NumberInputView, ArrowShape, LikeTag, IsDeletable, Entity, SplitView, EnumType, BorderStyle, Theme, Layer, Route, TextView, ColorStyle, Node, FillStyle, Field, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:1010 ==== */
export class Tag extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, LikeTag {
  readonly id: string;
  get parent(): Folder | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Folder | null | null;
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
  icon: Icon | null;

  constructor(options: {
    name: string,
    icon?: Icon | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.name = options.name;
    this.icon = options.icon ?? null;
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
    return new NodeReference(NodeType.TAG, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:1010 ==== */

/* ==== DESTACK_GENERATED_START:NODE:1011 ==== */
export class Tagging extends Node implements Spatial, Entity, IsTracked, IsDeletable, IsOrdered, IsTaggable {
  readonly id: string;
  get parent(): CustomEntityDefinition | CustomEnumDefinition | EditEvent | Field | Option | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Folder | Tagging | Action | Route | Service | Layer | Scene | Message | Thread | ColorStyle | BorderStyle | TransitionStyle | EffectStyle | GradientStyle | FillStyle | FontStyle | Palette | ShadowStyle | Theme | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | CustomEnumDefinition | EditEvent | Field | Option | CustomStructDefinition | CustomViewDefinition | CustomView | FrameView | LabelView | SplitView | TextView | NumberInputView | SliderInputView | WizardView | ThreadView | AnnotationShape | ArrowShape | Canvas | LineShape | PlaneShape | Folder | Tagging | Action | Route | Service | Layer | Scene | Message | Thread | ColorStyle | BorderStyle | TransitionStyle | EffectStyle | GradientStyle | FillStyle | FontStyle | Palette | ShadowStyle | Theme | null | null;
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
  get tag(): Tag | null | null {
      const nodePtr: NodeReference | null = this.tagPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Tag | null | null;
      }
      return null;
  }

  set tag(node: Tag | null) {
      if (node === null) {
          this.tagPtr = null;
      } else {
          this.tagPtr = node.toRef();
      }
  }
  ;
  tagPtr: NodeReference | null

  constructor(options: {
    tag?: Tag | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.tagPtr = options.tag != null ? (options.tag.metatype == StructType.NODE_REFERENCE ? (options.tag as NodeReference) : (options.tag as Node).toRef()) : null;
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
    return new NodeReference(NodeType.TAGGING, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Tagging[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:1011 ==== */