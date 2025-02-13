import { ReadNodeGraph } from "@/language/core/graph";
import { emptyTextLine } from "@/language/core/text";
import { newChangeId, Transaction } from "@/language/runtime/transaction";
import { createBlock } from "@/language/source/block";
import {
  BlockData,
  BlockType,
  NodeReferenceData,
  ObjectType,
  PageData,
  TextData,
  TextLineData,
  TextLineType,
  TextSpanData,
  TextSpanType,
} from "@/proto/wire";
import { toNodeRef } from "@/proto/wiring";
import { PM_SCHEMA } from "@/ui/prosemirror/schema";
import { assertNever, groupByScalar } from "@/utils/functools";
import { deepValueEquals } from "@/utils/ref";
import { Node as PmNode, type Mark as PmMark } from "prosemirror-model";
import { EditorState, Transaction as PmTransaction } from "prosemirror-state";
import { EditorView } from "prosemirror-view";
import { computed, type Ref } from "vue";

//
// Mapping
//

/** Line interface with text or block */
export type LineInterface =
  | { type: "text"; text: TextLineData; blockPtr: NodeReferenceData | null }
  | { type: "block"; blockPtr: NodeReferenceData; nodePtr: NodeReferenceData | undefined };

/** Read/write source of Text */
export type TextInterface = {
  /** Reads the Lines (reactive). */
  read: () => LineInterface[];
  /** Writes the Lines, returning the updated Lines (if changed). */
  write: (lines: LineInterface[]) => LineInterface[];
  /** Updates the given view. */
  updateView: (view: EditorView, prevLines: LineInterface[], newLines: LineInterface[]) => void;
};

/** Read/write directly from TextData. */
export function useTextModelValueInterface(options: {
  modelValue: Readonly<Ref<TextData | undefined | null>>;
  update: (text: TextData) => void;
}): TextInterface {
  const { modelValue, update } = options;

  const lines = computed(() => {
    const lines: LineInterface[] = [];
    for (const line of modelValue.value?.lines ?? []) {
      lines.push({ type: "text", text: line, blockPtr: null });
    }
    return lines;
  });

  function read(): LineInterface[] {
    return lines.value;
  }

  function write(lines: LineInterface[]): LineInterface[] {
    // only update text lines
    const textLines = lines.filter((l) => l.type === "text") as {
      type: "text";
      text: TextLineData;
      blockPtr: NodeReferenceData | null;
    }[];
    update({ metatype: ObjectType.TEXT, lines: textLines.map(({ text }) => text) });
    return lines;
  }

  function updateView(view: EditorView, prevLines: LineInterface[], newLines: LineInterface[]) {
    const doc = mapTextToPmNode(newLines);
    const updatedState = EditorState.create({ doc: doc, schema: PM_SCHEMA, plugins: view.state.plugins });
    view.updateState(updatedState);
  }

  return { read, write, updateView };
}

/** Read/write Text from Blocks in a Page. */
export function useTextPageInterface(options: {
  page: Ref<PageData | null | undefined>;
  blocks: Ref<BlockData[]>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
}): TextInterface {
  const { page, blocks, graph, txFactory } = options;

  // map blocks to lines
  const lines = computed(() => {
    const lines: LineInterface[] = [];
    for (const block of blocks.value) {
      const blockPtr = toNodeRef(block);
      if (block.type >= BlockType.PARAGRAPH) {
        const text = block.text ?? emptyTextLine();
        lines.push({ type: "text", text, blockPtr });
      } else {
        lines.push({ type: "block", blockPtr, nodePtr: block.nodePtr });
      }
    }
    return lines;
  });

  function read(): LineInterface[] {
    return lines.value;
  }

  function write(lines: LineInterface[]) {
    if (page.value == null) throw new Error("no page");
    return differenceUpdateBlocks(txFactory(), graph, page.value, blocks.value, lines);
  }

  function updateView(view: EditorView, prevLines: LineInterface[], newLines: LineInterface[]) {
    differenceUpdateLines(view, prevLines, newLines, view.dispatch);
  }

  return { read, write, updateView };
}

/** Difference update the Lines into our graph / Blocks. */
export function differenceUpdateBlocks(
  tx: Transaction,
  graph: ReadNodeGraph,
  page: PageData,
  blocks: BlockData[],
  lines: LineInterface[],
) {
  if (tx.change?.key == null) {
    tx = tx.with({ change: { title: "Edit", key: newChangeId() } });
  }

  // index
  const blocksById: Record<string, BlockData> = groupByScalar(blocks, (b) => b.id);
  const lineByBlockId: Record<string, LineInterface> = {};
  const newLines: LineInterface[] = lines.map((l) => ({ ...l }));
  for (const line of lines) {
    if (line.type === "text" || line.type === "block") {
      if (line.blockPtr?.id != null) {
        lineByBlockId[line.blockPtr.id] = line;
      }
    }
  }

  // delete removed blocks
  for (const block of blocks) {
    if (lineByBlockId[block.id] == null) {
      tx.delete(block);
      console.log("block.delete");
    }
  }

  // add new blocks
  let prevBlockId: string | undefined = undefined;
  let firstBlockId: string | undefined = blocks[0]?.id;
  for (const line of newLines) {
    // skip if block already exists
    if (line.blockPtr?.id != null) {
      prevBlockId = line.blockPtr.id;
      continue;
    }
    // position
    let target: PageData | BlockData;
    let anchor: "before" | "after" | "inside";
    if (prevBlockId != null) {
      target = blocksById[prevBlockId];
      anchor = "after";
    } else if (firstBlockId != null) {
      target = blocksById[firstBlockId];
      anchor = "before";
    } else {
      target = page;
      anchor = "inside";
    }
    if (target == null) {
      throw new Error(`cannot create block: no target`);
    }
    // create
    const block = createBlock(tx, graph, {
      block: {
        type: (line.type === "text" ? line.text.type + 10_000 : undefined) as any,
        text: line.type === "text" ? line.text : undefined,
      },
      anchor,
      target,
    });
    line.blockPtr = toNodeRef(block);
    prevBlockId = block.id;
    if (firstBlockId == null) {
      firstBlockId = block.id;
    }
    console.log("block.create", block.id);
  }

  // update blocks
  for (const block of blocks) {
    const line = lineByBlockId[block.id];
    if (line == null) continue;
    if (line.type == "text") {
      if (!deepValueEquals(block.text, line.text)) {
        const update: Partial<BlockData> = { text: line.text };
        const blockType = (line.text.type + 10_000) as any;
        if (blockType != block.type) {
          update.type = blockType;
        }
        tx.update(block, update, { debounce: "long" });
        console.log("block.update", block.id);
      }
    } else if (line.type == "block") {
      // blocks are only updated from the graph transaction (?)
    } else {
      assertNever(line);
    }
  }

  return newLines;
}

/** Compute the longest common subsequence of two arrays. */
function computeLCS(oldKeys: string[], newKeys: string[]): Array<{ oldIndex: number; newIndex: number }> {
  const m = oldKeys.length;
  const n = newKeys.length;
  const dp: number[][] = Array.from({ length: m + 1 }, () => new Array(n + 1).fill(0));
  for (let i = m - 1; i >= 0; i--) {
    for (let j = n - 1; j >= 0; j--) {
      if (oldKeys[i] === newKeys[j]) dp[i][j] = 1 + dp[i + 1][j + 1];
      else dp[i][j] = Math.max(dp[i + 1][j], dp[i][j + 1]);
    }
  }
  const result: Array<{ oldIndex: number; newIndex: number }> = [];
  let i = 0,
    j = 0;
  while (i < m && j < n) {
    if (oldKeys[i] === newKeys[j]) {
      result.push({ oldIndex: i, newIndex: j });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      i++;
    } else {
      j++;
    }
  }
  return result;
}

type DiffItem = {
  key: string;
  node: PmNode;
  line: LineInterface;
};
type PositionedDiffItem = DiffItem & { pos: number };
type DiffOp =
  | { type: "delete"; pos: number; end: number; key: string }
  | { type: "insert"; pos: number; node: PmNode; key: string }
  | { type: "update"; pos: number; end: number; node: PmNode; key: string };

/** Get the unique key of a Line (for Blocks only) */
function getLineKey(line: LineInterface): string {
  if (line.blockPtr == null) {
    throw new Error("line has no blockPtr");
  }
  return line.blockPtr.id!;
}

/**
 * Compare the content of two Lines.
 */
function lineContentEquals(a: LineInterface, b: LineInterface): boolean {
  if (a.type === "text" && b.type === "text") {
    return deepValueEquals(a.text, b.text);
  } else if (a.type === "block" && b.type === "block") {
    return a.blockPtr?.id === b.blockPtr?.id && a.nodePtr?.id === b.nodePtr?.id;
  }
  return false;
}

/**
 * Difference update Lines.
 */
export function differenceUpdateLines(
  view: EditorView,
  prevLines: LineInterface[],
  newLines: LineInterface[],
  dispatch?: (tr: PmTransaction) => void,
) {
  const oldItems: PositionedDiffItem[] = [];
  view.state.doc.descendants((node, pos) => {
    if (node.type.name === "orderedList" || node.type.name === "unorderedList") {
      // skip straight into the children
      return true;
    }
    if (node.type.isInGroup("line")) {
      const line = mapPmNodeToLine(node);
      const key = getLineKey(line);
      const item: PositionedDiffItem = { key, node, pos, line };
      oldItems.push(item);
      return false;
    }
    return true;
  });
  const newItems: DiffItem[] = newLines.map((line) => {
    const node = mapLineToPmNode(line);
    const item: DiffItem = { key: getLineKey(line), node, line };
    return item;
  });

  const oldKeys = oldItems.map((item) => item.key);
  const newKeys = newItems.map((item) => item.key);
  const lcsKeys = computeLCS(oldKeys, newKeys);

  // diff
  const ops: DiffOp[] = [];
  let oldIndex = 0;
  let newIndex = 0;
  for (const match of lcsKeys) {
    // delete any old items before the match
    while (oldIndex < match.oldIndex) {
      const item = oldItems[oldIndex];
      ops.push({ type: "delete", pos: item.pos, end: item.pos + item.node.nodeSize, key: item.key });
      oldIndex++;
    }
    // insert any new items before the match
    //  (ops are applied in reverse order, so invert the order here)
    const insertPos = match.oldIndex < oldItems.length ? oldItems[match.oldIndex].pos : view.state.doc.content.size;
    for (let i = match.newIndex - 1; i >= newIndex; i--) {
      const item = newItems[i];
      ops.push({ type: "insert", pos: insertPos, node: item.node, key: item.key });
    }
    newIndex = match.newIndex;
    // update if contents differ
    const oldItem = oldItems[match.oldIndex];
    const newItem = newItems[match.newIndex];
    if (!lineContentEquals(oldItem.line, newItem.line)) {
      ops.push({
        type: "update",
        pos: oldItem.pos,
        end: oldItem.pos + oldItem.node.nodeSize,
        node: newItem.node,
        key: newItem.key,
      });
    }
    oldIndex++;
    newIndex++;
  }
  // delete remaining old items
  while (oldIndex < oldItems.length) {
    const item = oldItems[oldIndex];
    ops.push({ type: "delete", pos: item.pos, end: item.pos + item.node.nodeSize, key: item.key });
    oldIndex++;
  }
  // insert remaining new items (again, in reverse order)
  const insertPos = view.state.doc.content.size;
  for (let i = newItems.length - 1; i >= newIndex; i--) {
    const item = newItems[i];
    ops.push({ type: "insert", pos: insertPos, node: item.node, key: item.key });
  }

  // apply diff ops
  ops.sort((a, b) => b.pos - a.pos);
  let tr = view.state.tr;
  for (const op of ops) {
    if (op.type === "delete") {
      tr = tr.delete(op.pos, op.end);
    } else if (op.type === "insert") {
      tr = tr.insert(op.pos, op.node);
    } else if (op.type === "update") {
      tr = tr.replaceWith(op.pos, op.end, op.node);
    }
  }

  if (tr.docChanged) {
    // remove any empty orderedList/unorderedList nodes.
    tr.doc.descendants((node, pos) => {
      if ((node.type.name === "orderedList" || node.type.name === "unorderedList") && node.childCount === 0) {
        tr = tr.delete(pos, pos + node.nodeSize);
        return false;
      }
      return true;
    });

    tr.setMeta("_ignoreDocChanged", true); // prevent recursive updates
    console.log("differenceUpdateLines", { ops });
    dispatch?.(tr);
  }
}

/** Convert a span to a PmNode */
function mapSpanToPmNode(span: TextSpanData): PmNode {
  let node;
  if (span.type === TextSpanType.TEXT || span.type === TextSpanType.UNSPECIFIED) {
    node = PM_SCHEMA.text(span.content ?? "");
  } else {
    if (span.type === TextSpanType.HARD_BREAK) {
      node = PM_SCHEMA.node("spanHardBreak", {});
    } else if (span.type === TextSpanType.NODE) {
      node = PM_SCHEMA.node("spanNode", { type: span.type, nodePtr: span.nodePtr });
    } else if (span.type === TextSpanType.LINK) {
      node = PM_SCHEMA.node("spanLink", { type: span.type, content: span.content, href: span.url });
    } else if (span.type === TextSpanType.EQUATION) {
      node = PM_SCHEMA.node("spanEquation", { type: span.type, content: span.content });
    } else {
      throw new Error(`unexpected span type: ${span.type}`);
    }
  }

  // marks
  const marks: PmMark[] = [];
  if (span.isBold) marks.push(PM_SCHEMA.mark("bold"));
  if (span.isItalic) marks.push(PM_SCHEMA.mark("italic"));
  if (span.isStrikethrough) marks.push(PM_SCHEMA.mark("strikethrough"));
  if (span.isUnderline) marks.push(PM_SCHEMA.mark("underline"));
  if (span.isCode) marks.push(PM_SCHEMA.mark("code"));
  return marks.length ? node.mark(marks) : node;
}

/** Convert a PmNode to a span */
function mapPmNodeToSpan(spanNode: PmNode): TextSpanData | null {
  let span: TextSpanData;
  if (spanNode.type.name === "spanHardBreak") {
    span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.HARD_BREAK };
  } else if (spanNode.type.name === "text") {
    span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.TEXT, content: spanNode.text };
  } else if (spanNode.type.name === "spanNode") {
    span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.NODE, nodePtr: spanNode.attrs.nodePtr };
  } else if (spanNode.type.name === "spanLink") {
    span = {
      metatype: ObjectType.TEXT_SPAN,
      type: TextSpanType.LINK,
      url: spanNode.attrs.href,
      content: spanNode.text,
    };
  } else if (spanNode.type.name === "spanEquation") {
    span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.EQUATION, content: spanNode.text };
  } else if (spanNode.type.name === "spanSpecialInput") {
    return null; // ignore temporary input spans
  } else {
    throw new Error(`unexpected span node type: ${spanNode.type.name}`);
  }

  for (const mark of spanNode.marks) {
    if (mark.type.name === "bold") {
      span.isBold = true;
    } else if (mark.type.name === "italic") {
      span.isItalic = true;
    } else if (mark.type.name === "strikethrough") {
      span.isStrikethrough = true;
    } else if (mark.type.name === "underline") {
      span.isUnderline = true;
    } else if (mark.type.name === "code") {
      span.isCode = true;
    } else {
      throw new Error(`unexpected mark type: ${mark.type.name}`);
    }
  }
  return span;
}

/** Convert a line to a PmNode */
function mapLineToPmNode(line: LineInterface): PmNode {
  if (line.type === "block") {
    return PM_SCHEMA.node("block", { blockPtr: line.blockPtr, nodePtr: line.nodePtr });
  }

  const spanNodes = line.text.spans.map(mapSpanToPmNode);
  const attrs = { type: line.text.type, blockPtr: line.blockPtr };

  if (line.text.type === TextLineType.LIST_ORDERED || line.text.type === TextLineType.LIST_UNORDERED) {
    const isOrdered = line.text.type === TextLineType.LIST_ORDERED;
    const nodeType = isOrdered ? "lineListOrdered" : "lineListUnordered";
    return PM_SCHEMA.node(nodeType, attrs, spanNodes);
  } else if (line.text.type >= TextLineType.HEADING_1 && line.text.type <= TextLineType.HEADING_4) {
    return PM_SCHEMA.node("lineHeading", attrs, spanNodes);
  } else if (line.text.type === TextLineType.PARAGRAPH) {
    return PM_SCHEMA.node("lineParagraph", attrs, spanNodes);
  } else if (line.text.type === TextLineType.DIVIDER) {
    return PM_SCHEMA.node("lineDivider", attrs, spanNodes);
  } else if (line.text.type === TextLineType.QUOTE) {
    return PM_SCHEMA.node("lineQuote", attrs, spanNodes);
  } else if (line.text.type === TextLineType.CALLOUT) {
    return PM_SCHEMA.node("lineCallout", attrs, spanNodes);
  } else if (line.text.type === TextLineType.CODE) {
    return PM_SCHEMA.node("lineCode", attrs, spanNodes);
  } else {
    throw new Error(`unexpected line type: ${line.text.type}`);
  }
}

/** Convert a PmNode to a LineInterface */
function mapPmNodeToLine(lineNode: PmNode): LineInterface {
  if (lineNode.type.name === "block") {
    return { type: "block", blockPtr: lineNode.attrs.blockPtr, nodePtr: lineNode.attrs.nodePtr };
  }

  const spans: TextSpanData[] = [];
  for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
    const spanNode = lineNode.child(spanIdx);
    const span = mapPmNodeToSpan(spanNode);
    if (span) spans.push(span);
  }

  const text: TextLineData = {
    metatype: ObjectType.TEXT_LINE,
    type: lineNode.attrs.type,
    spans,
    cells: [],
  };
  return { type: "text", text, blockPtr: lineNode.attrs.blockPtr };
}

/** Convert Lines to a PmNode. */
export function mapTextToPmNode(lines: LineInterface[]): PmNode {
  const nodes: PmNode[] = [];
  let listGroup: PmNode[] = [];
  let currentListType: "ordered" | "unordered" | null = null;

  const flushListGroup = () => {
    if (listGroup.length > 0) {
      nodes.push(PM_SCHEMA.node(currentListType === "ordered" ? "orderedList" : "unorderedList", {}, listGroup));
      listGroup = [];
      currentListType = null;
    }
  };

  for (const line of lines) {
    if (line.type === "text") {
      if (line.text.type === TextLineType.LIST_ORDERED || line.text.type === TextLineType.LIST_UNORDERED) {
        const isOrdered = line.text.type === TextLineType.LIST_ORDERED;
        const listType = isOrdered ? "ordered" : "unordered";

        if (currentListType !== listType) {
          flushListGroup();
          currentListType = listType;
        }
        listGroup.push(mapLineToPmNode(line));
      } else {
        flushListGroup();
        nodes.push(mapLineToPmNode(line));
      }
    } else if (line.type === "block") {
      flushListGroup();
      nodes.push(mapLineToPmNode(line));
    } else {
      assertNever(line);
    }
  }

  flushListGroup();

  // ensure at least one line exists
  if (nodes.length === 0) {
    nodes.push(PM_SCHEMA.node("lineParagraph"));
  }

  return PM_SCHEMA.node("doc", {}, nodes);
}

/** Convert a PmNode to Lines. */
export function mapPmNodeToText(node: PmNode): { lines: LineInterface[]; linesNodes: PmNode[]; linesPos: number[] } {
  const lines: LineInterface[] = [];
  const linesNodes: PmNode[] = [];
  const linesPos: number[] = [];

  // use .descendants to traverse the doc with proper positions
  node.descendants((child, pos, parent) => {
    // direct children of the doc.
    if (parent === node) {
      // if the direct child is a list, skip adding it—its children will be handled below.
      if (child.type.name === "orderedList" || child.type.name === "unorderedList") {
        return;
      }
      lines.push(mapPmNodeToLine(child));
      linesNodes.push(child);
      linesPos.push(pos);
      return false; // don't descend further into this node
    }
    // immediate children of a list node
    if (parent && (parent.type.name === "orderedList" || parent.type.name === "unorderedList")) {
      lines.push(mapPmNodeToLine(child));
      linesNodes.push(child);
      linesPos.push(pos);
      return false; // don't descend further into this node
    }
  });

  return { lines, linesNodes, linesPos };
}
