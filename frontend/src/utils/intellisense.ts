import { graphql, useFragment } from "@/gql";
import type { DependencyHeaderFragment } from "@/gql/graphql";
import { useEditorState, type FileHeader, type StatementHeader } from "@/utils/editor";
import {
  DependencyHeaderType,
  FileHeaderType,
  SchemaElementContentDeepType,
  StatementHeaderType,
} from "@/utils/fragments";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, reactive, ref, type ComputedRef, type Ref } from "vue";

const ProjectVersionContentSenseType = graphql(/* GraphQL */ `
  fragment ProjectVersionContentSense on ProjectVersion {
    id
    name
    description
    createdAt
    committed
    committedAt
    files(filters: { isVisible: true }) {
      ...FileHeader
      # exact same query as fileContentById to get immediate updates
      statements(filters: { isVisible: true }) {
        ...StatementHeader
      }
    }
    dependencies {
      id
      ...DependencyHeader
    }
  }
`);

type LocalFileHeader = FileHeader & {
  parent?: { id: string };
};
type LocalStatementHeader = StatementHeader & {
  file: { id: string; path: string };
  parent?: { id: string };
  content?: { id: string };
  reference?: { id: string };
};

/* IntelliSense is fully declarative and computed from the project version content. */
export type IntelliSense = {
  filesById: Readonly<Record<string, LocalFileHeader>>;
  statementsById: Readonly<Record<string, LocalStatementHeader>>;
  statementsByFileId: Readonly<Record<string, LocalStatementHeader[]>>;
  // statements and files can be nested but we get them flat, so build a tree
  statementsByParentId: Readonly<Record<string, LocalStatementHeader[]>>;
  dependenciesById: Readonly<Record<string, DependencyHeaderFragment>>;

  rootStatements(fileId: string): StatementHeader[];
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
    // only needed within the editor; delay/debounce to ensure open files load first
    () => ({ enabled: editorState.currentProjectVersionId != null, debounce: 500 })
  );
  const content = computed(() => useFragment(ProjectVersionContentSenseType, contentQuery.value?.projectVersion));
  const files = computed(
    () => content.value?.files.map((f) => useFragment(FileHeaderType, f)).filter((f) => f.deletedAt == null) || []
  );
  const statements = computed(
    () =>
      content.value?.files
        .flatMap((f) => f.statements)
        .map((s) => useFragment(StatementHeaderType, s))
        .filter((s) => s.deletedAt == null) || []
  );
  const activeStatements = computed(() => statements.value.filter((s) => s.commented === false));
  const dependencies = computed(
    () => content.value?.dependencies.map((d) => useFragment(DependencyHeaderType, d)) || []
  );

  const filesById: ComputedRef<Record<string, LocalFileHeader>> = computed(
    () =>
      Object.fromEntries(
        files.value.map((file) => [file.id, file]).sort((a, b) => a[1].path.localeCompare(b[1].path))
      ) as Record<string, LocalFileHeader>
  );
  const statementsById: ComputedRef<Record<string, LocalStatementHeader>> = computed(() =>
    Object.fromEntries(statements.value.map((statement) => [statement.id, statement]))
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
  const dependenciesById = computed(() =>
    Object.fromEntries(dependencies.value.map((dependency) => [dependency.id, dependency]))
  );

  function rootStatements(fileId: string): StatementHeader[] {
    return statementsByFileId.value[fileId]?.filter((statement) => statement.parent == null) || [];
  }

  const sense: IntelliSense = reactive({
    filesById,
    statementsById,
    dependenciesById,
    statementsByFileId,
    statementsByParentId,
    rootStatements,
  });
  return sense;
}

// share intellicense as a singleton instance across components
export const useIntelliSense = createSharedComposable(_useIntelliSense);

export function useSchemadSymbolSchema(file: Ref<FileHeader>, statement: Ref<StatementHeader>) {
  /* Get the current schema for a 'schemad' Symbol from context */
  const sense = useIntelliSense();
  // const schemaHeader = computed(() => {
  //   return sense.childSymbol(statement.value.id, SymbolType.Schema);
  // });
  // get schema content from gql
  const { result: schemaQuery } = useQuery(
    graphql(/* GraphQL */ `
      query schemaContentById($fileId: GlobalID!, $statementId: GlobalID!) {
        file(id: $fileId) {
          statements(filters: { id: $statementId }) {
            id
          }
        }
      }
    `),
    () => ({ fileId: file.value.id, statementId: null /* nocheckin */ }),
    () => ({ enabled: false /* nocheckin */ })
  );
  const schema = computed(() => null);
  const schemaElement = computed(() => useFragment(SchemaElementContentDeepType, schema.value?.element));

  return { schemaHeader: ref(null), schema: ref(null), schemaElement: ref(null) };
}
