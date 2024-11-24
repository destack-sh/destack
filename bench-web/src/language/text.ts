import { defaultSortStruct } from "@/language/order";
import { ObjectType, TextData, TextLineData, TextLineType, TextSpanData } from "@/proto/wire";
import { PM_SCHEMA, type TextMarkType } from "@/utils/prosemirror";
import { Node as PmNode } from "prosemirror-model";

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
    if (line.type == TextLineType.PLAIN) {
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

export function emptyText(): TextData {
  return { metatype: ObjectType.TEXT, lines: [] };
}

export function trimText(text: TextData, numLines: number): TextData {
  if (text.lines.length <= numLines) return text;
  const trimmed = { ...text, lines: text.lines.slice(0, numLines) };
  return trimmed;
}

export function isTextEmpty(text: TextData | null | undefined): boolean {
  return (
    text == null ||
    text.lines.length == 0 ||
    text.lines.every((line) => line.type == TextLineType.PLAIN && line.spans.length == 0)
  );
}

/** Rough estimate of the height of a Text */
export function estimateTextHeight(text: TextData, width?: number): number {
  if (width == null) width = 800;
  const charactersPerLine = 37;
  const lineHeight = 24;
  let height = 10;
  for (const line of text.lines) {
    const characters = line.spans.reduce((acc, span) => acc + (span.content?.length ?? 0), 0);
    if (characters == 0) height += lineHeight;
    else height += Math.ceil(characters / charactersPerLine) * lineHeight;
  }
  height *= 1.1; // padding for justify
  return height;
}
