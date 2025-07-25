import type {
  Boolean,
  Branch,
  Bytes,
  Datetime,
  Duration,
  Float32,
  NodeClass,
  NodeReference,
  Session,
  Snapshot,
  Space,
  String,
  UInt32,
  UInt64,
  UInt128,
  UUID,
  Value,
} from "@destack/language/core";
import {
  ACTIVE_BRANCH,
  ACTIVE_SNAPSHOT,
  ACTIVE_SPACE,
  type Entity,
  EnumType,
  type Event,
  type Materialization,
  type Node,
  NodeType,
  type Region,
  Resource,
  StructType,
} from "@destack/language/core";
import type { Script } from "@destack/language/logic";
import {
  registerEnumClass,
  registerNodeClass,
  STRUCT_CLASS_BY_TYPE,
} from "@destack/language/registry";
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
      return this._session.graph.get(nodePtr) as Space | null;
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
      return this._session.graph.get(nodePtr) as Space | null;
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
      return this._session.graph.get(nodePtr) as Entity | null;
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
      return this._session.graph.get(nodePtr) as Branch | null;
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
      return this._session.graph.get(nodePtr) as Snapshot | null;
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
      return this._session.graph.get(nodePtr) as File | null;
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
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly instancePtr: NodeReference | null;

  /**
   * The time this Entity was created (system time).
   */
  readonly createdAt: Datetime;

  /**
   * The logical time this Entity was created (system time).
   */
  readonly createdEpoch: UInt128;

  /**
   * The Actor that created this Entity.
   */
  get createdBy(): Entity | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference;

  /**
   * The time this Entity was last updated (system time).
   */
  readonly updatedAt: Datetime;

  /**
   * The logical time this Entity was last updated (system time).
   */
  readonly updatedEpoch: UInt128;

  /**
   * The Actor that last updated this Entity.
   */
  get updatedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference;

  /**
   * The time this Entity was deleted (system time).
   * Only set if the Entity is currently 'deleted'.
   * Deleting and restoring an Entity counts as an update, and thus updates updated_at/updated_epoch.
   */
  readonly deletedAt: Datetime | null;

  /**
   * Entity.ownedBy
   */
  get ownedBy(): Entity | null {
    const nodePtr: NodeReference | null = this.ownedByPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Entity | null;
    }
    return null;
  }
  set ownedBy(node: Entity | null) {
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
  get name(): String {
    return this._name;
  }
  set name(value: String) {
    const prop = (this.constructor as NodeClass).__properties__["name"];
    this._session.updateSetProperty(this, prop, value);
    this._name = value;
  }
  _name: String;

  /**
   * The absolute order key of this Entity in its parent.
   */
  readonly orderKey: String;

  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  /**
   * The custom Values of this Entity, keyed by custom Property id..
   */
  get customValues(): { readonly [key: UUID]: Value } {
    return this._customValues;
  }
  set customValues(value: { readonly [key: UUID]: Value }) {
    const prop = (this.constructor as NodeClass).__properties__["custom_values"];
    this._session.updateSetProperty(this, prop, value);
    this._customValues = value;
  }
  _customValues: { readonly [key: UUID]: Value };

  /**
   * The Script of this Entity.
   */
  get script(): Script | null {
    const nodePtr: NodeReference | null = this.scriptPtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
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
  readonly isExtensible: Boolean | null;

  /**
   * The Script that defines this Node.
   */
  get source(): Script | null {
    const nodePtr: NodeReference | null = this.sourcePtr;
    if (nodePtr != null) {
      return this._session.graph.get(nodePtr) as Script | null;
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
  get key(): String | null {
    return this._key;
  }
  set key(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["key"];
    this._session.updateSetProperty(this, prop, value);
    this._key = value;
  }
  _key: String | null;

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
  get mimeType(): String | null {
    return this._mimeType;
  }
  set mimeType(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["mime_type"];
    this._session.updateSetProperty(this, prop, value);
    this._mimeType = value;
  }
  _mimeType: String | null;

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
  get size(): UInt64 | null {
    return this._size;
  }
  set size(value: UInt64 | null) {
    const prop = (this.constructor as NodeClass).__properties__["size"];
    this._session.updateSetProperty(this, prop, value);
    this._size = value;
  }
  _size: UInt64 | null;

  /**
   * File.sha256
   */
  /**
   * File.sha256
   */
  get sha256(): String | null {
    return this._sha256;
  }
  set sha256(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["sha256"];
    this._session.updateSetProperty(this, prop, value);
    this._sha256 = value;
  }
  _sha256: String | null;

  /**
   * File.width
   */
  /**
   * File.width
   */
  get width(): UInt32 | null {
    return this._width;
  }
  set width(value: UInt32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["width"];
    this._session.updateSetProperty(this, prop, value);
    this._width = value;
  }
  _width: UInt32 | null;

  /**
   * File.height
   */
  /**
   * File.height
   */
  get height(): UInt32 | null {
    return this._height;
  }
  set height(value: UInt32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["height"];
    this._session.updateSetProperty(this, prop, value);
    this._height = value;
  }
  _height: UInt32 | null;

  /**
   * File.aspectRatio
   */
  /**
   * File.aspectRatio
   */
  get aspectRatio(): Float32 | null {
    return this._aspectRatio;
  }
  set aspectRatio(value: Float32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["aspect_ratio"];
    this._session.updateSetProperty(this, prop, value);
    this._aspectRatio = value;
  }
  _aspectRatio: Float32 | null;

  /**
   * File.codec
   */
  /**
   * File.codec
   */
  get codec(): String | null {
    return this._codec;
  }
  set codec(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["codec"];
    this._session.updateSetProperty(this, prop, value);
    this._codec = value;
  }
  _codec: String | null;

  /**
   * File.duration
   */
  /**
   * File.duration
   */
  get duration(): Duration | null {
    return this._duration;
  }
  set duration(value: Duration | null) {
    const prop = (this.constructor as NodeClass).__properties__["duration"];
    this._session.updateSetProperty(this, prop, value);
    this._duration = value;
  }
  _duration: Duration | null;

  /**
   * File.url
   */
  /**
   * File.url
   */
  get url(): String | null {
    return this._url;
  }
  set url(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["url"];
    this._session.updateSetProperty(this, prop, value);
    this._url = value;
  }
  _url: String | null;

  /**
   * File.contentUrl
   */
  /**
   * File.contentUrl
   */
  get contentUrl(): String | null {
    return this._contentUrl;
  }
  set contentUrl(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["content_url"];
    this._session.updateSetProperty(this, prop, value);
    this._contentUrl = value;
  }
  _contentUrl: String | null;

  /**
   * File.thumbnailUrl
   */
  /**
   * File.thumbnailUrl
   */
  get thumbnailUrl(): String | null {
    return this._thumbnailUrl;
  }
  set thumbnailUrl(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_url"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailUrl = value;
  }
  _thumbnailUrl: String | null;

  /**
   * File.faviconUrl
   */
  /**
   * File.faviconUrl
   */
  get faviconUrl(): String | null {
    return this._faviconUrl;
  }
  set faviconUrl(value: String | null) {
    const prop = (this.constructor as NodeClass).__properties__["favicon_url"];
    this._session.updateSetProperty(this, prop, value);
    this._faviconUrl = value;
  }
  _faviconUrl: String | null;

  /**
   * File.thumbnailWidth
   */
  /**
   * File.thumbnailWidth
   */
  get thumbnailWidth(): UInt32 | null {
    return this._thumbnailWidth;
  }
  set thumbnailWidth(value: UInt32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_width"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailWidth = value;
  }
  _thumbnailWidth: UInt32 | null;

  /**
   * File.thumbnailHeight
   */
  /**
   * File.thumbnailHeight
   */
  get thumbnailHeight(): UInt32 | null {
    return this._thumbnailHeight;
  }
  set thumbnailHeight(value: UInt32 | null) {
    const prop = (this.constructor as NodeClass).__properties__["thumbnail_height"];
    this._session.updateSetProperty(this, prop, value);
    this._thumbnailHeight = value;
  }
  _thumbnailHeight: UInt32 | null;

  /**
   * File.content
   */
  /**
   * File.content
   */
  get content(): Bytes | null {
    return this._content;
  }
  set content(value: Bytes | null) {
    const prop = (this.constructor as NodeClass).__properties__["content"];
    this._session.updateSetProperty(this, prop, value);
    this._content = value;
  }
  _content: Bytes | null;

  constructor(options: {
    id?: UUID;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference;
    materialization?: Materialization;
    definition?: Entity | NodeReference | null;
    branch?: Branch | NodeReference;
    snapshot?: Snapshot | NodeReference;
    precededBy?: File | NodeReference | null;
    instance?: Entity | NodeReference | null;
    createdAt?: Datetime;
    createdEpoch?: UInt128;
    createdBy?: Entity | NodeReference;
    updatedAt?: Datetime;
    updatedEpoch?: UInt128;
    updatedBy?: Entity | NodeReference;
    deletedAt?: Datetime | null;
    ownedBy?: Entity | NodeReference | null;
    name?: String;
    orderKey?: String;
    customValues?: { readonly [key: UUID]: Value };
    script?: Script | NodeReference | null;
    isExtensible?: Boolean | null;
    source?: Script | NodeReference | null;
    key?: String | null;
    type: FileType;
    region?: Region | null;
    mimeType?: String | null;
    format?: FileFormat | null;
    size?: UInt64 | null;
    sha256?: String | null;
    width?: UInt32 | null;
    height?: UInt32 | null;
    aspectRatio?: Float32 | null;
    codec?: String | null;
    duration?: Duration | null;
    url?: String | null;
    contentUrl?: String | null;
    thumbnailUrl?: String | null;
    faviconUrl?: String | null;
    thumbnailWidth?: UInt32 | null;
    thumbnailHeight?: UInt32 | null;
    content?: Bytes | null;
    _session?: Session | null;
  }) {
    /* super */
    super(
      /* id */
      options.id ?? null,
      /* parent */
      options.parent != null ? options.parent.toRef() : null,
      /* session */
      options._session ?? null,
      /* _isNew */
      options.id == null,
    );

    /* properties */
    let _parent = options.parent ?? null;
    if (_parent != null && _parent.constructor.name !== "NodeReference") {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent as NodeReference | null;
    let _space = options.space ?? null;
    if (_space != null && _space.constructor.name !== "NodeReference") {
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
    this.spacePtr = _space as NodeReference;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 11 /* Materialization.ROOT */;
    }
    if (_materialization === null) {
      throw new Error(`File.materialization is required`);
    }
    this.materialization = _materialization;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.constructor.name !== "NodeReference") {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition as NodeReference | null;
    let _branch = options.branch ?? null;
    if (_branch != null && _branch.constructor.name !== "NodeReference") {
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
    this.branchPtr = _branch as NodeReference;
    let _snapshot = options.snapshot ?? null;
    if (_snapshot != null && _snapshot.constructor.name !== "NodeReference") {
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
    this.snapshotPtr = _snapshot as NodeReference;
    let _precededBy = options.precededBy ?? null;
    if (_precededBy != null && _precededBy.constructor.name !== "NodeReference") {
      _precededBy = (_precededBy as Node).toRef();
    }
    this.precededByPtr = _precededBy as NodeReference | null;
    let _instance = options.instance ?? null;
    if (_instance != null && _instance.constructor.name !== "NodeReference") {
      _instance = (_instance as Node).toRef();
    }
    this.instancePtr = _instance as NodeReference | null;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _ownedBy = options.ownedBy ?? null;
    if (_ownedBy != null && _ownedBy.constructor.name !== "NodeReference") {
      _ownedBy = (_ownedBy as Node).toRef();
    }
    this._ownedByPtr = _ownedBy as NodeReference | null;
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
    if (_script != null && _script.constructor.name !== "NodeReference") {
      _script = (_script as Node).toRef();
    }
    this._scriptPtr = _script as NodeReference | null;
    let _isExtensible = options.isExtensible ?? null;
    this.isExtensible = _isExtensible;
    let _source = options.source ?? null;
    if (_source != null && _source.constructor.name !== "NodeReference") {
      _source = (_source as Node).toRef();
    }
    this.sourcePtr = _source as NodeReference | null;
    let _key = options.key ?? null;
    this._key = _key;
    let _type = options.type;
    if (_type === null) {
      throw new Error(`File.type is required`);
    }
    this._type = _type;
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

    /* identity */
    if (options.id == null) {
      const now = Temporal.Now.zonedDateTimeISO("UTC");
      const epoch = this._session.epoch;
      this.createdAt = now;
      this.createdEpoch = epoch;
      this.createdByPtr = this._session.actorPtr;
      this.updatedAt = now;
      this.updatedEpoch = epoch;
      this.updatedByPtr = this._session.actorPtr;
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
        options.createdBy != null ? options.createdBy.toRef() : this._session.actorPtr;
      this.updatedAt = options.updatedAt;
      this.updatedEpoch = options.updatedEpoch;
      this.updatedByPtr =
        options.updatedBy != null ? options.updatedBy.toRef() : this._session.actorPtr;
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
    if (this._region != null) {
      h = (h * 31 + this._region) & 0xffffffff;
    }
    if (this.definitionPtr != null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
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
      spaceId: this.spacePtr.id,
      definitionId: this.definitionPtr?.id ?? null,
      branchId: this.branchPtr.id,
      snapshotId: this.snapshotPtr.id,
      _session: this._session,
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerNodeClass(NodeType.FILE, File);
/* ==== DESTACK_GENERATED_END:NODE:480000 ==== */
