import { supergraph } from "@/globals";
import { getBaseFromNodeReference } from "@/language/core/const";
import { defaultSortStruct } from "@/language/core/order";
import { uploadFile } from "@/language/resource/file";
import {
  NodeReferenceData,
  NodeType,
  ObjectType,
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
import {} from "@/ui/prosemirror";
import { getColorHex } from "@/ui/style";
import { copy, cyrb53a } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
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
import { Node as PmNode } from "prosemirror-model";
import {
  Command,
  Selection as EditorSelection,
  EditorState,
  Plugin,
  TextSelection,
  type SelectionBookmark as EditorSelectionBookmark,
} from "prosemirror-state";
import { EditorView, type NodeView as PmNodeView } from "prosemirror-view";
import { computed, onBeforeUnmount, toRef, watch, type Ref } from "vue";

import { NodeType as PmNodeType, Schema as PmSchema, type DOMOutputSpec } from "prosemirror-model";
import { Transaction as PmTransaction } from "prosemirror-state";

export type TextMarkType = "bold" | "italic" | "strikethrough" | "underline";

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
const LIST_BULLET_DOM: DOMOutputSpec = ["ul", 0];
const LIST_NUMBERED_DOM: DOMOutputSpec = ["ol", 0];
const LIST_CHECKED_DOM: DOMOutputSpec = ["div", { class: "list-checked" }, 0];
const LIST_UNCHECKED_DOM: DOMOutputSpec = ["div", { class: "list-unchecked" }, 0];

export const PM_SCHEMA = new PmSchema({
  nodes: {
    doc: { content: "line+" },
    //
    // line
    //
    lineParagraph: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.PARAGRAPH } },
      toDOM(node) {
        return P_DOM;
      },
    },
    // headings
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
    // highlights
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
    // list
    lineListBullet: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.LIST_BULLET } },
      toDOM(node) {
        return LIST_BULLET_DOM;
      },
      parseDOM: [{ tag: "ul", attrs: { type: TextLineType.LIST_BULLET } }],
    },
    lineListNumber: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.LIST_NUMBERED } },
      toDOM(node) {
        return LIST_NUMBERED_DOM;
      },
      parseDOM: [{ tag: "ol", attrs: { type: TextLineType.LIST_NUMBERED } }],
    },
    lineListChecked: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.LIST_CHECKED } },
      toDOM(node) {
        return LIST_CHECKED_DOM;
      },
      parseDOM: [{ tag: "div.list-checked", attrs: { type: TextLineType.LIST_CHECKED } }],
    },
    lineListUnchecked: {
      group: "line",
      content: "span*",
      attrs: { id: { default: null }, type: { default: TextLineType.LIST_UNCHECKED } },
      toDOM(node) {
        return LIST_UNCHECKED_DOM;
      },
      parseDOM: [{ tag: "div.list-unchecked", attrs: { type: TextLineType.LIST_UNCHECKED } }],
    },
    // presentation
    lineDivider: {
      group: "line",
      attrs: { id: { default: null }, type: { default: TextLineType.DIVIDER } },
      toDOM(node) {
        return HR_DOM;
      },
      parseDOM: [{ tag: "hr" }],
    },
    // table
    // ...
    // code
    lineCode: {
      group: "line",
      content: "text*",
      attrs: { id: { default: null }, type: { default: TextLineType.CODE } },
      code: true,
      toDOM(node) {
        return CODE_DOM;
      },
      parseDOM: [{ tag: "code", attrs: { type: TextLineType.CODE } }],
    },
    // span
    text: {
      group: "span",
      inline: true,
      marks: "_", // all marks
    },
    spanMention: {
      group: "span",
      draggable: true,
      inline: true,
      atom: true,
      marks: "",
      attrs: { nodePtr: {} },
      // render manually, can't parse mention nodes
    },
    spanCode: {
      group: "span",
      inline: true,
      code: true,
      attrs: { id: { default: null }, type: { default: TextSpanType.CODE } },
      marks: "",
      toDOM(node) {
        return CODE_DOM;
      },
      parseDOM: [{ tag: "code", attrs: { type: TextSpanType.CODE } }],
    },
    spanHardBreak: {
      group: "span",
      inline: true,
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
  },
});

function lineTypeRule(char: string, nodeType: PmNodeType, type: TextLineType) {
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
  tr.insert(start, state.schema.nodes.lineParagraph.create());
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
  lineTypeRule("#", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_1),
  lineTypeRule("##", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_2),
  lineTypeRule("###", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_3),
  lineTypeRule("####", PM_SCHEMA.nodes.lineHeading, TextLineType.HEADING_4),
  lineDividerRule,
  lineTypeRule(">", PM_SCHEMA.nodes.lineQuote, TextLineType.QUOTE),
  lineTypeRule("!", PM_SCHEMA.nodes.lineCallout, TextLineType.CALLOUT),
];

/* Convert non-plain text nodes to plain text nodes when deleting */
function convertToPlainBeforeDelete(state: EditorState, dispatch?: (tr: PmTransaction) => void) {
  const { $from, $to } = state.selection;
  if (!dispatch) {
    return false;
  } else if (
    $from.node().type.name !== "lineParagraph" &&
    $from.parentOffset === 0 &&
    $from.start() == $from.end() &&
    $to.pos === $from.end()
  ) {
    // convert to plain
    dispatch(
      state.tr.setBlockType($from.pos, $to.pos, state.schema.nodes.lineParagraph, { type: TextLineType.PARAGRAPH }),
    );
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

//
// Mapping
//

/** Convert TextData to a PmNode. */
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
    } else {
      throw new Error(`unexpected line type: ${line.type}`);
    }
    lineNodes.push(lineNode);
  }

  // map doc
  if (lineNodes.length == 0) {
    lineNodes.push(schema.node("lineParagraph")); // ensure at least one line
  }
  const docNode = schema.node("doc", {}, lineNodes);
  return docNode;
}

/** Convert a PmNode to TextData. */
export function mapPmNodeToText(node: PmNode): TextData {
  const lines: TextLineData[] = [];
  for (let lineIdx = 0; lineIdx < node.childCount; lineIdx++) {
    const lineNode = node.child(lineIdx);

    // map spans
    const spans: TextSpanData[] = [];
    for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
      const spanNode = lineNode.child(spanIdx);
      let span: TextSpanData;
      if (spanNode.type.name == "hardBreak") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.HARD_BREAK, content: "\n" };
      } else if (spanNode.type.name == "text") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.TEXT, content: spanNode.text };
      } else if (spanNode.type.name == "mention") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.NODE, nodePtr: spanNode.attrs.nodePtr };
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
    lines.push(line);
  }

  const text: TextData = { metatype: ObjectType.TEXT, lines };
  return text;
}

/** Mini-component for PM mentions */
class MentionView implements PmNodeView {
  dom: HTMLElement;
  nodePtr: NodeReferenceData;
  iconDom: HTMLElement;
  nameDom: HTMLElement;

  constructor(pmNode: PmNode, view: EditorView) {
    this.dom = document.createElement("span");
    (this.dom as any).__pmView = this;
    this.dom.classList.add("mention");
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

    this.updateMention();
  }

  updateMention() {
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

export function useTextEditor(props: {
  textRef: Ref<HTMLElement | null>;
  modelValue: Ref<TextData | undefined | null>;
  isInput: Ref<boolean>;
  suppressEnter: Ref<boolean>;
  suppressDrop: Ref<boolean>;
}) {
  const previousSelectionByState: Record<number, EditorSelectionBookmark> = {};
  const { textRef, modelValue, isInput, suppressEnter, suppressDrop } = props;

  const nodePtrs: Ref<NodeReferenceData[]> = computed(() => {
    const nodePtrs: NodeReferenceData[] = [];
    for (const line of modelValue?.value?.lines ?? []) {
      for (const span of line.spans ?? []) {
        if (span.nodePtr != null) nodePtrs.push(span.nodePtr);
      }
    }
    return nodePtrs;
  });
  const basePtrs = computed(() => nodePtrs.value.map((ptr) => getBaseFromNodeReference(ptr)).filter((b) => b != null));
  const bases = supergraph.getManyRef(basePtrs);
  const mentions = supergraph.getManyRef(nodePtrs);

  function makeEditorState(text: TextData | undefined | null, options?: { restoreSelection?: boolean }): EditorState {
    const doc = text != null ? mapTextToPmNode(text, undefined) : undefined;
    let selection: EditorSelection | undefined = undefined;
    if (options?.restoreSelection && doc != null) {
      selection = previousSelectionByState[cyrb53a(text)]?.resolve(doc);
    }
    const bindings: Record<string, Command> = {
      ...commands.baseKeymap,
      ...PM_KEYMAP_EXTRA,
    };
    if (suppressEnter.value) {
      bindings["Shift-Enter"] = commands.baseKeymap["Enter"];
      bindings.Enter = () => true;
    }
    return EditorState.create({
      doc: doc,
      schema: PM_SCHEMA,
      selection,
      plugins: [keymap(bindings), inputRules({ rules: PM_INPUT_RULES })],
    });
  }

  let view: EditorView | null = null;
  let lastAppliedModelValue: TextData | null = null;

  function makeEditorView(): EditorView {
    const plugins: Plugin[] = [];
    if (!props.suppressDrop) {
      plugins.push(dropCursor({ width: 2, color: "#fbbf24" }));
    }
    return new EditorView(textRef.value, {
      state: makeEditorState(modelValue?.value),
      editable: () => isInput?.value ?? false,
      nodeViews: {
        mention: (node, view, getPos) => new MentionView(node, view),
      },
      plugins,
      dispatchTransaction(tx) {
        if (view == null) throw new Error("view not mounted");

        // update the state directly for responsiveness & performance
        const newState = view.state.apply(tx);
        view.updateState(newState);
        const updatedText = mapPmNodeToText(newState.doc);
        if (view?.state.selection != null) {
          // remember selection for this state
          previousSelectionByState[cyrb53a(updatedText)] = newState.selection.getBookmark();
        }
        // also update the modelValue if underlying doc changed
        if (tx.docChanged) {
          lastAppliedModelValue = updatedText;
          // emit("update:modelValue", updatedText);
        }
      },
    });
  }

  // sync mentions with mention views
  watch(
    mentions,
    () => {
      if (view == null) return;
      view.dom.querySelectorAll(".mention").forEach((mentionDom) => {
        if (!(mentionDom instanceof HTMLElement)) return;
        const pmView = (mentionDom as any).__pmView as MentionView;
        if (pmView == null) return;
        pmView.updateMention();
      });
    },
    { immediate: true },
  );

  // mount the editor view
  whenever(textRef, () => {
    if (view) throw new Error("view already exists");
    lastAppliedModelValue = copy(modelValue?.value ?? null);
    view = makeEditorView();
  });
  onBeforeUnmount(() => {
    view?.destroy();
    view = null;
  });

  // overwrite state from modelValue if different
  watch(toRef(props, "modelValue"), () => {
    if (view == null) return;
    if (deepValueEquals(modelValue?.value, lastAppliedModelValue)) return;
    const updatedState = makeEditorState(modelValue?.value, { restoreSelection: true });
    view.updateState(updatedState);
    lastAppliedModelValue = modelValue?.value ?? null;
  });

  function insertMention(nodePtr: NodeReferenceData, pos: { pos: number }) {
    if (view == null) throw new Error("view not mounted");
    const pmNode = PM_SCHEMA.node("mention", { nodePtr });
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
        // upload files and insert as mentions at position (surrounded by spaces)
        if (dragged.files == null) return;
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        Array.from(dragged.files).forEach(async (file) => {
          // upload and insert each file individually
          if (bench.value == null) throw new Error("no current bench");
          const upload = uploadFile(() => pkgConnection.tx, file, { bench: bench.value });
          await upload.completion.wait();
          if (view == null) throw new Error("view no mounted");
          insertMention(toNodeRef(upload.file.value!), pos);
        });
      } else if (dragged.kind == "node") {
        // insert node mention at position (surrounded by spaces)
        const pos = view.posAtCoords({ left: event.clientX, top: event.clientY });
        if (pos == null) return; // not in editor
        insertMention(toNodeRef(dragged.node), pos);
      }
    },
  });

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
        const hardBreak = PM_SCHEMA.node("hardBreak");
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

  function focus() {
    if (view) {
      const { state } = view;
      const end = state.doc.content.size;
      view.focus();
      view.dispatch(state.tr.setSelection(TextSelection.create(state.doc, end)));
    }
  }

  return { focus, actions, isInDropZone };
}
