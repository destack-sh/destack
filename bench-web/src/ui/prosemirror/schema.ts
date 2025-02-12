import { TextLineType, TextSpanType } from "@/proto/wire";
import { assertNever } from "@/utils/functools";
import { Node as PmNode, Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";

//
// PM Schema
//

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline" | "code";
export type SpanSpecialInputType = "/" | "@";

const P_DOM: DOMOutputSpec = ["p", { class: "line" }, 0];
const H1_DOM: DOMOutputSpec = ["h1", { class: "line" }, 0];
const H2_DOM: DOMOutputSpec = ["h2", { class: "line" }, 0];
const H3_DOM: DOMOutputSpec = ["h3", { class: "line" }, 0];
const H4_DOM: DOMOutputSpec = ["h4", { class: "line" }, 0];
const HR_DOM: DOMOutputSpec = ["hr", { clas: "line" }];
const CALLOUT_DOM: DOMOutputSpec = ["p", { class: "line callout" }, 0];
const QUOTE_DOM: DOMOutputSpec = ["blockquote", { class: "line" }, 0];
const LIST_UNORDERED_DOM: DOMOutputSpec = ["li", { class: "line list-unordered" }, 0];
const LIST_ORDERED_DOM: DOMOutputSpec = ["li", { class: "line list-ordered" }, 0];
const LINE_CODE_DOM: DOMOutputSpec = ["code", { class: "line" }, 0];

const SPAN_STRONG_DOM: DOMOutputSpec = ["strong", 0];
const SPAN_ITALIC_DOM: DOMOutputSpec = ["em", 0];
const SPAN_STRIKETHROUGH_DOM: DOMOutputSpec = ["s", 0];
const SPAN_UNDERLINE_DOM: DOMOutputSpec = ["u", 0];
const SPAN_CODE_DOM: DOMOutputSpec = ["code", 0];
const SPAN_HARD_BREAK_DOM: DOMOutputSpec = ["br"];

/** Make a DOMOutputSpec with node-specific metadata. */
function toLineDom(node: PmNode, spec: readonly [string, ...any[]]): DOMOutputSpec {
  const [tag, origAttrs = {}, ...rest] = spec;
  const attrs = { ...origAttrs };
  // blockPtr
  if (node.attrs.blockPtr) {
    attrs["data-node-id"] = node.attrs.blockPtr.id;
    attrs["data-node-ck"] = node.attrs.blockPtr.ck;
    attrs["data-node-type"] = node.attrs.blockPtr.nodeType;
  }
  return [tag, attrs, ...rest];
}

export const PM_SCHEMA = new PmSchema({
  nodes: {
    doc: { content: "line+" },
    //
    // line
    //
    lineParagraph: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.PARAGRAPH } },
      toDOM(node) {
        return toLineDom(node, P_DOM);
      },
    },
    // headings
    lineHeading: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.HEADING_1 } },
      toDOM(node) {
        const type = node.attrs.type;
        if (type == TextLineType.HEADING_1) return toLineDom(node, H1_DOM);
        else if (type == TextLineType.HEADING_2) return toLineDom(node, H2_DOM);
        else if (type == TextLineType.HEADING_3) return toLineDom(node, H3_DOM);
        else if (type == TextLineType.HEADING_4) return toLineDom(node, H4_DOM);
        else throw new Error(`unexpected heading type ${type}`);
      },
      parseDOM: [
        { tag: "h1", attrs: { type: TextLineType.HEADING_1 } },
        { tag: "h2", attrs: { type: TextLineType.HEADING_2 } },
        { tag: "h3", attrs: { type: TextLineType.HEADING_3 } },
        { tag: "h4", attrs: { type: TextLineType.HEADING_4 } },
      ],
    },
    // highlights
    lineCallout: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.CALLOUT } },
      toDOM(node) {
        return toLineDom(node, CALLOUT_DOM);
      },
      parseDOM: [{ tag: "p.callout", attrs: { type: TextLineType.CALLOUT } }],
    },
    lineQuote: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.QUOTE } },
      toDOM(node) {
        return toLineDom(node, QUOTE_DOM);
      },
      parseDOM: [{ tag: "blockquote", attrs: { type: TextLineType.QUOTE } }],
    },
    // list
    orderedList: {
      group: "line",
      content: "lineListOrdered+",
      toDOM(node) {
        return toLineDom(node, ["ol", { class: "ordered-list" }, 0]);
      },
      parseDOM: [{ tag: "ol" }],
    },
    lineListUnordered: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.LIST_UNORDERED } },
      toDOM(node) {
        return toLineDom(node, LIST_UNORDERED_DOM);
      },
      parseDOM: [{ tag: "li.list-unordered", attrs: { type: TextLineType.LIST_UNORDERED } }],
    },
    unorderedList: {
      group: "line",
      content: "lineListUnordered+",
      toDOM(node) {
        return toLineDom(node, ["ul", { class: "unordered-list" }, 0]);
      },
      parseDOM: [{ tag: "ul" }],
    },
    lineListOrdered: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.LIST_ORDERED } },
      // This node will now only be created inside an orderedList wrapper.
      toDOM(node) {
        return toLineDom(node, LIST_ORDERED_DOM);
      },
      parseDOM: [{ tag: "li.list-ordered", attrs: { type: TextLineType.LIST_ORDERED } }],
    },
    // presentation
    lineDivider: {
      group: "line",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.DIVIDER } },
      toDOM(node) {
        return toLineDom(node, HR_DOM);
      },
      parseDOM: [{ tag: "hr" }],
    },
    // table
    // ... TODO :Incomplete: TextTables
    // code
    lineCode: {
      group: "line",
      content: "text*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.CODE } },
      code: true,
      toDOM(node) {
        return toLineDom(node, LINE_CODE_DOM);
      },
      parseDOM: [{ tag: "code.line", attrs: { type: TextLineType.CODE } }],
    },
    // span
    text: {
      group: "span",
      inline: true,
      marks: "_", // all marks
    },
    spanHardBreak: {
      group: "span",
      inline: true,
      selectable: false,
      toDOM() {
        return SPAN_HARD_BREAK_DOM;
      },
      parseDOM: [{ tag: "br" }],
    },
    spanNode: {
      group: "span",
      draggable: true,
      inline: true,
      atom: true,
      marks: "",
      attrs: { nodePtr: {}, type: {} },
      // render custom; not parseable
    },
    spanLink: {
      group: "span",
      content: "text*",
      inline: true,
      marks: "",
      attrs: { href: {} },
      toDOM(node) {
        return ["a", { href: node.attrs.href }, 0];
      },
      parseDOM: [
        {
          tag: "a",
          getAttrs(dom) {
            return { href: (dom as HTMLAnchorElement).href };
          },
        },
      ],
    },
    spanEquation: {
      group: "span",
      inline: true,
      code: true,
      attrs: { blockPtr: { default: null }, type: { default: TextSpanType.EQUATION } },
      marks: "",
    },
    spanSpecialInput: {
      group: "span",
      inline: true,
      atom: true,
      attrs: { type: {} },
      // render custom; not parseable
    },
    // custom block node for non-text blocks
    block: {
      group: "line",
      atom: true,
      selectable: true,
      attrs: { blockPtr: {}, nodePtr: {} },
    },
  },
  marks: {
    bold: {
      parseDOM: [
        { tag: "strong" },
        // NOTE: work around a Google Docs misbehavior where pasted content will be inexplicably wrapped in `<b>` tags with a font-weight normal.
        { tag: "b", getAttrs: (node: HTMLElement) => node.style.fontWeight != "normal" && null },
        {
          style: "font-weight",
          getAttrs: (value) => /^(bold(er)?|[5-9]\d{2,})$/.test(value as string) && null,
        },
      ],
      toDOM() {
        return SPAN_STRONG_DOM;
      },
    },
    italic: {
      parseDOM: [
        { tag: "i" },
        { tag: "em" },
        { style: "font-style=italic" },
        { style: "font-style=normal", clearMark: (m) => m.type.name == "em" },
      ],
      toDOM() {
        return SPAN_ITALIC_DOM;
      },
    },
    strikethrough: {
      parseDOM: [{ tag: "s" }, { tag: "del" }, { tag: "strike" }],
      toDOM() {
        return SPAN_STRIKETHROUGH_DOM;
      },
    },
    underline: {
      parseDOM: [{ tag: "u" }, { style: "text-decoration=underline" }],
      toDOM() {
        return SPAN_UNDERLINE_DOM;
      },
    },
    code: {
      parseDOM: [{ tag: "code" }],
      toDOM() {
        return SPAN_CODE_DOM;
      },
    },
    // colors
    // TODO :Incomplete: Text foregroundColor, backgroundColor (for TextSpans and TextLines)
  },
});

/** Get the PM node type for a given span type. */
export function getPmSpanType(type: TextSpanType) {
  if (type == TextSpanType.UNSPECIFIED || type === TextSpanType.TEXT) {
    return PM_SCHEMA.nodes.text;
  } else if (type === TextSpanType.HARD_BREAK) {
    return PM_SCHEMA.nodes.spanHardBreak;
  } else if (type === TextSpanType.NODE) {
    return PM_SCHEMA.nodes.spanNode;
  } else if (type === TextSpanType.LINK) {
    return PM_SCHEMA.nodes.spanLink;
  } else if (type === TextSpanType.EQUATION) {
    return PM_SCHEMA.nodes.spanEquation;
  } else {
    assertNever(type);
  }
}
