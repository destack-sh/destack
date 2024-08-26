import { defaultSortStruct } from "@/language/order";
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
  const height = code.lines.length * 20 + 10;
  return height;
}