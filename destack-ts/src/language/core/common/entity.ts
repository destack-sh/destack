import { Script, EnumType, StructFrozen, IsOrdered, Team, IsActionable, Folder, Agent, StructType, QueryConnection, NodeType, IsScriptable, IsCustomNode, Supergraph, Session, TraitType, Node, IsTaggable, IsOwnable, Organization, IsDeletable, IsTracked, Role, Spatial, Struct, MaterializationType, IsSourceable, Graph, User, IsCustomNodeDefinition, Entity, IsExtensible, BuiltinObject, Value, NodeReference, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:NODE:2000 ==== */
export class CustomEntityDefinition extends Node implements Spatial, Entity, IsCustomNodeDefinition, IsTracked, IsDeletable, IsOrdered, IsOwnable, IsTaggable, IsActionable, IsScriptable, IsSourceable {
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
  get prototype(): CustomEntity | null | null {
      const nodePtr: NodeReference | null = this.prototypePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntity | null | null;
      }
      return null;
  }

  set prototype(value: CustomEntity | null) {
      if (value === null) {
          this.prototypePtr = null;
      } else {
          this.prototypePtr = value.toRef();
      }
  }
  ;
  prototypePtr: NodeReference | null
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
  get ownedBy(): Role | Agent | Organization | Team | User | null | null {
      const nodePtr: NodeReference | null = this.ownedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Role | Agent | Organization | Team | User | null | null;
      }
      return null;
  }

  set ownedBy(value: Role | Agent | Organization | Team | User | null) {
      if (value === null) {
          this.ownedByPtr = null;
      } else {
          this.ownedByPtr = value.toRef();
      }
  }
  ;
  ownedByPtr: NodeReference | null
  name: string;
  traits: Array<TraitType>;
  get script(): Script | null | null {
      const nodePtr: NodeReference | null = this.scriptPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }

  set script(value: Script | null) {
      if (value === null) {
          this.scriptPtr = null;
      } else {
          this.scriptPtr = value.toRef();
      }
  }
  ;
  scriptPtr: NodeReference | null
  get source(): Script | null | null {
      const nodePtr: NodeReference | null = this.sourcePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Script | null | null;
      }
      return null;
  }
  ;
  sourcePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    prototypePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    orderKey: string,
    ownedByPtr: NodeReference | null,
    name: string,
    traits: Array<TraitType>,
    scriptPtr: NodeReference | null,
    sourcePtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.prototypePtr = prototypePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.orderKey = orderKey;
    this.ownedByPtr = ownedByPtr;
    this.name = name;
    this.traits = traits;
    this.scriptPtr = scriptPtr;
    this.sourcePtr = sourcePtr;
  }


  static create(options: {
    prototype?: CustomEntity | NodeReference | null,
    ownedBy?: Role | Agent | Organization | Team | User | NodeReference | null,
    name: string,
    traits?: Array<TraitType>,
    script?: Script | NodeReference | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): CustomEntityDefinition {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new CustomEntityDefinition(
      options.prototype != null ? (options.prototype.metatype == StructType.NODE_REFERENCE ? options.prototype : options.prototype.toRef()) : null,
      options.ownedBy != null ? (options.ownedBy.metatype == StructType.NODE_REFERENCE ? options.ownedBy : options.ownedBy.toRef()) : null,
      options.name,
      options.traits ?? [],
      options.script != null ? (options.script.metatype == StructType.NODE_REFERENCE ? options.script : options.script.toRef()) : null,
      session,
      supergraph,
      options._graph,
      options._connection
    );
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
    return new NodeReference(NodeType.CUSTOM_ENTITY_DEFINITION, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
/* ==== DESTACK_GENERATED_END:NODE:2000 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2001 ==== */
export class CustomEntity extends Node implements Spatial, Entity, IsCustomNode, IsTracked, IsDeletable, IsExtensible {
  readonly id: string;
  get parent(): CustomEntityDefinition | CustomEntity | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | CustomEntity | null | null;
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
  get definition(): CustomEntityDefinition | null {
      const nodePtr: NodeReference | null = this.definitionPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as CustomEntityDefinition | null;
      }
      return null;
  }
  ;
  definitionPtr: NodeReference
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
  value: Map<string, Value>;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    definitionPtr: NodeReference,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    deletedAt: Temporal.ZonedDateTime | null,
    value: Map<string, Value>,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.definitionPtr = definitionPtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.deletedAt = deletedAt;
    this.value = value;
  }


  static create(options: {
    value?: Map<string, Value>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): CustomEntity {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new CustomEntity(
      options.value ?? new Map(),
      session,
      supergraph,
      options._graph,
      options._connection
    );
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
    return new NodeReference(NodeType.CUSTOM_ENTITY, this.id, this.spacePtr?.id ?? null, this.definitionPtr?.id ?? null, this._supergraph);
  }

  get _pathKey(): string {
      return "CustomEntity[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:2001 ==== */