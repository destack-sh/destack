import { TextLineType } from "@/proto/wire";
import {
  InputRule,
  closeDoubleQuote,
  closeSingleQuote,
  ellipsis,
  emDash,
  openDoubleQuote,
  openSingleQuote,
  smartQuotes,
} from "prosemirror-inputrules";
import { Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";
import { TextSelection } from "prosemirror-state";

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline" | "code";

const P_DOM: DOMOutputSpec = ["p", { class: "line" }, 0];
const H1_DOM: DOMOutputSpec = ["h1", { class: "line" }, 0];
const H2_DOM: DOMOutputSpec = ["h2", { class: "line" }, 0];
const H3_DOM: DOMOutputSpec = ["h3", { class: "line" }, 0];
const HR_DOM: DOMOutputSpec = ["hr", { clas: "line" }];
const CALLOUT_DOM: DOMOutputSpec = ["div", { class: "line callout" }, 0];
const QUOTE_DOM: DOMOutputSpec = ["blockquote", { class: "line" }, 0];

const STRONG_DOM: DOMOutputSpec = ["strong", 0];
const ITALIC_DOM: DOMOutputSpec = ["em", 0];
const STRIKETHROUGH_DOM: DOMOutputSpec = ["s", 0];
const UNDERLINE_DOM: DOMOutputSpec = ["u", 0];
const CODE_DOM: DOMOutputSpec = ["code", 0];

export const PM_SCHEMA = new PmSchema({
  nodes: {
    doc: { content: "line+" },
    // line
    linePlain: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.PLAIN } },
      toDOM(node) {
        return P_DOM;
      },
    },
    lineHeading: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.HEADING_LARGE } },
      toDOM(node) {
        const type = node.attrs.type;
        if (type == TextLineType.HEADING_LARGE) return H1_DOM;
        else if (type == TextLineType.HEADING_MEDIUM) return H2_DOM;
        else if (type == TextLineType.HEADING_SMALL) return H3_DOM;
        else throw new Error(`unexpected heading type ${type}`);
      },
      parseDOM: [
        { tag: "h1", attrs: { type: TextLineType.HEADING_LARGE } },
        { tag: "h2", attrs: { type: TextLineType.HEADING_MEDIUM } },
        { tag: "h3", attrs: { type: TextLineType.HEADING_SMALL } },
      ],
    },
    lineCallout: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.CALLOUT } },
      toDOM(node) {
        return CALLOUT_DOM;
      },
      parseDOM: [{ tag: "div.callout", attrs: { type: TextLineType.CALLOUT } }],
    },
    lineQuote: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.QUOTE } },
      toDOM(node) {
        return QUOTE_DOM;
      },
      parseDOM: [{ tag: "blockquote", attrs: { type: TextLineType.QUOTE } }],
    },
    lineDivider: {
      group: "line",
      attrs: { id: { default: null }, type: { default: TextLineType.DIVIDER } },
      toDOM(node) {
        return HR_DOM;
      },
      parseDOM: [{ tag: "hr" }],
    },
    // span
    text: {
      group: "span",
      inline: true,
      marks: "_", // all marks
    },
    mention: {
      group: "span",
      draggable: true,
      inline: true,
      atom: true,
      marks: "",
      attrs: { nodePtr: {} },
      // render manually, can't parse mention nodes
    },
    hardBreak: {
      inline: true,
      group: "span",
      selectable: false,
      toDOM() {
        return ["br"];
      },
      parseDOM: [{ tag: "br" }],
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
        return STRONG_DOM;
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
        return ITALIC_DOM;
      },
    },
    strikethrough: {
      parseDOM: [{ tag: "s" }, { tag: "del" }, { tag: "strike" }],
      toDOM() {
        return STRIKETHROUGH_DOM;
      },
    },
    underline: {
      parseDOM: [{ tag: "u" }, { style: "text-decoration=underline" }],
      toDOM() {
        return UNDERLINE_DOM;
      },
    },
    code: {
      parseDOM: [{ tag: "code" }],
      toDOM() {
        return CODE_DOM;
      },
    },
  },
});

const lineHeadingRule = (char: string, type: TextLineType) => {
  return new InputRule(new RegExp(`^(#{${char.length}})\\s$`), (state, match, start, end) => {
    const { tr } = state;
    // replace with heading line
    tr.replaceWith(start - match.length + 1, end, state.schema.nodes.lineHeading.create({ type }));
    // and move cursor to the end of the line
    tr.setSelection(TextSelection.near(tr.doc.resolve(start - char.length + 1)));
    return tr;
  });
};
const lineDividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start, end, state.schema.nodes.lineDivider.create());
  return tr;
});
const lineQuoteRule = new InputRule(/(^> )/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start - match.length, end, state.schema.nodes.lineQuote.create());
  tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
  return tr;
});
const lineCalloutRule = new InputRule(/(^! )/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start - match.length, end, state.schema.nodes.lineCallout.create());
  tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
  return tr;
});

export const PM_INPUT_RULES: InputRule[] = [
  // existing rules
  emDash,
  ellipsis,
  openDoubleQuote,
  closeDoubleQuote,
  openSingleQuote,
  closeSingleQuote,
  ...smartQuotes,
  // line rules
  lineHeadingRule("#", TextLineType.HEADING_LARGE),
  lineHeadingRule("##", TextLineType.HEADING_MEDIUM),
  lineHeadingRule("###", TextLineType.HEADING_SMALL),
  lineDividerRule,
  lineQuoteRule,
  lineCalloutRule,
];
