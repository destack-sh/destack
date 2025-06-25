import { packProtoDuration, packProtoTimestamp, unpackProtoDuration, unpackProtoTimestamp } from "@destack/grpc";
import {
  EnumType,
  Global,
  Graph,
  HasName,
  IsSubject,
  MaterializationType,
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
import { Node } from "@destack/language/core/builtin";
import { registerEnumClass, registerNodeClass } from "@destack/language/registry";
import { Space } from "@destack/language/space";
import {
  FileFormatProto,
  FileProto,
  FileSourceProto,
  FileTypeProto,
  MaterializationTypeProto,
  ResourceStatusProto,
} from "@destack/proto";
import { timedeltaFromISOFormat, timedeltaToISOFormat } from "@destack/utils";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2541 ==== */
/**
 * FileSource
 */
export enum FileSource {
  SPACE = 1,
  INLINE = 3,
  EXTERNAL = 10,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILE_SOURCE, FileSource);
/* ==== DESTACK_GENERATED_END:ENUM:2541 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2540 ==== */
/**
 * FileRetentionMode
 */
export enum FileRetentionMode {
  AUTOMATIC = 1,
  MANUAL = 2,
  TIMED = 3,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILE_RETENTION_MODE, FileRetentionMode);
/* ==== DESTACK_GENERATED_END:ENUM:2540 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2542 ==== */
/**
 * FileType
 */
export enum FileType {
  TEXT = 1,
  CODE = 2,
  IMAGE = 3,
  AUDIO = 4,
  VIDEO = 5,
  DOCUMENT = 6,
  DATA = 7,
  ARCHIVE = 8,
  EXECUTABLE = 9,
  GENERIC = 99,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILE_TYPE, FileType);
/* ==== DESTACK_GENERATED_END:ENUM:2542 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2543 ==== */
/**
 * FileFormat
 */
export enum FileFormat {
  TXT = 10000,
  MARKDOWN = 10001,
  RTF = 10002,
  INI = 10003,
  LOG = 10004,
  PYTHON = 20000,
  JAVASCRIPT = 20001,
  TYPESCRIPT = 20002,
  GO = 20003,
  C_LANG = 20004,
  CPP = 20005,
  OBJECTIVE_C = 20006,
  SWIFT = 20007,
  RUBY = 20008,
  PHP = 20009,
  CSS = 20011,
  JAVA = 20012,
  KOTLIN = 20013,
  RUST = 20014,
  SCALA = 20015,
  SHELL = 20016,
  SQL = 20017,
  POWERSHELL = 20018,
  ASSEMBLY = 20019,
  LATEX = 20020,
  JPEG = 30000,
  PNG = 30001,
  GIF = 30002,
  BMP = 30003,
  TIFF = 30004,
  WEBP = 30005,
  SVG = 30006,
  ICO = 30007,
  RAW = 30008,
  HEIC = 30009,
  HEIF = 30010,
  MP3 = 40000,
  WAV = 40001,
  FLAC = 40002,
  AAC = 40003,
  OGG = 40004,
  M4A = 40005,
  WMA = 40006,
  MP4 = 50000,
  WEBM = 50001,
  AVI = 50002,
  MOV = 50003,
  WMV = 50004,
  FLV = 50005,
  MKV = 50006,
  PDF = 60000,
  DOCX = 60001,
  PPTX = 60002,
  ODT = 60003,
  XLSX = 60004,
  ODS = 60005,
  EPUB = 60006,
  MOBI = 60007,
  CHM = 60008,
  DOC = 60009,
  XLS = 60100,
  PPT = 60101,
  HTML = 60102,
  JSON = 70000,
  YAML = 70001,
  CSV = 70002,
  XML = 70003,
  TOML = 70004,
  SQLITE = 70100,
  PARQUET = 70101,
  ZIP = 80000,
  RAR = 80001,
  TAR = 80002,
  SEVENZIP = 80003,
  CAB = 80004,
  GZIP = 80005,
  BZIP2 = 80006,
  XZ = 80007,
  EXE = 90000,
  APP_IMAGE = 90001,
  APK = 90002,
  DMG = 90003,
  JAR = 90004,
  MSI = 90005,
  DEB = 90006,
  RPM = 90007,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.FILE_FORMAT, FileFormat);
/* ==== DESTACK_GENERATED_END:ENUM:2543 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2540 ==== */
/**
 * A File stored somewhere.
 */
export class File extends Node implements Spatial, Global, Resource, HasName {
  static metatype: NodeType = NodeType.FILE;
  static __traits__: TraitType[] = [
    TraitType.GLOBAL,
    TraitType.SPATIAL,
    TraitType.ENTITY,
    TraitType.TRACKED,
    TraitType.RESOURCE,
  ];
  static __rootType__: NodeType | null = NodeType.SPACE;
  static __parentTypes__: NodeType[] = [NodeType.SPACE];
  static __childTypes__: NodeType[] = [];
  static __ancestorTypes__: NodeType[] = [NodeType.SPACE];
  static __descendantTypes__: NodeType[] = [];

  /**
   * Spatial.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
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
   * Entity.materialization
   */
  readonly materialization: MaterializationType;

  /**
   * IsTracked.createdAt
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.createdBy
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
   * IsTracked.updatedAt
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * IsTracked.updatedBy
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
   * File.type
   */
  type: FileType;

  /**
   * HasName.name
   */
  name: string;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * Resource.targetStatus
   */
  targetStatus: Temporal.ZonedDateTime | null;

  /**
   * File.source
   */
  source: FileSource;

  /**
   * File.mimeType
   */
  mimeType: string | null;

  /**
   * File.format
   */
  format: FileFormat | null;

  /**
   * File.size
   */
  size: number | null;

  /**
   * File.sha256
   */
  sha256: string | null;

  /**
   * File.width
   */
  width: number | null;

  /**
   * File.height
   */
  height: number | null;

  /**
   * File.aspectRatio
   */
  aspectRatio: number | null;

  /**
   * File.codec
   */
  codec: string | null;

  /**
   * File.duration
   */
  duration: Temporal.Duration | null;

  /**
   * File.url
   */
  url: string | null;

  /**
   * File.contentUrl
   */
  contentUrl: string | null;

  /**
   * File.thumbnailUrl
   */
  thumbnailUrl: string | null;

  /**
   * File.faviconUrl
   */
  faviconUrl: string | null;

  /**
   * File.thumbnailWidth
   */
  thumbnailWidth: number | null;

  /**
   * File.thumbnailHeight
   */
  thumbnailHeight: number | null;

  /**
   * File.content
   */
  content: Uint8Array | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    type: FileType;
    name: string;
    status?: ResourceStatus;
    targetStatus?: Temporal.ZonedDateTime | null;
    source: FileSource;
    mimeType?: string | null;
    format?: FileFormat | null;
    size?: number | null;
    sha256?: string | null;
    width?: number | null;
    height?: number | null;
    aspectRatio?: number | null;
    codec?: string | null;
    duration?: Temporal.Duration | null;
    url?: string | null;
    contentUrl?: string | null;
    thumbnailUrl?: string | null;
    faviconUrl?: string | null;
    thumbnailWidth?: number | null;
    thumbnailHeight?: number | null;
    content?: Uint8Array | null;
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
      throw new Error(`File.materialization is required`);
    }
    this.materialization = _materialization;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`File.type is required`);
    }
    this.type = _type;
    let _name = options.name;
    if (_name === null) {
      throw new Error(`File.name is required`);
    }
    this.name = _name;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = ResourceStatus.PENDING;
    }
    if (_status === null) {
      throw new Error(`File.status is required`);
    }
    this.status = _status;
    let _targetStatus = options.targetStatus ?? null;
    this.targetStatus = _targetStatus;
    let _source = options.source;
    if (_source === null) {
      throw new Error(`File.source is required`);
    }
    this.source = _source;
    let _mimeType = options.mimeType ?? null;
    this.mimeType = _mimeType;
    let _format = options.format ?? null;
    this.format = _format;
    let _size = options.size ?? null;
    this.size = _size;
    let _sha256 = options.sha256 ?? null;
    this.sha256 = _sha256;
    let _width = options.width ?? null;
    this.width = _width;
    let _height = options.height ?? null;
    this.height = _height;
    let _aspectRatio = options.aspectRatio ?? null;
    this.aspectRatio = _aspectRatio;
    let _codec = options.codec ?? null;
    this.codec = _codec;
    let _duration = options.duration ?? null;
    this.duration = _duration;
    let _url = options.url ?? null;
    this.url = _url;
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
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.source === other.source)) {
      return false;
    }
    if (!(this.mimeType === other.mimeType)) {
      return false;
    }
    if (!(this.format === other.format)) {
      return false;
    }
    if (!(this.size === other.size)) {
      return false;
    }
    if (!(this.sha256 === other.sha256)) {
      return false;
    }
    if (!(this.width === other.width)) {
      return false;
    }
    if (!(this.height === other.height)) {
      return false;
    }
    if (
      (this.aspectRatio == null) !== (other.aspectRatio == null) ||
      (this.aspectRatio != null &&
        !(this.aspectRatio === other.aspectRatio || Math.abs(this.aspectRatio - other.aspectRatio) < 1e-10))
    ) {
      return false;
    }
    if (!(this.codec === other.codec)) {
      return false;
    }
    if (!(this.duration === other.duration)) {
      return false;
    }
    if (!(this.url === other.url)) {
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
    if (!(this.spacePtr?.id === other.spacePtr?.id)) {
      return false;
    }
    if (!(this.status === other.status)) {
      return false;
    }
    if (!(this.targetStatus === other.targetStatus)) {
      return false;
    }
    if (!(this.name === other.name)) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    return new NodeReference({
      nodeType: NodeType.FILE,
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

  toValue(): { [key: string]: any } {
    return File.__packValue__(this);
  }

  static __packValue__(object: File): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2540;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    objectValue["7"] = object.materialization;
    objectValue["15"] = object.createdAt.toString();
    if (object.createdByPtr != null) {
      objectValue["16"] = object.createdByPtr.toValue();
    }
    objectValue["17"] = object.updatedAt.toString();
    if (object.updatedByPtr != null) {
      objectValue["18"] = object.updatedByPtr.toValue();
    }
    objectValue["30"] = object.type;
    objectValue["31"] = object.name;
    objectValue["40"] = object.status;
    if (object.targetStatus != null) {
      objectValue["41"] = object.targetStatus.toString();
    }
    objectValue["60"] = object.source;
    if (object.mimeType != null) {
      objectValue["61"] = object.mimeType;
    }
    if (object.format != null) {
      objectValue["62"] = object.format;
    }
    if (object.size != null) {
      objectValue["63"] = object.size;
    }
    if (object.sha256 != null) {
      objectValue["64"] = object.sha256;
    }
    if (object.width != null) {
      objectValue["65"] = object.width;
    }
    if (object.height != null) {
      objectValue["66"] = object.height;
    }
    if (object.aspectRatio != null) {
      objectValue["67"] = object.aspectRatio;
    }
    if (object.codec != null) {
      objectValue["68"] = object.codec;
    }
    if (object.duration != null) {
      objectValue["69"] = timedeltaToISOFormat(object.duration);
    }
    if (object.url != null) {
      objectValue["70"] = object.url;
    }
    if (object.contentUrl != null) {
      objectValue["71"] = object.contentUrl;
    }
    if (object.thumbnailUrl != null) {
      objectValue["72"] = object.thumbnailUrl;
    }
    if (object.faviconUrl != null) {
      objectValue["73"] = object.faviconUrl;
    }
    if (object.thumbnailWidth != null) {
      objectValue["74"] = object.thumbnailWidth;
    }
    if (object.thumbnailHeight != null) {
      objectValue["75"] = object.thumbnailHeight;
    }
    if (object.content != null) {
      objectValue["76"] = Buffer.from(object.content).toString("base64");
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    const mimeTypeValue = objectValue["61"];
    const unpackedMimeType = mimeTypeValue != undefined ? mimeTypeValue : null;
    const formatValue = objectValue["62"];
    const unpackedFormat = formatValue != undefined ? Number(formatValue) : null;
    const sizeValue = objectValue["63"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const sha256Value = objectValue["64"];
    const unpackedSha256 = sha256Value != undefined ? sha256Value : null;
    const widthValue = objectValue["65"];
    const unpackedWidth = widthValue != undefined ? Number(widthValue) : null;
    const heightValue = objectValue["66"];
    const unpackedHeight = heightValue != undefined ? Number(heightValue) : null;
    const aspectRatioValue = objectValue["67"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const codecValue = objectValue["68"];
    const unpackedCodec = codecValue != undefined ? codecValue : null;
    const durationValue = objectValue["69"];
    const unpackedDuration = durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const urlValue = objectValue["70"];
    const unpackedUrl = urlValue != undefined ? urlValue : null;
    const contentUrlValue = objectValue["71"];
    const unpackedContentUrl = contentUrlValue != undefined ? contentUrlValue : null;
    const thumbnailUrlValue = objectValue["72"];
    const unpackedThumbnailUrl = thumbnailUrlValue != undefined ? thumbnailUrlValue : null;
    const faviconUrlValue = objectValue["73"];
    const unpackedFaviconUrl = faviconUrlValue != undefined ? faviconUrlValue : null;
    const thumbnailWidthValue = objectValue["74"];
    const unpackedThumbnailWidth = thumbnailWidthValue != undefined ? Number(thumbnailWidthValue) : null;
    const thumbnailHeightValue = objectValue["75"];
    const unpackedThumbnailHeight = thumbnailHeightValue != undefined ? Number(thumbnailHeightValue) : null;
    const contentValue = objectValue["76"];
    const unpackedContent = contentValue != undefined ? Buffer.from(contentValue, "base64") : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const targetStatusValue = objectValue["41"];
    const unpackedTargetStatus = targetStatusValue != undefined ? Temporal.ZonedDateTime.from(targetStatusValue) : null;
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
    return new File({
      type: Number(objectValue["30"]),
      source: Number(objectValue["60"]),
      mimeType: unpackedMimeType,
      format: unpackedFormat,
      size: unpackedSize,
      sha256: unpackedSha256,
      width: unpackedWidth,
      height: unpackedHeight,
      aspectRatio: unpackedAspectRatio,
      codec: unpackedCodec,
      duration: unpackedDuration,
      url: unpackedUrl,
      contentUrl: unpackedContentUrl,
      thumbnailUrl: unpackedThumbnailUrl,
      faviconUrl: unpackedFaviconUrl,
      thumbnailWidth: unpackedThumbnailWidth,
      thumbnailHeight: unpackedThumbnailHeight,
      content: unpackedContent,
      parent: unpackedParentPtr,
      space: unpackedSpacePtr,
      id: String(objectValue["2"]),
      status: Number(objectValue["40"]),
      targetStatus: unpackedTargetStatus,
      materialization: Number(objectValue["7"]),
      createdAt: Temporal.ZonedDateTime.from(objectValue["15"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.ZonedDateTime.from(objectValue["17"]),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["31"],
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
  ): File {
    return File.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): FileProto {
    return File.__packProto__(this);
  }

  static __packProto__(object: File): FileProto {
    const objectProto: Partial<FileProto> = { metatype: 2540 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationTypeProto;
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    objectProto.type = Number(object.type) as FileTypeProto;
    objectProto.name = object.name;
    objectProto.status = Number(object.status) as ResourceStatusProto;
    if (object.targetStatus != null) {
      objectProto.targetStatus = packProtoTimestamp(object.targetStatus);
    }
    objectProto.source = Number(object.source) as FileSourceProto;
    if (object.mimeType != null) {
      objectProto.mimeType = object.mimeType;
    }
    if (object.format != null) {
      objectProto.format = Number(object.format) as FileFormatProto;
    }
    if (object.size != null) {
      objectProto.size = object.size;
    }
    if (object.sha256 != null) {
      objectProto.sha256 = object.sha256;
    }
    if (object.width != null) {
      objectProto.width = object.width;
    }
    if (object.height != null) {
      objectProto.height = object.height;
    }
    if (object.aspectRatio != null) {
      objectProto.aspectRatio = object.aspectRatio;
    }
    if (object.codec != null) {
      objectProto.codec = object.codec;
    }
    if (object.duration != null) {
      objectProto.duration = packProtoDuration(object.duration);
    }
    if (object.url != null) {
      objectProto.url = object.url;
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
    return objectProto as FileProto;
  }

  static __unpackProto__(
    objectProto: FileProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    return new File({
      type: Number(objectProto.type) as FileType,
      source: Number(objectProto.source) as FileSource,
      mimeType: objectProto.mimeType != undefined ? objectProto.mimeType : null,
      format: objectProto.format != undefined ? (Number(objectProto.format) as FileFormat) : null,
      size: objectProto.size != undefined ? Number(objectProto.size) : null,
      sha256: objectProto.sha256 != undefined ? objectProto.sha256 : null,
      width: objectProto.width != undefined ? Number(objectProto.width) : null,
      height: objectProto.height != undefined ? Number(objectProto.height) : null,
      aspectRatio: objectProto.aspectRatio != undefined ? objectProto.aspectRatio : null,
      codec: objectProto.codec != undefined ? objectProto.codec : null,
      duration: objectProto.duration != undefined ? unpackProtoDuration(objectProto.duration!) : null,
      url: objectProto.url != undefined ? objectProto.url : null,
      contentUrl: objectProto.contentUrl != undefined ? objectProto.contentUrl : null,
      thumbnailUrl: objectProto.thumbnailUrl != undefined ? objectProto.thumbnailUrl : null,
      faviconUrl: objectProto.faviconUrl != undefined ? objectProto.faviconUrl : null,
      thumbnailWidth: objectProto.thumbnailWidth != undefined ? Number(objectProto.thumbnailWidth) : null,
      thumbnailHeight: objectProto.thumbnailHeight != undefined ? Number(objectProto.thumbnailHeight) : null,
      content: objectProto.content != undefined ? objectProto.content : null,
      parent:
        objectProto.parentPtr != undefined
          ? NodeReference.fromProto(objectProto.parentPtr!, _session, _supergraph, _graph, _connection)
          : null,
      space:
        objectProto.spacePtr != undefined
          ? NodeReference.fromProto(objectProto.spacePtr!, _session, _supergraph, _graph, _connection)
          : null,
      id: String(objectProto.id),
      status: Number(objectProto.status) as ResourceStatus,
      targetStatus: objectProto.targetStatus != undefined ? unpackProtoTimestamp(objectProto.targetStatus!) : null,
      materialization: Number(objectProto.materialization) as MaterializationType,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? NodeReference.fromProto(objectProto.createdByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      updatedAt: unpackProtoTimestamp(objectProto.updatedAt!),
      updatedBy:
        objectProto.updatedByPtr != undefined
          ? NodeReference.fromProto(objectProto.updatedByPtr!, _session, _supergraph, _graph, _connection)
          : null,
      name: objectProto.name,
      _session,
      _graph,
      _connection,
    });
  }

  static fromProto(
    objectProto: FileProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    return File.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILE, File);
/* ==== DESTACK_GENERATED_END:NODE:2540 ==== */
