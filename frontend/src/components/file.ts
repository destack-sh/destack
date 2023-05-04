import { StatementType, SymbolType, type StatementContentFragment } from "@/gql/graphql";
import type { FileHeader, StatementHeader } from "@/state/editor";
import { INTEGER_ZERO } from "@/utils/fractional";
import { onBeforeUnmount, watchEffect, type Ref, ref, computed, inject, provide } from "vue";

export const FILE_CONTEXT = "__fileContext__" as const;

export type FileState = {
  editorId: string;
  focused: boolean;
  file: FileHeader;
  statementsUnordered: StatementHeader[]; // unordered
  navigateUp: () => void;
  navigateDown: () => void;
};

export type FileContext = FileState & {
  statements: StatementHeader[]; // ordered
  statementsById: Record<string, StatementHeader>;
  statementsByParentId: Record<string, StatementHeader[]>;
  statementPositions: Record<string, number>;
  positionedStatements: PositionedStatement[];
  depths: number[];
};

export type PositionedStatement = {
  depth: number;
  lineNumberBase: number;
  isFirstInGroup: boolean;
  isLastInGroup: boolean;
  statement: StatementContentFragment;
  ancestors: string[];
};

// There can only be one active file to provide file shortcuts,
// so we have a global reference here that is automatically set to the focused file.
// We can't just use singleton actions here because multiple files may have
// 'focused' set during moves or transition.
export const activeFileState = ref<FileState | null>(null);
export const fileContexts = ref<Record<string, FileContext>>({});

export function provideFileState(file: Ref<FileState | null>) {
  // set active file if focused
  watchEffect(() => {
    if (file.value?.focused) {
      activeFileState.value = file.value;
    }
  });
  onBeforeUnmount(() => {
    if (activeFileState.value?.editorId === file.value?.editorId) {
      activeFileState.value = null;
    }
  });

  // file context
  const statementsUnordered = computed(() => file.value?.statementsUnordered ?? []);
  const rootStatements = computed(() => statementsUnordered.value.filter((statement) => statement.parent == null));

  const statementsById = computed(() => {
    const statementsById = {};
    statementsUnordered.value.forEach((statement) => {
      statementsById[statement.id] = statement;
    });
    return statementsById;
  });

  // statementsByParentId must be ordered like orderedStatements
  const statementsByParentId: Ref<Record<string, StatementHeader[]>> = computed(() => {
    const result: Record<string, StatementHeader[]> = {};
    for (const statement of statements.value) {
      const parentId = statement.parent?.id ?? "";
      if (!result[parentId]) result[parentId] = [];
      result[parentId].push(statement);
    }
    return result;
  });

  const statementPositions = computed(() => {
    const result: Record<string, number> = {};
    for (let i = 0; i < statements.value.length; i++) {
      result[statements.value[i].id] = i;
    }
    return result;
  });

  /* Statements are hierarchical but laid out linearly (in one column) */

  const positionedStatements = computed(() => {
    const positionedStatements: PositionedStatement[] = [];
    let lineNumberBase = 0;

    // depth first traversal
    function walkDfs(statement: StatementContentFragment, ancestors: string[], isLast: boolean) {
      if (statement?.id == null) {
        // bail in case a bad statement ends in here due to some other bug to prevent recursion death
        console.warn("got bad statement with null id", statement, ancestors, isLast);
        return;
      }

      const children = statementsUnordered.value.filter((child) => child.parent?.id == statement.id);
      const isFirstInGroup = ancestors.length == 0;
      const isLastInRoot = isLast && children.length == 0;

      positionedStatements.push({
        depth: ancestors.length,
        lineNumberBase,
        statement,
        ancestors,
        isFirstInGroup,
        isLastInGroup: isLastInRoot,
      });
      lineNumberBase += 1;

      // walk children, sorted by order key
      ancestors = [...ancestors, statement.id];
      children.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
      children.forEach((child, i) => walkDfs(child, ancestors, isLast && i == children.length - 1));
    }

    // start with roots sorted by order key
    const roots = rootStatements.value;
    roots.sort((a, b) => ((a.orderKey ?? INTEGER_ZERO) < (b.orderKey ?? INTEGER_ZERO) ? -1 : 1));
    roots.forEach((root) => walkDfs(root, [], true));

    // group groupable sibling statements at root
    for (const [i, positioned] of positionedStatements.entries()) {
      if (
        positioned.statement.type == StatementType.Import ||
        positioned.statement.type == StatementType.Comment ||
        positioned.statement.type == StatementType.Blank ||
        positioned.statement.symbolType == SymbolType.Requirement
      ) {
        const next = positionedStatements[i + 1];
        if (next && next.depth == 0 && next?.statement.type == positioned.statement.type) {
          positioned.isLastInGroup = false;
          next.isFirstInGroup = false;
        }
      }
    }

    return positionedStatements;
  });
  const statements = computed(() => positionedStatements.value.map((positioned) => positioned.statement));
  const depths = computed(() => positionedStatements.value.map((positioned) => positioned.depth));

  // provide context
  const context = computed(
    () =>
      ({
        ...file.value,
        statements: statements.value,
        statementsById: statementsById.value,
        statementsByParentId: statementsByParentId.value,
        statementPositions: statementPositions.value,
        positionedStatements: positionedStatements.value,
        depths: depths.value,
      } as FileContext)
  );
  provide(FILE_CONTEXT, context);
  watchEffect(() => {
    fileContexts.value[file.value?.file?.id ?? ""] = context.value;
  });
  onBeforeUnmount(() => {
    delete fileContexts.value[file.value?.file?.id ?? ""];
  });
  return context;
}

export function useFileContext(): FileContext {
  const context = inject(FILE_CONTEXT) as FileContext | undefined;
  if (context == null) {
    throw new Error("File context not provided");
  }
  return context;
}

export function useFileContextOptional(): FileContext | undefined {
  return inject(FILE_CONTEXT) as FileContext | undefined;
}
