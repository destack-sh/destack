import { EnumType, Node, NodeReference, Session, StructFrozen, StructType, Supergraph } from "@destack/language/core";
import { File } from "@destack/language/data";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
import { Color } from "@destack/language/style";
import { IconProto, IconTypeProto } from "@destack/proto";

/* ==== DESTACK_GENERATED_START:ENUM:2531 ==== */
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
/* ==== DESTACK_GENERATED_END:ENUM:2531 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2531 ==== */
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
   * file
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
    if (_file != null && _file instanceof Node) {
      _file = _file.toRef();
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
    if ((this.emoji == null) !== (other.emoji == null) || (this.emoji != null && !(this.emoji === other.emoji))) {
      return false;
    }
    if ((this.faName == null) !== (other.faName == null) || (this.faName != null && !(this.faName === other.faName))) {
      return false;
    }
    if (
      (this.vscName == null) !== (other.vscName == null) ||
      (this.vscName != null && !(this.vscName === other.vscName))
    ) {
      return false;
    }
    if (
      (this.filePtr == null) !== (other.filePtr == null) ||
      (this.filePtr != null && !(this.filePtr.id === other.filePtr.id))
    ) {
      return false;
    }
    if (
      (this.fileUrl == null) !== (other.fileUrl == null) ||
      (this.fileUrl != null && !(this.fileUrl === other.fileUrl))
    ) {
      return false;
    }
    if ((this.color == null) !== (other.color == null) || (this.color != null && !this.color.equals(other.color))) {
      return false;
    }
    return true;
  }

  hash(): number {
    throw new Error("not implemented");
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
    objectValue["1"] = 2531;
    objectValue["30"] = object.type;
    if (object.emoji != null) {
      objectValue["31"] = object.emoji;
    }
    if (object.faName != null) {
      objectValue["33"] = object.faName;
    }
    if (object.vscName != null) {
      objectValue["34"] = object.vscName;
    }
    if (object.filePtr != null) {
      objectValue["35"] = object.filePtr.toValue();
    }
    if (object.fileUrl != null) {
      objectValue["36"] = object.fileUrl;
    }
    if (object.color != null) {
      objectValue["40"] = object.color.toValue();
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
    const emojiValue = objectValue["31"];
    const unpackedEmoji = emojiValue != undefined ? emojiValue : null;
    const faNameValue = objectValue["33"];
    const unpackedFaName = faNameValue != undefined ? faNameValue : null;
    const vscNameValue = objectValue["34"];
    const unpackedVscName = vscNameValue != undefined ? vscNameValue : null;
    const filePtrValue = objectValue["35"];
    const unpackedFilePtr =
      filePtrValue != undefined
        ? NodeReference.fromValue(filePtrValue, _session, _supergraph, _graph, _connection)
        : null;
    const fileUrlValue = objectValue["36"];
    const unpackedFileUrl = fileUrlValue != undefined ? fileUrlValue : null;
    const colorValue = objectValue["40"];
    const unpackedColor =
      colorValue != undefined ? Color.fromValue(colorValue, _session, _supergraph, _graph, _connection) : null;
    return new Icon({
      type: Number(objectValue["30"]),
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
    const objectProto: Partial<IconProto> = { metatype: 2531 };
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
    return new Icon({
      type: Number(objectProto.type) as IconType,
      emoji: objectProto.emoji != undefined ? objectProto.emoji : null,
      faName: objectProto.faName != undefined ? objectProto.faName : null,
      vscName: objectProto.vscName != undefined ? objectProto.vscName : null,
      file:
        objectProto.filePtr != undefined
          ? NodeReference.fromProto(objectProto.filePtr!, _session, _supergraph, _graph, _connection)
          : null,
      fileUrl: objectProto.fileUrl != undefined ? objectProto.fileUrl : null,
      color:
        objectProto.color != undefined
          ? Color.fromProto(objectProto.color!, _session, _supergraph, _graph, _connection)
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.ICON, Icon);
/* ==== DESTACK_GENERATED_END:STRUCT:2531 ==== */
