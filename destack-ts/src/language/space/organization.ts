import { Session, Entity, IsJoinable, IsOwner, User, NodeReference, BuiltinObject, Handle, Agent, MaterializationType, IsTracked, Graph, Icon, Struct, Supergraph, Global, QueryConnection, NodeType, Node, Space } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:40 ==== */
export enum OrganizationStatus {
  CREATING = 1,
  ACTIVE = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:40 ==== */

/* ==== DESTACK_GENERATED_START:NODE:40 ==== */
export class Organization extends Node implements Global, Entity, IsTracked, IsJoinable, IsOwner {
  readonly id: string;
  get parent(): Node | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Node | null;
      }
      return null;
  }
  ;
  parentPtr: NodeReference | null
  materialization: MaterializationType;
  readonly createdAt: Temporal.ZonedDateTime;
  get createdBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.createdByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  createdByPtr: NodeReference | null
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null {
      const nodePtr: NodeReference | null = this.updatedByPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Agent | User | null;
      }
      return null;
  }
  ;
  updatedByPtr: NodeReference | null
  name: string;
  slug: string;
  icon: Icon | null;
  readonly status: OrganizationStatus;
  get space(): Space | null {
      const nodePtr: NodeReference | null = this.spacePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null;
      }
      return null;
  }
  ;
  spacePtr: NodeReference
  get handle(): Handle | null {
      const nodePtr: NodeReference | null = this.handlePtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Handle | null;
      }
      return null;
  }
  ;
  handlePtr: NodeReference | null

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    name: string,
    slug: string,
    icon: Icon | null,
    status: OrganizationStatus,
    spacePtr: NodeReference,
    handlePtr: NodeReference | null,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.name = name;
    this.slug = slug;
    this.icon = icon;
    this.status = status;
    this.spacePtr = spacePtr;
    this.handlePtr = handlePtr;
  }


  static create(): Organization {

    return new Organization();
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
    return new NodeReference(NodeType.ORGANIZATION, this.id, null, null, this._supergraph);

  get _pathKey(): string {
      return this.slug or this.name;
  }

  get path(): string {
      return this.slug or this.name;
  }
}
/* ==== DESTACK_GENERATED_END:NODE:40 ==== */