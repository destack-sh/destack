import {
  useCurrentModule,
  type ModuleObjectTypename,
  type Statement,
  type File,
  type Field,
  type NodeBase,
} from "@/state/module";
import { computed, type Ref } from "vue";
import uFuzzy from "@leeoniya/ufuzzy";
import { v4 as uuidv4 } from "uuid";
import { getStatementIconOutline, getStatementIconSolid } from "@/state/statement";
import { CodeBracketIcon as CodeBracketIconOutline } from "@heroicons/vue/24/outline";
import { CodeBracketIcon as CodeBracketIconSolid } from "@heroicons/vue/24/solid";
import { StatementType, TypeTag } from "@/gql/graphql";
import { EditFilePanel, useBenchState, type NavElement, type StatementHeader } from "@/state/bench";

export type TextMention = {
  type: "mention";
  id: string;
  text?: string;
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

export function makeTextMention(node: MentionableNode): TextMention {
  return {
    type: "mention",
    id: uuidv4(),
    referenceCk: node.ck,
    referenceType: node.__typename as ModuleObjectTypename,
  };
}

export function makeTextPlain(text: string): TextPlain {
  return {
    type: "text",
    id: uuidv4(),
    text,
  };
}

// :TextFormat
export const TEXT_MENTION_REGEX =
  /<span data-ref-ck="(?<ck>[a-f0-9-]+)" data-ref-type="(?<type>[a-zA-Z]+)" data-ref-path="(?<path>[^"]*)"><\/span>/g;

export function parseTextHtml(textRaw: string): TextSpan[] {
  const spans: TextSpan[] = [];
  let lastEnd = 0;

  let match: RegExpExecArray | null;
  while ((match = TEXT_MENTION_REGEX.exec(textRaw)) !== null) {
    if (match.index > lastEnd) {
      spans.push({ type: "text", id: uuidv4(), text: textRaw.slice(lastEnd, match.index) });
    }

    // mention
    const text = match[0];
    const ck = match.groups?.ck;
    if (ck == null) throw new Error(`missing ck in ${text}`);
    const type = match.groups?.type;
    const path = match.groups?.path;
    spans.push({
      type: "mention",
      id: uuidv4(),
      text: textRaw.slice(match.index, match.index + text.length),
      referenceCk: ck,
      referenceType: type,
      referencePath: path,
    } as TextMention);

    lastEnd = match.index + text.length;
  }

  // ensure there's always a trailing text span
  spans.push({ type: "text", id: uuidv4(), text: textRaw.slice(lastEnd) });
  // and a leading one
  if (spans.length > 0 && spans[0].type != "text") {
    spans.unshift({ type: "text", id: uuidv4(), text: "" });
  }

  return spans;
}

export function renderTextHtml(spans: TextSpan[]): string {
  return spans
    .map((span) =>
      span.type == "text"
        ? span.text
        : `<span data-ref-ck="${span.referenceCk}" data-ref-type="${span.referenceType}" data-ref-path="${
            span.referencePath ?? ""
          }"></span>`
    )
    .join("");
}

export type MentionableNode = StatementHeader | File | Field;

export type Mentionable = {
  node: MentionableNode;
  icon: any;
  name?: string;
  path?: string;
};

export function useTextMentions(
  spans: Ref<TextSpan[]>,
  options?: {
    query?: Ref<string | null>;
    searching?: Ref<boolean>;
    file?: Ref<{ ck: string } | undefined>; // not used yet, should affect ranking
    statement?: Ref<{ ck: string } | undefined>;
  }
) {
  const module = useCurrentModule();
  const bench = useBenchState();

  const resolvedMentions: Ref<Record<number, Mentionable>> = computed(() => {
    const mentions: Record<number, Mentionable> = {};
    for (const [i, span] of spans.value.entries()) {
      if (span.type != "mention") continue;
      const reference = module.nodeOf(span.referenceCk) as MentionableNode | undefined;
      if (reference == null) continue;

      // prefer actual name, but use text for statements if not set
      let name = reference.name;
      if (!name) {
        if (reference.__typename == "Statement") {
          if (reference.type == StatementType.Blank) {
            name = "<blank>";
          } else {
            name = reference.text?.replace(/<[^>]*>/g, "");
          }
        } else {
          name = reference.__typename;
        }
      }

      mentions[i] = {
        node: reference,
        icon: getIconSolid(reference),
        name: name ?? undefined,
        path: getPath(reference),
      };
    }
    return mentions;
  });

  const uf = new uFuzzy({ intraMode: 0 });
  const filteredMentions: Ref<Mentionable[]> = computed(() => {
    if (options?.query?.value == null || options?.searching?.value === false) return [];
    const availableStatements = Object.values(module.idx.value?.statementsById ?? {})
      .filter((s) => (s.name ?? "").length > 0)
      .map((s) => s as Statement);
    const availableFiles = Object.values(module.idx.value?.filesById ?? {})
      .filter((f) => (f.name ?? "").length > 0)
      .map((f) => f as File);

    const ancestorNodes = module.nodePathOf(options?.statement?.value?.ck ?? "") ?? [];
    const availableFields: Field[] = [];
    // ancestor fields (all of them)
    for (const node of ancestorNodes) {
      const statement = module.statementOf(node.ck);
      if (statement == null) continue;
      availableFields.push(...statement.fields.filter((f) => f.deletedAt == null && (f.name ?? "").length > 0));
    }
    // all other enum, struct, variable fields
    for (const statement of availableStatements) {
      if (
        statement.type != StatementType.Class &&
        statement.type != StatementType.Choice &&
        statement.type != StatementType.Variable &&
        statement.type != StatementType.Database
      )
        continue;
      availableFields.push(...statement.fields.filter((f) => f.deletedAt == null && (f.name ?? "").length > 0));
    }

    const availableNodes: MentionableNode[] = [...availableStatements, ...availableFiles, ...availableFields];
    let filteredNodes: MentionableNode[] = availableNodes;
    if (options?.query.value.length != 0) {
      const [idxs, info, order] = uf.search(
        availableNodes.map((a) => a.name as string),
        options.query.value,
        true
      );
      if (idxs && order) {
        filteredNodes = order.map((i) => availableNodes[idxs[i]]);
      }
    } else if (options?.statement?.value != null) {
      // rank nodes in same file higher
      const file = module.fileOf(options.statement.value.ck);
      if (file != null) {
        filteredNodes = filteredNodes.sort((a, b) => {
          const aIsInFile = module.fileOf(a.ck)?.id == file.id;
          const bIsInFile = module.fileOf(b.ck)?.id == file.id;
          if (aIsInFile && !bIsInFile) return -1;
          if (!aIsInFile && bIsInFile) return 1;
          return 0;
        });
      }
    }

    const filteredMentions: Mentionable[] = filteredNodes.map((n) => ({
      node: n,
      icon: getIconSolid(n),
      name: n.name ?? undefined,
      path: getPath(n) ?? undefined,
    }));
    return filteredMentions;
  });

  function getPath(node: MentionableNode): string | undefined {
    return module
      .nodePathOf(node.ck)
      ?.slice(0, -1)
      ?.map((n) => n.name)
      .join(".");
  }

  function getIconOutline(node: MentionableNode) {
    if (node.__typename == "Statement") {
      return getStatementIconOutline(node.type);
    } else if (node.__typename == "File") {
      return CodeBracketIconOutline;
    } else if (node.__typename == "Field") {
      if (node.tag == TypeTag.Literal) {
        return getStatementIconOutline(StatementType.Choice);
      } else {
        return getStatementIconOutline(StatementType.Class);
      }
    }
  }

  function getIconSolid(node: MentionableNode) {
    if (node.__typename == "Statement") {
      return getStatementIconSolid(node.type);
    } else if (node.__typename == "File") {
      return CodeBracketIconSolid;
    } else if (node.__typename == "Field") {
      if (node.tag == TypeTag.Literal) {
        return getStatementIconSolid(StatementType.Choice);
      } else {
        return getStatementIconSolid(StatementType.Class);
      }
    }
  }

  function focus(node: MentionableNode) {
    if (node.__typename == "File") {
      bench.focusFile(node as NodeBase);
    } else if (node.__typename == "Statement") {
      const file = module.fileOf(node.ck);
      if (file != null) {
        const panel = bench.focusFile(file as NodeBase) as EditFilePanel;
        panel.editElement(node);
      }
    } else if (node.__typename == "Field") {
      const statement = module.statementOf(node.parent.id);
      const file = module.fileOf(statement?.ck);
      if (file != null) {
        const panel = bench.focusFile(file as NodeBase) as EditFilePanel;
        panel.editElement(statement as NavElement);
      }
    }
  }

  return {
    resolvedMentions,
    filteredMentions,
    getIconOutline,
    getIconSolid,
    focus,
  };
}
