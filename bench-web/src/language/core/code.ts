import { defaultSortStruct } from "@/language/core/order";
import { ObjectType, type CodeData, type CodeLineData } from "@/proto/wire";
import { Text as PmText } from "@codemirror/state";

export function mapCodeToCmDoc(code: CodeData): string {
  defaultSortStruct(code.lines);

  // map lines
  const linesPm: string[] = [];
  for (const line of code.lines) {
    linesPm.push(line.content ?? "");
  }
  return linesPm.join("\n");
}

export function mapPmDocToCode(doc: PmText, prev: CodeData | undefined): CodeData {
  const lines: CodeLineData[] = [];
  for (const linePm of doc.iterLines()) {
    const line: CodeLineData = { metatype: ObjectType.CODE_LINE };
    if (linePm.length > 0) {
      line.content = linePm;
    }
    lines.push(line);
  }
  const code: CodeData = { metatype: ObjectType.CODE, lines };
  return code;
}

/** Rough estimate of the height of a Code */
export function estimateCodeHeight(code: CodeData, width?: number): number {
  if (width == null) width = 800;
  const lineHeight = 20;
  const charactersPerLine = 27;
  let height = 10;
  for (const line of code.lines) {
    if (line.content == null || line.content.length == 0) height += lineHeight;
    else height += Math.ceil((line.content ?? "").length / charactersPerLine) * lineHeight;
  }
  return height;
}

