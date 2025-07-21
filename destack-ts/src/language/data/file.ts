import {
  packProtoDuration,
  packProtoTimestamp,
  unpackProtoDuration,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  Branch,
  Graph,
  IsActor,
  NodeClass,
  NodeReference,
  QueryConnection,
  Session,
  Snapshot,
  Space,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  Entity,
  EnumType,
  Event,
  Materialization,
  Node,
  NodeType,
  Region,
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
import {
  FileFormatProto,
  FileProto,
  FileTypeProto,
  MaterializationProto,
  RegionProto,
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
   * Entity.materialization
   */
  readonly materialization: Materialization;

  /**
   * The definition this Entity is an instance of.
   */
  get definition(): Entity | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * The Branch this Entity is part of.
   */
  get branch(): Branch | null {
    const nodePtr: NodeReference | null = this.branchPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Branch | null;
    }
    return null;
  }
  readonly branchPtr: NodeReference;

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
  readonly snapshotPtr: NodeReference;

  /**
   * The previous Entity this Entity is based on (from the base Branch, if any).
   * This invariant must hold: `Entity.preceded_by.branch == Entity.branch.preceded_by`.
   */
  get precededBy(): File | null {
    const nodePtr: NodeReference | null = this.precededByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as File | null;
    }
    return null;
  }
  readonly precededByPtr: NodeReference | null;

  /**
   * The (root) Entity that is being instantiated.
   */
  get instance(): Entity | null {
    const nodePtr: NodeReference | null = this.instancePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: number;

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
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Temporal.ZonedDateTime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: number;

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
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Temporal.ZonedDateTime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): (Entity & IsActor) | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as (Entity & IsActor) | null;
    }
    return null;
  }
  set ownedBy(node: (Entity & IsActor) | null) {
    if (node === null) {
      this.ownedByPtr = null;
    } else {
      this.ownedByPtr = node.toRef();
    }
  }
  /**
   * Entity.ownedBy
   */
  get ownedByPtr(): NodeReference | null {
    return this._ownedByPtr;
  }
  set ownedByPtr(value: NodeReference | null) {
    const prop = (this.constructor as NodeClass).__properties__["owned_by"];
    this._session.updateSetProperty(this, prop, value);
    this._ownedByPtr = value;
  }
  _ownedByPtr: NodeReference | null;

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
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: string;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
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
   * The Script of this Entity.
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
   * The Script of this Entity.
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
   * Whether this Entity can be instanced.
   */
  readonly isExtensible: boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._supergraph.get(nodePtr.id) as Script | null;
    }
    return null;
  }
  readonly sourcePtr: NodeReference | null;

  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  /**
   * The key to uniquely identify this Node in reconciliation. If not set, name is used.
   */
  get key(): string | null {
    return this._key;
  }
  set key(value: string | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: string | null;

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
   * Resource.status
   */
  /**
   * Resource.status
   */
  get status(): ResourceStatus | null {
    return this._status;
  }
  set status(value: ResourceStatus | null) {
    const prop = (this.constructor as NodeClass).__properties__["status"];
    this._session.updateSetProperty(this, prop, value);
    this._status = value;
  }
  _status: ResourceStatus | null;

  /**
   * Resource.region
   */
  /**
   * Resource.region
   */
  get region(): Region | null {
    return this._region;
  }
  set region(value: Region | null) {
    const prop = (this.constructor as NodeClass).__properties__["region"];
    this._session.updateSetProperty(this, prop, value);
    this._region = value;
  }
  _region: Region | null;

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
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: File | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Temporal.ZonedDateTime;
    createdEpoch?: number;
    createdBy?: (Entity & IsActor) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedEpoch?: number;
    updatedBy?: (Entity & IsActor) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    ownedBy?: (Entity & IsActor) | NodeReference | null;
    name?: string;
    orderKey?: string;
    customValues?: { readonly [key: string]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: boolean | null;
    source?: Script | NodeReference | null;
    key?: string | null;
    type: FileType;
    status?: ResourceStatus | null;
    region?: Region | null;
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
      _space = ACTIVE_SPACE.get();
      if (_space === null) {
        throw new Error(`no active Space for File`);
      }
      _space = _space.toRef();
    }
    if (_space === null) {
      throw new Error(`File.space is required`);
    }
    this.spacePtr = _space;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`File.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.metatype != StructType.NODE_REFERENCE) {
      _branch = (_branch as Node).toRef();
    }
    if (_branch === null) {
      _branch = ACTIVE_BRANCH.get();
      if (_branch === null) {
        throw new Error(`no active Branch for File`);
      }
      _branch = _branch.toRef();
    }
    if (_branch === null) {
      throw new Error(`File.branch is required`);
    }
    this.branchPtr = _branch;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.metatype != StructType.NODE_REFERENCE) {
      _snapshot = (_snapshot as Node).toRef();
    }
    if (_snapshot === null) {
      _snapshot = ACTIVE_SNAPSHOT.get();
      if (_snapshot === null) {
        throw new Error(`no active Snapshot for File`);
      }
      _snapshot = _snapshot.toRef();
    }
    if (_snapshot === null) {
      throw new Error(`File.snapshot is required`);
    }
    this.snapshotPtr = _snapshot;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.metatype != StructType.NODE_REFERENCE) {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.metatype != StructType.NODE_REFERENCE) {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.metatype != StructType.NODE_REFERENCE) {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy;
    let _name = options.name ?? null;
    if (_name === null) {
      _name = "File";
    }
    if (_name === null) {
      throw new Error(`File.name is required`);
    }
    this._name = _name;
    let _orderKey = options.orderKey ?? null;
    if (_orderKey === null) {
      _orderKey = "a0";
    }
    if (_orderKey === null) {
      throw new Error(`File.orderKey is required`);
    }
    this.orderKey = _orderKey;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = {};
    }
    this._customValues = _customValues;
    let _script = options.script ?? null;
    if (_script != null && _script.metatype != StructType.NODE_REFERENCE) {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.metatype != StructType.NODE_REFERENCE) {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`File.type is required`);
    }
    this._type = _type;
    let _status = options.status ?? null;
    this._status = _status;
    let _region = options.region ?? null;
    this._region = _region;
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
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = null;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = null;
    } else {
      if (
        options.createdAt == null ||
        options.updatedAt == null ||
        options.createdEpoch == null ||
        options.updatedEpoch == null
      ) {
        throw new Error(`File.createdAt and File.updatedAt are required for existing Nodes`);
      }
      this.createdAt = options.createdAt;
      this.createdEpoch = options.createdEpoch;
      this.createdByPtr =
        options.createdBy != null
          ? options.createdBy.metatype == StructType.NODE_REFERENCE
            ? (options.createdBy as NodeReference)
            : (options.createdBy as Node).toRef()
          : null;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
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
    if (!(this._region === other._region)) {
      return false;
    }
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (!(this._ownedByPtr?.id === other._ownedByPtr?.id)) {
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
    if (!(this.isExtensible === other.isExtensible)) {
      return false;
    }
    if (!(this.sourcePtr?.id === other.sourcePtr?.id)) {
      return false;
    }
    if (!(this._key === other._key)) {
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
    if (this._status != null) {
      h = (h * 31 + this._status) & 0xffffffff;
    }
    if (this._region != null) {
      h = (h * 31 + this._region) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr != null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr != null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt != null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this._ownedByPtr != null) {
      h = (h * 31 + hashString(this._ownedByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this._name)) & 0xffffffff;
    h = (h * 31 + hashString(this.orderKey)) & 0xffffffff;
    if (this._customValues && Object.keys(this._customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this._customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }
    if (this._scriptPtr != null) {
      h = (h * 31 + hashString(this._scriptPtr.id)) & 0xffffffff;
    }
    if (this.isExtensible != null) {
      h = (h * 31 + hashBool(this.isExtensible)) & 0xffffffff;
    }
    if (this.sourcePtr != null) {
      h = (h * 31 + hashString(this.sourcePtr.id)) & 0xffffffff;
    }
    if (this._key != null) {
      h = (h * 31 + hashString(this._key)) & 0xffffffff;
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
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr?.id ?? null,
      snapshotId: this.snapshotPtr?.id ?? null,
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
    if (this.ownedBy != null) {
      propertyReprs.push(`ownedBy=${this.ownedBy?.repr()}`);
    }
    propertyReprs.push(`name=${`"${this.name}"`}`);
    return `<File "${this.path}" ${propertyReprs.join(" ")}>`;
  }

  toCson(): { [key: string]: any } {
    return File.__packCson__(this);
  }

  static __packCson__(object: File): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 480000;
    objectCson["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectCson["3"] = object.parentPtr.toCson();
    }
    objectCson["5"] = object.spacePtr.toCson();
    objectCson["10"] = object.materialization;
    if (object.definitionPtr != null) {
      objectCson["11"] = object.definitionPtr.toCson();
    }
    objectCson["12"] = object.branchPtr.toCson();
    objectCson["13"] = object.snapshotPtr.toCson();
    if (object.precededByPtr != null) {
      objectCson["14"] = object.precededByPtr.toCson();
    }
    if (object.instancePtr != null) {
      objectCson["15"] = object.instancePtr.toCson();
    }
    objectCson["20"] = object.createdAt.toString({ timeZoneName: "never" });
    objectCson["21"] = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectCson["22"] = object.createdByPtr.toCson();
    }
    objectCson["23"] = object.updatedAt.toString({ timeZoneName: "never" });
    objectCson["24"] = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectCson["25"] = object.updatedByPtr.toCson();
    }
    if (object.deletedAt != null) {
      objectCson["26"] = object.deletedAt.toString({ timeZoneName: "never" });
    }
    if (object._ownedByPtr != null) {
      objectCson["30"] = object._ownedByPtr.toCson();
    }
    objectCson["40"] = object._name;
    objectCson["41"] = object.orderKey;
    if (Object.keys(object._customValues).length > 0) {
      const packedCustomValues: { [key: string]: any } = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        packedCustomValues[String(String(key))] = value.toCson();
      }
      objectCson["45"] = packedCustomValues;
    }
    if (object._scriptPtr != null) {
      objectCson["46"] = object._scriptPtr.toCson();
    }
    if (object.isExtensible != null) {
      objectCson["50"] = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectCson["80"] = object.sourcePtr.toCson();
    }
    if (object._key != null) {
      objectCson["85"] = object._key;
    }
    objectCson["100"] = object._type;
    if (object._status != null) {
      objectCson["110"] = object._status;
    }
    if (object._region != null) {
      objectCson["111"] = object._region;
    }
    if (object._mimeType != null) {
      objectCson["120"] = object._mimeType;
    }
    if (object._format != null) {
      objectCson["121"] = object._format;
    }
    if (object._size != null) {
      objectCson["122"] = object._size;
    }
    if (object._sha256 != null) {
      objectCson["123"] = object._sha256;
    }
    if (object._width != null) {
      objectCson["124"] = object._width;
    }
    if (object._height != null) {
      objectCson["125"] = object._height;
    }
    if (object._aspectRatio != null) {
      objectCson["126"] = object._aspectRatio;
    }
    if (object._codec != null) {
      objectCson["127"] = object._codec;
    }
    if (object._duration != null) {
      objectCson["128"] = timedeltaToISOFormat(object._duration);
    }
    if (object._url != null) {
      objectCson["130"] = object._url;
    }
    if (object._contentUrl != null) {
      objectCson["131"] = object._contentUrl;
    }
    if (object._thumbnailUrl != null) {
      objectCson["132"] = object._thumbnailUrl;
    }
    if (object._faviconUrl != null) {
      objectCson["133"] = object._faviconUrl;
    }
    if (object._thumbnailWidth != null) {
      objectCson["134"] = object._thumbnailWidth;
    }
    if (object._thumbnailHeight != null) {
      objectCson["135"] = object._thumbnailHeight;
    }
    if (object._content != null) {
      objectCson["136"] = base64Encode(object._content);
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const parentPtrValue = objectCson["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromCson(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const mimeTypeValue = objectCson["120"];
    const unpackedMimeType = mimeTypeValue != undefined ? mimeTypeValue : null;
    const formatValue = objectCson["121"];
    const unpackedFormat = formatValue != undefined ? Number(formatValue) : null;
    const sizeValue = objectCson["122"];
    const unpackedSize = sizeValue != undefined ? Number(sizeValue) : null;
    const sha256Value = objectCson["123"];
    const unpackedSha256 = sha256Value != undefined ? sha256Value : null;
    const widthValue = objectCson["124"];
    const unpackedWidth = widthValue != undefined ? Number(widthValue) : null;
    const heightValue = objectCson["125"];
    const unpackedHeight = heightValue != undefined ? Number(heightValue) : null;
    const aspectRatioValue = objectCson["126"];
    const unpackedAspectRatio = aspectRatioValue != undefined ? aspectRatioValue : null;
    const codecValue = objectCson["127"];
    const unpackedCodec = codecValue != undefined ? codecValue : null;
    const durationValue = objectCson["128"];
    const unpackedDuration =
      durationValue != undefined ? timedeltaFromISOFormat(durationValue) : null;
    const urlValue = objectCson["130"];
    const unpackedUrl = urlValue != undefined ? urlValue : null;
    const contentUrlValue = objectCson["131"];
    const unpackedContentUrl = contentUrlValue != undefined ? contentUrlValue : null;
    const thumbnailUrlValue = objectCson["132"];
    const unpackedThumbnailUrl = thumbnailUrlValue != undefined ? thumbnailUrlValue : null;
    const faviconUrlValue = objectCson["133"];
    const unpackedFaviconUrl = faviconUrlValue != undefined ? faviconUrlValue : null;
    const thumbnailWidthValue = objectCson["134"];
    const unpackedThumbnailWidth =
      thumbnailWidthValue != undefined ? Number(thumbnailWidthValue) : null;
    const thumbnailHeightValue = objectCson["135"];
    const unpackedThumbnailHeight =
      thumbnailHeightValue != undefined ? Number(thumbnailHeightValue) : null;
    const contentValue = objectCson["136"];
    const unpackedContent = contentValue != undefined ? base64Decode(contentValue) : null;
    const statusValue = objectCson["110"];
    const unpackedStatus = statusValue != undefined ? Number(statusValue) : null;
    const regionValue = objectCson["111"];
    const unpackedRegion = regionValue != undefined ? Number(regionValue) : null;
    const definitionPtrValue = objectCson["11"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromCson(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const precededByPtrValue = objectCson["14"];
    const unpackedPrecededByPtr =
      precededByPtrValue != undefined
        ? _NodeReference.fromCson(precededByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const instancePtrValue = objectCson["15"];
    const unpackedInstancePtr =
      instancePtrValue != undefined
        ? _NodeReference.fromCson(instancePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const createdByPtrValue = objectCson["22"];
    const unpackedCreatedByPtr =
      createdByPtrValue != undefined
        ? _NodeReference.fromCson(createdByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const updatedByPtrValue = objectCson["25"];
    const unpackedUpdatedByPtr =
      updatedByPtrValue != undefined
        ? _NodeReference.fromCson(updatedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectCson["26"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const ownedByPtrValue = objectCson["30"];
    const unpackedOwnedByPtr =
      ownedByPtrValue != undefined
        ? _NodeReference.fromCson(ownedByPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const unpackedCustomValues = {} as any;
    if (objectCson["45"] != undefined) {
      for (const [key, value] of Object.entries(objectCson["45"])) {
        unpackedCustomValues[String(key)] = _Value.fromCson(
          value as any,
          _session,
          _supergraph,
          _graph,
          _connection,
        );
      }
    }
    const scriptPtrValue = objectCson["46"];
    const unpackedScriptPtr =
      scriptPtrValue != undefined
        ? _NodeReference.fromCson(scriptPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const isExtensibleValue = objectCson["50"];
    const unpackedIsExtensible = isExtensibleValue != undefined ? isExtensibleValue : null;
    const sourcePtrValue = objectCson["80"];
    const unpackedSourcePtr =
      sourcePtrValue != undefined
        ? _NodeReference.fromCson(sourcePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const keyValue = objectCson["85"];
    const unpackedKey = keyValue != undefined ? keyValue : null;
    return new File({
      parent: unpackedParentPtr,
      type: Number(objectCson["100"]),
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
      status: unpackedStatus,
      region: unpackedRegion,
      materialization: Number(objectCson["10"]),
      definition: unpackedDefinitionPtr,
      branch: _NodeReference.fromCson(objectCson["12"], _session, _supergraph, _graph, _connection),
      snapshot: _NodeReference.fromCson(
        objectCson["13"],
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy: unpackedPrecededByPtr,
      instance: unpackedInstancePtr,
      createdAt: Temporal.Instant.from(objectCson["20"]).toZonedDateTimeISO("UTC"),
      createdEpoch: Number(objectCson["21"]),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectCson["23"]).toZonedDateTimeISO("UTC"),
      updatedEpoch: Number(objectCson["24"]),
      updatedBy: unpackedUpdatedByPtr,
      deletedAt: unpackedDeletedAt,
      ownedBy: unpackedOwnedByPtr,
      name: objectCson["40"],
      orderKey: objectCson["41"],
      customValues: unpackedCustomValues,
      script: unpackedScriptPtr,
      isExtensible: unpackedIsExtensible,
      source: unpackedSourcePtr,
      key: unpackedKey,
      id: String(objectCson["2"]),
      space: _NodeReference.fromCson(objectCson["5"], _session, _supergraph, _graph, _connection),
      _session,
      _graph,
      _connection,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): File {
    return File.__unpackCson__(objectCson, _session, _supergraph, _graph, _connection);
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
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    objectProto.branchPtr = object.branchPtr.toProto();
    objectProto.snapshotPtr = object.snapshotPtr.toProto();
    if (object.precededByPtr != null) {
      objectProto.precededByPtr = object.precededByPtr.toProto();
    }
    if (object.instancePtr != null) {
      objectProto.instancePtr = object.instancePtr.toProto();
    }
    objectProto.createdAt = packProtoTimestamp(object.createdAt);
    objectProto.createdEpoch = object.createdEpoch;
    if (object.createdByPtr != null) {
      objectProto.createdByPtr = object.createdByPtr.toProto();
    }
    objectProto.updatedAt = packProtoTimestamp(object.updatedAt);
    objectProto.updatedEpoch = object.updatedEpoch;
    if (object.updatedByPtr != null) {
      objectProto.updatedByPtr = object.updatedByPtr.toProto();
    }
    if (object.deletedAt != null) {
      objectProto.deletedAt = packProtoTimestamp(object.deletedAt);
    }
    if (object._ownedByPtr != null) {
      objectProto.ownedByPtr = object._ownedByPtr.toProto();
    }
    objectProto.name = object._name;
    objectProto.orderKey = object.orderKey;
    if (object._customValues) {
      objectProto.customValues = {} as any;
      for (const [key, value] of Object.entries(object._customValues)) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    if (object._scriptPtr != null) {
      objectProto.scriptPtr = object._scriptPtr.toProto();
    }
    if (object.isExtensible != null) {
      objectProto.isExtensible = object.isExtensible;
    }
    if (object.sourcePtr != null) {
      objectProto.sourcePtr = object.sourcePtr.toProto();
    }
    if (object._key != null) {
      objectProto.key = object._key;
    }
    objectProto.type = Number(object._type) as FileTypeProto;
    if (object._status != null) {
      objectProto.status = Number(object._status) as ResourceStatusProto;
    }
    if (object._region != null) {
      objectProto.region = Number(object._region) as RegionProto;
    }
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
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
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
      status:
        objectProto.status != undefined ? (Number(objectProto.status) as ResourceStatus) : null,
      region: objectProto.region != undefined ? (Number(objectProto.region) as Region) : null,
      materialization: Number(objectProto.materialization) as Materialization,
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
      branch: _NodeReference.fromProto(
        objectProto.branchPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      snapshot: _NodeReference.fromProto(
        objectProto.snapshotPtr!,
        _session,
        _supergraph,
        _graph,
        _connection,
      ),
      precededBy:
        objectProto.precededByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.precededByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      instance:
        objectProto.instancePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.instancePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      createdAt: unpackProtoTimestamp(objectProto.createdAt!),
      createdEpoch: Number(objectProto.createdEpoch),
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
      updatedEpoch: Number(objectProto.updatedEpoch),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
      ownedBy:
        objectProto.ownedByPtr != undefined
          ? _NodeReference.fromProto(
              objectProto.ownedByPtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      name: objectProto.name,
      orderKey: objectProto.orderKey,
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
      isExtensible: objectProto.isExtensible != undefined ? objectProto.isExtensible : null,
      source:
        objectProto.sourcePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.sourcePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      key: objectProto.key != undefined ? objectProto.key : null,
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
