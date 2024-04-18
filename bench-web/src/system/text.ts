import { ObjectType, TextData, TextLineData, TextLineType, TextSpanData } from "@/proto/wire";
import { newStructId } from "@/proto/wiring";
import { defaultSortStruct } from "@/system/lang";
import { generateOrderKeys } from "@/utils/fractional";
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
import { Node as PmNode, Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";
import { TextSelection } from "prosemirror-state";

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline" | "code";

const P_DOM: DOMOutputSpec = ["p", 0];
const H1_DOM: DOMOutputSpec = ["h1", 0];
const H2_DOM: DOMOutputSpec = ["h2", 0];
const H3_DOM: DOMOutputSpec = ["h3", 0];
const HR_DOM: DOMOutputSpec = ["hr"];
const STRONG_DOM: DOMOutputSpec = ["strong", 0];
const ITALIC_DOM: DOMOutputSpec = ["em", 0];

export const PM_SCHEMA = new PmSchema({
  nodes: {
    doc: { content: "line+" },
    // line types
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
      content: "text*",
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
    striketrough: {
      parseDOM: [{ tag: "s" }, { tag: "del" }, { tag: "strike" }],
      toDOM() {
        return ["s", 0];
      },
    },
    underline: {
      parseDOM: [{ tag: "u" }, { style: "text-decoration=underline" }],
      toDOM() {
        return ["u", 0];
      },
    },
    code: {
      parseDOM: [{ tag: "code" }],
      toDOM() {
        return ["code", 0];
      },
    },
  },
});

// nocheckin: text.marks

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
      if (span.content != null) {
        spanNode = schema.text(span.content);
      } else if (span.nodePtr != null) {
        spanNode = schema.node("mention", { nodePtr: span.nodePtr });
      } else {
        throw new Error(`unexpected span: ${span.id}`);
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
    const attrs = { id: line.id, type: line.type };
    if (line.type == TextLineType.PLAIN) {
      lineNode = schema.node("linePlain", attrs, spanNodes);
    } else if (
      line.type == TextLineType.HEADING_LARGE ||
      line.type == TextLineType.HEADING_MEDIUM ||
      line.type == TextLineType.HEADING_SMALL
    ) {
      lineNode = schema.node("lineHeading", attrs, spanNodes);
    } else if (line.type == TextLineType.DIVIDER) {
      lineNode = schema.node("lineDivider", attrs);
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
  const orderKeys = generateOrderKeys(null, null, node.childCount);
  for (let lineIdx = 0; lineIdx < node.childCount; lineIdx++) {
    const lineNode = node.child(lineIdx);

    // map spans
    const spans: TextSpanData[] = [];
    for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
      const spanNode = lineNode.child(spanIdx);
      let span: TextSpanData;
      if (spanNode.type.name == "text") {
        span = { metatype: ObjectType.TEXT_SPAN, id: spanIdx, content: spanNode.text };
      } else if (spanNode.type.name == "mention") {
        span = { metatype: ObjectType.TEXT_SPAN, id: spanIdx, nodePtr: spanNode.attrs.nodePtr };
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
    const line: TextLineData = {
      metatype: ObjectType.TEXT_LINE,
      id: lineNode.attrs.id ?? newStructId(),
      orderKey: orderKeys[lineIdx],
      setProperties: [],
      type: TextLineType.PLAIN,
      spans,
    };
    lines.push(line);
  }

  const text: TextData = { metatype: ObjectType.TEXT, id: prev?.id ?? newStructId(), setProperties: [], lines };
  return text;
}

const headingRule = (char: string, type: TextLineType) => {
  return new InputRule(new RegExp(`^(#{${char.length}})\\s$`), (state, match, start, end) => {
    const { tr } = state;
    // replace with heading line
    tr.replaceWith(start - char.length, end, state.schema.nodes.lineHeading.create({ type }));
    // and move cursor to the end of the line
    tr.setSelection(TextSelection.near(tr.doc.resolve(start - char.length + 1)));
    return tr;
  });
};

const dividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start, end, state.schema.nodes.lineDivider.create());
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
  // specific rules
  headingRule("#", TextLineType.HEADING_LARGE),
  headingRule("##", TextLineType.HEADING_MEDIUM),
  headingRule("###", TextLineType.HEADING_SMALL),
  dividerRule,
];
