import { ObjectType, TextData, TextLineType } from "@/proto/wire";


export function emptyText(): TextData {
  return { metatype: ObjectType.TEXT, lines: [] };
}

export function trimText(text: TextData, numLines: number): TextData {
  if (text.lines.length <= numLines) return text;
  const trimmed = { ...text, lines: text.lines.slice(0, numLines) };
  return trimmed;
}

/** Gets up to maxLines lines of text joined together. */
export function getTextLine(text: TextData, maxLines: number = 3): string | undefined {
  const lines: string[] = [];
  for (const line of text.lines) {
    if (line.spans.some((span) => span.content != null)) {
      lines.push(line.spans.map((span) => span.content).join(""));
      if (lines.length >= maxLines) break;
    }
  }
  return lines.length > 0 ? lines.join(" ") : undefined;
}

export function isTextEmpty(text: TextData | null | undefined): boolean {
  return (
    text == null ||
    text.lines.length == 0 ||
    text.lines.every((line) => line.type == TextLineType.PARAGRAPH && line.spans.length == 0)
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

