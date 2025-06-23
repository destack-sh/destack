import { Color, File, Node, NodeReference, Session, StructFrozen, StructType, Supergraph } from "@destack/language";

/* ==== DESTACK_GENERATED_START:ENUM:2531 ==== */
export enum IconType {
  EMOJI = 1,
  FONT_AWESOME = 3,
  VS_CODE = 4,
  FILE = 10,
  FILE_URL = 11,
}
/* ==== DESTACK_GENERATED_END:ENUM:2531 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2531 ==== */
export class Icon extends StructFrozen {
  static metatype: StructType = StructType.ICON;
  static __isFrozen__: boolean = true;

  readonly type: IconType;
  readonly emoji: string | null;
  readonly faName: string | null;
  readonly vscName: string | null;
  get file(): File | null | null {
    const nodePtr: NodeReference | null = this.filePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as File | null;
    }
    return null;
  }
  readonly filePtr: NodeReference | null;
  readonly fileUrl: string | null;
  readonly color: Color | null;

  constructor(options: {
    type: IconType;
    emoji?: string | null;
    faName?: string | null;
    vscName?: string | null;
    file?: File | NodeReference | null;
    fileUrl?: string | null;
    color?: Color | null;
    _session?: Session | null;
    _supergraph?: Supergraph | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._supergraph ?? null,
    );

    // properties
    let _type = options.type;
    if (_type === null) {
      throw new Error(`Icon.type is required`);
    }
    this.type = _type;
    let _emoji = options.emoji ?? null;
    this.emoji = _emoji;
    let _faName = options.faName ?? null;
    this.faName = _faName;
    let _vscName = options.vscName ?? null;
    this.vscName = _vscName;
    let _file = options.file ?? null;
    if (_file != null && _file instanceof Node) {
      _file = _file.toRef();
    }
    this.filePtr = _file;
    let _fileUrl = options.fileUrl ?? null;
    this.fileUrl = _fileUrl;
    let _color = options.color ?? null;
    this.color = _color;

    // identity
    // ...
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:2531 ==== */
