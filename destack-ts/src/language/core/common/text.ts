import { Node, NodeReference, Session, StructFrozen, StructType, Supergraph } from "@destack/language/core";

/* ==== DESTACK_GENERATED_START:ENUM:2521 ==== */
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
}
/* ==== DESTACK_GENERATED_END:ENUM:2521 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2521 ==== */
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
   * node
   */
  get node(): Node | null {
    const nodePtr: NodeReference | null = this.nodePtr;
    if (nodePtr !== null) {
      if (this._supergraph === null) {
        return null;
      }
      return this._supergraph.get(nodePtr.id) as Node | null;
    }
    return null;
  }
  readonly nodePtr: NodeReference | null;

  /**
   * TextSpan.url
   */
  readonly url: string | null;

  /**
   * TextOptionsBase.isBold
   */
  readonly isBold: boolean | null;

  /**
   * TextOptionsBase.isItalic
   */
  readonly isItalic: boolean | null;

  /**
   * TextOptionsBase.isStrikethrough
   */
  readonly isStrikethrough: boolean | null;

  /**
   * TextOptionsBase.isUnderline
   */
  readonly isUnderline: boolean | null;

  /**
   * TextOptionsBase.isCode
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
    let _type = options.type ?? null;
    if (_type === null) {
      _type = TextSpanType.TEXT;
    }
    if (_type === null) {
      throw new Error(`TextSpan.type is required`);
    }
    this.type = _type;
    let _content = options.content ?? null;
    this.content = _content;
    let _node = options.node ?? null;
    if (_node != null && _node instanceof Node) {
      _node = _node.toRef();
    }
    this.nodePtr = _node;
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
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = TextSpan.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: TextSpan): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2521;
    objectValue["30"] = object.type;
    if (object.content !== null) {
      objectValue["33"] = object.content;
    }
    if (object.nodePtr !== null) {
      objectValue["34"] = object.nodePtr.toValue();
    }
    if (object.url !== null) {
      objectValue["35"] = object.url;
    }
    if (object.isBold !== null) {
      objectValue["60"] = object.isBold;
    }
    if (object.isItalic !== null) {
      objectValue["61"] = object.isItalic;
    }
    if (object.isStrikethrough !== null) {
      objectValue["62"] = object.isStrikethrough;
    }
    if (object.isUnderline !== null) {
      objectValue["63"] = object.isUnderline;
    }
    if (object.isCode !== null) {
      objectValue["64"] = object.isCode;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): TextSpan {
    const contentValue = objectValue["33"];
    const unpackedContent = contentValue !== undefined ? contentValue : null;
    const urlValue = objectValue["35"];
    const unpackedUrl = urlValue !== undefined ? urlValue : null;
    const isBoldValue = objectValue["60"];
    const unpackedIsBold = isBoldValue !== undefined ? isBoldValue : null;
    const isItalicValue = objectValue["61"];
    const unpackedIsItalic = isItalicValue !== undefined ? isItalicValue : null;
    const isStrikethroughValue = objectValue["62"];
    const unpackedIsStrikethrough = isStrikethroughValue !== undefined ? isStrikethroughValue : null;
    const isUnderlineValue = objectValue["63"];
    const unpackedIsUnderline = isUnderlineValue !== undefined ? isUnderlineValue : null;
    const isCodeValue = objectValue["64"];
    const unpackedIsCode = isCodeValue !== undefined ? isCodeValue : null;
    const nodeValue = objectValue["34"];
    const unpackedNode =
      nodeValue !== undefined ? NodeReference.fromValue(nodeValue, _session, _supergraph, _graph, _connection) : null;
    return new TextSpan({
      type: Number(objectValue["30"]),
      content: unpackedContent,
      url: unpackedUrl,
      isBold: unpackedIsBold,
      isItalic: unpackedIsItalic,
      isStrikethrough: unpackedIsStrikethrough,
      isUnderline: unpackedIsUnderline,
      isCode: unpackedIsCode,
      node: unpackedNode,
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
  ): TextSpan {
    return TextSpan.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2521 ==== */

/* ==== DESTACK_GENERATED_START:STRUCT:2520 ==== */
/**
 * Rich Text; a single paragraph composed of TextSpans with inline formatting.
 */
export class Text extends StructFrozen {
  static metatype: StructType = StructType.TEXT;
  static __isFrozen__: boolean = true;

  /**
   * Text.spans
   */
  readonly spans: Array<TextSpan>;

  /**
   * TextOptionsBase.isBold
   */
  readonly isBold: boolean | null;

  /**
   * TextOptionsBase.isItalic
   */
  readonly isItalic: boolean | null;

  /**
   * TextOptionsBase.isStrikethrough
   */
  readonly isStrikethrough: boolean | null;

  /**
   * TextOptionsBase.isUnderline
   */
  readonly isUnderline: boolean | null;

  /**
   * TextOptionsBase.isCode
   */
  readonly isCode: boolean | null;

  constructor(options: {
    spans?: Array<TextSpan>;
    isBold?: boolean | null;
    isItalic?: boolean | null;
    isStrikethrough?: boolean | null;
    isUnderline?: boolean | null;
    isCode?: boolean | null;
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
    let _spans = options.spans ?? null;
    if (_spans === null) {
      throw new Error(`Text.spans is required`);
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
    this._value = options._value ?? null;
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

  toValue(): { [key: string]: any } {
    if (this._value === null) {
      // @ts-expect-error(readonly)
      this._value = Text.__packValue__(this);
    }
    return this._value;
  }

  static __packValue__(object: Text): { [key: string]: any } {
    const objectValue: { [key: string]: any } = {};
    objectValue["1"] = 2520;
    if (object.spans) {
      const packedSpans: any[] = [];
      for (const item of object.spans) {
        packedSpans.push(item.toValue());
      }
      objectValue["33"] = packedSpans;
    }
    if (object.isBold !== null) {
      objectValue["60"] = object.isBold;
    }
    if (object.isItalic !== null) {
      objectValue["61"] = object.isItalic;
    }
    if (object.isStrikethrough !== null) {
      objectValue["62"] = object.isStrikethrough;
    }
    if (object.isUnderline !== null) {
      objectValue["63"] = object.isUnderline;
    }
    if (object.isCode !== null) {
      objectValue["64"] = object.isCode;
    }
    return objectValue;
  }

  static __unpackValue__(
    objectValue: { [key: string]: any },
    _session?: Session | null,
    _supergraph?: Supergraph | null,
    _graph?: any | null,
    _connection?: any | null,
  ): Text {
    const unpackedSpans: any[] = [];
    if (objectValue["33"] !== undefined) {
      for (const item of objectValue["33"]) {
        unpackedSpans.push(TextSpan.fromValue(item, _session, _supergraph, _graph, _connection));
      }
    }
    const isBoldValue = objectValue["60"];
    const unpackedIsBold = isBoldValue !== undefined ? isBoldValue : null;
    const isItalicValue = objectValue["61"];
    const unpackedIsItalic = isItalicValue !== undefined ? isItalicValue : null;
    const isStrikethroughValue = objectValue["62"];
    const unpackedIsStrikethrough = isStrikethroughValue !== undefined ? isStrikethroughValue : null;
    const isUnderlineValue = objectValue["63"];
    const unpackedIsUnderline = isUnderlineValue !== undefined ? isUnderlineValue : null;
    const isCodeValue = objectValue["64"];
    const unpackedIsCode = isCodeValue !== undefined ? isCodeValue : null;
    return new Text({
      spans: unpackedSpans,
      isBold: unpackedIsBold,
      isItalic: unpackedIsItalic,
      isStrikethrough: unpackedIsStrikethrough,
      isUnderline: unpackedIsUnderline,
      isCode: unpackedIsCode,
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
  ): Text {
    return Text.__unpackValue__(objectValue, _session, _supergraph, _graph, _connection);
  }
}
/* ==== DESTACK_GENERATED_END:STRUCT:2520 ==== */
