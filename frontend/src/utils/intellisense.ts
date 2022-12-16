import { graphql, useFragment } from "@/gql";
import { SymbolType } from "@/gql/graphql";
import { useEditorState, type FileHeader, type StatementHeader, type SymbolHeader } from "@/utils/editor";
import { FileHeaderType, SchemaElementContentDeepType, StatementHeaderType } from "@/utils/fragments";
import { SchemaContentType } from "@/utils/schema";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, reactive, type ComputedRef, type Ref } from "vue";

const ProjectVersionContentSenseType = graphql(/* GraphQL */ `
  fragment ProjectVersionContentSense on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    files {
      ...FileHeader
    }
    statements {
      ...StatementHeader
      symbol {
        ...SymbolHeader
        statement {
          id
        }
      }
    }
  }
`);

type LocalFileHeader = FileHeader & {
  parent?: { id: string };
};
type LocalStatementHeader = StatementHeader & {
  file: { id: string; path: string };
  parent?: { id: string };
  symbol?: { id: string };
  reference?: { id: string };
};
type LocalSymbolHeader = SymbolHeader & {
  parent?: { id: string };
  statement: { id: string };
};

/* IntelliSense is fully declarative and computed from the project version content. */
export type IntelliSense = {
  filesById: Readonly<Record<string, LocalFileHeader>>;
  statementsById: Readonly<Record<string, LocalStatementHeader>>;
  symbolsById: Readonly<Record<string, LocalSymbolHeader>>;
  statementsByFileId: Readonly<Record<string, LocalStatementHeader[]>>;
  // statements and files can be nested but we get them flat, so build a tree
  statementsByParentId: Readonly<Record<string, LocalStatementHeader[]>>;

  rootStatements(fileId: string): StatementHeader[];
  childSymbols(symbolId: string, ofType?: SymbolType): SymbolHeader[];
  childSymbol(symbolId: string, ofType?: SymbolType): SymbolHeader | undefined;
};

function _useIntelliSense() {
  const editorState = useEditorState();
  const { result: contentQuery } = useQuery(
    graphql(/* GraphQL */ `
      query projectVersionContentSense($id: GlobalID!) {
        projectVersion(id: $id) {
          id
          ...ProjectVersionContentSense
        }
      }
    `),
    // TODO @Robustness: somehow this query is fired on start when id is null
    () => ({ id: editorState.currentProjectVersionId }),
    () => ({ enabled: editorState.currentProjectVersionId != null })
  );
  const content = computed(() => useFragment(ProjectVersionContentSenseType, contentQuery.value?.projectVersion));
  const files = computed(() => content.value?.files.map((f) => useFragment(FileHeaderType, f)) || []);
  const statements = computed(() => content.value?.statements.map((s) => useFragment(StatementHeaderType, s)) || []);

  const filesById: ComputedRef<Record<string, LocalFileHeader>> = computed(
    () =>
      Object.fromEntries(
        files.value.map((file) => [file.id, file]).sort((a, b) => a[1].path.localeCompare(b[1].path))
      ) as Record<string, LocalFileHeader>
  );
  const statementsById: ComputedRef<Record<string, LocalStatementHeader>> = computed(() =>
    Object.fromEntries(statements.value.map((statement) => [statement.id, statement]))
  );
  const symbolsById = computed(
    () =>
      Object.fromEntries(
        statements.value
          .filter((statement) => statement.symbol != null)
          .map((s) => [(s.symbol as SymbolHeader).id, s.symbol])
      ) as Record<string, LocalSymbolHeader>
  );
  const statementsByFileId = computed(
    () =>
      Object.fromEntries(
        files.value.map((file) => [
          file.id,
          statements.value
            .filter((statement) => statement.file.id === file.id)
            .sort((a, b) => (a.index as number) - (b.index as number)),
        ])
      ) as Record<string, LocalStatementHeader[]>
  );
  const statementsByParentId = computed(() => {
    const statementsByParentId: Record<string, LocalStatementHeader[]> = {};
    for (const statement of statements.value) {
      if (statement.parent == null) {
        continue;
      }
      if (statementsByParentId[statement.parent.id] == null) {
        statementsByParentId[statement.parent.id] = [];
      }
      statementsByParentId[statement.parent.id].push(statement);
    }
    // sort values by index
    for (const statements of Object.values(statementsByParentId)) {
      statements.sort((a, b) => (a.index as number) - (b.index as number));
    }
    return statementsByParentId;
  });

  function rootStatements(fileId: string): StatementHeader[] {
    return statementsByFileId.value[fileId].filter((statement) => statement.parent == null);
  }

  function childSymbols(symbolId: string, ofType?: SymbolType): SymbolHeader[] {
    const statementId = symbolsById.value[symbolId]?.statement?.id;
    const statements = statementsByParentId.value[statementId];
    if (statements == null) {
      return [];
    }
    return statements
      .filter((statement) => statement.symbol != null || statement.reference != null)
      .map((statement) => (statement.symbol ?? statement.reference).id)
      .map((symbolId) => symbolsById.value[symbolId])
      .filter((symbol) => symbol != null && (ofType == null || symbol.type === ofType));
  }

  function childSymbol(symbolId: string, ofType?: SymbolType): SymbolHeader | undefined {
    const candidates = childSymbols(symbolId, ofType);
    return candidates.length == 1 ? candidates[0] : undefined;
  }

  const sense: IntelliSense = reactive({
    filesById,
    statementsById,
    symbolsById,
    statementsByFileId,
    statementsByParentId,
    rootStatements,
    childSymbols,
    childSymbol,
  });
  return sense;
}

// share intellicense as a singleton instance across components
export const useIntelliSense = createSharedComposable(_useIntelliSense);

export function useSchemadSymbolSchema(symbol: Ref<SymbolHeader>) {
  /* Get the current schema for a 'schemad' Symbol from context */
  const sense = useIntelliSense();
  const schemaHeader = computed(() => {
    return sense.childSymbol(symbol.value.id, SymbolType.Schema);
  });
  // get schema content from gql
  const { result: schemaQuery } = useQuery(
    graphql(/* GraphQL */ `
      query schemaContentById($symbolId: GlobalID!) {
        symbol(id: $symbolId) {
          id
          content {
            ...SchemaContent
          }
          statement {
            id
          }
        }
      }
    `),
    () => ({ symbolId: schemaHeader.value?.id }),
    () => ({ enabled: !!schemaHeader.value })
  );
  const schema = computed(() => useFragment(SchemaContentType, schemaQuery.value?.symbol?.content));
  const schemaElement = computed(() => useFragment(SchemaElementContentDeepType, schema.value?.element));

  return { schemaHeader, schema, schemaElement };
}
