import { EnumType, StructType } from "@destack/language/core/builtin/builtin";
import type { Node } from "@destack/language/core/builtin/node";
import type { PackedCache } from "@destack/language/core/builtin/object";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { Session } from "@destack/language/core/runtime/session";
import type { File } from "@destack/language/data";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import type { Color } from "@destack/language/style";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:ENUM:400005 ==== */
/**
 * IconType
 */
export enum IconType {
  EMOJI = 1,
  FONT_AWESOME = 3,
  VS_CODE = 4,
  FILE = 10,
  FILE_URL = 11,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.ICON_TYPE, IconType);
/* ==== DESTACK_GENERATED_END:ENUM:400005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:400031 ==== */
/**
 * An icon to be displayed in some view.
 */
export class Icon extends StructFrozen {
  static metatype: StructType = StructType.ICON;
  static __isFrozen__: boolean = true;

  /**
   * Icon.type
   */
  readonly type: IconType;

  /**
   * Icon.emoji
   */
  readonly emoji: string | null;

  /**
   * Icon.faName
   */
  readonly faName: string | null;

  /**
   * Icon.vscName
   */
  readonly vscName: string | null;

  /**
   * Icon.file
   */
  get file(): File | null {
    const nodePtr: NodeReference | null = this.filePtr;
    if (nodePtr != null) {
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as File | null;
    }
    return null;
  }
  readonly filePtr: NodeReference | null;

  /**
   * Icon.fileUrl
   */
  readonly fileUrl: string | null;

  /**
   * Icon.color
   */
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
    _hash?: number | null;
    _repr?: string | null;
    _packedCache?: PackedCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
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
    if (_file != null && _file.constructor.name !== "NodeReference") {
      _file = (_file as Node).toRef();
    }
    this.filePtr = _file as NodeReference | null;
    let _fileUrl = options.fileUrl ?? null;
    this.fileUrl = _fileUrl;
    let _color = options.color ?? null;
    this.color = _color;

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._packedCache = options._packedCache ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.emoji === other.emoji)) {
      return false;
    }
    if (!(this.faName === other.faName)) {
      return false;
    }
    if (!(this.vscName === other.vscName)) {
      return false;
    }
    if (!(this.filePtr?.id === other.filePtr?.id)) {
      return false;
    }
    if (!(this.fileUrl === other.fileUrl)) {
      return false;
    }
    if (
      (this.color == null) !== (other.color == null) ||
      (this.color != null && !this.color.equals(other.color))
    ) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Icon>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }
    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.emoji != null) {
      h = (h * 31 + hashString(this.emoji)) & 0xffffffff;
    }
    if (this.faName != null) {
      h = (h * 31 + hashString(this.faName)) & 0xffffffff;
    }
    if (this.vscName != null) {
      h = (h * 31 + hashString(this.vscName)) & 0xffffffff;
    }
    if (this.filePtr != null) {
      h = (h * 31 + hashString(this.filePtr.id)) & 0xffffffff;
    }
    if (this.fileUrl != null) {
      h = (h * 31 + hashString(this.fileUrl)) & 0xffffffff;
    }
    if (this.color != null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }
    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ICON, Icon);
/* ==== DESTACK_GENERATED_END:STRUCT:400031 ==== */
