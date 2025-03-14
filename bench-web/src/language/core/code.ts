import { ObjectType, type CodeData } from "@/proto/wire";
import { Text as PmText } from "@codemirror/state";

export function mapCodeToCmDoc(code: CodeData): string {
  return code.content ?? "";
}

export function mapPmDocToCode(doc: PmText): CodeData {
  const lines: string[] = [];
  for (const linePm of doc.iterLines()) {
    lines.push(linePm);
  }
  const code: CodeData = { metatype: ObjectType.CODE, content: lines.join("\n") };
  return code;
}

/** Rough estimate of the height of a Code */
export function estimateCodeHeight(code: CodeData, width?: number): number {
  if (width == null) width = 800;
  const lineHeight = 20;
  const charactersPerLine = 27;
  let height = 10;
  for (const line of code.content?.split("\n") ?? []) {
    if (line.length == 0) height += lineHeight;
    else height += Math.ceil(line.length / charactersPerLine) * lineHeight;
  }
  return height;
}
