import { ObjectType, type CodeData, type CodeLineData } from "@/proto/wire";
import { newStructId } from "@/proto/wiring";
import { defaultSortStruct } from "@/system/lang";
import { Text as PmText, type Line as PmLine } from "@codemirror/state";

export function mapCodeToCmDoc(code: CodeData): string {
  defaultSortStruct(code.lines);

  // map lines
  // nocheckin: preserve ids & line metadata in CM
  const linesPm: string[] = [];
  for (const line of code.lines) {
    linesPm.push(line.content);
  }
  return linesPm.join("\n");
}

export function mapPmDocToCode(doc: PmText, prev: CodeData | undefined): CodeData {
  const lines: CodeLineData[] = [];
  for (const linePm of doc.iterLines()) {
    const line: CodeLineData = {
      metatype: ObjectType.CODE_LINE,
      id: newStructId(),
      setProperties: [],
      content: linePm,
    };
    lines.push(line);
  }
  const code: CodeData = {
    metatype: ObjectType.CODE,
    id: prev?.id ?? newStructId(),
    setProperties: [],
    lines,
  };
  return code;
}
