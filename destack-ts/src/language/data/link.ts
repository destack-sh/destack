import {
  Entity,
  Graph,
  IsSubject,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Resource,
  ResourceStatus,
  Session,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
} from "@destack/language/core";
import { Space } from "@destack/language/space";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2550 ==== */
export enum LinkType {
  WEB = 1,
}
/* ==== DESTACK_GENERATED_END:ENUM:2550 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2550 ==== */
export class Link extends Node implements Spatial, Entity, Resource, IsTracked {
  static metatype: NodeType = NodeType.LINK;
  static __traits__: TraitType[] = [TraitType.TRACKED, TraitType.SPATIAL, TraitType.ENTITY, TraitType.RESOURCE];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  get parent(): Space | null | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null | null;
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
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type: LinkType;
    status?: ResourceStatus;
    targetStatus?: Temporal.ZonedDateTime | null;
    url?: string | null;
    domain?: string | null;
    contentUrl?: string | null;
    thumbnailUrl?: string | null;
    faviconUrl?: string | null;
    thumbnailWidth?: number | null;
    thumbnailHeight?: number | null;
    content?: string | null;
    attribution?: string | null;
    attributionTag?: string | null;
    publishedAt?: Temporal.ZonedDateTime | null;
    expiresAt?: Temporal.ZonedDateTime | null;
    imageUrls?: Array<string>;
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
      throw new Error(`Link.materialization is required`);
    }
    this.materialization = _materialization;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Link.type is required`);
    }
    this.type = _type;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = ResourceStatus.PENDING;
    }
    if (_status === null) {
      throw new Error(`Link.status is required`);
    }
    this.status = _status;
    let _targetStatus = options.targetStatus ?? null;
    this.targetStatus = _targetStatus;
    let _url = options.url ?? null;
    this.url = _url;
    let _domain = options.domain ?? null;
    this.domain = _domain;
    let _contentUrl = options.contentUrl ?? null;
    this.contentUrl = _contentUrl;
    let _thumbnailUrl = options.thumbnailUrl ?? null;
    this.thumbnailUrl = _thumbnailUrl;
    let _faviconUrl = options.faviconUrl ?? null;
    this.faviconUrl = _faviconUrl;
    let _thumbnailWidth = options.thumbnailWidth ?? null;
    this.thumbnailWidth = _thumbnailWidth;
    let _thumbnailHeight = options.thumbnailHeight ?? null;
    this.thumbnailHeight = _thumbnailHeight;
    let _content = options.content ?? null;
    this.content = _content;
    let _attribution = options.attribution ?? null;
    this.attribution = _attribution;
    let _attributionTag = options.attributionTag ?? null;
    this.attributionTag = _attributionTag;
    let _publishedAt = options.publishedAt ?? null;
    this.publishedAt = _publishedAt;
    let _expiresAt = options.expiresAt ?? null;
    this.expiresAt = _expiresAt;
    let _imageUrls = options.imageUrls ?? null;
    if (_imageUrls === null) {
      throw new Error(`Link.imageUrls is required`);
    }
    this.imageUrls = _imageUrls;

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
      nodeType: NodeType.LINK,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
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
