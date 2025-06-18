import { Graph, Supergraph, StructFrozen, File, QueryConnection, StructType, NodeReference, NodeType, Session, Node, Struct, Color, EnumType, BuiltinObject } from '@/language';
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
          return this._supergraph.get(nodePtr.id);
      }
      return null;
  }
  ;
  filePtr: NodeReference | null
  readonly fileUrl: string | null;
  readonly color: Color | null;

  constructor(
    type: IconType,
    emoji: string | null,
    faName: string | null,
    vscName: string | null,
    filePtr: NodeReference | null,
    fileUrl: string | null,
    color: Color | null,
    _supergraph: Supergraph
  ) {
    super(_supergraph);
    this.type = type;
    this.emoji = emoji;
    this.faName = faName;
    this.vscName = vscName;
    this.filePtr = filePtr;
    this.fileUrl = fileUrl;
    this.color = color;
  }


  static create(options: {
    type?: IconType,
    emoji?: string | null,
    faName?: string | null,
    vscName?: string | null,
    file?: File | NodeReference | null,
    fileUrl?: string | null,
    color?: Color | null
  }): Icon {

    return new Icon(

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
}
/* ==== DESTACK_GENERATED_END:STRUCT:2531 ==== */