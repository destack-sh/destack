import { StructType, TextData, TextLineData, TextLineType } from "@/proto/wire";
import { makeStruct } from "@/proto/wiring";

export const HIGHLIGHTED_TEXT_LINE_TYPES: TextLineType[] = [
  TextLineType.HEADING_1,
  TextLineType.HEADING_2,
  TextLineType.HEADING_3,
  TextLineType.HEADING_4,
  TextLineType.CALLOUT,
  TextLineType.QUOTE,
  TextLineType.CODE,
];
export const STANDARD_TEXT_LINE_TYPES: TextLineType[] = [
  TextLineType.PARAGRAPH,
  TextLineType.LIST_UNORDERED,
  TextLineType.LIST_ORDERED,
];

export function emptyText(): TextData {
  return makeStruct({ metatype: StructType.TEXT, lines: [] });
}

export function emptyTextLine(type: TextLineType = TextLineType.PARAGRAPH): TextLineData {
  return makeStruct({ metatype: StructType.TEXT_LINE, type, spans: [] });
}

/** Returns true if the line is empty. */
export function isTextLineEmpty(line: TextLineData): boolean {
  return (
    line.type == TextLineType.PARAGRAPH &&
    (line.spans.length == 0 || line.spans.every((span) => span.content == null || span.content == ""))
  );
}

/** Returns true if the text is empty. */
export function isTextEmpty(text: TextData): boolean {
  return text.lines.length == 0 || text.lines.every(isTextLineEmpty);
}

/** Strip leading and trailing empty lines. */
export function trimTextLines(lines: TextLineData[]): TextLineData[] {
  if (lines.length == 0) return lines;
  while (lines.length > 0 && isTextLineEmpty(lines[0])) lines.shift();
  while (lines.length > 0 && isTextLineEmpty(lines[lines.length - 1])) lines.pop();
  return lines;
}

/** Strip leading and trailing empty lines. */
export function trimText(text: TextData): TextData {
  return { ...text, lines: trimTextLines(text.lines) };
}

/** Gets up to maxLines lines of text joined together. */
export function renderText(text: TextData, maxLines: number = 3): string | undefined {
  const lines: string[] = [];
  for (const line of text.lines) {
    if (line.spans.some((span) => span.content != null)) {
      lines.push(line.spans.map((span) => span.content).join(""));
      if (lines.length >= maxLines) break;
    }
  }
  const line = lines.join(" ");
  if (line.length == 0) return undefined;
  return line;
}

/** Renders a single line of text. */
export function renderTextLine(line: TextLineData): string | undefined {
  if (line.spans?.some((span) => span.content != null)) {
    return line.spans.map((span) => span.content).join("");
  }
  return undefined;
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
