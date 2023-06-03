import { graphql, useFragment } from "@/gql";
import type { InterpFileFragment, InterpStatementFragment, SymbolType, StatementType } from "@/gql/graphql";
import { FileEditor, useBenchState } from "@/state/bench";
import { InterpFileType, InterpStatementType } from "@/state/fragments";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { toValueRef } from "@/utils/functools";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { v4 as uuidv4 } from "uuid";
import { computed, isRef, ref, type Ref } from "vue";

export type InterpFile = InterpFileFragment;
export type InterpStatement = InterpStatementFragment;

export enum TypeFlag { // :TypeFlags
  Zero = 0,
  IsOutput = 1 << 0,
  IsArray = 1 << 1,
  IsNullable = 1 << 2,
  IsUnionWith = 1 << 3,
  IsSecret = 1 << 4,
}

type ModuleIndex = {
  id: string;
  name: string;
  path: string;
  statementsById: Record<string, InterpStatement>;
  filesById: Record<string, InterpFile>;
};

// TODO @Performance: moduleRuntimeChanged should be partial updates :PartialModuleUpdates
function _useModule(projectVersionId: Ref<string | null>) {
  projectVersionId = toValueRef(projectVersionId);

  const { result: module } = useQuery(
    graphql(/* GraphQL */ `
      query module($projectVersionId: GlobalID!) {
        projectVersion(id: $projectVersionId) {
          id
          project {
            path
            name
          }
          files {
            edges {
              node {
                ...InterpFile
                statements {
                  ...InterpStatement
                  issues {
                    ...IssueContent
                  }
                }
              }
            }
          }
        }
      }
    `),
    () => ({ projectVersionId: projectVersionId.value }),
    () => ({ enabled: !!projectVersionId.value })
  );

  const issues = computed(() => {
    if (module.value?.projectVersion == null) return [];
    const issues = [];
    for (const file of module.value.projectVersion.files.edges.map((e: any) => e.node)) {
      for (const statement of file.statements) {
        for (const issue of statement.issues) {
          issues.push({
            ...issue,
            statement: statement,
          });
        }
      }
    }
    return issues;
  });

  const moduleIndex: Ref<ModuleIndex | null> = computed(() => {
    if (module.value?.projectVersion == null) return null;
    const statementsById: Record<string, InterpStatement> = {};
    const filesById: Record<string, InterpFile> = {};
    for (const fileEdge of module.value.projectVersion.files.edges) {
      const file = useFragment(InterpFileType, fileEdge.node);
      if (file.deletedAt != null) continue;
      filesById[fileEdge.node.id] = file;
      for (const statement of fileEdge.node.statements) {
        if (statement.deletedAt != null) continue;
        statementsById[statement.id] = useFragment(InterpStatementType, statement);
      }
    }
    return {
      id: module.value.projectVersion.id,
      name: module.value.projectVersion.project.name,
      path: module.value.projectVersion.project.path,
      statementsById: statementsById,
      filesById: filesById,
    } as ModuleIndex;
  });

  // TODO @Broken: get dependencies
  const dependencies = computed(() => []);
  const dependenciesIndex: Ref<ModuleIndex[]> = computed(() => []);

  return {
    module,
    id: computed(() => module.value?.projectVersion?.id),
    name: computed(() => module.value?.projectVersion?.project.name),
    path: computed(() => module.value?.projectVersion?.project.path),
    issues,
    moduleIndex,
    dependenciesIndex,
  };
}

export const useModule = createSharedComposable(_useModule);

export function useCurrentModule(projectVersionId?: Ref<string | null>) {
  const bench = useBenchState();
  const activeVersionId = computed(() =>
    projectVersionId?.value != null ? projectVersionId.value : bench.currentProjectVersionId
  );
  return _useModule(activeVersionId);
}

export function fileOf(statement: { id: string }, projectVersionId?: Ref<string | null>) {
  const { moduleIndex } = useCurrentModule(projectVersionId);
  return moduleIndex.value?.filesById[moduleIndex.value?.statementsById[statement.id]?.file?.id];
}

export function contextOf(symbol: { id: string }, projectVersionId?: Ref<string | null>) {
  const { moduleIndex, dependenciesIndex } = useCurrentModule(projectVersionId);
  for (const idx of [moduleIndex.value, ...dependenciesIndex.value]) {
    if (idx && symbol.id in idx.filesById) {
      const statement = idx.statementsById[symbol.id];
      return {
        id: idx.id,
        name: idx.name,
        path: idx.path,
        file: idx.filesById[statement.file?.id],
        statement,
      };
    }
  }
  return undefined;
}

export function statementOf(id: string, projectVersionId?: Ref<string | null>) {
  if (id == undefined) {
    return undefined;
  }
  // TODO @Performance: symbol lookup by id is awfully inefficient (iterates dependencies)
  const { moduleIndex, dependenciesIndex } = useCurrentModule(projectVersionId);
  for (const idx of [moduleIndex.value, ...dependenciesIndex.value]) {
    if (idx && id in idx.statementsById) {
      return idx.statementsById[id];
    }
  }
  return undefined;
}

export function relativePath(from_: InterpStatement, to_: InterpStatement, projectVersionId?: Ref<string | null>) {
  const from = contextOf(from_, projectVersionId);
  const to = contextOf(to_, projectVersionId);
  if (!from || !to) {
    return undefined;
  } else if (from.path == to.path) {
    return `.${to.file.path}`;
  } else {
    return `${to.path}.${to.file.path}`;
  }
}

export function localErrorsOf(symbol: Ref<{ id: string }>, projectVersionId?: Ref<string | null>) {
  const { issues: errors } = useCurrentModule(projectVersionId);
  return computed(() => errors.value?.filter((e) => e.symbol?.id == symbol.value.id));
}

export type SymbolFilter = {
  types?: StatementType[];
  symbolTypes?: SymbolType[];
  includeAnonymous?: boolean;
  includeDependencies?: boolean;
};
export function symbolsLike(filter: Ref<SymbolFilter> | SymbolFilter, projectVersionId?: Ref<string | null>) {
  const filterRef = isRef(filter) ? filter : ref(filter);
  const { moduleIndex, dependenciesIndex } = useCurrentModule(projectVersionId);
  const symbols = computed(() => {
    if (!moduleIndex.value) {
      return [];
    }
    const allSymbols = Object.values(moduleIndex.value.statementsById);
    if (filterRef.value.includeDependencies) {
      for (const dependencyIndex of dependenciesIndex.value) {
        allSymbols.push(...Object.values(dependencyIndex.statementsById));
      }
    }
    return allSymbols.filter((s) => {
      if (filterRef.value.includeAnonymous || (s.name?.length ?? 0) == 0) {
        return false;
      }
      if (filterRef.value.types != null && !filterRef.value.types.includes(s.type)) {
        return false;
      }
      if (
        filterRef.value.symbolTypes != null &&
        (s.symbolType == null || !filterRef.value.symbolTypes.includes(s.symbolType))
      ) {
        return false;
      }

      return true;
    });
  });
  return symbols;
}

export function useNavigation() {
  const bench = useBenchState();

  function focusSymbol(symbol: { id: string }) {
    const context = contextOf(symbol);
    if (!context?.file) return;
    // can't focus external modules yet
    if (context.id != bench.currentProjectVersionId) return;

    const editor = bench.focusFile(context.file as any) as FileEditor;
    editor.editElement(symbol as any);
  }

  return { focus: focusSymbol };
}

export function newExecutionId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`Execution:${nodeId}`);
}

export function useSymbolOps() {
  const ops = useOperations();
  const notifications = useNotifications();

  async function run(symbol: { id: string; name?: string | null }, executionId?: string) {
    const ret = await ops.runtime.run(symbol.id, undefined, executionId);
    if (ret?.data?.run.__typename != "RunState" || !ret.data.run.success) {
      notifications.show({
        type: "run.fail",
        kind: "error",
        message: "Run failed",
        description: `Failed to run ${symbol.name}: ${ret?.data?.run?.error ?? "rejected"}`,
      });
    }
    return ret;
  }

  async function cancel(executionId: string) {
    return await ops.runtime.cancel(executionId);
  }

  return { run, cancel };
}
