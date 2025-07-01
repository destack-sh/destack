import { packProtoTimestamp, unpackProtoTimestamp } from "@destack/grpc";
import type {
  Graph,
  IsSpatial,
  IsSubject,
  QueryConnection,
  Session,
  Supergraph,
} from "@destack/language/core";
import {
  EnumType,
  Node,
  NodeReference,
  NodeType,
  Resource,
  ResourceStatus,
  StructType,
} from "@destack/language/core";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import type { Space } from "@destack/language/space";
import { LinkProto, LinkTypeProto, ResourceStatusProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:60100 ==== */
/**
 * LinkType
 */
export enum LinkType {
  WEB = 1,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.LINK_TYPE, LinkType);
/* ==== DESTACK_GENERATED_END:ENUM:60100 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60100 ==== */
/**
 * A Link to an external resource (like a web URL, or anything that doesn't fit into other Nodes).
 */
export class Link extends Resource implements IsSpatial {
  static metatype: NodeType = NodeType.LINK;

  /**
   * Trait.parent
   */
  get parent(): Node | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly parentPtr: NodeReference | null;

  /**
   * The Space this Node is in.
   */
  get space(): Space | null {
    const nodePtr: NodeReference | null = this.spacePtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference | null;

  /**
   * Entity.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * Entity.createdBy
   */
  get createdBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * Entity.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * Entity.updatedBy
   */
  get updatedBy(): (Node & IsSubject) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as (Node & IsSubject) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Link.type
   */
  type: LinkType;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /**
   * Link.url
   */
  url: string | null;

  /**
   * Link.domain
   */
  domain: string | null;

  /**
   * Link.contentUrl
   */
  contentUrl: string | null;

  /**
   * Link.thumbnailUrl
   */
  thumbnailUrl: string | null;

  /**
   * Link.faviconUrl
   */
  faviconUrl: string | null;

  /**
   * Link.thumbnailWidth
   */
  thumbnailWidth: number | null;

  /**
   * Link.thumbnailHeight
   */
  thumbnailHeight: number | null;

  /**
   * Link.content
   */
  content: string | null;

  /**
   * Link.attribution
   */
  attribution: string | null;

  /**
   * Link.attributionTag
   */
  attributionTag: string | null;

  /**
   * Link.publishedAt
   */
  publishedAt: Temporal.ZonedDateTime | null;

  /**
   * Link.expiresAt
   */
  expiresAt: Temporal.ZonedDateTime | null;

  /**
   * Link.imageUrls
   */
  imageUrls: Array<string>;

  constructor(options: {
    id?: string;
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
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
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
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
      _imageUrls = [];
    }
    this.imageUrls = _imageUrls;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(
          `{cls.__name__}.createdAt and {cls.__name__}.updatedAt are required for existing Nodes`,
        );
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.url === other.url)) {
      return false;
    }
    if (!(this.domain === other.domain)) {
      return false;
    }
    if (!(this.contentUrl === other.contentUrl)) {
      return false;
    }
    if (!(this.thumbnailUrl === other.thumbnailUrl)) {
      return false;
    }
    if (!(this.faviconUrl === other.faviconUrl)) {
      return false;
    }
    if (!(this.thumbnailWidth === other.thumbnailWidth)) {
      return false;
    }
    if (!(this.thumbnailHeight === other.thumbnailHeight)) {
      return false;
    }
    if (!(this.content === other.content)) {
      return false;
    }
    if (!(this.attribution === other.attribution)) {
      return false;
    }
    if (!(this.attributionTag === other.attributionTag)) {
      return false;
    }
    if (!(this.publishedAt === other.publishedAt)) {
      return false;
    }
    if (!(this.expiresAt === other.expiresAt)) {
      return false;
    }
    if (this.imageUrls.length !== other.imageUrls.length) {
      return false;
    }
    for (let i = 0; i < this.imageUrls.length; i++) {
      if (!(this.imageUrls[i] === other.imageUrls[i])) {
        return false;
      }
    }
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.targetStatus === other.targetStatus)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.url !== null) {
      h = (h * 31 + hashString(this.url)) & 0xffffffff;
    }
    if (this.domain !== null) {
      h = (h * 31 + hashString(this.domain)) & 0xffffffff;
    }
    if (this.contentUrl !== null) {
      h = (h * 31 + hashString(this.contentUrl)) & 0xffffffff;
    }
    if (this.thumbnailUrl !== null) {
      h = (h * 31 + hashString(this.thumbnailUrl)) & 0xffffffff;
    }
    if (this.faviconUrl !== null) {
      h = (h * 31 + hashString(this.faviconUrl)) & 0xffffffff;
    }
    if (this.thumbnailWidth !== null) {
      h = (h * 31 + hashInt(this.thumbnailWidth)) & 0xffffffff;
    }
    if (this.thumbnailHeight !== null) {
      h = (h * 31 + hashInt(this.thumbnailHeight)) & 0xffffffff;
    }
    if (this.content !== null) {
      h = (h * 31 + hashString(this.content)) & 0xffffffff;
    }
    if (this.attribution !== null) {
      h = (h * 31 + hashString(this.attribution)) & 0xffffffff;
    }
    if (this.attributionTag !== null) {
      h = (h * 31 + hashString(this.attributionTag)) & 0xffffffff;
    }
    if (this.publishedAt !== null) {
      h = (h * 31 + hashString(this.publishedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.expiresAt !== null) {
      h = (h * 31 + hashString(this.expiresAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.imageUrls && this.imageUrls.length > 0) {
      for (const _item of this.imageUrls) {
        h = (h * 31 + hashString(_item)) & 0xffffffff;
      }
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    if (this.targetStatus !== null) {
      h = (h * 31 + hashString(this.targetStatus.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }

    return h;
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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${LinkType[this.type]}`);
    if (this.url !== null) {
      propertyReprs.push(`url=${this.url}`);
    }
    if (this.domain !== null) {
      propertyReprs.push(`domain=${this.domain}`);
    }
    return `<Link '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return Link.__packValue__(this);
  }

  static __packValue__(object: Link): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60100;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["15"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["20"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    objectValue["30"] = object.type;
    objectValue["40"] = object.status;
    if (object.targetStatus != null) {
      objectValue["41"] = object.targetStatus.toString({ timeZoneName: "never" });
    }
    if (object.url != null) {
      objectValue["50"] = object.url;
    }
    if (object.domain != null) {
      objectValue["51"] = object.domain;
    }
    if (object.contentUrl != null) {
      objectValue["52"] = object.contentUrl;
    }
    if (object.thumbnailUrl != null) {
      objectValue["53"] = object.thumbnailUrl;
    }
    if (object.faviconUrl != null) {
      objectValue["54"] = object.faviconUrl;
    }
    if (object.thumbnailWidth != null) {
      objectValue["55"] = object.thumbnailWidth;
    }
    if (object.thumbnailHeight != null) {
      objectValue["56"] = object.thumbnailHeight;
    }
    if (object.content != null) {
      objectValue["60"] = object.content;
    }
    if (object.attribution != null) {
      objectValue["62"] = object.attribution;
    }
    if (object.attributionTag != null) {
      objectValue["63"] = object.attributionTag;
    }
    if (object.publishedAt != null) {
      objectValue["64"] = object.publishedAt.toString({ timeZoneName: "never" });
    }
    if (object.expiresAt != null) {
      objectValue["65"] = object.expiresAt.toString({ timeZoneName: "never" });
    }
    if (object.imageUrls.length > 0) {
      const packedImageUrls: any[] = [];
      for (const item of object.imageUrls) {
        packedImageUrls.push(item);
      }
      objectValue["70"] = packedImageUrls;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Link {
    const urlValue = objectValue["50"];
    const unpackedUrl = urlValue != undefined ? urlValue : null;
    const domainValue = objectValue["51"];
    const unpackedDomain = domainValue != undefined ? domainValue : null;
    const contentUrlValue = objectValue["52"];
    const unpackedContentUrl = contentUrlValue != undefined ? contentUrlValue : null;
    const thumbnailUrlValue = objectValue["53"];
    const unpackedThumbnailUrl = thumbnailUrlValue != undefined ? thumbnailUrlValue : null;
    const faviconUrlValue = objectValue["54"];
    const unpackedFaviconUrl = faviconUrlValue != undefined ? faviconUrlValue : null;
    const thumbnailWidthValue = objectValue["55"];
    const unpackedThumbnailWidth =
      thumbnailWidthValue != undefined ? Number(thumbnailWidthValue) : null;
    const thumbnailHeightValue = objectValue["56"];
    const unpackedThumbnailHeight =
      thumbnailHeightValue != undefined ? Number(thumbnailHeightValue) : null;
    const contentValue = objectValue["60"];
    const unpackedContent = contentValue != undefined ? contentValue : null;
    const attributionValue = objectValue["62"];
    const unpackedAttribution = attributionValue != undefined ? attributionValue : null;
    const attributionTagValue = objectValue["63"];
    const unpackedAttributionTag = attributionTagValue != undefined ? attributionTagValue : null;
    const publishedAtValue = objectValue["64"];
    const unpackedPublishedAt =
      publishedAtValue != undefined
        ? Temporal.Instant.from(publishedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const expiresAtValue = objectValue["65"];
    const unpackedExpiresAt =
      expiresAtValue != undefined
        ? Temporal.Instant.from(expiresAtValue).toZonedDateTimeISO("UTC")
        : null;
    const unpackedImageUrls: any[] = [];
    if (objectValue["70"] != undefined) {
      for (const item of objectValue["70"]) {
        unpackedImageUrls.push(item);
      }
    }
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetStatusValue = objectValue["41"];
    const unpackedTargetStatus =
      targetStatusValue != undefined
        ? Temporal.Instant.from(targetStatusValue).toZonedDateTimeISO("UTC")
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["20"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const createdByPtrValue = objectValue["16"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["18"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Link({
      type: Number(objectValue["30"]),
      url: unpackedUrl,
      domain: unpackedDomain,
      contentUrl: unpackedContentUrl,
      thumbnailUrl: unpackedThumbnailUrl,
      faviconUrl: unpackedFaviconUrl,
      thumbnailWidth: unpackedThumbnailWidth,
      thumbnailHeight: unpackedThumbnailHeight,
      content: unpackedContent,
      attribution: unpackedAttribution,
      attributionTag: unpackedAttributionTag,
      publishedAt: unpackedPublishedAt,
      expiresAt: unpackedExpiresAt,
      imageUrls: unpackedImageUrls,
      space: unpackedSpacePtr,
      status: Number(objectValue["40"]),
      targetStatus: unpackedTargetStatus,
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      deletedAt: unpackedDeletedAt,
      createdAt: Temporal.Instant.from(objectValue["15"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["17"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Link {
    return Link.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): LinkProto {
    return Link.__packProto__(this);
  }

  static __packProto__(object: Link): LinkProto {
    const objectProto: Partial<LinkProto> = { metatype: 60100 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    objectProto.type = Number(object.type) as LinkTypeProto;
    objectProto.status = Number(object.status) as ResourceStatusProto;
    if (object.targetStatus != null) {
      objectProto.targetStatus = packProtoTimestamp(object.targetStatus);
    }
    if (object.url != null) {
      objectProto.url = object.url;
    }
    if (object.domain != null) {
      objectProto.domain = object.domain;
    }
    if (object.contentUrl != null) {
      objectProto.contentUrl = object.contentUrl;
    }
    if (object.thumbnailUrl != null) {
      objectProto.thumbnailUrl = object.thumbnailUrl;
    }
    if (object.faviconUrl != null) {
      objectProto.faviconUrl = object.faviconUrl;
    }
    if (object.thumbnailWidth != null) {
      objectProto.thumbnailWidth = object.thumbnailWidth;
    }
    if (object.thumbnailHeight != null) {
      objectProto.thumbnailHeight = object.thumbnailHeight;
    }
    if (object.content != null) {
      objectProto.content = object.content;
    }
    if (object.attribution != null) {
      objectProto.attribution = object.attribution;
    }
    if (object.attributionTag != null) {
      objectProto.attributionTag = object.attributionTag;
    }
    if (object.publishedAt != null) {
      objectProto.publishedAt = packProtoTimestamp(object.publishedAt);
    }
    if (object.expiresAt != null) {
      objectProto.expiresAt = packProtoTimestamp(object.expiresAt);
    }
    if (object.imageUrls) {
      const packedImageUrls: any[] = [];
      for (const item of object.imageUrls) {
        packedImageUrls.push(item);
      }
      objectProto.imageUrls = packedImageUrls;
    }
    return objectProto as LinkProto;
  }

  static __unpackProto__(
    objectProto: LinkProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Link {
    const unpackedImageUrls: any[] = [];
    if (objectProto.imageUrls) {
      for (const item of objectProto.imageUrls) {
        unpackedImageUrls.push(item);
      }
    }
    return new Link({
      type: Number(objectProto.type) as LinkType,
      url: objectProto.url != undefined ? objectProto.url : null,
      domain: objectProto.domain != undefined ? objectProto.domain : null,
      contentUrl: objectProto.contentUrl != undefined ? objectProto.contentUrl : null,
      thumbnailUrl: objectProto.thumbnailUrl != undefined ? objectProto.thumbnailUrl : null,
      faviconUrl: objectProto.faviconUrl != undefined ? objectProto.faviconUrl : null,
      thumbnailWidth:
        objectProto.thumbnailWidth != undefined ? Number(objectProto.thumbnailWidth) : null,
      thumbnailHeight:
        objectProto.thumbnailHeight != undefined ? Number(objectProto.thumbnailHeight) : null,
      content: objectProto.content != undefined ? objectProto.content : null,
      attribution: objectProto.attribution != undefined ? objectProto.attribution : null,
      attributionTag: objectProto.attributionTag != undefined ? objectProto.attributionTag : null,
      publishedAt:
        objectProto.publishedAt != undefined
          ? unpackProtoTimestamp(objectProto.publishedAt!)
          : null,
      expiresAt:
        objectProto.expiresAt != undefined ? unpackProtoTimestamp(objectProto.expiresAt!) : null,
      imageUrls: unpackedImageUrls,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as ResourceStatus,
      targetStatus:
        objectProto.targetStatus != undefined
          ? unpackProtoTimestamp(objectProto.targetStatus!)
          : null,
      id: String(objectProto.id),
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.createdByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: LinkProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Link {
    return Link.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Link {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = LinkProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.LINK, Link);
/* ==== DESTACK_GENERATED_END:NODE:60100 ==== */
