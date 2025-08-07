import { EnumType, StructType } from "@destack/language/core/builtin/builtin";
import type { Node } from "@destack/language/core/builtin/node";
import type { PackedObjectCache } from "@destack/language/core/builtin/object";
import type { NodeReference } from "@destack/language/core/builtin/relation";
import { ImmutableStruct } from "@destack/language/core/builtin/struct";
import type { Session } from "@destack/language/core/runtime/session";
import { registerEnumClass, registerStructClass } from "@destack/language/registry";
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
export class TextSpan extends ImmutableStruct {
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
      if (this._session === null) {
        return null;
      }
      return this._session.graph.get(nodePtr) as Node | null;
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _type = options.type ?? null;
    if (_type == null) {
      _type = 1 /* TextSpanType.TEXT */;
    }
    if (_type == null) {
      throw new Error(`TextSpan.type is required`);
    }
    this.type = _type;
    let _content = options.content ?? null;
    this.content = _content;
    let _node = options.node ?? null;
    if (_node != null && _node.constructor.name !== "NodeReference") {
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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
export class Text extends ImmutableStruct {
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
    _hash?: number | null;
    _repr?: string | null;
    _PackedObjectCache?: PackedObjectCache[] | null;
  }) {
    /* super */
    super(
      /* session */
      options._session ?? null,
    );

    /* properties */
    let _spans = options.spans ?? null;
    if (_spans == null) {
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

    /* identity */
    // @ts-expect-error(readonly)
    this._hash = options._hash ?? null;
    // @ts-expect-error(readonly)
    this._repr = options._repr ?? null;
    // @ts-expect-error(readonly)
    this._PackedObjectCache = options._PackedObjectCache ?? null;
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

  /* ==== DESTACK_CUSTOM_START ==== */
  // ...
  /* ==== DESTACK_CUSTOM_END ==== */
}
registerStructClass(StructType.TEXT, Text);
/* ==== DESTACK_GENERATED_END:STRUCT:400020 ==== */
