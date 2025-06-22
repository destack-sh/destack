import { IsTracked, EnumType, Session, User, Supergraph, Space, MaterializationType, Global, Entity, Struct, ResourceStatus, QueryConnection, BuiltinObject, StructFrozen, Spatial, Resource, StructType, Node, NodeType, NodeReference, Agent, Graph } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

/* ==== DESTACK_GENERATED_START:ENUM:2541 ==== */
export enum FileSource {
  SPACE = 1,
  INLINE = 3,
  EXTERNAL = 10,
}
/* ==== DESTACK_GENERATED_END:ENUM:2541 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2540 ==== */
export enum FileRetentionMode {
  AUTOMATIC = 1,
  MANUAL = 2,
  TIMED = 3,
}
/* ==== DESTACK_GENERATED_END:ENUM:2540 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2542 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:2542 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2543 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:2543 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2540 ==== */
export class File extends Node implements Global, Spatial, Entity, Resource, IsTracked {
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
  type: FileType;
  name: string;
  status: ResourceStatus;
  targetStatus: Temporal.ZonedDateTime | null;
  source: FileSource;
  mimeType: string | null;
  format: FileFormat | null;
  size: number | null;
  sha256: string | null;
  width: number | null;
  height: number | null;
  aspectRatio: number | null;
  codec: string | null;
  duration: Temporal.Duration | null;
  url: string | null;
  contentUrl: string | null;
  thumbnailUrl: string | null;
  faviconUrl: string | null;
  thumbnailWidth: number | null;
  thumbnailHeight: number | null;
  content: Uint8Array | null;

  constructor(
    id: string,
    parentPtr: NodeReference | null,
    spacePtr: NodeReference | null,
    materialization: MaterializationType,
    createdAt: Temporal.ZonedDateTime,
    createdByPtr: NodeReference | null,
    updatedAt: Temporal.ZonedDateTime,
    updatedByPtr: NodeReference | null,
    type: FileType,
    name: string,
    status: ResourceStatus,
    targetStatus: Temporal.ZonedDateTime | null,
    source: FileSource,
    mimeType: string | null,
    format: FileFormat | null,
    size: number | null,
    sha256: string | null,
    width: number | null,
    height: number | null,
    aspectRatio: number | null,
    codec: string | null,
    duration: Temporal.Duration | null,
    url: string | null,
    contentUrl: string | null,
    thumbnailUrl: string | null,
    faviconUrl: string | null,
    thumbnailWidth: number | null,
    thumbnailHeight: number | null,
    content: Uint8Array | null,
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
    this.name = name;
    this.status = status;
    this.targetStatus = targetStatus;
    this.source = source;
    this.mimeType = mimeType;
    this.format = format;
    this.size = size;
    this.sha256 = sha256;
    this.width = width;
    this.height = height;
    this.aspectRatio = aspectRatio;
    this.codec = codec;
    this.duration = duration;
    this.url = url;
    this.contentUrl = contentUrl;
    this.thumbnailUrl = thumbnailUrl;
    this.faviconUrl = faviconUrl;
    this.thumbnailWidth = thumbnailWidth;
    this.thumbnailHeight = thumbnailHeight;
    this.content = content;
  }


  static from(options: {
    type: FileType,
    name: string,
    status?: ResourceStatus,
    targetStatus?: Temporal.ZonedDateTime | null,
    source: FileSource,
    mimeType?: string | null,
    format?: FileFormat | null,
    size?: number | null,
    sha256?: string | null,
    width?: number | null,
    height?: number | null,
    aspectRatio?: number | null,
    codec?: string | null,
    duration?: Temporal.Duration | null,
    url?: string | null,
    contentUrl?: string | null,
    thumbnailUrl?: string | null,
    faviconUrl?: string | null,
    thumbnailWidth?: number | null,
    thumbnailHeight?: number | null,
    content?: Uint8Array | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: Graph | null,
    _connection?: QueryConnection | null
  }): File {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    return new File(
      options.type,
      options.name,
      options.status ?? ResourceStatus.PENDING,
      options.targetStatus ?? null,
      options.source,
      options.mimeType ?? null,
      options.format ?? null,
      options.size ?? null,
      options.sha256 ?? null,
      options.width ?? null,
      options.height ?? null,
      options.aspectRatio ?? null,
      options.codec ?? null,
      options.duration ?? null,
      options.url ?? null,
      options.contentUrl ?? null,
      options.thumbnailUrl ?? null,
      options.faviconUrl ?? null,
      options.thumbnailWidth ?? null,
      options.thumbnailHeight ?? null,
      options.content ?? null,
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
    return new NodeReference(NodeType.FILE, this.id, this.spacePtr?.id ?? null, null, this._supergraph);
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
}
/* ==== DESTACK_GENERATED_END:NODE:2540 ==== */