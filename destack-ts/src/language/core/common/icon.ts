import { EnumType, StructType } from "@destack/language/core/builtin/common";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import type { File } from "@destack/language/data";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import type { Color } from "@destack/language/style";
import { IconProto, IconTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:ENUM:80005 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:80005 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:80031 ==== */
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
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as File | null;
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
    _supergraph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _value?: { [key: string]: any } | null;
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
    if (_file != null && _file.metatype != StructType.NODE_REFERENCE) {
      _file = (_file as Node).toRef();
    }
    this.filePtr = _file;
    let _fileUrl = options.fileUrl ?? null;
    this.fileUrl = _fileUrl;
    let _color = options.color ?? null;
    this.color = _color;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._value = options._value ?? null;
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
    if (this._hash !== null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.emoji !== null) {
      h = (h * 31 + hashString(this.emoji)) & 0xffffffff;
    }
    if (this.faName !== null) {
      h = (h * 31 + hashString(this.faName)) & 0xffffffff;
    }
    if (this.vscName !== null) {
      h = (h * 31 + hashString(this.vscName)) & 0xffffffff;
    }
    if (this.filePtr !== null) {
      h = (h * 31 + hashString(this.filePtr.id)) & 0xffffffff;
    }
    if (this.fileUrl !== null) {
      h = (h * 31 + hashString(this.fileUrl)) & 0xffffffff;
    }
    if (this.color !== null) {
      h = (h * 31 + this.color.hash()) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Icon.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Icon): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 80031;
    objectValue["100"] = object.type;
    if (object.emoji != null) {
      objectValue["101"] = object.emoji;
    }
    if (object.faName != null) {
      objectValue["102"] = object.faName;
    }
    if (object.vscName != null) {
      objectValue["103"] = object.vscName;
    }
    if (object.filePtr != null) {
      objectValue["104"] = object.filePtr.toValue();
    }
    if (object.fileUrl != null) {
      objectValue["105"] = object.fileUrl;
    }
    if (object.color != null) {
      objectValue["110"] = object.color.toValue();
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Icon {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    const emojiValue = objectValue["101"];
    const unpackedEmoji = emojiValue != undefined ? emojiValue : null;
    const faNameValue = objectValue["102"];
    const unpackedFaName = faNameValue != undefined ? faNameValue : null;
    const vscNameValue = objectValue["103"];
    const unpackedVscName = vscNameValue != undefined ? vscNameValue : null;
    const filePtrValue = objectValue["104"];
    const unpackedFilePtr =
      filePtrValue != undefined
        ? _NodeReference.fromValue(filePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fileUrlValue = objectValue["105"];
    const unpackedFileUrl = fileUrlValue != undefined ? fileUrlValue : null;
    const colorValue = objectValue["110"];
    const unpackedColor =
      colorValue != undefined
        ? _Color.fromValue(colorValue, _session, _supergraph, _graph, _connection)
        : null;
    return new Icon({
      type: Number(objectValue["100"]),
      emoji: unpackedEmoji,
      faName: unpackedFaName,
      vscName: unpackedVscName,
      file: unpackedFilePtr,
      fileUrl: unpackedFileUrl,
      color: unpackedColor,
      _value: objectValue,
      _supergraph,
    });
  }

  static fromValue(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Icon {
    return Icon.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }

  toProto(): IconProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Icon.__packProto__(this);
    }
    return this._proto as IconProto;
  }

  static __packProto__(object: Icon): IconProto {
    const objectProto: Partial<IconProto> = { metatype: 80031 };
    objectProto.type = Number(object.type) as IconTypeProto;
    if (object.emoji != null) {
      objectProto.emoji = object.emoji;
    }
    if (object.faName != null) {
      objectProto.faName = object.faName;
    }
    if (object.vscName != null) {
      objectProto.vscName = object.vscName;
    }
    if (object.filePtr != null) {
      objectProto.filePtr = object.filePtr.toProto();
    }
    if (object.fileUrl != null) {
      objectProto.fileUrl = object.fileUrl;
    }
    if (object.color != null) {
      objectProto.color = object.color.toProto();
    }
    return objectProto as IconProto;
  }

  static __unpackProto__(
    objectProto: IconProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Icon {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const _Color = STRUCT_CLASS_BY_TYPE[StructType.COLOR] as typeof Color;
    return new Icon({
      type: Number(objectProto.type) as IconType,
      emoji: objectProto.emoji != undefined ? objectProto.emoji : null,
      faName: objectProto.faName != undefined ? objectProto.faName : null,
      vscName: objectProto.vscName != undefined ? objectProto.vscName : null,
      file:
        objectProto.filePtr != undefined
          ? _NodeReference.fromProto(
              objectProto.filePtr!,
              _session,
              _supergraph,
              _graph,
              _connection,
            )
          : null,
      fileUrl: objectProto.fileUrl != undefined ? objectProto.fileUrl : null,
      color:
        objectProto.color != undefined
          ? _Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
          : null,
      _proto: objectProto,
      _supergraph,
    });
  }

  static fromProto(
    objectProto: IconProto,
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Icon {
    return Icon.__unpackProto__(objectProto, _session, _supergraph, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Icon {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = IconProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ICON, Icon);
/* ==== DESTACK_GENERATED_END:STRUCT:80031 ==== */
