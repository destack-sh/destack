import { Graph, Supergraph, Struct, QueryConnection, File, NodeReference, activeSession, StructType, BuiltinObject, EnumType, ACTIVE_SESSION, NodeType, Session, StructFrozen, Node, Color } from '@/language';
import { Temporal } from 'temporal-polyfill'; // until Temporal ships natively

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
  readonly type: IconType;
  readonly emoji: string | null;
  readonly faName: string | null;
  readonly vscName: string | null;
  get file(): File | null {
      const nodePtr: NodeReference | null = this.filePtr;
      if (nodePtr !== null) {
          if (this._supergraph === null) {
              return null;
          }
          return this._supergraph.get(nodePtr.id) as File | null;
      }
      return null;
  }
  ;
  filePtr: NodeReference | null
  readonly fileUrl: string | null;
  readonly color: Color | null;

  constructor(options: {
    type: IconType,
    emoji?: string | null,
    faName?: string | null,
    vscName?: string | null,
    file?: File | NodeReference | null,
    fileUrl?: string | null,
    color?: Color | null,
    _session?: Session | null,
    _supergraph?: Supergraph | null
  }) {
    const session = options._session ?? ACTIVE_SESSION.get();
    const supergraph = options._supergraph ?? session.supergraph;
    super(supergraph);
    this.type = options.type;
    this.emoji = options.emoji ?? null;
    this.faName = options.faName ?? null;
    this.vscName = options.vscName ?? null;
    this.filePtr = options.file != null ? (options.file.metatype == StructType.NODE_REFERENCE ? (options.file as NodeReference) : (options.file as Node).toRef()) : null;
    this.fileUrl = options.fileUrl ?? null;
    this.color = options.color ?? null;
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
}
/* ==== DESTACK_GENERATED_END:STRUCT:2531 ==== */