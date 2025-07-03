import {
  packProtoDuration,
  packProtoTimestamp,
  unpackProtoDuration,
  unpackProtoTimestamp,
} from "@destack/grpc";
import type {
  CustomEntityDefinition,
  CustomEventDefinition,
  Graph,
  IsGlobal,
  IsSpatial,
  IsSubject,
  NodeDefinitionReference,
  NodeReference,
  QueryConnection,
  Session,
  Supergraph,
  Value,
} from "@destack/language/core";
import {
  EnumType,
  Materialization,
  Node,
  NodeType,
  Resource,
  ResourceStatus,
  StructType,
} from "@destack/language/core";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerNodeClass,
} from "@destack/language/registry";
import type { Space } from "@destack/language/space";
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
import { hashBytes, hashFloat, hashInt, hashString } from "@destack/utils/hash";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:60001 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:60001 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:60000 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:60000 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:60002 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:60002 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:60003 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:60003 ==== */

/* ==== DESTACK_GENERATED_START:NODE:60000 ==== */
/**
 * A File stored somewhere.
 */
export class File extends Resource implements IsSpatial, IsGlobal {
  static metatype: NodeType = NodeType.FILE;

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
   * The definitionthis CustomEntity is an instance of.
   */
  get definition(): CustomEntityDefinition | CustomEventDefinition | null {
    const nodePtr: NodeReference | null = this.definitionPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as
        | CustomEntityDefinition
        | CustomEventDefinition
        | null;
    }
    return null;
  }
  readonly definitionPtr: NodeReference | null;

  /**
   * Inlined base type of this extensible Node (if extended).
   */
  readonly baseType: NodeDefinitionReference | null;

  /**
   * Entity.materialization
   */
  readonly materialization: Materialization;

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
   * The custom Values of this Node, keyed by custom Property id. May hold both static and instance values.
   */
  customValues: Map<string, Value>;

  /**
   * Resource.status
   */
  status: ResourceStatus;

  /**
   * File.type
   */
  type: FileType;

  /**
   * File.name
   */
  name: string;

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
    parent?: Node | NodeReference | null;
    space?: Space | NodeReference | null;
    definition?: CustomEntityDefinition | CustomEventDefinition | NodeReference | null;
    baseType?: NodeDefinitionReference | null;
    materialization?: Materialization;
    createdAt?: Temporal.ZonedDateTime;
    createdBy?: (Node & IsSubject) | NodeReference | null;
    updatedAt?: Temporal.ZonedDateTime;
    updatedBy?: (Node & IsSubject) | NodeReference | null;
    deletedAt?: Temporal.ZonedDateTime | null;
    customValues?: Map<string, Value>;
    status?: ResourceStatus;
    type: FileType;
    name: string;
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
    if (_parent != null && _parent.metatype != StructType.NODE_REFERENCE) {
      _parent = (_parent as Node).toRef();
    }
    this.parentPtr = _parent;
    let _space = options.space ?? null;
    if (_space != null && _space.metatype != StructType.NODE_REFERENCE) {
      _space = (_space as Node).toRef();
    }
    this.spacePtr = _space;
    let _definition = options.definition ?? null;
    if (_definition != null && _definition.metatype != StructType.NODE_REFERENCE) {
      _definition = (_definition as Node).toRef();
    }
    this.definitionPtr = _definition;
    let _baseType = options.baseType ?? null;
    this.baseType = _baseType;
    let _materialization = options.materialization ?? null;
    if (_materialization === null) {
      _materialization = 3 /* Materialization.FULL */;
    }
    if (_materialization === null) {
      throw new Error(`File.materialization is required`);
    }
    this.materialization = _materialization;
    let _deletedAt = options.deletedAt ?? null;
    this.deletedAt = _deletedAt;
    let _customValues = options.customValues ?? null;
    if (_customValues === null) {
      _customValues = new Map();
    }
    this.customValues = _customValues;
    let _status = options.status ?? null;
    if (_status === null) {
      _status = 1 /* ResourceStatus.PENDING */;
    }
    if (_status === null) {
      throw new Error(`File.status is required`);
    }
    this.status = _status;
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
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.name === other.name)) {
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
        !(
          this.aspectRatio === other.aspectRatio ||
          Math.abs(this.aspectRatio - other.aspectRatio) < 1e-10
        ))
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
    if (!(this.definitionPtr?.id === other.definitionPtr?.id)) {
      return false;
    }
    if (
      (this.baseType == null) !== (other.baseType == null) ||
      (this.baseType != null && !this.baseType.equals(other.baseType))
    ) {
      return false;
    }
    if (Object.keys(this.customValues).length !== Object.keys(other.customValues).length) {
      return false;
    }
    for (const key in this.customValues) {
      if (!(key in other.customValues)) {
        return false;
      }
      if (!this.customValues.get(key)!.equals(other.customValues.get(key)!)) {
        return false;
      }
    }
    return true;
  }

  hash(): number {
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    h = (h * 31 + hashString(this.name)) & 0xffffffff;
    h = (h * 31 + this.source) & 0xffffffff;
    if (this.mimeType !== null) {
      h = (h * 31 + hashString(this.mimeType)) & 0xffffffff;
    }
    if (this.format !== null) {
      h = (h * 31 + this.format) & 0xffffffff;
    }
    if (this.size !== null) {
      h = (h * 31 + hashInt(this.size)) & 0xffffffff;
    }
    if (this.sha256 !== null) {
      h = (h * 31 + hashString(this.sha256)) & 0xffffffff;
    }
    if (this.width !== null) {
      h = (h * 31 + hashInt(this.width)) & 0xffffffff;
    }
    if (this.height !== null) {
      h = (h * 31 + hashInt(this.height)) & 0xffffffff;
    }
    if (this.aspectRatio !== null) {
      h = (h * 31 + hashFloat(this.aspectRatio)) & 0xffffffff;
    }
    if (this.codec !== null) {
      h = (h * 31 + hashString(this.codec)) & 0xffffffff;
    }
    if (this.duration !== null) {
      h = (h * 31 + hashFloat(this.duration.total("seconds"))) & 0xffffffff;
    }
    if (this.url !== null) {
      h = (h * 31 + hashString(this.url)) & 0xffffffff;
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
      h = (h * 31 + hashBytes(this.content)) & 0xffffffff;
    }
    if (this.spacePtr !== null) {
      h = (h * 31 + hashString(this.spacePtr.id)) & 0xffffffff;
    }
    h = (h * 31 + this.status) & 0xffffffff;
    h = (h * 31 + hashString(this.id.toString())) & 0xffffffff;
    if (this.parentPtr !== null) {
      h = (h * 31 + hashString(this.parentPtr.id)) & 0xffffffff;
    }
    if (this.deletedAt !== null) {
      h = (h * 31 + hashString(this.deletedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    }
    if (this.definitionPtr !== null) {
      h = (h * 31 + hashString(this.definitionPtr.id)) & 0xffffffff;
    }
    if (this.baseType !== null) {
      h = (h * 31 + this.baseType.hash()) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.createdAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.createdByPtr !== null) {
      h = (h * 31 + hashString(this.createdByPtr.id)) & 0xffffffff;
    }
    h = (h * 31 + hashString(this.updatedAt.toString({ timeZoneName: "never" }))) & 0xffffffff;
    if (this.updatedByPtr !== null) {
      h = (h * 31 + hashString(this.updatedByPtr.id)) & 0xffffffff;
    }
    if (this.customValues && Object.keys(this.customValues).length > 0) {
      for (const [_key, _value] of Object.entries(this.customValues)) {
        h = (h * 31 + hashString(_key.toString())) & 0xffffffff;
        h = (h * 31 + _value.hash()) & 0xffffffff;
      }
    }

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

  repr(): string {
    const propertyReprs: string[] = [];
    propertyReprs.push(`type=${FileType[this.type]}`);
    propertyReprs.push(`name=${this.name}`);
    propertyReprs.push(`source=${FileSource[this.source]}`);
    if (this.mimeType !== null) {
      propertyReprs.push(`mimeType=${this.mimeType}`);
    }
    if (this.format !== null) {
      propertyReprs.push(`format=${FileFormat[this.format]}`);
    }
    if (this.size !== null) {
      propertyReprs.push(`size=${this.size}`);
    }
    if (this.url !== null) {
      propertyReprs.push(`url=${this.url}`);
    }
    return `<File '${this.path}' ${propertyReprs.join(" ")}>`;
  }

  toValue(): { [key: string]: any } {
    return File.__packValue__(this);
  }

  static __packValue__(object: File): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 60000;
    objectValue["2"] = String(object.id);
    if (object.parentPtr != null) {
      objectValue["3"] = object.parentPtr.toValue();
    }
    if (object.spacePtr != null) {
      objectValue["5"] = object.spacePtr.toValue();
    }
    if (object.definitionPtr != null) {
      objectValue["6"] = object.definitionPtr.toValue();
    }
    if (object.baseType != null) {
      objectValue["7"] = object.baseType.toValue();
    }
    objectValue["10"] = object.materialization;
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
    if (object.customValues.size > 0) {
      const packedCustomValues: { [key: string]: any } = {};
      for (const [key, value] of object.customValues) {
        packedCustomValues[String(String(key))] = value.toValue();
      }
      objectValue["26"] = packedCustomValues;
    }
    objectValue["90"] = object.status;
    objectValue["100"] = object.type;
    objectValue["101"] = object.name;
    objectValue["110"] = object.source;
    if (object.mimeType != null) {
      objectValue["111"] = object.mimeType;
    }
    if (object.format != null) {
      objectValue["112"] = object.format;
    }
    if (object.size != null) {
      objectValue["113"] = object.size;
    }
    if (object.sha256 != null) {
      objectValue["114"] = object.sha256;
    }
    if (object.width != null) {
      objectValue["115"] = object.width;
    }
    if (object.height != null) {
      objectValue["116"] = object.height;
    }
    if (object.aspectRatio != null) {
      objectValue["117"] = object.aspectRatio;
    }
    if (object.codec != null) {
      objectValue["118"] = object.codec;
    }
    if (object.duration != null) {
      objectValue["119"] = timedeltaToISOFormat(object.duration);
    }
    if (object.url != null) {
      objectValue["120"] = object.url;
    }
    if (object.contentUrl != null) {
      objectValue["121"] = object.contentUrl;
    }
    if (object.thumbnailUrl != null) {
      objectValue["122"] = object.thumbnailUrl;
    }
    if (object.faviconUrl != null) {
      objectValue["123"] = object.faviconUrl;
    }
    if (object.thumbnailWidth != null) {
      objectValue["124"] = object.thumbnailWidth;
    }
    if (object.thumbnailHeight != null) {
      objectValue["125"] = object.thumbnailHeight;
    }
    if (object.content != null) {
      objectValue["126"] = base64Encode(object.content);
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
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
    const spacePtrValue = objectValue["5"];
    const unpackedSpacePtr =
      spacePtrValue != undefined
        ? _NodeReference.fromValue(spacePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const parentPtrValue = objectValue["3"];
    const unpackedParentPtr =
      parentPtrValue != undefined
        ? _NodeReference.fromValue(parentPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const deletedAtValue = objectValue["25"];
    const unpackedDeletedAt =
      deletedAtValue != undefined
        ? Temporal.Instant.from(deletedAtValue).toZonedDateTimeISO("UTC")
        : null;
    const definitionPtrValue = objectValue["6"];
    const unpackedDefinitionPtr =
      definitionPtrValue != undefined
        ? _NodeReference.fromValue(definitionPtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const baseTypeValue = objectValue["7"];
    const unpackedBaseType =
      baseTypeValue != undefined
        ? _NodeDefinitionReference.fromValue(
            baseTypeValue,
            _session,
            _supergraph,
            _graph,
            _connection,
          )
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
    const unpackedCustomValues = new Map();
    if (objectValue["26"] != undefined) {
      for (const [key, value] of Object.entries(objectValue["26"])) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromValue(value as any, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new File({
      type: Number(objectValue["100"]),
      name: objectValue["101"],
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
      space: unpackedSpacePtr,
      status: Number(objectValue["90"]),
      id: String(objectValue["2"]),
      parent: unpackedParentPtr,
      deletedAt: unpackedDeletedAt,
      definition: unpackedDefinitionPtr,
      baseType: unpackedBaseType,
      materialization: Number(objectValue["10"]),
      createdAt: Temporal.Instant.from(objectValue["20"]).toZonedDateTimeISO("UTC"),
      createdBy: unpackedCreatedByPtr,
      updatedAt: Temporal.Instant.from(objectValue["22"]).toZonedDateTimeISO("UTC"),
      updatedBy: unpackedUpdatedByPtr,
      customValues: unpackedCustomValues,
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
    const objectProto: Partial<FileProto> = { metatype: 60000 };
    objectProto.id = String(object.id);
    if (object.parentPtr != null) {
      objectProto.parentPtr = object.parentPtr.toProto();
    }
    if (object.spacePtr != null) {
      objectProto.spacePtr = object.spacePtr.toProto();
    }
    if (object.definitionPtr != null) {
      objectProto.definitionPtr = object.definitionPtr.toProto();
    }
    if (object.baseType != null) {
      objectProto.baseType = object.baseType.toProto();
    }
    objectProto.materialization = Number(object.materialization) as MaterializationProto;
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
    if (object.customValues) {
      objectProto.customValues = {};
      for (const [key, value] of object.customValues) {
        objectProto.customValues![String(key)] = value.toProto();
      }
    }
    objectProto.status = Number(object.status) as ResourceStatusProto;
    objectProto.type = Number(object.type) as FileTypeProto;
    objectProto.name = object.name;
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
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _NodeDefinitionReference = STRUCT_CLASS_BY_TYPE[
      StructType.NODE_DEFINITION_REFERENCE
    ] as typeof NodeDefinitionReference;
    const _Value = STRUCT_CLASS_BY_TYPE[StructType.VALUE] as typeof Value;
    const unpackedCustomValues = new Map();
    if (objectProto.customValues) {
      for (const [key, value] of Object.entries(objectProto.customValues)) {
        unpackedCustomValues.set(
          String(key),
          _Value.fromProto((value as any)!, _session, _supergraph, _graph, _connection),
        );
      }
    }
    return new File({
      type: Number(objectProto.type) as FileType,
      name: objectProto.name,
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
      space:
        objectProto.spacePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.spacePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      status: Number(objectProto.status) as ResourceStatus,
      id: String(objectProto.id),
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
      deletedAt:
        objectProto.deletedAt != undefined ? unpackProtoTimestamp(objectProto.deletedAt!) : null,
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
      baseType:
        objectProto.baseType != undefined
          ? _NodeDefinitionReference.fromProto(
              objectProto.baseType!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      materialization: Number(objectProto.materialization) as Materialization,
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
      customValues: unpackedCustomValues,
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
/* ==== DESTACK_GENERATED_END:NODE:60000 ==== */
