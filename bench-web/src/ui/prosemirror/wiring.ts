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
import { computed, type Ref } from "vue";

//
// Mapping
//

/** Line interface with text or block */
export type LineInterface =
  | { type: "text"; text: TextLineData; blockPtr: NodeReferenceData | null }
  | { type: "block"; blockPtr: NodeReferenceData };

/** Read/write source of Text */
export type TextInterface = {
  /** Reads the Lines (reactive). */
  read: () => LineInterface[];
  /** Writes the Lines, returning the updated Lines (if changed). */
  write: (lines: LineInterface[]) => LineInterface[];
};

/** Read/write directly from TextData. */
export function useTextModelValueInterface(options: {
  modelValue: Readonly<Ref<TextData | undefined | null>>;
  update: (text: TextData) => void;
}) {
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

  return { read, write };
}

/** Read/write Text from Blocks in a Page. */
export function useTextPageInterface(options: {
  page: Ref<PageData | null | undefined>;
  blocks: Ref<BlockData[]>;
  graph: ReadNodeGraph;
  txFactory: () => Transaction;
}) {
  const { page, blocks, graph, txFactory } = options;

  const blocksById: Ref<Record<string, BlockData>> = computed(() => groupByScalar(blocks.value, (b) => b.id));

  // map blocks to lines
  const lines = computed(() => {
    const lines: LineInterface[] = [];
    for (const block of blocks.value) {
      if (block.type >= BlockType.PARAGRAPH) {
        const text = block.text ?? emptyTextLine();
        const blockPtr = toNodeRef(block);
        lines.push({ type: "text", text, blockPtr });
      } else {
        const blockPtr = toNodeRef(block);
        lines.push({ type: "block", blockPtr });
      }
    }
    return lines;
  });

  function read(): LineInterface[] {
    return lines.value;
  }

  /** Difference update the Lines with Blocks. */
  function write(lines: LineInterface[]) {
    const lineByBlockId: Record<string, LineInterface> = {};
    const newLines: LineInterface[] = lines.map((l) => ({ ...l }));
    for (const line of lines) {
      if (line.type === "text" || line.type === "block") {
        if (line.blockPtr?.id != null) {
          lineByBlockId[line.blockPtr.id] = line;
        }
      }
    }

    const tx = txFactory().with({ change: { title: "Edit", key: newChangeId() } });

    // delete removed blocks
    for (const block of blocks.value) {
      if (lineByBlockId[block.id] == null) {
        tx.delete(block);
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
          block: {
            type: (line.type === "text" ? line.text.type + 10_000 : undefined) as any,
            text: line.type === "text" ? line.text : undefined,
          },
          anchor: prevBlock != null ? "after" : "inside",
          target: prevBlock ?? page.value,
        });
        line.blockPtr = toNodeRef(block);
        prevBlockId = block.id;
      }
    }

    // update blocks
    for (const block of blocks.value) {
      const line = lineByBlockId[block.id];
      if (line?.type == "text") {
        if (!deepValueEquals(block.text, line.text)) {
          const update: Partial<BlockData> = { text: line.text };
          const blockType = (line.text.type + 10_000) as any;
          if (blockType != block.type) {
            update.type = blockType;
          }
          tx.update(block, update, { debounce: "long" });
        }
      }
    }

    return newLines;
  }

  return { read, write };
}

/** Convert Lines to a PmNode. */
export function mapTextToPmNode(lines: LineInterface[]): PmNode {
  const schema = PM_SCHEMA;
  const nodes: PmNode[] = [];
  let listGroup: PmNode[] = [];
  let currentListType: "ordered" | "unordered" | null = null;

  const flushListGroup = () => {
    if (listGroup.length > 0) {
      nodes.push(schema.node(currentListType === "ordered" ? "orderedList" : "unorderedList", {}, listGroup));
      listGroup = [];
      currentListType = null;
    }
  };

  for (const line of lines) {
    if (line.type === "text") {
      const spanNodes = line.text.spans.map((span) => {
        let node;
        if (span.type === TextSpanType.TEXT || span.type === TextSpanType.UNSPECIFIED) {
          node = schema.text(span.content ?? "");
        } else {
          if (span.type === TextSpanType.HARD_BREAK) {
            node = schema.node("spanHardBreak", {});
          } else if (span.type === TextSpanType.NODE) {
            node = schema.node("spanNode", { nodePtr: span.nodePtr });
          } else if (span.type === TextSpanType.LINK) {
            node = schema.node("spanLink", { content: span.content, href: span.url });
          } else if (span.type === TextSpanType.CODE) {
            node = schema.node("spanCode", { content: span.content });
          } else if (span.type === TextSpanType.EQUATION) {
            node = schema.node("spanEquation", { content: span.content });
          } else {
            throw new Error(`unexpected span type: ${span.type}`);
          }
        }

        // marks
        const marks: PmMark[] = [];
        if (span.isBold) marks.push(schema.mark("bold"));
        if (span.isItalic) marks.push(schema.mark("italic"));
        if (span.isStrikethrough) marks.push(schema.mark("strikethrough"));
        if (span.isUnderline) marks.push(schema.mark("underline"));
        return marks.length ? node.mark(marks) : node;
      });

      const attrs = { type: line.text.type, blockPtr: line.blockPtr };
      if (line.text.type === TextLineType.LIST_ORDERED || line.text.type === TextLineType.LIST_UNORDERED) {
        const isOrdered = line.text.type === TextLineType.LIST_ORDERED;
        const listType = isOrdered ? "ordered" : "unordered";
        const nodeType = isOrdered ? "lineListOrdered" : "lineListUnordered";

        if (currentListType !== listType) {
          flushListGroup();
          currentListType = listType;
        }
        listGroup.push(schema.node(nodeType, attrs, spanNodes));
      } else if (line.text.type >= TextLineType.HEADING_1 && line.text.type <= TextLineType.HEADING_4) {
        flushListGroup();
        nodes.push(schema.node("lineHeading", attrs, spanNodes));
      } else if (line.text.type === TextLineType.PARAGRAPH) {
        flushListGroup();
        nodes.push(schema.node("lineParagraph", attrs, spanNodes));
      } else if (line.text.type === TextLineType.DIVIDER) {
        flushListGroup();
        nodes.push(schema.node("lineDivider", attrs, spanNodes));
      } else if (line.text.type === TextLineType.QUOTE) {
        flushListGroup();
        nodes.push(schema.node("lineQuote", attrs, spanNodes));
      } else if (line.text.type === TextLineType.CALLOUT) {
        flushListGroup();
        nodes.push(schema.node("lineCallout", attrs, spanNodes));
      } else if (line.text.type === TextLineType.CODE) {
        flushListGroup();
        nodes.push(schema.node("lineCode", attrs, spanNodes));
      } else {
        throw new Error(`unexpected line type: ${line.text.type}`);
      }
    } else if (line.type === "block") {
      flushListGroup();
      nodes.push(schema.node("block", { blockPtr: line.blockPtr }));
    } else {
      assertNever(line);
    }
  }

  flushListGroup();

  // ensure at least one line exists
  if (nodes.length === 0) {
    nodes.push(schema.node("lineParagraph"));
  }

  return schema.node("doc", {}, nodes);
}

/** Convert a PmNode to Lines. */
export function mapPmNodeToText(node: PmNode): { lines: LineInterface[]; linesNodes: PmNode[]; linesPos: number[] } {
  const lines: LineInterface[] = [];
  const linesNodes: PmNode[] = [];
  const linesPos: number[] = [];

  /** Convert a PmNode to a LineInterface. */
  function mapPmLineToTextLine(lineNode: PmNode): LineInterface {
    if (lineNode.type.name === "block") {
      return { type: "block", blockPtr: lineNode.attrs.blockPtr };
    }
    const spans: TextSpanData[] = [];
    for (let spanIdx = 0; spanIdx < lineNode.childCount; spanIdx++) {
      const spanNode = lineNode.child(spanIdx);
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
      } else if (spanNode.type.name === "spanCode") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.CODE, content: spanNode.text };
      } else if (spanNode.type.name === "spanEquation") {
        span = { metatype: ObjectType.TEXT_SPAN, type: TextSpanType.EQUATION, content: spanNode.text };
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
        } else {
          throw new Error(`unexpected mark type: ${mark.type.name}`);
        }
      }
      spans.push(span);
    }
    const text: TextLineData = {
      metatype: ObjectType.TEXT_LINE,
      type: lineNode.attrs.type,
      spans,
      cells: [],
    };
    return { type: "text", text, blockPtr: lineNode.attrs.blockPtr };
  }

  // use .descendants to traverse the doc with proper positions
  node.descendants((child, pos, parent) => {
    // direct children of the doc.
    if (parent === node) {
      // if the direct child is a list, skip adding it—its children will be handled below.
      if (child.type.name === "orderedList" || child.type.name === "unorderedList") {
        return;
      }
      lines.push(mapPmLineToTextLine(child));
      linesNodes.push(child);
      linesPos.push(pos);
      return false; // don't descend further into this node
    }
    // immediate children of a list node
    if (parent && (parent.type.name === "orderedList" || parent.type.name === "unorderedList")) {
      // only add the direct children of the list.
      if ((parent as any).parent === node) {
        lines.push(mapPmLineToTextLine(child));
        linesNodes.push(child);
        linesPos.push(pos);
        return false; // don't descend further into this node
      }
    }
    // otherwise, skip deeper descendants
    return;
  });

  return { lines, linesNodes, linesPos };
}
