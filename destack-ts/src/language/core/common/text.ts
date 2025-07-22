import { EnumType, StructType } from "@destack/language/core/builtin/common";
import { Node } from "@destack/language/core/builtin/node";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import { StructFrozen } from "@destack/language/core/builtin/struct";
import type { GraphConnection } from "@destack/language/core/runtime/connection";
import type { Graph, Supergraph } from "@destack/language/core/runtime/graph";
import type { Session } from "@destack/language/core/runtime/session";
import {
  STRUCT_CLASS_BY_TYPE,
  registerEnumClass,
  registerStructClass,
} from "@destack/language/registry";
import { TextProto, TextSpanProto, TextSpanTypeProto } from "@destack/proto";
import { base64Decode } from "@destack/utils";
import { hashBool, hashString } from "@destack/utils/hash";

/* ==== DESTACK_GENERATED_START:ENUM:400004 ==== */
/**
 * TextSpanType
 */
export enum TextSpanType {
  TEXT = 1,
  HARD_BREAK = 2,
  MENTION = 10,
  LINK = 11,
  CITATION = 12,
  EQUATION = 20,

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerEnumClass(EnumType.TEXT_SPAN_TYPE, TextSpanType);
/* ==== DESTACK_GENERATED_END:ENUM:400004 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:400021 ==== */
/**
 * A span of text with optional formatting
 */
export class TextSpan extends StructFrozen {
  static metatype: StructType = StructType.TEXT_SPAN;
  static __isFrozen__: boolean = true;

  /**
   * TextSpan.type
   */
  readonly type: TextSpanType;

  /**
   * TextSpan.content
   */
  readonly content: string | null;

  /**
   * TextSpan.node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr != null) {
      if (this._graph === null) {
        return null;
      }
      return this._graph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * TextSpan.url
   */
  readonly url: string | null;

  /**
   * TextSpan.isBold
   */
  readonly isBold: boolean | null;

  /**
   * TextSpan.isItalic
   */
  readonly isItalic: boolean | null;

  /**
   * TextSpan.isStrikethrough
   */
  readonly isStrikethrough: boolean | null;

  /**
   * TextSpan.isUnderline
   */
  readonly isUnderline: boolean | null;

  /**
   * TextSpan.isCode
   */
  readonly isCode: boolean | null;

  constructor(options: {
    type?: TextSpanType;
    content?: string | null;
    node?: Node | NodeReference | null;
    url?: string | null;
    isBold?: boolean | null;
    isItalic?: boolean | null;
    isStrikethrough?: boolean | null;
    isUnderline?: boolean | null;
    isCode?: boolean | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
    );

    // properties
    let _type = options.type ?? null;
    if (_type === null) {
      _type = 1 /* TextSpanType.TEXT */;
    }
    if (_type === null) {
      throw new Error(`TextSpan.type is required`);
    }
    this.type = _type;
    let _content = options.content ?? null;
    this.content = _content;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name != "NodeReference") {
      _node = (_node as Node).toRef();
    }
    this.nodePtr = _node as NodeReference | null;
    let _url = options.url ?? null;
    this.url = _url;
    let _isBold = options.isBold ?? null;
    this.isBold = _isBold;
    let _isItalic = options.isItalic ?? null;
    this.isItalic = _isItalic;
    let _isStrikethrough = options.isStrikethrough ?? null;
    this.isStrikethrough = _isStrikethrough;
    let _isUnderline = options.isUnderline ?? null;
    this.isUnderline = _isUnderline;
    let _isCode = options.isCode ?? null;
    this.isCode = _isCode;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (!(this.type === other.type)) {
      return false;
    }
    if (!(this.content === other.content)) {
      return false;
    }
    if (!(this.nodePtr?.id === other.nodePtr?.id)) {
      return false;
    }
    if (!(this.url === other.url)) {
      return false;
    }
    if (!(this.isBold === other.isBold)) {
      return false;
    }
    if (!(this.isItalic === other.isItalic)) {
      return false;
    }
    if (!(this.isStrikethrough === other.isStrikethrough)) {
      return false;
    }
    if (!(this.isUnderline === other.isUnderline)) {
      return false;
    }
    if (!(this.isCode === other.isCode)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<TextSpan>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    h = (h * 31 + this.type) & 0xffffffff;
    if (this.content != null) {
      h = (h * 31 + hashString(this.content)) & 0xffffffff;
    }
    if (this.nodePtr != null) {
      h = (h * 31 + hashString(this.nodePtr.id)) & 0xffffffff;
    }
    if (this.url != null) {
      h = (h * 31 + hashString(this.url)) & 0xffffffff;
    }
    if (this.isBold != null) {
      h = (h * 31 + hashBool(this.isBold)) & 0xffffffff;
    }
    if (this.isItalic != null) {
      h = (h * 31 + hashBool(this.isItalic)) & 0xffffffff;
    }
    if (this.isStrikethrough != null) {
      h = (h * 31 + hashBool(this.isStrikethrough)) & 0xffffffff;
    }
    if (this.isUnderline != null) {
      h = (h * 31 + hashBool(this.isUnderline)) & 0xffffffff;
    }
    if (this.isCode != null) {
      h = (h * 31 + hashBool(this.isCode)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = TextSpan.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: TextSpan): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 400021;
    objectCson["100"] = object.type;
    if (object.content != null) {
      objectCson["101"] = object.content;
    }
    if (object.nodePtr != null) {
      objectCson["102"] = object.nodePtr.toCson();
    }
    if (object.url != null) {
      objectCson["105"] = object.url;
    }
    if (object.isBold != null) {
      objectCson["150"] = object.isBold;
    }
    if (object.isItalic != null) {
      objectCson["151"] = object.isItalic;
    }
    if (object.isStrikethrough != null) {
      objectCson["152"] = object.isStrikethrough;
    }
    if (object.isUnderline != null) {
      objectCson["153"] = object.isUnderline;
    }
    if (object.isCode != null) {
      objectCson["154"] = object.isCode;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): TextSpan {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    const contentValue = objectCson["101"];
    const unpackedContent = contentValue != undefined ? contentValue : null;
    const nodePtrValue = objectCson["102"];
    const unpackedNodePtr =
      nodePtrValue != undefined
        ? _NodeReference.fromCson(nodePtrValue, _session, _graph, _connection)
        : null;
    const urlValue = objectCson["105"];
    const unpackedUrl = urlValue != undefined ? urlValue : null;
    const isBoldValue = objectCson["150"];
    const unpackedIsBold = isBoldValue != undefined ? isBoldValue : null;
    const isItalicValue = objectCson["151"];
    const unpackedIsItalic = isItalicValue != undefined ? isItalicValue : null;
    const isStrikethroughValue = objectCson["152"];
    const unpackedIsStrikethrough = isStrikethroughValue != undefined ? isStrikethroughValue : null;
    const isUnderlineValue = objectCson["153"];
    const unpackedIsUnderline = isUnderlineValue != undefined ? isUnderlineValue : null;
    const isCodeValue = objectCson["154"];
    const unpackedIsCode = isCodeValue != undefined ? isCodeValue : null;
    return new TextSpan({
      type: Number(objectCson["100"]),
      content: unpackedContent,
      node: unpackedNodePtr,
      url: unpackedUrl,
      isBold: unpackedIsBold,
      isItalic: unpackedIsItalic,
      isStrikethrough: unpackedIsStrikethrough,
      isUnderline: unpackedIsUnderline,
      isCode: unpackedIsCode,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): TextSpan {
    return TextSpan.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): TextSpanProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = TextSpan.__packProto__(this);
    }
    return this._proto as TextSpanProto;
  }

  static __packProto__(object: TextSpan): TextSpanProto {
    const objectProto: Partial<TextSpanProto> = { metatype: 400021 };
    objectProto.type = Number(object.type) as TextSpanTypeProto;
    if (object.content != null) {
      objectProto.content = object.content;
    }
    if (object.nodePtr != null) {
      objectProto.nodePtr = object.nodePtr.toProto();
    }
    if (object.url != null) {
      objectProto.url = object.url;
    }
    if (object.isBold != null) {
      objectProto.isBold = object.isBold;
    }
    if (object.isItalic != null) {
      objectProto.isItalic = object.isItalic;
    }
    if (object.isStrikethrough != null) {
      objectProto.isStrikethrough = object.isStrikethrough;
    }
    if (object.isUnderline != null) {
      objectProto.isUnderline = object.isUnderline;
    }
    if (object.isCode != null) {
      objectProto.isCode = object.isCode;
    }
    return objectProto as TextSpanProto;
  }

  static __unpackProto__(
    objectProto: TextSpanProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): TextSpan {
    const _NodeReference = STRUCT_CLASS_BY_TYPE[StructType.NODE_REFERENCE] as typeof NodeReference;
    return new TextSpan({
      type: Number(objectProto.type) as TextSpanType,
      content: objectProto.content != undefined ? objectProto.content : null,
      node:
        objectProto.nodePtr != undefined
          ? _NodeReference.fromProto(objectProto.nodePtr!, _session, _graph, _graph, _connection)
          : null,
      url: objectProto.url != undefined ? objectProto.url : null,
      isBold: objectProto.isBold != undefined ? objectProto.isBold : null,
      isItalic: objectProto.isItalic != undefined ? objectProto.isItalic : null,
      isStrikethrough:
        objectProto.isStrikethrough != undefined ? objectProto.isStrikethrough : null,
      isUnderline: objectProto.isUnderline != undefined ? objectProto.isUnderline : null,
      isCode: objectProto.isCode != undefined ? objectProto.isCode : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: TextSpanProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): TextSpan {
    return TextSpan.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): TextSpan {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TextSpanProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TEXT_SPAN, TextSpan);
/* ==== DESTACK_GENERATED_END:STRUCT:400021 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:400020 ==== */
/**
 * Rich Text; a single paragraph composed of TextSpans with inline formatting.
 */
export class Text extends StructFrozen {
  static metatype: StructType = StructType.TEXT;
  static __isFrozen__: boolean = true;

  /**
   * Text.spans
   */
  readonly spans: readonly TextSpan[];

  /**
   * Text.isBold
   */
  readonly isBold: boolean | null;

  /**
   * Text.isItalic
   */
  readonly isItalic: boolean | null;

  /**
   * Text.isStrikethrough
   */
  readonly isStrikethrough: boolean | null;

  /**
   * Text.isUnderline
   */
  readonly isUnderline: boolean | null;

  /**
   * Text.isCode
   */
  readonly isCode: boolean | null;

  constructor(options: {
    spans?: readonly TextSpan[];
    isBold?: boolean | null;
    isItalic?: boolean | null;
    isStrikethrough?: boolean | null;
    isUnderline?: boolean | null;
    isCode?: boolean | null;
    _session?: Session | null;
    _graph?: Supergraph | null;
    _hash?: number | null;
    _repr?: string | null;
    _proto?: any | null;
    _cson?: any | null;
  }) {
    super(
      // session
      options._session ?? null,
      // supergraph
      options._graph ?? null,
    );

    // properties
    let _spans = options.spans ?? null;
    if (_spans === null) {
      _spans = [];
    }
    this.spans = _spans;
    let _isBold = options.isBold ?? null;
    this.isBold = _isBold;
    let _isItalic = options.isItalic ?? null;
    this.isItalic = _isItalic;
    let _isStrikethrough = options.isStrikethrough ?? null;
    this.isStrikethrough = _isStrikethrough;
    let _isUnderline = options.isUnderline ?? null;
    this.isUnderline = _isUnderline;
    let _isCode = options.isCode ?? null;
    this.isCode = _isCode;

    // identity
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._proto = options._proto ?? null;
    // @ts-expect-error(readonly)
    this._cson = options._cson ?? null;
  }

  equals(other: any): boolean {
    if (!(this.metatype === other.metatype)) {
      return false;
    }
    if (this.spans.length != other.spans.length) {
      return false;
    }
    for (let i = 0; i < this.spans.length; i++) {
      if (!this.spans[i].equals(other.spans[i])) {
        return false;
      }
    }
    if (!(this.isBold === other.isBold)) {
      return false;
    }
    if (!(this.isItalic === other.isItalic)) {
      return false;
    }
    if (!(this.isStrikethrough === other.isStrikethrough)) {
      return false;
    }
    if (!(this.isUnderline === other.isUnderline)) {
      return false;
    }
    if (!(this.isCode === other.isCode)) {
      return false;
    }
    return true;
  }

  repr(): string {
    return `<Text>`;
  }

  hash(): number {
    if (this._hash != null) {
      return this._hash;
    }

    let h = 1;
    h = (h * 31 + this.metatype) & 0xffffffff;
    if (this.spans && this.spans.length > 0) {
      for (const _item of this.spans) {
        h = (h * 31 + _item.hash()) & 0xffffffff;
      }
    }
    if (this.isBold != null) {
      h = (h * 31 + hashBool(this.isBold)) & 0xffffffff;
    }
    if (this.isItalic != null) {
      h = (h * 31 + hashBool(this.isItalic)) & 0xffffffff;
    }
    if (this.isStrikethrough != null) {
      h = (h * 31 + hashBool(this.isStrikethrough)) & 0xffffffff;
    }
    if (this.isUnderline != null) {
      h = (h * 31 + hashBool(this.isUnderline)) & 0xffffffff;
    }
    if (this.isCode != null) {
      h = (h * 31 + hashBool(this.isCode)) & 0xffffffff;
    }

    // @ts-expect-error(readonly)
    this._hash = h;
    return h;
  }

  validate(): void {
    throw new Error("not implemented");
  }

  toCson(): { [key: string]: any } {
    if (this._cson === null) {
      // @ts-expect-error(readonly)
      this._cson = Text.__packCson__(this);
    }
    return this._cson;
  }

  static __packCson__(object: Text): { [key: string]: any } {
    const objectCson: { [key: string]: any } = {};
    objectCson["1"] = 400020;
    if (object.spans.length > 0) {
      const packedSpans: any[] = [];
      for (const item of object.spans) {
        packedSpans.push(item.toCson());
      }
      objectCson["103"] = packedSpans;
    }
    if (object.isBold != null) {
      objectCson["150"] = object.isBold;
    }
    if (object.isItalic != null) {
      objectCson["151"] = object.isItalic;
    }
    if (object.isStrikethrough != null) {
      objectCson["152"] = object.isStrikethrough;
    }
    if (object.isUnderline != null) {
      objectCson["153"] = object.isUnderline;
    }
    if (object.isCode != null) {
      objectCson["154"] = object.isCode;
    }
    return objectCson;
  }

  static __unpackCson__(
    objectCson: { [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Text {
    const _TextSpan = STRUCT_CLASS_BY_TYPE[StructType.TEXT_SPAN] as typeof TextSpan;
    const unpackedSpans: any[] = [];
    if (objectCson["103"] != undefined) {
      for (const item of objectCson["103"]) {
        unpackedSpans.push(_TextSpan.fromCson(item, _session, _graph, _connection));
      }
    }
    const isBoldValue = objectCson["150"];
    const unpackedIsBold = isBoldValue != undefined ? isBoldValue : null;
    const isItalicValue = objectCson["151"];
    const unpackedIsItalic = isItalicValue != undefined ? isItalicValue : null;
    const isStrikethroughValue = objectCson["152"];
    const unpackedIsStrikethrough = isStrikethroughValue != undefined ? isStrikethroughValue : null;
    const isUnderlineValue = objectCson["153"];
    const unpackedIsUnderline = isUnderlineValue != undefined ? isUnderlineValue : null;
    const isCodeValue = objectCson["154"];
    const unpackedIsCode = isCodeValue != undefined ? isCodeValue : null;
    return new Text({
      spans: unpackedSpans,
      isBold: unpackedIsBold,
      isItalic: unpackedIsItalic,
      isStrikethrough: unpackedIsStrikethrough,
      isUnderline: unpackedIsUnderline,
      isCode: unpackedIsCode,
      _cson: objectCson,
      _graph,
    });
  }

  static fromCson(
    objectCson: { readonly [key: string]: any },
    _session?: Session | null,
    _graph?: Graph | null,
    _connection?: GraphConnection | null,
  ): Text {
    return Text.__unpackCson__(objectCson, _session, _graph, _connection);
  }

  toProto(): TextProto {
    if (this._proto === null) {
      // @ts-expect-error(readonly)
      this._proto = Text.__packProto__(this);
    }
    return this._proto as TextProto;
  }

  static __packProto__(object: Text): TextProto {
    const objectProto: Partial<TextProto> = { metatype: 400020 };
    if (object.spans) {
      const packedSpans: any[] = [];
      for (const item of object.spans) {
        packedSpans.push(item.toProto());
      }
      objectProto.spans = packedSpans;
    }
    if (object.isBold != null) {
      objectProto.isBold = object.isBold;
    }
    if (object.isItalic != null) {
      objectProto.isItalic = object.isItalic;
    }
    if (object.isStrikethrough != null) {
      objectProto.isStrikethrough = object.isStrikethrough;
    }
    if (object.isUnderline != null) {
      objectProto.isUnderline = object.isUnderline;
    }
    if (object.isCode != null) {
      objectProto.isCode = object.isCode;
    }
    return objectProto as TextProto;
  }

  static __unpackProto__(
    objectProto: TextProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Text {
    const _TextSpan = STRUCT_CLASS_BY_TYPE[StructType.TEXT_SPAN] as typeof TextSpan;
    const unpackedSpans: any[] = [];
    if (objectProto.spans) {
      for (const item of objectProto.spans) {
        unpackedSpans.push(_TextSpan.fromProto(item!, _session, _graph, _graph, _connection));
      }
    }
    return new Text({
      spans: unpackedSpans,
      isBold: objectProto.isBold != undefined ? objectProto.isBold : null,
      isItalic: objectProto.isItalic != undefined ? objectProto.isItalic : null,
      isStrikethrough:
        objectProto.isStrikethrough != undefined ? objectProto.isStrikethrough : null,
      isUnderline: objectProto.isUnderline != undefined ? objectProto.isUnderline : null,
      isCode: objectProto.isCode != undefined ? objectProto.isCode : null,
      _proto: objectProto,
      _graph,
    });
  }

  static fromProto(
    objectProto: TextProto,
    _session?: Session | null,
    _graph?: Supergraph | null,
    _connection?: GraphConnection | null,
  ): Text {
    return Text.__unpackProto__(objectProto, _session, _graph, _connection);
  }

  static fromProtoString(packedProtoString: string): Text {
    const packedProtoBytes = base64Decode(packedProtoString);
    const packedProto = TextProto.fromBinary(packedProtoBytes);
    return this.fromProto(packedProto);
  }

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TEXT, Text);
/* ==== DESTACK_GENERATED_END:STRUCT:400020 ==== */
