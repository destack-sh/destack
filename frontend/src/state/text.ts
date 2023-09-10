import { useCurrentModule, type ModuleObjectTypename, type Statement, type File, type Field } from "@/state/module";
import { computed, type Ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";

export type TextMention = {
  type: "mention";
  id: string;
  text: string;
  referenceCk: string;
  referenceType: ModuleObjectTypename;
  referencePath?: string;
};

export type TextPlain = {
  type: "text";
  id: string;
  text: string;
};

export type TextSpan = TextMention | TextPlain;

// :TextFormat
export const TEXT_MENTION_REGEX =
  /<span data-ref-ck="(?<ck>[a-f0-9-]+)" data-ref-type="(?<type>[a-zA-Z]+)" data-ref-path="(?<path>[^"]*)"><\/span>/g;
export const TEXT_MENTION_TEMPLATE = '<span data-ref-ck="{ck}" data-ref-type="{type}" data-ref-path="{path}"></span>';

export function parseTextHtml(textRaw: string): TextSpan[] {
  const spans: TextSpan[] = [];
  let lastEnd = 0;

  let match: RegExpExecArray | null;
  while ((match = TEXT_MENTION_REGEX.exec(textRaw)) !== null) {
    if (match.index > lastEnd) {
      spans.push({ type: "text", id: spans.length.toString(), text: textRaw.slice(lastEnd, match.index) });
    }

    // mention
    const text = match[0];
    const ck = match.groups?.ck;
    if (ck == null) throw new Error(`missing ck in ${text}`);
    const type = match.groups?.type;
    const path = match.groups?.path;
    spans.push({
      type: "mention",
      text: textRaw.slice(match.index, match.index + text.length),
      referenceCk: ck,
      referenceType: type,
      referencePath: path,
    } as TextMention);

    lastEnd = match.index + text.length;
  }

  if (lastEnd < textRaw.length) {
    spans.push({ type: "text", id: spans.length.toString(), text: textRaw.slice(lastEnd) });
  }

  return spans;
}

export type MentionableNode = Statement | File | Field;

export function useTextMentions(spans: Ref<TextSpan[]>, query?: Ref<string>, statement?: Ref<{ ck: string }>) {
  const module = useCurrentModule();

  const resolvedMentions: Ref<Record<number, MentionableNode>> = computed(() => {
    const mentions: Record<number, MentionableNode> = {};
    for (const span of spans.value) {
      if (span.type != "mention") continue;
      const reference = module.nodeOf(span.referenceCk);
      if (reference == null) continue;
      mentions[parseInt(span.id)] = reference;
    }
    return mentions;
  });

  const uf = new uFuzzy({ intraMode: 0 });
  const filteredMentions: Ref<MentionableNode[]> = computed(() => {
    if (query == null) return [];
    const availableStatements = Object.values(module.idx.value?.statementsById ?? {})
      .filter((s) => (s.name ?? "").length > 0)
      .map((s) => s as Statement);
    const availableFiles = Object.values(module.idx.value?.filesById ?? {})
      .filter((f) => (f.name ?? "").length > 0)
      .map((f) => f as File);

    // nocheckin: and all fields of ancestor nodes
    const ancestorNodes = module.nodePathOf(statement?.value?.ck ?? "");
    const availableFields: Field[] = [];

    const availableNodes: MentionableNode[] = [...availableStatements, ...availableFiles, ...availableFields];
    if (query.value.length == 0) return availableNodes;

    const [idxs] = uf.search(
      availableNodes.map((a) => a.name as string),
      query.value
    );
    return idxs?.map((idx) => availableNodes[idx]) ?? [];
  });

  return {
    resolvedMentions,
    filteredMentions,
  };
}
