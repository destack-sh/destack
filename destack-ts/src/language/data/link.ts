import { Agent, BuiltinObject, ACTIVE_SESSION, Session, StructFrozen, Graph, ResourceStatus, Struct, activeSession, StructType, NodeType, Supergraph, QueryConnection, NodeReference, MaterializationType, User, Spatial, Space, Resource, Entity, EnumType, Node, IsTracked } from '@/language';
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

  constructor(options: {
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
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(options.id, options.parent, session, supergraph, options._graph, options._connection);
    this.type = options.type;
    this.status = options.status ?? ResourceStatus.PENDING;
    this.targetStatus = options.targetStatus ?? null;
    this.url = options.url ?? null;
    this.domain = options.domain ?? null;
    this.contentUrl = options.contentUrl ?? null;
    this.thumbnailUrl = options.thumbnailUrl ?? null;
    this.faviconUrl = options.faviconUrl ?? null;
    this.thumbnailWidth = options.thumbnailWidth ?? null;
    this.thumbnailHeight = options.thumbnailHeight ?? null;
    this.content = options.content ?? null;
    this.attribution = options.attribution ?? null;
    this.attributionTag = options.attributionTag ?? null;
    this.publishedAt = options.publishedAt ?? null;
    this.expiresAt = options.expiresAt ?? null;
    this.imageUrls = options.imageUrls ?? [];
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