import {
  Agent,
  Entity,
  Global,
  Graph,
  IsTracked,
  MaterializationType,
  Node,
  NodeReference,
  NodeType,
  QueryConnection,
  Resource,
  ResourceStatus,
  Session,
  Space,
  Spatial,
  StructType,
  Supergraph,
  TraitType,
  User,
} from "@/language";
import { Temporal } from "temporal-polyfill";

/* ==== DESTACK_GENERATED_START:ENUM:2541 ==== */
export enum FileSource {
  SPACE = 1,
  INLINE = 3,
  EXTERNAL = 10,
} /* ==== DESTACK_GENERATED_END:ENUM:2541 ==== */

/* ==== DESTACK_GENERATED_START:ENUM:2540 ==== */
export enum FileRetentionMode {
  AUTOMATIC = 1,
  MANUAL = 2,
  TIMED = 3,
} /* ==== DESTACK_GENERATED_END:ENUM:2540 ==== */

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
} /* ==== DESTACK_GENERATED_END:ENUM:2542 ==== */

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
} /* ==== DESTACK_GENERATED_END:ENUM:2543 ==== */

/* ==== DESTACK_GENERATED_START:NODE:2540 ==== */
export class File extends Node implements Global, Spatial, Entity, Resource, IsTracked {
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

  readonly id: string;
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
  get createdBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.createdByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly createdByPtr: NodeReference | null;
  readonly updatedAt: Temporal.ZonedDateTime;
  get updatedBy(): Agent | User | null | null {
    const nodePtr: NodeReference | null = this.updatedByPtr;
    if (nodePtr !== null) {
      return this._supergraph.get(nodePtr.id) as Agent | User | null | null;
    }
    return null;
  }
  readonly updatedByPtr: NodeReference | null;
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

  constructor(options: {
    id: string;
    parent?: Space | NodeReference | null;
    space?: Space | NodeReference | null;
    materialization?: MaterializationType;
    createdAt: Temporal.ZonedDateTime;
    createdBy?: Agent | User | NodeReference | null;
    updatedAt: Temporal.ZonedDateTime;
    updatedBy?: Agent | User | NodeReference | null;
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
      options.id,
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
      options.id != null,
    );

    this.id = options.id;
    this.parentPtr =
      options.parent != null
        ? options.parent.metatype == StructType.NODE_REFERENCE
          ? (options.parent as NodeReference)
          : (options.parent as Node).toRef()
        : null;
    this.spacePtr =
      options.space != null
        ? options.space.metatype == StructType.NODE_REFERENCE
          ? (options.space as NodeReference)
          : (options.space as Node).toRef()
        : null;
    this.materialization = options.materialization ?? MaterializationType.FULL_GRAPH;
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
    this.type = options.type;
    this.name = options.name;
    this.status = options.status ?? ResourceStatus.PENDING;
    this.targetStatus = options.targetStatus ?? null;
    this.source = options.source;
    this.mimeType = options.mimeType ?? null;
    this.format = options.format ?? null;
    this.size = options.size ?? null;
    this.sha256 = options.sha256 ?? null;
    this.width = options.width ?? null;
    this.height = options.height ?? null;
    this.aspectRatio = options.aspectRatio ?? null;
    this.codec = options.codec ?? null;
    this.duration = options.duration ?? null;
    this.url = options.url ?? null;
    this.contentUrl = options.contentUrl ?? null;
    this.thumbnailUrl = options.thumbnailUrl ?? null;
    this.faviconUrl = options.faviconUrl ?? null;
    this.thumbnailWidth = options.thumbnailWidth ?? null;
    this.thumbnailHeight = options.thumbnailHeight ?? null;
    this.content = options.content ?? null;
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
} /* ==== DESTACK_GENERATED_END:NODE:2540 ==== */
