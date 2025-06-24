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
/* ==== DESTACK_GENERATED_END:STRUCT:2520 ==== */
