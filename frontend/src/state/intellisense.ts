import { graphql, useFragment, type FragmentType } from "@/gql";
import { StatementModifier, StatementType, SymbolType, type ProjectVersionAsDependencyFragment } from "@/gql/graphql";
import { useEditorState, type FileHeader, type StatementHeader } from "@/state/editor";
import {
  ProjectVersionAsDependencyType,
  FileHeaderType,
  StatementContentType,
  StatementHeaderType,
  TypeContentType,
} from "@/state/fragments";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { assert } from "ts-essentials";
import { computed, reactive, toRef, type ComputedRef, type Ref } from "vue";

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
      ...ProjectVersionAsDependency
    }
  }
`);

export type LocalFileHeader = FileHeader & {
  parent?: { id: string };
};
export type LocalStatementHeader = StatementHeader & {
  file: { id: string; path: string; pathWithoutExtension: string };
  parent?: { id: string };
  content?: { id: string };
  reference?: { id: string };
};

export type IntelliSenseRegistry = {
  filesById: Readonly<Record<string, LocalFileHeader>>;
  statementsById: Readonly<Record<string, LocalStatementHeader>>;
  statementsByFileId: Readonly<Record<string, LocalStatementHeader[]>>;
  // statements and files can be nested but we get them flat, so build a tree
  statementsByParentId: Readonly<Record<string, LocalStatementHeader[]>>;
  dependenciesById: Readonly<Record<string, ProjectVersionAsDependencyFragment>>;
};

/* IntelliSense is fully declarative and computed from the project version content. */
export type IntelliSense = {
  registry: IntelliSenseRegistry;

  family(statementId: string): LocalStatementHeader[];
  rootStatements(fileId: string): LocalStatementHeader[];
  availableSymbols(fileId: string, statementId?: string): LocalStatementHeader[];
  allSymbols(): LocalStatementHeader[];
  activeChildrenLike(
    statementId: string,
    filter: { type?: StatementType; symbolType?: SymbolType; modifier?: StatementModifier }
  ): LocalStatementHeader[];
};

function _useIntelliSenseRegistry(projectVersionId: Ref<string | null>): IntelliSenseRegistry {
  const { result: contentQuery } = useQuery(
    graphql(/* GraphQL */ `
      query projectVersionContentSense($id: GlobalID!) {
        projectVersion(id: $id) {
          id
          ...ProjectVersionContentSense
        }
      }
    `),
    () => ({ id: projectVersionId.value }),
    // TODO @Robustness: project version query is fired even when id is null due to a vuejs/apollo bug (https://github.com/vuejs/apollo/pull/1428)
    //  Ideally, we want to delay fetching this query to prioritise other queries,
    //  but enable debounce halts the debugger on error, which is very annoying.
    () => ({ enabled: !!projectVersionId.value })
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
  const dependencies = computed(
    () => content.value?.dependencies.map((d) => useFragment(ProjectVersionAsDependencyType, d)) || []
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

  return reactive({
    filesById,
    statementsById,
    statementsByFileId,
    statementsByParentId,
    dependenciesById,
  });
}

function _useIntelliSense() {
  const editor = useEditorState();
  const registry = _useIntelliSenseRegistry(toRef(editor, "currentProjectVersionId"));

  function family(statementId: string): LocalStatementHeader[] {
    // the sum of ancestors and descendants
    const family: LocalStatementHeader[] = [];
    let rootParent = registry.statementsById[statementId];
    while (rootParent?.parent != null) {
      rootParent = registry.statementsById[rootParent.parent.id];
    }
    // now get all descendants dfs
    const queue = [rootParent];
    while (queue.length > 0) {
      const statement = queue.shift();
      if (statement == null) {
        continue;
      }
      family.push(statement);
      queue.push(...(registry.statementsByParentId[statement.id] || []));
    }
    return family;
  }

  function rootStatements(fileId: string): LocalStatementHeader[] {
    return registry.statementsByFileId[fileId]?.filter((statement) => statement.parent == null) || [];
  }

  function availableSymbols(fileId: string, statementId?: string): LocalStatementHeader[] {
    // available symbols are all imported or defined symbols in the file
    let symbols =
      registry.statementsByFileId[fileId]?.filter(
        (statement) =>
          statement.deletedAt == null &&
          !statement.commented &&
          statement.type !== StatementType.Comment &&
          statement.type !== StatementType.Reference
      ) || [];
    if (statementId != null) {
      // filter to root statements and children
      symbols = symbols.filter((statement) => statement.parent == null || statement.parent.id === statementId);
    }
    return symbols;
  }

  function allSymbols(): LocalStatementHeader[] {
    return Object.values(registry.statementsById)
      .filter((statement) => statement.deletedAt == null && !statement.commented)
      .filter((statement) => statement.parent == null)
      .filter((statement) => statement.type != StatementType.Comment && statement.type != StatementType.Import);
  }

  function activeChildrenLike(
    statementId: string,
    filter: { type?: StatementType; symbolType?: SymbolType; modifier?: StatementModifier }
  ): LocalStatementHeader[] {
    const children = registry.statementsByParentId[statementId] || [];
    let relevantChildren = children.filter((child) => child.deletedAt == null && !child.commented);
    if (filter.type != null) {
      relevantChildren = relevantChildren.filter((child) => child.type === filter.type);
    }
    if (filter.symbolType != null) {
      relevantChildren = relevantChildren.filter((child) => child.symbolType === filter.symbolType);
    }
    if (filter.modifier != null) {
      relevantChildren = relevantChildren.filter((child) => child.modifier === filter.modifier);
    }
    return relevantChildren;
  }

  const sense: IntelliSense = reactive({
    registry,
    family,
    rootStatements,
    availableSymbols,
    allSymbols,
    activeChildrenLike,
  });
  return sense;
}

// share intellicense as a singleton instance across components
export const useIntelliSense = createSharedComposable(_useIntelliSense);

export type StatementMetadata = {
  isRedefinition: boolean;
  isDefinition: boolean;
  isReference: boolean;
  isArgument: boolean;
  isParameter: boolean;
  isImport: boolean;
  isRequirement: boolean;
  isCompilation: boolean;
  isRunconfig: boolean;
  isComment: boolean;
  isCommented: boolean;
  isDeleted: boolean;
  isRunnable: boolean;
  isAlias: boolean;
  requirementPath?: string | null;
  importPath?: string | null;
  parameters?: LocalStatementHeader[];
  arguments?: LocalStatementHeader[];
};

export function useStatementMetadata(
  fileRef: Ref<FragmentType<typeof FileHeaderType>>,
  statementRef: Ref<FragmentType<typeof StatementContentType>>
): StatementMetadata {
  const sense = useIntelliSense();

  const file = computed(() => useFragment(FileHeaderType, fileRef.value));
  const statement = computed(() => useFragment(StatementContentType, statementRef.value));
  const reference = computed(() => useFragment(StatementHeaderType, statement.value?.reference));

  const isRedefinition = computed(() => statement.value?.type == StatementType.Redefinition);
  const isDefinition = computed(() => statement.value?.type == StatementType.Definition || isRedefinition.value);
  const isReference = computed(() => statement.value?.type == StatementType.Reference || isRedefinition.value);
  const isRequirement = computed(() => statement.value?.type == StatementType.Requirement);
  const isCompilation = computed(() => statement.value?.type == StatementType.Compilation);
  const isRunconfig = computed(() => statement.value?.type == StatementType.Runconfig);
  const isParameter = computed(() => isReference.value && statement.value.modifier == StatementModifier.With);
  const isArgument = computed(() => isDefinition.value && statement.value.modifier == StatementModifier.With);
  const isImport = computed(() => statement.value?.type == StatementType.Import);
  const isComment = computed(() => statement.value?.type == StatementType.Comment);
  const isCommented = computed(() => statement.value?.commented);
  const isRunnable = computed(
    () =>
      !isImport.value &&
      (statement.value?.symbolType == SymbolType.Code || statement.value?.symbolType == SymbolType.Task)
  );
  const isDeleted = computed(() => statement.value?.deletedAt != null);

  const isAlias = computed(
    () => isImport.value && reference.value != null && reference.value?.name != statement.value.name
  );
  const requirementPath = computed(() => {
    assert(isRequirement.value, "statement is dependency");
    const dependencyVersion = useFragment(ProjectVersionAsDependencyType, statement.value.requirement?.projectVersion);
    if (!dependencyVersion) {
      return null; // dependency not registered or not yet loaded
    } else {
      return dependencyVersion.project.path + "@" + dependencyVersion.name ?? dependencyVersion.id;
    }
  });
  const importPath = computed(() => {
    assert(isImport.value, "statement is import");
    if (reference.value?.file.projectVersion.id != file.value.projectVersion.id) {
      // absolute import to dependency
      const dependency = sense.registry.dependenciesById[reference.value?.file.projectVersion.id];
      if (!dependency) {
        return null; // dependency not registered or not yet loaded
      } else {
        return dependency.project.path + "." + reference.value?.file.pathWithoutExtension;
      }
    } else {
      // relative import
      return "." + reference.value?.file.pathWithoutExtension;
    }
  });

  const parameters = computed(() =>
    sense.activeChildrenLike(statement.value.id, {
      type: StatementType.Reference,
      modifier: StatementModifier.With,
    })
  );
  const arguments_ = computed(() =>
    sense
      .activeChildrenLike(statement.value.id, {
        modifier: StatementModifier.With,
      })
      .filter((statement) => statement.type == StatementType.Definition || statement.type == StatementType.Redefinition)
  );

  return reactive({
    isRedefinition,
    isDefinition,
    isReference,
    isRequirement,
    isCompilation,
    isRunconfig,
    isArgument,
    isParameter,
    isImport,
    isComment,
    isCommented,
    isDeleted,
    isRunnable,
    isAlias,
    requirementPath,
    importPath,
    parameters,
    arguments: arguments_,
  });
}

export function useSchemadSymbolSchema(file: Ref<FileHeader>, statement: Ref<StatementHeader>) {
  /* Get the current schema for a 'schemad' Symbol from context */
  const sense = useIntelliSense();
  const schemaHeader: Ref<LocalStatementHeader | null> = computed(() => {
    const childSchema = sense.activeChildrenLike(statement.value.id, { symbolType: SymbolType.Schema })[0];
    if (childSchema?.reference != null) {
      // use source definition (only works for single-level references)
      return sense.registry.statementsById[childSchema.reference.id];
    } else {
      return childSchema;
    }
  });

  // get schema content from gql
  const { result: schemaQuery } = useQuery(
    graphql(/* GraphQL */ `
      query TypeContentById($statementId: GlobalID!) {
        statement(id: $statementId) {
          id
          ...StatementContent
        }
      }
    `),
    () => ({ statementId: schemaHeader.value?.id }),
    () => ({ enabled: !!schemaHeader.value })
  );
  const schema = computed(() => useFragment(StatementContentType, schemaQuery.value?.statement));
  const TypeContent = computed(() => useFragment(TypeContentType, schema.value?.content));

  return { schemaHeader, schema, TypeContent };
}
