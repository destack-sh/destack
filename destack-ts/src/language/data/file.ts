import {
  packProtoDuration,
  packProtoTimestamp,
  unpackProtoDuration,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  Graph,
  IsActor,
  IsExtensible,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  Resource,
  ResourceStatus,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/universe";
import {
  FileFormatProto,
  FileProto,
  FileSourceProto,
  FileTypeProto,
  MaterializationProto,
  ResourceStatusProto,
} from "@destack/proto";
import {
  base64Decode,
  base64Encode,
  timedeltaFromISOFormat,
  timedeltaToISOFormat,
} from "@destack/utils";
import { hashBool, hashBytes, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:400001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:400001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:400000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:400000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:400002 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:400002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:400003 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:400003 ==== */

/* ==== DESTACK_GENERATED_START:NODE:480000 ==== */
/**
 * A File stored somewhere.
 */
export class File extends Resource {
  static metatype: NodeType = NodeType.FILE;

  /**
   * File.parent
   */
  get parent(): Space | null {
    const nodePtr: NodeReference | null = this.parentPtr;
    if (nodePtr != null) {
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
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Space | null;
    }
    return null;
  }
  readonly spacePtr: NodeReference;

  /**
   * The definition this CustomEntity is an instance of.
   */
  get definition(): (Entity & IsExtensible) | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsExtensible) | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The Snapshot this Entity is part of.
   */
  get snapshot(): Snapshot | null {
    const nodePtr: NodeReference | null = this.snapshotPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Snapshot | null;
    }
    return null;
  }
  readonly snapshotPtr: NodeReference | null;

  /**
   * The previous Entity this Entity is based on (from another Snapshot).
   */
  get predecessor(): File | null {
    const nodePtr: NodeReference | null = this.predecessorPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as File | null;
    }
    return null;
  }
  readonly predecessorPtr: NodeReference | null;

  /**
   * The time this Entity was created.
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;

  /**
   * The time this Entity was last updated.
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;

  /**
   * IsDeletable.deletedAt
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  /**
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  get customValues(): { readonly [key: string]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: string]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: string]: Value };

  /**
   * Resource.status
   */
  /**
   * Resource.status
   */
  get status(): ResourceStatus {
    return this._status;
  }
  set status(value: ResourceStatus) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: ResourceStatus;

  /**
   * Entity.name
   */
  /**
   * Entity.name
   */
  get name(): string {
    return this._name;
  }
  set name(value: string) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: string;

  /**
   * The main / root Script of this Node.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
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
  /**
   * The main / root Script of this Node.
   */
  get scriptPtr(): NodeReference | null {
    return this._scriptPtr;
  }
  set scriptPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["script"];
    this._session.updateSetProperty(this, prop, value);
    this._scriptPtr = value;
  }
  _scriptPtr: NodeReference | null;

  /**
   * Whether this Node is extensible (whether it can be instanced).
   */
  readonly isExtensible: boolean;

  /**
   * File.type
   */
  /**
   * File.type
   */
  get type(): FileType {
    return this._type;
  }
  set type(value: FileType) {
    const prop = (this.constructor as NodeClass).__properties__["type"];
    this._session.updateSetProperty(this, prop, value);
    this._type = value;
  }
  _type: FileType;

  /**
   * File.source
   */
  /**
   * File.source
   */
  get source(): FileSource {
    return this._source;
  }
  set source(value: FileSource) {
    const prop = (this.constructor as NodeClass).__properties__["source"];
    this._session.updateSetProperty(this, prop, value);
    this._source = value;
  }
  _source: FileSource;

  /**
   * File.mimeType
   */
  /**
   * File.mimeType
   */
  get mimeType(): string | null {
    return this._mimeType;
  }
  set mimeType(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["mime_type"];
    this._session.updateSetProperty(this, prop, value);
    this._mimeType = value;
  }
  _mimeType: string | null;

  /**
   * File.format
   */
  /**
   * File.format
   */
  get format(): FileFormat | null {
    return this._format;
  }
  set format(value: FileFormat | null) {
    const prop = (this.constructor as NodeClass).__properties__["format"];
    this._session.updateSetProperty(this, prop, value);
    this._format = value;
  }
  _format: FileFormat | null;

  /**
   * File.size
   */
  /**
   * File.size
   */
  get size(): number | null {
    return this._size;
  }
  set size(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["size"];
    this._session.updateSetProperty(this, prop, value);
    this._size = value;
  }
  _size: number | null;

  /**
   * File.sha256
   */
  /**
   * File.sha256
   */
  get sha256(): string | null {
    return this._sha256;
  }
  set sha256(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["sha256"];
    this._session.updateSetProperty(this, prop, value);
    this._sha256 = value;
  }
  _sha256: string | null;

  /**
   * File.width
   */
  /**
   * File.width
   */
  get width(): number | null {
    return this._width;
  }
  set width(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: number | null;

  /**
   * File.height
   */
  /**
   * File.height
   */
  get height(): number | null {
    return this._height;
  }
  set height(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: number | null;

  /**
   * File.aspectRatio
   */
  /**
   * File.aspectRatio
   */
  get aspectRatio(): number | null {
    return this._aspectRatio;
  }
  set aspectRatio(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["aspect_ratio"];
    this._session.updateSetProperty(this, prop, value);
    this._aspectRatio = value;
  }
  _aspectRatio: number | null;

  /**
   * File.codec
   */
  /**
   * File.codec
   */
  get codec(): string | null {
    return this._codec;
  }
  set codec(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["codec"];
    this._session.updateSetProperty(this, prop, value);
    this._codec = value;
  }
  _codec: string | null;

  /**
   * File.duration
   */
  /**
   * File.duration
   */
  get duration(): Temporal.Duration | null {
    return this._duration;
  }
  set duration(value: Temporal.Duration | null) {
    const prop = (this.constructor as NodeClass).__properties__["duration"];
    this._session.updateSetProperty(this, prop, value);
    this._duration = value;
  }
  _duration: Temporal.Duration | null;

  /**
   * File.url
   */
  /**
   * File.url
   */
  get url(): string | null {
    return this._url;
  }
  set url(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["url"];
    this._session.updateSetProperty(this, prop, value);
    this._url = value;
  }
  _url: string | null;

  /**
   * File.contentUrl
   */
  /**
   * File.contentUrl
   */
  get contentUrl(): string | null {
    return this._contentUrl;
  }
  set contentUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["content_url"];
    this._session.updateSetProperty(this, prop, value);
    this._contentUrl = value;
  }
  _contentUrl: string | null;

  /**
   * File.thumbnailUrl
   */
  /**
   * File.thumbnailUrl
   */
  get thumbnailUrl(): string | null {
    return this._thumbnailUrl;
  }
  set thumbnailUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_url"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailUrl = value;
  }
  _thumbnailUrl: string | null;

  /**
   * File.faviconUrl
   */
  /**
   * File.faviconUrl
   */
  get faviconUrl(): string | null {
    return this._faviconUrl;
  }
  set faviconUrl(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["favicon_url"];
    this._session.updateSetProperty(this, prop, value);
    this._faviconUrl = value;
  }
  _faviconUrl: string | null;

  /**
   * File.thumbnailWidth
   */
  /**
   * File.thumbnailWidth
   */
  get thumbnailWidth(): number | null {
    return this._thumbnailWidth;
  }
  set thumbnailWidth(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_width"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailWidth = value;
  }
  _thumbnailWidth: number | null;

  /**
   * File.thumbnailHeight
   */
  /**
   * File.thumbnailHeight
   */
  get thumbnailHeight(): number | null {
    return this._thumbnailHeight;
  }
  set thumbnailHeight(value: number | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_height"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailHeight = value;
  }
  _thumbnailHeight: number | null;

  /**
   * File.content
   */
  /**
   * File.content
   */
  get content(): Uint8Array | null {
    return this._content;
  }
  set content(value: Uint8Array | null) {
    const prop = (this.constructor as NodeClass).__properties__["content"];
    this._session.updateSetProperty(this, prop, value);
    this._content = value;
  }
  _content: Uint8Array | null;

  constructor(options: {
    id?: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    definition?: (Entity & IsExtensible) | NodeReference | null;
    materialization?: Materialization;
    snapshot?: Snapshot | NodeReference | null;
    predecessor?: File | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: { readonly [key: string]: Value };
    status?: ResourceStatus;
    name?: string;
    script?: Script | NodeReference | null;
    isExtensible?: boolean;
    type: FileType;
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
    );

    // properties
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    if (_space === null) {
      if (this._session === null) {
        throw new Error(`File has no Session`);
      }
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`File has no Space`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`File.space is required`);
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`File.materialization is required`);
    }
    this.materialization = _materialization;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    this.snapshotPtr = _snapshot;
    let _predecessor = options.predecessor ?? null;
    if (_predecessor != null && _predecessor.metatype != StructType.NODE_REFERENCE) {
      _predecessor = (_predecessor as Node).toRef();
    }
    this.predecessorPtr = _predecessor;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`File.status is required`);
    }
    this._status = _status;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "File";
    }
    if (_name === null) {
      throw new Error(`File.name is required`);
    }
    this._name = _name;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    if (_isExtensible === null) {
      _isExtensible = false;
    }
    if (_isExtensible === null) {
      throw new Error(`File.isExtensible is required`);
    }
    this.isExtensible = _isExtensible;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`File.type is required`);
    }
    this._type = _type;
    let _source = options.source;
    if (_source === null) {
      throw new Error(`File.source is required`);
    }
    this._source = _source;
    let _mimeType = options.mimeType ?? null;
    this._mimeType = _mimeType;
    let _format = options.format ?? null;
    this._format = _format;
    let _size = options.size ?? null;
    this._size = _size;
    let _sha256 = options.sha256 ?? null;
    this._sha256 = _sha256;
    let _width = options.width ?? null;
    this._width = _width;
    let _height = options.height ?? null;
    this._height = _height;
    let _aspectRatio = options.aspectRatio ?? null;
    this._aspectRatio = _aspectRatio;
    let _codec = options.codec ?? null;
    this._codec = _codec;
    let _duration = options.duration ?? null;
    this._duration = _duration;
    let _url = options.url ?? null;
    this._url = _url;
    let _contentUrl = options.contentUrl ?? null;
    this._contentUrl = _contentUrl;
    let _thumbnailUrl = options.thumbnailUrl ?? null;
    this._thumbnailUrl = _thumbnailUrl;
    let _faviconUrl = options.faviconUrl ?? null;
    this._faviconUrl = _faviconUrl;
    let _thumbnailWidth = options.thumbnailWidth ?? null;
    this._thumbnailWidth = _thumbnailWidth;
    let _thumbnailHeight = options.thumbnailHeight ?? null;
    this._thumbnailHeight = _thumbnailHeight;
    let _content = options.content ?? null;
    this._content = _content;

    // identity
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      this.createdAt = now;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedByPtr = null;
    } else {
      if (options.createdAt == null || options.updatedAt == null) {
        throw new Error(`File.createdAt and File.updatedAt are required for existing Nodes`);
      }
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
    }
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this._type === other._type)) {
      return false;
    }
    if (!(this._source === other._source)) {
      return false;
    }
    if (!(this._mimeType === other._mimeType)) {
      return false;
    }
    if (!(this._format === other._format)) {
      return false;
    }
    if (!(this._size === other._size)) {
      return false;
    }
    if (!(this._sha256 === other._sha256)) {
      return false;
    }
    if (!(this._width === other._width)) {
      return false;
    }
    if (!(this._height === other._height)) {
      return false;
    }
    if (
      (this._aspectRatio == null) !== (other._aspectRatio == null) ||
      (this._aspectRatio != null &&
        !(
          this._aspectRatio === other._aspectRatio ||
          Math.abs(this._aspectRatio - other._aspectRatio) < 1e-10
        ))
    ) {
      return false;
    }
    if (!(this._codec === other._codec)) {
      return false;
    }
    if (!(this._duration === other._duration)) {
      return false;
    }
    if (!(this._url === other._url)) {
      return false;
    }
    if (!(this._contentUrl === other._contentUrl)) {
      return false;
    }
    if (!(this._thumbnailUrl === other._thumbnailUrl)) {
      return false;
    }
    if (!(this._faviconUrl === other._faviconUrl)) {
      return false;
    }
    if (!(this._thumbnailWidth === other._thumbnailWidth)) {
      return false;
    }
    if (!(this._thumbnailHeight === other._thumbnailHeight)) {
      return false;
    }
    if (!(this._content === other._content)) {
      return false;
    }
    if (!(this._status === other._status)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.snapshotPtr?.id === other.snapshotPtr?.id)) {
      return false;
    }
    if (!(this.predecessorPtr?.id === other.predecessorPtr?.id)) {
      return false;
    }
    if (!(this._name === other._name)) {
      return false;
    }
    if (Object.keys(this._customValues).length !== Object.keys(other._customValues).length) {
      return false;
    }
    for (const key in this._customValues) {
      if (!(key in other._customValues)) {
        return false;
      }
      if (!this._customValues[key].equals(other._customValues[key])) {
        return false;
      }
    }
    if (!(this._scriptPtr?.id === other._scriptPtr?.id)) {
      return false;
    }
    if (!(this.spacePtr.id === other.spacePtr.id)) {
      return false;
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.parentPtr != null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this._type) & 0xffffffff;
    h = (h * 31 + this._source) & 0xffffffff;
    if (this._mimeType != null) {
      h = (h * 31 + hashString(this._mimeType)) & 0xffffffff;
    }
    if (this._format != null) {
      h = (h * 31 + this._format) & 0xffffffff;
    }
    if (this._size != null) {
      h = (h * 31 + hashInt(this._size)) & 0xffffffff;
    }
    if (this._sha256 != null) {
      h = (h * 31 + hashString(this._sha256)) & 0xffffffff;
    }
    if (this._width != null) {
      h = (h * 31 + hashInt(this._width)) & 0xffffffff;
    }
    if (this._height != null) {
      h = (h * 31 + hashInt(this._height)) & 0xffffffff;
    }
    if (this._aspectRatio != null) {
      h = (h * 31 + hashFloat(this._aspectRatio)) & 0xffffffff;
    }
    if (this._codec != null) {
      h = (h * 31 + hashString(this._codec)) & 0xffffffff;
    }
    if (this._duration != null) {
      h = (h * 31 + hashFloat(this._duration.total("seconds"))) & 0xffffffff;
    }
    if (this._url != null) {
      h = (h * 31 + hashString(this._url)) & 0xffffffff;
    }
    if (this._contentUrl != null) {
      h = (h * 31 + hashString(this._contentUrl)) & 0xffffffff;
    }
    if (this._thumbnailUrl != null) {
      h = (h * 31 + hashString(this._thumbnailUrl)) & 0xffffffff;
    }
    if (this._faviconUrl != null) {
      h = (h * 31 + hashString(this._faviconUrl)) & 0xffffffff;
    }
    if (this._thumbnailWidth != null) {
      h = (h * 31 + hashInt(this._thumbnailWidth)) & 0xffffffff;
    }
    if (this._thumbnailHeight != null) {
      h = (h * 31 + hashInt(this._thumbnailHeight)) & 0xffffffff;
    }
    if (this._content != null) {
      h = (h * 31 + hashBytes(this._content)) & 0xffffffff;
    }
    h = (h * 31 + this._status) & 0xffffffff;
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.snapshotPtr != null) {
      h = (h * 31 + hashString(this.snapshotPtr.id)) & 0xffffffff;
    }
    if (this.predecessorPtr != null) {
      h = (h * 31 + hashString(this.predecessorPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;

    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  __toRef__(): NodeReference {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new _NodeReference({
      type: NodeType.FILE,
      id: this.id,
      spaceId: this.spacePtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
      definitionId: this.definitionPtr?.id ?? null,
      _session: this._session,
      _supergraph: this._supergraph,
    });
  }

  get _pathKey(): string {
    return this.name;
  }

  get path(): string {
    const pathParts: string[] = [];
    let node: Entity | Event | null = this;
    let lastNode: Entity | Event | null = this;
    while (node != null) {
      pathParts.push(node._pathKey);
      lastNode = node;
      node = node.parent;
    }
    if (!lastNode.isRoot) {
      pathParts.push("<detached>");
    }
    return pathParts.reverse().join("/");
  }

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${FileType[this.type]}`);
    propertyReprs.push(`source=${FileSource[this.source]}`);
    if (this.mimeType != null) {
      propertyReprs.push(`mimeType=${`"${this.mimeType}"`}`);
    }
    if (this.format != null) {
      propertyReprs.push(`format=${FileFormat[this.format]}`);
    }
    if (this.size != null) {
      propertyReprs.push(`size=${this.size}`);
    }
    if (this.url != null) {
      propertyReprs.push(`url=${`"${this.url}"`}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<File "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toValue(): { readonly [key: string]: any } {
    return File.__packValue__(this);
  }

  static __packValue__(object: File): { readonly [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 480000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    objectValue["5"] = object.spacePtr.toValue();
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    objectValue["10"] = object.materialization;
    if (object.snapshotPtr != null) {
      objectValue["11"] = object.snapshotPtr.toValue();
    }
    if (object.predecessorPtr != null) {
      objectValue["12"] = object.predecessorPtr.toValue();
    }
    objectValue["20"] = object.createdAt.toString({ timeZoneName: "never" });
    if (object.createdByPtr != null) {
      objectValue["21"] = object.createdByPtr.toValue();
    }
    objectValue["22"] = object.updatedAt.toString({ timeZoneName: "never" });
    if (object.updatedByPtr != null) {
      objectValue["23"] = object.updatedByPtr.toValue();
    }
    if (object.deletedAt != null) {
      objectValue["25"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["40"] = object._status;
    objectValue["50"] = object._name;
    if (object._scriptPtr != null) {
      objectValue["80"] = object._scriptPtr.toValue();
    }
    objectValue["90"] = object.isExtensible;
    objectValue["100"] = object._type;
    objectValue["110"] = object._source;
    if (object._mimeType != null) {
      objectValue["111"] = object._mimeType;
    }
    if (object._format != null) {
      objectValue["112"] = object._format;
    }
    if (object._size != null) {
      objectValue["113"] = object._size;
    }
    if (object._sha256 != null) {
      objectValue["114"] = object._sha256;
    }
    if (object._width != null) {
      objectValue["115"] = object._width;
    }
    if (object._height != null) {
      objectValue["116"] = object._height;
    }
    if (object._aspectRatio != null) {
      objectValue["117"] = object._aspectRatio;
    }
    if (object._codec != null) {
      objectValue["118"] = object._codec;
    }
    if (object._duration != null) {
      objectValue["119"] = timedeltaToISOFormat(object._duration);
    }
    if (object._url != null) {
      objectValue["120"] = object._url;
    }
    if (object._contentUrl != null) {
      objectValue["121"] = object._contentUrl;
    }
    if (object._thumbnailUrl != null) {
      objectValue["122"] = object._thumbnailUrl;
    }
    if (object._faviconUrl != null) {
      objectValue["123"] = object._faviconUrl;
    }
    if (object._thumbnailWidth != null) {
      objectValue["124"] = object._thumbnailWidth;
    }
    if (object._thumbnailHeight != null) {
      objectValue["125"] = object._thumbnailHeight;
    }
    if (object._content != null) {
      objectValue["126"] = base64Encode(object._content);
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const mimeTypeValue = objectValue["111"];
    const unpackedMimeType = mimeTypeValue != undefined ? mimeTypeValue : null;
    const formatValue = objectValue["112"];
    const unpackedFormat = formatValue != undefined ? Number(formatValue) : null;
    const sizeValue = objectValue["113"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const sha256Value = objectValue["114"];
    const unpackedSha256 = sha256Value != undefined ? sha256Value : null;
    const widthValue = objectValue["115"];
    const unpackedWidth = widthValue != undefined ? Number(widthValue) : null;
    const heightValue = objectValue["116"];
    const unpackedHeight = heightValue != undefined ? Number(heightValue) : null;
    const aspectRatioValue = objectValue["117"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const codecValue = objectValue["118"];
    const unpackedCodec = codecValue != undefined ? codecValue : null;
    const durationValue = objectValue["119"];
    const unpackedDuration =
      durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const urlValue = objectValue["120"];
    const unpackedUrl = urlValue != undefined ? urlValue : null;
    const contentUrlValue = objectValue["121"];
    const unpackedContentUrl = contentUrlValue != undefined ? contentUrlValue : null;
    const thumbnailUrlValue = objectValue["122"];
    const unpackedThumbnailUrl = thumbnailUrlValue != undefined ? thumbnailUrlValue : null;
    const faviconUrlValue = objectValue["123"];
    const unpackedFaviconUrl = faviconUrlValue != undefined ? faviconUrlValue : null;
    const thumbnailWidthValue = objectValue["124"];
    const unpackedThumbnailWidth =
      thumbnailWidthValue != undefined ? Number(thumbnailWidthValue) : null;
    const thumbnailHeightValue = objectValue["125"];
    const unpackedThumbnailHeight =
      thumbnailHeightValue != undefined ? Number(thumbnailHeightValue) : null;
    const contentValue = objectValue["126"];
    const unpackedContent = contentValue != undefined ? base64Decode(contentValue) : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const snapshotPtrValue = objectValue["11"];
    const unpackedSnapshotPtr =
      snapshotPtrValue != undefined
        ? _NodeReference.fromValue(snapshotPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const predecessorPtrValue = objectValue["12"];
    const unpackedPredecessorPtr =
      predecessorPtrValue != undefined
        ? _NodeReference.fromValue(predecessorPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectValue["21"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromValue(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectValue["23"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromValue(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues[String(key)] = _Value.fromValue(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectValue["80"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromValue(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    return new File({
      parent: unpackedParentPtr,
      type: Number(objectValue["100"]),
      source: Number(objectValue["110"]),
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
      status: Number(objectValue["40"]),
      definition: unpackedDefinitionPtr,
      isExtensible: objectValue["90"],
      deletedAt: unpackedDeletedAt,
      materialization: Number(objectValue["10"]),
      snapshot: unpackedSnapshotPtr,
      predecessor: unpackedPredecessorPtr,
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      name: objectValue["50"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      id: String(objectValue["2"]),
      space: _NodeReference.fromValue(objectValue["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromValue(
    objectValue: { readonly [key: string]: any },
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
    const objectProto: Partial<FileProto> = { metatype: 480000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    objectProto.spacePtr = object.spacePtr.toProto();
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.snapshotPtr != null) {
      objectProto.snapshotPtr = object.snapshotPtr.toProto();
    }
    if (object.predecessorPtr != null) {
      objectProto.predecessorPtr = object.predecessorPtr.toProto();
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
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object._status) as ResourceStatusProto;
    objectProto.name = object._name;
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    objectProto.isExtensible = object.isExtensible;
    objectProto.type = Number(object._type) as FileTypeProto;
    objectProto.source = Number(object._source) as FileSourceProto;
    if (object._mimeType != null) {
      objectProto.mimeType = object._mimeType;
    }
    if (object._format != null) {
      objectProto.format = Number(object._format) as FileFormatProto;
    }
    if (object._size != null) {
      objectProto.size = object._size;
    }
    if (object._sha256 != null) {
      objectProto.sha256 = object._sha256;
    }
    if (object._width != null) {
      objectProto.width = object._width;
    }
    if (object._height != null) {
      objectProto.height = object._height;
    }
    if (object._aspectRatio != null) {
      objectProto.aspectRatio = object._aspectRatio;
    }
    if (object._codec != null) {
      objectProto.codec = object._codec;
    }
    if (object._duration != null) {
      objectProto.duration = packProtoDuration(object._duration);
    }
    if (object._url != null) {
      objectProto.url = object._url;
    }
    if (object._contentUrl != null) {
      objectProto.contentUrl = object._contentUrl;
    }
    if (object._thumbnailUrl != null) {
      objectProto.thumbnailUrl = object._thumbnailUrl;
    }
    if (object._faviconUrl != null) {
      objectProto.faviconUrl = object._faviconUrl;
    }
    if (object._thumbnailWidth != null) {
      objectProto.thumbnailWidth = object._thumbnailWidth;
    }
    if (object._thumbnailHeight != null) {
      objectProto.thumbnailHeight = object._thumbnailHeight;
    }
    if (object._content != null) {
      objectProto.content = object._content;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = {} as any;
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new File({
      parent:
        objectProto.parentPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.parentPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
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
      duration:
        objectProto.duration != undefined ? unpackProtoDuration(objectProto.duration!) : null,
      url: objectProto.url != undefined ? objectProto.url : null,
      contentUrl: objectProto.contentUrl != undefined ? objectProto.contentUrl : null,
      thumbnailUrl: objectProto.thumbnailUrl != undefined ? objectProto.thumbnailUrl : null,
      faviconUrl: objectProto.faviconUrl != undefined ? objectProto.faviconUrl : null,
      thumbnailWidth:
        objectProto.thumbnailWidth != undefined ? Number(objectProto.thumbnailWidth) : null,
      thumbnailHeight:
        objectProto.thumbnailHeight != undefined ? Number(objectProto.thumbnailHeight) : null,
      content: objectProto.content != undefined ? objectProto.content : null,
      status: Number(objectProto.status) as ResourceStatus,
      definition:
        objectProto.definitionPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.definitionPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      isExtensible: objectProto.isExtensible,
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      materialization: Number(objectProto.materialization) as Materialization,
      snapshot:
        objectProto.snapshotPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.snapshotPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      predecessor:
        objectProto.predecessorPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.predecessorPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdBy:
        objectProto.createdByPtr != undefined
          ? _NodeReference.fromProto(
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
          ? _NodeReference.fromProto(
              objectProto.updatedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      customValues: unpackedCustomValues,
      script:
        objectProto.scriptPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.scriptPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      id: String(objectProto.id),
      space: _NodeReference.fromProto(
        objectProto.spacePtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
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

  static fromProtoString(packedProtoString: string): File {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = FileProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILE, File);
/* ==== DESTACK_GENERATED_END:NODE:480000 ==== */
