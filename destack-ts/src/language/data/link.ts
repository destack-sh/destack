import { ResourceStatus, EnumType, StructType, Node, QueryConnection, User, NodeReference, Graph, Spatial, Agent, Space, StructFrozen, Struct, Resource, BuiltinObject, NodeType, Session, MaterializationType, Entity, Supergraph, IsTracked } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:2550 ==== */
export enum LinkType {
  WEB = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:2550 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2550 ==== */
export class Link extends Node implements Spatial, Entity, Resource, IsTracked {
  readonly id: string;
  get parent(): Space | null | null {
      const nodePtr: NodeReference | null = this.parentPtr;
      if (nodePtr !== null) {
          return this._supergraph.get(nodePtr.id) as Space | null | null;
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
  type: LinkType;
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
  url: string | null;
  domain: string | null;
  contentUrl: string | null;
  thumbnailUrl: string | null;
  faviconUrl: string | null;
  thumbnailWidth: number | null;
  thumbnailHeight: number | null;
  content: string | null;
  attribution: string | null;
  attributionTag: string | null;
  publishedAt: Temporal.ZonedDateTime | null;
  expiresAt: Temporal.ZonedDateTime | null;
  imageUrls: Array<string>;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: LinkType,
    status: ResourceStatus,
    targetStatus: Temporal.ZonedDateTime | null,
    url: string | null,
    domain: string | null,
    contentUrl: string | null,
    thumbnailUrl: string | null,
    faviconUrl: string | null,
    thumbnailWidth: number | null,
    thumbnailHeight: number | null,
    content: string | null,
    attribution: string | null,
    attributionTag: string | null,
    publishedAt: Temporal.ZonedDateTime | null,
    expiresAt: Temporal.ZonedDateTime | null,
    imageUrls: Array<string>,
    _session: Session,
    _supergraph: Supergraph,
    _graph: Graph,
    _connection: QueryConnection | null
  ) {
    super(id, _session, _supergraph, _graph, _connection);
    this.id = id;
    this.parentPtr = parentPtr;
    this.spacePtr = spacePtr;
    this.materialization = materialization;
    this.createdAt = createdAt;
    this.createdByPtr = createdByPtr;
    this.updatedAt = updatedAt;
    this.updatedByPtr = updatedByPtr;
    this.type = type;
    this.status = status;
    this.targetStatus = targetStatus;
    this.url = url;
    this.domain = domain;
    this.contentUrl = contentUrl;
    this.thumbnailUrl = thumbnailUrl;
    this.faviconUrl = faviconUrl;
    this.thumbnailWidth = thumbnailWidth;
    this.thumbnailHeight = thumbnailHeight;
    this.content = content;
    this.attribution = attribution;
    this.attributionTag = attributionTag;
    this.publishedAt = publishedAt;
    this.expiresAt = expiresAt;
    this.imageUrls = imageUrls;
  }


  static create(options: {
    type: LinkType,
    status?: ResourceStatus,
    targetStatus?: Temporal.ZonedDateTime | null,
    url?: string | null,
    domain?: string | null,
    contentUrl?: string | null,
    thumbnailUrl?: string | null,
    faviconUrl?: string | null,
    thumbnailWidth?: number | null,
    thumbnailHeight?: number | null,
    content?: string | null,
    attribution?: string | null,
    attributionTag?: string | null,
    publishedAt?: Temporal.ZonedDateTime | null,
    expiresAt?: Temporal.ZonedDateTime | null,
    imageUrls?: Array<string>,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): Link {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new Link(
      options.type,
      options.status ?? ResourceStatus.PENDING,
      options.targetStatus ?? null,
      options.url ?? null,
      options.domain ?? null,
      options.contentUrl ?? null,
      options.thumbnailUrl ?? null,
      options.faviconUrl ?? null,
      options.thumbnailWidth ?? null,
      options.thumbnailHeight ?? null,
      options.content ?? null,
      options.attribution ?? null,
      options.attributionTag ?? null,
      options.publishedAt ?? null,
      options.expiresAt ?? null,
      options.imageUrls ?? [],
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
    return new NodeReference(NodeType.LINK, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
  }

  get _pathKey(): string {
      return "Link[id={this.id}]";
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
/* ==== DESTACK_GENERATED_END:NODE:2550 ==== */