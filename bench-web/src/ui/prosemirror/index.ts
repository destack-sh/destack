import { supergraph } from "@/globals";
import { getBaseFromNodeReference } from "@/language/core/const";
import { ReadNodeGraph } from "@/language/core/graph";
import { uploadFile } from "@/language/resource/file";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { createBlock } from "@/language/source/block";
import {
  BlockData,
  NodeReferenceData,
  NodeType,
  ObjectType,
  PageData,
  TextData,
  TextLineData,
  TextLineType,
  TextSpanData,
  TextSpanType,
} from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { bench, pkgConnection } from "@/system/space";
import { type ActionImplementation, type ActionMapImplementation } from "@/ui/action";
import { useDropZone } from "@/ui/drag";
import { DEFAULT_MISSING_ICON, getNodeIcon, getNodeName, ICON_BY_NODE_TYPE } from "@/ui/icon";
import { getColorHex } from "@/ui/style";
import { groupByScalar } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
import { FocusAnchor, NavigationDirection } from "@/views/common";
import { whenever } from "@vueuse/core";
import * as commands from "prosemirror-commands";
import { dropCursor } from "prosemirror-dropcursor";
import {
  closeDoubleQuote,
  closeSingleQuote,
  ellipsis,
  emDash,
  InputRule,
  inputRules,
  openDoubleQuote,
  openSingleQuote,
  smartQuotes,
} from "prosemirror-inputrules";
import { keymap } from "prosemirror-keymap";
import { Node as PmNode, NodeType as PmNodeType, Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";
import {
  Command,
  EditorState,
  Plugin,
  TextSelection,
  type SelectionBookmark as EditorSelectionBookmark,
} from "prosemirror-state";
import { EditorView, type NodeView as PmNodeView } from "prosemirror-view";
import { computed, onBeforeUnmount, watch, type Ref } from "vue";

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline";

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
const LINE_CODE_DOM: DOMOutputSpec = ["p", { class: "line code" }, 0];

const SPAN_STRONG_DOM: DOMOutputSpec = ["strong", 0];
const SPAN_ITALIC_DOM: DOMOutputSpec = ["em", 0];
const SPAN_STRIKETHROUGH_DOM: DOMOutputSpec = ["s", 0];
const SPAN_UNDERLINE_DOM: DOMOutputSpec = ["u", 0];
const SPAN_CODE_DOM: DOMOutputSpec = ["code", 0];
const SPAN_HARD_BREAK_DOM: DOMOutputSpec = ["br"];

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
        return P_DOM;
      },
    },
    // headings
    lineHeading: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.HEADING_1 } },
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
    // highlights
    lineCallout: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.CALLOUT } },
      toDOM(node) {
        return CALLOUT_DOM;
      },
      parseDOM: [{ tag: "p.callout", attrs: { type: TextLineType.CALLOUT } }],
    },
    lineQuote: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.QUOTE } },
      toDOM(node) {
        return QUOTE_DOM;
      },
      parseDOM: [{ tag: "blockquote", attrs: { type: TextLineType.QUOTE } }],
    },
    // list
    lineListUnordered: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.LIST_UNORDERED } },
      toDOM(node) {
        return LIST_UNORDERED_DOM;
      },
      parseDOM: [{ tag: "li.list-unordered", attrs: { type: TextLineType.LIST_UNORDERED } }],
    },
    lineListOrdered: {
      group: "line",
      content: "span*",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.LIST_ORDERED } },
      toDOM(node) {
        return LIST_ORDERED_DOM;
      },
      parseDOM: [{ tag: "li.list-ordered", attrs: { type: TextLineType.LIST_ORDERED } }],
    },
    // presentation
    lineDivider: {
      group: "line",
      attrs: { blockPtr: { default: null }, type: { default: TextLineType.DIVIDER } },
      toDOM(node) {
        return HR_DOM;
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
        return LINE_CODE_DOM;
      },
      parseDOM: [{ tag: "div.line.code", attrs: { type: TextLineType.CODE } }],
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
      attrs: { nodePtr: {}, type: { default: TextSpanType.NODE } },
      // render manually, can't parse nodes
    },
    spanLink: {
      group: "span",
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
    spanCode: {
      group: "span",
      inline: true,
      code: true,
      attrs: { blockPtr: { default: null }, type: { default: TextSpanType.CODE } },
      marks: "",
      toDOM(node) {
        return SPAN_CODE_DOM;
      },
      parseDOM: [{ tag: "code", attrs: { type: TextSpanType.CODE } }],
    },
    spanEquation: {
      group: "span",
      inline: true,
      code: true,
      attrs: { blockPtr: { default: null }, type: { default: TextSpanType.EQUATION } },
      marks: "",
      // render manually & can't parse equation nodes
    },
  },
  marks: {
    // basic options
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
    // colors
    // TODO :Incomplete: foregroundColor, backgroundColor
  },
});

//
// Mapping
//

/** TextLine + metadata */
type TextLineInterface = {
  line: TextLineData;
  blockPtr: NodeReferenceData | null; // if the TextLine comes from a Block
};

/** Read/write source of Text */
type TextInterface = {
  /** Reads the TextLines (reactive). */
  read: () => TextLineInterface[];
  /** Writes the TextLine, returning the updated TextLines (if changed). */
  write: (lines: TextLineInterface[]) => TextLineInterface[];
};

/** Read/write directly from TextData. */
export function useTextModelValueInterface(options: {
  modelValue: Readonly<Ref<TextData | undefined | null>>;
  update: (text: TextData) => void;
}) {
  const { modelValue, update } = options;

  const lines = computed(() => {
    const lines: TextLineInterface[] = [];
    for (const line of modelValue.value?.lines ?? []) {
      lines.push({ line, blockPtr: null });
    }
    return lines;
  });

  function read(): TextLineInterface[] {
    return lines.value;
  }

  function write(lines: TextLineInterface[]): TextLineInterface[] {
    update({ metatype: ObjectType.TEXT, lines: lines.map(({ line }) => line) });
    return lines;
  }

  return { read, write };
}

/** Read/write Text from Blocks. */
export function useTextBlockGroupInterface(options: {
  page: Ref<PageData>;
  blocks: Ref<BlockData[]>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
}) {
  const { page, blocks, graph, txFactory } = options;

  const blocksById: Ref<Record<string, BlockData>> = computed(() => groupByScalar(blocks.value, (b) => b.id));

  // map blocks to lines
  const lines = computed(() => {
    const lines: TextLineInterface[] = [];
    for (const block of blocks.value) {
      const line: TextLineData = block.text ?? {
        metatype: ObjectType.TEXT_LINE,
        type: TextLineType.PARAGRAPH,
        spans: [],
        cells: [],
      };
      const blockPtr = toNodeRef(block);
      lines.push({ line, blockPtr });
    }
    return lines;
  });

  function read(): TextLineInterface[] {
    return lines.value;
  }

  /** Difference update the TextLines with Blocks. */
  function write(lines: TextLineInterface[]) {
    const lineByBlockId: Record<string, TextLineInterface> = {};
    const newLines: TextLineInterface[] = lines.map((l) => ({ ...l }));
    for (const line of lines) {
      if (line.blockPtr?.id != null) {
        lineByBlockId[line.blockPtr.id] = line;
      }
    }

    const tx = txFactory().with({ change: { title: "Edit", key: newChangeId() } });
    console.log("block.write", { lines, lineByBlockId });

    // delete removed blocks
    for (const block of blocks.value) {
      if (lineByBlockId[block.id] == null) {
        tx.delete(block);
        console.log("block.delete", block.id, { block });
      }
    }

    // add new blocks
    let prevBlockId: string | undefined = undefined;
    for (const line of newLines) {
      if (line.blockPtr?.id != null) {
        prevBlockId = line.blockPtr.id;
      } else {
        const prevBlock = blocksById.value[prevBlockId!];
        const block = createBlock(tx, graph, {
          block: { type: (line.line.type + 10_000) as any, text: line.line },
          anchor: prevBlock != null ? "after" : "inside",
          target: prevBlock ?? page.value,
        });
        line.blockPtr = toNodeRef(block);
        console.log("block.create", block.id, { line, block });
      }
    }

    // update blocks
    for (const block of blocks.value) {
      const line = lineByBlockId[block.id];
      if (line != null && !deepValueEquals(block.text, line.line)) {
        tx.update(block, { text: line.line }, { debounce: "long" });
        console.log("block.update", block.id, { line, block });
      }
    }

    return newLines;
  }

  return { read, write };
}

/** Convert TextData to a PmNode. */
export function mapTextToPmNode(lines: TextLineInterface[]): PmNode {
  const schema = PM_SCHEMA;

  // lines
  const lineNodes: PmNode[] = [];
  for (const { line, blockPtr } of lines) {
    // spans
    const spanNodes: PmNode[] = [];
    for (const span of line.spans) {
      let spanNode;
      if (span.type == TextSpanType.UNSPECIFIED || span.type == TextSpanType.TEXT) {
        spanNode = schema.text(span.content ?? "");
      } else if (span.type == TextSpanType.HARD_BREAK) {
        spanNode = schema.node("spanHardBreak");
      } else if (span.type == TextSpanType.NODE) {
        spanNode = schema.node("spanNode", { nodePtr: span.nodePtr });
      } else if (span.type == TextSpanType.LINK) {
        spanNode = schema.node("spanLink", { content: span.content, href: span.url });
      } else if (span.type == TextSpanType.CODE) {
        spanNode = schema.node("spanCode", { content: span.content });
      } else if (span.type == TextSpanType.EQUATION) {
        spanNode = schema.node("spanEquation", { content: span.content });
      } else {
        throw new Error(`unexpected span: ${JSON.stringify(span)}`);
      }
      // marks
      const markTypes: TextMarkType[] = [];
      if (span.isBold) markTypes.push("bold");
      if (span.isItalic) markTypes.push("italic");
      if (span.isStrikethrough) markTypes.push("strikethrough");
      if (span.isUnderline) markTypes.push("underline");
      if (markTypes.length > 0) {
        const marks = markTypes.map((type) => schema.mark(type));
        spanNode = spanNode.mark(marks);
      }
      spanNodes.push(spanNode);
    }

    // line
    let lineNode: PmNode;
    const attrs = { type: line.type, blockPtr };
    if (line.type == TextLineType.PARAGRAPH) {
      lineNode = schema.node("lineParagraph", attrs, spanNodes);
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
    } else if (line.type == TextLineType.CODE) {
      lineNode = schema.node("lineCode", attrs, spanNodes);
    } else if (line.type == TextLineType.LIST_UNORDERED) {
      lineNode = schema.node("lineListUnordered", attrs, spanNodes);
    } else if (line.type == TextLineType.LIST_ORDERED) {
      lineNode = schema.node("lineListOrdered", attrs, spanNodes);
    } else {
      throw new Error(`unexpected line type: ${line.type}`);
    }
    lineNodes.push(lineNode);
  }

  // doc
  if (lineNodes.length == 0) {
    lineNodes.push(schema.node("lineParagraph")); // ensure at least one line
  }
  const docNode = schema.node("doc", {}, lineNodes);
  return docNode;
}

/** Convert a PmNode to TextData. */
export function mapPmNodeToText(node: PmNode): TextLineInterface[] {
  const lines: TextLineInterface[] = [];
  for (let lineIdx = 0; lineIdx < node.childCount; lineIdx++) {
    const lineNode = node.child(lineIdx);

    // map spans
    const spans: TextSpanData[] = [];
    for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
      const spanNode = lineNode.child(spanIdx);
      let span: TextSpanData;
      if (spanNode.type.name == "spanHardBreak") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.HARD_BREAK };
      } else if (spanNode.type.name == "text") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.TEXT, content: spanNode.text };
      } else if (spanNode.type.name == "spanNode") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.NODE, nodePtr: spanNode.attrs.nodePtr };
      } else if (spanNode.type.name == "spanLink") {
        span = {
          metatype: ObjectType.TEXT_SPAN,
          type: TextSpanType.LINK,
          url: spanNode.attrs.href,
          content: spanNode.text,
        };
      } else if (spanNode.type.name == "spanCode") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.CODE, content: spanNode.text };
      } else if (spanNode.type.name == "spanEquation") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.EQUATION, content: spanNode.text };
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
        } else {
          throw new Error(`unexpected mark type: ${mark.type.name}`);
        }
      }
      spans.push(span);
    }

    // map line
    const line: TextLineData = { metatype: ObjectType.TEXT_LINE, type: lineNode.attrs.type, spans, cells: [] };
    const blockPtr = lineNode.attrs.blockPtr;
    lines.push({ line, blockPtr });
  }

  return lines;
}

//
// Editor
//

/** Mini-component for PM Node spans */
class SpanNodeView implements PmNodeView {
  dom: HTMLElement;
  nodePtr: NodeReferenceData;
  iconDom: HTMLElement;
  nameDom: HTMLElement;

  constructor(pmNode: PmNode, view: EditorView) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("spanNode");
    this.dom.dataset.nodeType = pmNode.attrs.nodePtr.nodeType;
    this.dom.dataset.nodeId = pmNode.attrs.nodePtr.id;
    this.dom.dataset.nodeCk = pmNode.attrs.nodePtr.ck;
    this.nodePtr = {
      ...pmNode.attrs.nodePtr,
      nodeType: Number(pmNode.attrs.nodePtr.nodeType),
      metatype: ObjectType.NODE_REFERENCE,
    };
    this.iconDom = this.dom.appendChild(document.createElement("span"));
    this.iconDom.classList.add(
      "icon",
      ...(ICON_BY_NODE_TYPE[pmNode.attrs.nodePtr.nodeType as unknown as NodeType]?.faName?.split(" ") ?? [
        "fas",
        "fa-question",
      ]),
    );
    this.nameDom = this.dom.appendChild(document.createElement("span"));
    this.nameDom.classList.add("name");
    this.nameDom.textContent = "???";

    this.updateNode();
  }

  updateNode() {
    const nodePtr = this.nodePtr;
    const node = supergraph.get(nodePtr);
    this.nameDom.textContent = (node != null ? getNodeName(node) : null) ?? "???";
    const icon = (node != null ? getNodeIcon(node) : null) ?? DEFAULT_MISSING_ICON;
    this.iconDom.className = icon?.faName != null ? `icon ${icon.faName}` : "icon fa fa-question";
    if (icon.color != null) this.iconDom.style.color = getColorHex(icon.color)!;
    else this.iconDom.style.removeProperty("color");
    this.dom.dataset.nodeType = nodePtr.nodeType.toString();
  }
}

function linePrefixRule(pattern: string | RegExp, nodeType: PmNodeType, type: TextLineType) {
  const regexp = typeof pattern == "string" ? new RegExp(`^(${pattern})$`) : pattern;
  const rule = new InputRule(regexp, (state, match, start, end) => {
    const { tr } = state;
    const $start = tr.doc.resolve(start);
    const block = $start.parent; // keep existing attrs, set new type
    tr.setBlockType(start, end, nodeType, { ...block.attrs, type });
    tr.delete(start, end);
    tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
    return tr;
  });
  return rule;
}
const lineDividerRule = new InputRule(/(^---$)|(^—-$)/, (state, match, start, end) => {
  const { tr } = state;
  tr.replaceWith(start - 1, end, state.schema.nodes.lineDivider.create());
  tr.insert(start, state.schema.nodes.lineParagraph.create());
  tr.setSelection(TextSelection.near(tr.doc.resolve(start)));
  return tr;
});

const UNORDERED_LIST_CHARS = ["-", "\\*", "•"];
const PM_INPUT_RULES: InputRule[] = [
  // existing rules
  emDash,
  ellipsis,
  openDoubleQuote,
  closeDoubleQuote,
  openSingleQuote,
  closeSingleQuote,
  ...smartQuotes,
  // line rules
  linePrefixRule("# ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_1),
  linePrefixRule("## ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_2),
  linePrefixRule("### ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_3),
  linePrefixRule("#### ", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_4),
  lineDividerRule,
  ...UNORDERED_LIST_CHARS.map((char) =>
    linePrefixRule(`${char} `, PM_SCHEMA.nodes.lineListUnordered, TextLineType.LIST_UNORDERED),
  ),
  ...UNORDERED_LIST_CHARS.map((char) =>
    linePrefixRule(` ${char} `, PM_SCHEMA.nodes.lineListUnordered, TextLineType.LIST_UNORDERED),
  ),
  linePrefixRule(/^[0-9a-z]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule(/^\s[0-9a-z]+\.\s/, PM_SCHEMA.nodes.lineListOrdered, TextLineType.LIST_ORDERED),
  linePrefixRule("> ", PM_SCHEMA.nodes.lineQuote, TextLineType.QUOTE),
  linePrefixRule("! ", PM_SCHEMA.nodes.lineCallout, TextLineType.CALLOUT),
  linePrefixRule("``` ", PM_SCHEMA.nodes.lineCode, TextLineType.CODE),
  linePrefixRule("```", PM_SCHEMA.nodes.lineCode, TextLineType.CODE),
];

function getPMCommands(options: { navigate: (direction: NavigationDirection) => void; deleteSelf: () => void }) {
  const { navigate, deleteSelf } = options;
  const extraCommands: Record<string, Command> = {
    ArrowLeft(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[0] &&
        (state.doc.textContent == "" || view?.endOfTextblock("left", state))
      ) {
        navigate("left");
        return true;
      }
      return false;
    },
    ArrowRight(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[state.doc.children.length - 1] &&
        (state.doc.textContent == "" || view?.endOfTextblock("right", state))
      ) {
        navigate("right");
        return true;
      }
      return false;
    },
    ArrowUp(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[0] &&
        (state.doc.textContent == "" || view?.endOfTextblock("up", state))
      ) {
        navigate("up");
        return true;
      }
      return false;
    },
    ArrowDown(state, dispatch, view) {
      const { selection } = state;
      if (
        selection.$anchor.parent === state.doc.children[state.doc.children.length - 1] &&
        (state.doc.textContent == "" || view?.endOfTextblock("down", state))
      ) {
        navigate("down");
        return true;
      }
      return false;
    },
    Backspace: (state, dispatch) => {
      const { empty, $cursor, $from, $to } = state.selection as TextSelection;
      if (
        $from.parent.type.name !== "lineParagraph" &&
        $from.parentOffset === 0 &&
        $from.start() == $from.end() &&
        $to.pos === $from.end()
      ) {
        // convert to plain
        dispatch?.(
          state.tr.setBlockType($from.pos, $to.pos, state.schema.nodes.lineParagraph, {
            type: TextLineType.PARAGRAPH,
          }),
        );
        return true;
      } else if (empty && state.doc.textContent == "" && state.doc.children.length <= 1) {
        // delete self
        deleteSelf();
        return true;
      } else {
        // imitate default behavior
        return commands.chainCommands(
          commands.deleteSelection,
          commands.joinBackward,
          commands.selectNodeBackward,
        )(state, dispatch);
      }
    },
    "Mod-Enter": (state, dispatch, view) => {
      // if the cmd key is held, do the default split behavior
      return commands.splitBlock(state, dispatch);
    },
    Enter: (state, dispatch, view) => {
      if (!state.selection.empty) {
        return commands.splitBlock(state, dispatch);
      }
      const { $from } = state.selection;
      const parentType = $from.parent.type.name;
      if (parentType === "lineListOrdered" || parentType === "lineListUnordered") {
        // if the list item line is empty, convert it to a paragraph
        if ($from.parent.textContent.trim() === "") {
          dispatch?.(
            state.tr.setBlockType($from.start(), $from.end(), state.schema.nodes.lineParagraph, {
              ...$from.parent.attrs, // keep attrs incl. blockPtr
              type: TextLineType.PARAGRAPH,
            }),
          );
          return true;
        } else {
          // otherwise, split the list item at the cursor position (keep attrs except blockPtr)
          dispatch?.(
            state.tr.split($from.pos, 1, [
              { type: $from.parent.type, attrs: { ...$from.parent.attrs, blockPtr: null } },
            ]),
          );
          return true;
        }
      }
      return commands.splitBlock(state, dispatch);
    },
  };
  return extraCommands;
}

/**
 * Install a Text editor on a DOM element.
 */
export function useTextEditor(options: {
  textRef: Ref<HTMLElement | null>;
  text: TextInterface;
  isInput: Ref<boolean>;
  suppressEnter: Ref<boolean>;
  suppressDrop: Ref<boolean>;
  navigate: (direction: NavigationDirection) => void;
  deleteSelf: () => void;
}) {
  const previousSelectionByState: Record<number, EditorSelectionBookmark> = {};
  const { textRef, text, isInput, suppressEnter, suppressDrop, navigate, deleteSelf } = options;

  // state
  const lines = computed(() => text.read());
  const nodePtrs: Ref<NodeReferenceData[]> = computed(() => {
    const nodePtrs: NodeReferenceData[] = [];
    for (const line of lines.value) {
      for (const span of line.line.spans ?? []) {
        if (span.nodePtr != null) nodePtrs.push(span.nodePtr);
      }
    }
    return nodePtrs;
  });
  const basePtrs = computed(() => nodePtrs.value.map((ptr) => getBaseFromNodeReference(ptr)).filter((b) => b != null));
  const bases = supergraph.getManyRef(basePtrs);
  const nodes = supergraph.getManyRef(nodePtrs);

  // prosemirror state
  function makeEditorState(options: { lines: TextLineInterface[]; selection?: EditorSelectionBookmark }): EditorState {
    const doc = mapTextToPmNode(options.lines);
    console.log("makeEditorState", { doc });
    const bindings: Record<string, Command> = {
      ...commands.baseKeymap,
      ...getPMCommands({ navigate, deleteSelf }),
    };
    if (suppressEnter.value) {
      bindings["Shift-Enter"] = bindings.Enter;
      bindings.Enter = () => false;
    }
    const selection = options.selection?.resolve(doc);
    const state = EditorState.create({
      doc: doc,
      schema: PM_SCHEMA,
      selection,
      plugins: [keymap(bindings), inputRules({ rules: PM_INPUT_RULES })],
    });
    return state;
  }

  // prosemirror view
  let view: EditorView | null = null;
  let lastAppliedModelValue: TextLineInterface[] = [];
  function makeEditorView(): EditorView {
    const plugins: Plugin[] = [];
    if (!options.suppressDrop) {
      plugins.push(dropCursor({ width: 2, color: "#fbbf24" }));
    }
    const view = new EditorView(textRef.value, {
      state: makeEditorState({ lines: lines.value }),
      editable: () => isInput?.value ?? false,
      nodeViews: {
        spanNode: (node, view, getPos) => new SpanNodeView(node, view),
      },
      plugins,
      dispatchTransaction(tx) {
        if (view == null) throw new Error("view not mounted");
        // update the state
        let newState = view.state.apply(tx);
        const updatedText = mapPmNodeToText(newState.doc);
        // also write the doc (if changed)
        if (tx.docChanged) {
          lastAppliedModelValue = text.write(updatedText);
          if (lastAppliedModelValue !== updatedText) {
            // re-derive state in case we modified the doc (like when creating a new line)
            // NOTE :Performance: re-deriving the ProseMirror state (twice) seems wasteful.. but it's chugging fine so far
            newState = makeEditorState({ lines: lastAppliedModelValue, selection: newState.selection.getBookmark() });
          }
        }
        view.updateState(newState);
      },
    });
    return view;
  }

  // sync nodes with their views
  watch(
    nodes,
    () => {
      if (view == null) return;
      view.dom.querySelectorAll(".spanNode").forEach((nodeDom) => {
        if (!(nodeDom instanceof HTMLElement)) return;
        const pmView = (nodeDom as any).__pmView as SpanNodeView;
        if (pmView == null) return;
        pmView.updateNode();
      });
    },
    { immediate: true },
  );

  // mount the editor view
  whenever(textRef, () => {
    if (view) throw new Error("view already exists");
    lastAppliedModelValue = lines.value.map((l) => l);
    view = makeEditorView();
  });
  onBeforeUnmount(() => {
    view?.destroy();
    view = null;
  });

  // overwrite state from modelValue if different
  watch(lines, () => {
    if (view == null) return;
    if (deepValueEquals(lines.value, lastAppliedModelValue)) return; // already applied
    console.log("overwrite state from modelValue", { lines: lines.value, lastAppliedModelValue });
    const updatedState = makeEditorState({ lines: lines.value });
    view.updateState(updatedState);
    lastAppliedModelValue = lines.value;
  });

  function insertNode(nodePtr: NodeReferenceData, pos: { pos: number }) {
    if (view == null) throw new Error("view not mounted");
    const pmNode = PM_SCHEMA.node("spanNode", { type: TextSpanType.NODE, nodePtr });
    view.dispatch(view.state.tr.insert(pos.pos, pmNode).insertText(" ", pos.pos + 1, pos.pos + 1));
  }

  // drag/drop
  const { isInDropZone } = useDropZone({
    name: "text",
    container: textRef,
    isEnabled: computed(() => isInput?.value && !suppressDrop?.value),
    kinds: ["node", "file"],
    onDrop: (dragged, event) => {
      if (view == null) return;
      if (dragged.kind == "file") {
        // upload files and insert as nodes at position (surrounded by spaces)
        if (dragged.files == null) return;
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        Array.from(dragged.files).forEach(async (file) => {
          // upload and insert each file individually
          if (bench.value == null) throw new Error("no current bench");
          const upload = uploadFile(() => pkgConnection.tx, file, { bench: bench.value });
          await upload.completion.wait();
          if (view == null) throw new Error("view no mounted");
          insertNode(toNodeRef(upload.file.value!), pos);
        });
      } else if (dragged.kind == "node") {
        // insert node at position (surrounded by spaces)
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        insertNode(toNodeRef(dragged.node), pos);
      }
    },
  });

  // formatting
  function formatAction(mark: TextMarkType): ActionImplementation {
    return {
      isEnabled: () => isInput?.value ?? false,
      isChecked: () => {
        if (view == null) return false;
        const { from, to } = view.state.selection;
        let hasMark = false;
        view.state.doc.nodesBetween(from, to, (node) => {
          if (node.marks.some((markType) => markType.type.name === mark)) {
            hasMark = true;
          }
        });
        return hasMark;
      },
      action: () => {
        if (view == null) throw new Error("view not mounted");
        commands.toggleMark(view.state.schema.marks[mark])(view.state, view.dispatch);
      },
    };
  }

  // actions
  const actions: ActionMapImplementation<"text"> & Partial<ActionMapImplementation<"space">> = {
    // text
    "text.format.bold": formatAction("bold"),
    "text.format.italic": formatAction("italic"),
    "text.format.strikethrough": formatAction("strikethrough"),
    "text.format.underline": formatAction("underline"),
    "text.edit.hardBreak": {
      action: () => {
        // insert 'hardBreak' node at cursor
        if (view == null) return;
        const { from } = view.state.selection;
        const hardBreak = PM_SCHEMA.node("spanHardBreak");
        view.dispatch(view.state.tr.insert(from, hardBreak));
      },
    },
    // space
    "space.edit.delete": {
      action: () => commands.deleteSelection(view!.state, view!.dispatch),
    },
    "space.select.all": {
      action: () => commands.selectAll(view!.state, view!.dispatch),
    },
  };

  // focus the editor
  function focus(anchor: FocusAnchor | NodeReferenceData = "bottom") {
    if (view == null) throw new Error("view not mounted");
    const { state } = view;
    view.focus();
    let selection;
    if (anchor == "top" || anchor == "left") {
      selection = TextSelection.atStart(state.doc);
    } else {
      selection = TextSelection.atEnd(state.doc);
    }
    view.dispatch(state.tr.setSelection(selection));
  }

  return { focus, actions, isInDropZone };
}
