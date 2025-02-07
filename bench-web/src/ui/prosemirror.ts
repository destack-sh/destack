import { defaultSortStruct } from "@/language/core/order";
import { ObjectType, TextData, TextLineData, TextLineType, TextSpanData } from "@/proto/wire";
import * as commands from "prosemirror-commands";
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
import { Node as PmNode, NodeType as PmNodeType, Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";
import { EditorState, Transaction as PmTransaction, TextSelection } from "prosemirror-state";

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline" | "code";

const P_DOM: DOMOutputSpec = ["p", { class: "line" }, 0];
const H1_DOM: DOMOutputSpec = ["h1", { class: "line" }, 0];
const H2_DOM: DOMOutputSpec = ["h2", { class: "line" }, 0];
const H3_DOM: DOMOutputSpec = ["h3", { class: "line" }, 0];
const H4_DOM: DOMOutputSpec = ["h4", { class: "line" }, 0];
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
      attrs: { id: { default: null }, type: { default: TextLineType.PARAGRAPH } },
      toDOM(node) {
        return P_DOM;
      },
    },
    lineHeading: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.HEADING_1 } },
      toDOM(node) {
        const type = node.attrs.type;
        if (type == TextLineType.HEADING_1) return H1_DOM;
        else if (type == TextLineType.HEADING_2) return H2_DOM;
        else if (type == TextLineType.HEADING_3) return H3_DOM;
        else if (type == TextLineType.HEADING_4) return H4_DOM;
        else throw new Error(`unexpected heading type ${type}`);
      },
      parseDOM: [
        { tag: "h1", attrs: { type: TextLineType.HEADING_1 } },
        { tag: "h2", attrs: { type: TextLineType.HEADING_2 } },
        { tag: "h3", attrs: { type: TextLineType.HEADING_3 } },
        { tag: "h4", attrs: { type: TextLineType.HEADING_4 } },
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

function blockTypeRule(char: string, nodeType: PmNodeType, type: TextLineType) {
  return new InputRule(new RegExp(`^(${char})\\s$`), (state, match, start, end) => {
    const { tr } = state;
    tr.setBlockType(start, end, nodeType, { type });
    tr.delete(start, end);
    tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
    return tr;
  });
}
const lineDividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start - 1, end, state.schema.nodes.lineDivider.create());
  tr.insert(start, state.schema.nodes.linePlain.create());
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
  blockTypeRule("#", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_1),
  blockTypeRule("##", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_2),
  blockTypeRule("###", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_3),
  blockTypeRule("####", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_4),
  lineDividerRule,
  blockTypeRule(">", PM_SCHEMA.nodes.lineQuote, TextLineType.QUOTE),
  blockTypeRule("!", PM_SCHEMA.nodes.lineCallout, TextLineType.CALLOUT),
];

/* Convert non-plain text nodes to plain text nodes when deleting */
function convertToPlainBeforeDelete(state: EditorState, dispatch?: (tr: PmTransaction) => void) {
  const { $from, $to } = state.selection;
  if (!dispatch) {
    return false;
  } else if (
    $from.node().type.name !== "linePlain" &&
    $from.parentOffset === 0 &&
    $from.start() == $from.end() &&
    $to.pos === $from.end()
  ) {
    // convert to plain
    dispatch(state.tr.setBlockType($from.pos, $to.pos, state.schema.nodes.linePlain, { type: TextLineType.PARAGRAPH }));
    return true;
  } else {
    // imitate default behavior
    return commands.chainCommands(
      commands.deleteSelection,
      commands.joinBackward,
      commands.selectNodeBackward,
    )(state, dispatch);
  }
}

// Keymap integration
export const PM_KEYMAP_EXTRA = {
  Backspace: convertToPlainBeforeDelete,
};


// TODO :Performance: mapTextToPmNode/mapPmNodeToText should cache somehow?
//  (we re-create the entire deep object on every conversion)

export function mapTextToPmNode(text: TextData, prev: PmNode | undefined): PmNode {
  const schema = PM_SCHEMA;
  defaultSortStruct(text.lines);

  // map lines
  const lineNodes: PmNode[] = [];
  for (const line of text.lines) {
    // map spans
    const spanNodes: PmNode[] = [];
    for (const span of line.spans) {
      let spanNode;
      if (span.content == "\n") {
        spanNode = schema.node("hardBreak");
      } else if (span.content != null) {
        spanNode = schema.text(span.content);
      } else if (span.nodePtr != null) {
        spanNode = schema.node("mention", { nodePtr: span.nodePtr });
      } else {
        throw new Error(`unexpected span: ${JSON.stringify(span)}`);
      }
      // map marks
      const markTypes: TextMarkType[] = [];
      if (span.isBold) markTypes.push("bold");
      if (span.isItalic) markTypes.push("italic");
      if (span.isStrikethrough) markTypes.push("strikethrough");
      if (span.isUnderline) markTypes.push("underline");
      if (span.isCode) markTypes.push("code");
      if (markTypes.length > 0) {
        const marks = markTypes.map((type) => schema.mark(type));
        spanNode = spanNode.mark(marks);
      }
      spanNodes.push(spanNode);
    }

    // map line
    let lineNode: PmNode;
    const attrs = { type: line.type };
    if (line.type == TextLineType.PARAGRAPH) {
      lineNode = schema.node("linePlain", attrs, spanNodes);
    } else if (
      line.type == TextLineType.HEADING_1 ||
      line.type == TextLineType.HEADING_2 ||
      line.type == TextLineType.HEADING_3 ||
      line.type == TextLineType.HEADING_4
    ) {
      lineNode = schema.node("lineHeading", attrs, spanNodes);
    } else if (line.type == TextLineType.DIVIDER) {
      lineNode = schema.node("lineDivider", attrs);
    } else if (line.type == TextLineType.QUOTE) {
      lineNode = schema.node("lineQuote", attrs, spanNodes);
    } else if (line.type == TextLineType.CALLOUT) {
      lineNode = schema.node("lineCallout", attrs, spanNodes);
    } else {
      throw new Error(`unexpected line type: ${line.type}`);
    }
    lineNodes.push(lineNode);
  }

  // map doc
  if (lineNodes.length == 0) {
    lineNodes.push(schema.node("linePlain")); // ensure at least one line
  }
  const docNode = schema.node("doc", {}, lineNodes);
  return docNode;
}

export function mapPmNodeToText(node: PmNode, prev: TextData | undefined): TextData {
  const lines: TextLineData[] = [];
  for (let lineIdx = 0; lineIdx < node.childCount; lineIdx++) {
    const lineNode = node.child(lineIdx);

    // map spans
    const spans: TextSpanData[] = [];
    for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
      const spanNode = lineNode.child(spanIdx);
      let span: TextSpanData;
      if (spanNode.type.name == "hardBreak") {
        span = { metatype: ObjectType.TEXT_SPAN, content: "\n" };
      } else if (spanNode.type.name == "text") {
        span = { metatype: ObjectType.TEXT_SPAN, content: spanNode.text };
      } else if (spanNode.type.name == "mention") {
        span = { metatype: ObjectType.TEXT_SPAN, nodePtr: spanNode.attrs.nodePtr };
      } else {
        throw new Error(`unexpected span node type: ${spanNode.type.name}`);
      }
      // map marks
      for (const mark of spanNode.marks) {
        if (mark.type.name == "bold") {
          span.isBold = true;
        } else if (mark.type.name == "italic") {
          span.isItalic = true;
        } else if (mark.type.name == "strikethrough") {
          span.isStrikethrough = true;
        } else if (mark.type.name == "underline") {
          span.isUnderline = true;
        } else if (mark.type.name == "code") {
          span.isCode = true;
        } else {
          throw new Error(`unexpected mark type: ${mark.type.name}`);
        }
      }
      spans.push(span);
    }

    // map line
    const line: TextLineData = { metatype: ObjectType.TEXT_LINE, type: lineNode.attrs.type, spans };
    lines.push(line);
  }

  const text: TextData = { metatype: ObjectType.TEXT, lines };
  return text;
}
