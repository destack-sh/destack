import { graphql, useFragment } from "@/gql";
import type { InterpError, InterpFile, InterpModule, InterpSymbol, StatementType, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { WS_CONNECTED } from "@/utils/globals";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, isRef, ref, watch, type Ref } from "vue";

export const InterpSymbolContentType = graphql(/* GraphQL */ `
  fragment InterpSymbolContent on InterpSymbol {
    id
    name
    type
    orderKey
    parentId
    modifier
    symbolType
    rootTypeTag
    generated
    typeNodes {
      # not using SimpleTypeNodeContent fragment because it's for the editable node
      # and using a shared fragment seems overkill
      id
      name
      tag
      description
      value
      orderKey
      reference {
        id
      }
      isOutput
      isArray
      isNullable
    }
  }
`);

export const InterpModuleContentType = graphql(/* GraphQL */ `
  fragment InterpModuleContent on InterpModule {
    id
    name
    files {
      id
      path
      symbols {
        ...InterpSymbolContent
      }
    }
  }
`);

export const InterpErrorContentType = graphql(/* GraphQL */ `
  fragment InterpErrorContent on InterpError {
    type
    message
    symbol {
      ...InterpSymbolContent
    }
  }
`);

export const InterpJobContentType = graphql(/* GraphQL */ `
  fragment InterpJobContent on InterpJob {
    id
    type
    status
    startedAt
    terminatedAt
  }
`);

type ModuleIndex = {
  id: string;
  module: InterpModule;
  symbolsById: Record<string, InterpSymbol>;
  fileByStatementId: Record<string, InterpFile>;
};

function indexModule(module: InterpModule): ModuleIndex {
  const symbolsById: Record<string, InterpSymbol> = {};
  const fileByStatementId: Record<string, InterpFile> = {};
  for (const file of module.files) {
    for (const symbol of file.symbols) {
      symbolsById[symbol.id] = symbol;
      fileByStatementId[symbol.id] = file;
    }
  }
  return { id: module.id, module, symbolsById, fileByStatementId };
}

// TODO @Performance: moduleRuntimeChanged should be partial updates :PartialModuleUpdates
function _useModuleRuntime(projectVersionId: Ref<string | null>) {
  const {
    result: fetchedRuntime,
    loading,
    error,
    start,
    stop,
    onResult: runtimeUpdated,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {
        moduleRuntimeChanged(projectVersionId: $projectVersionId) {
          updatedAt
          module {
            ...InterpModuleContent
          }
          dependencies {
            ...InterpModuleContent
          }
          errors {
            ...InterpErrorContent
          }
          jobs {
            ...InterpJobContent
          }
          staleSymbols {
            id
            name
            type
            symbolType
            modifier
            parentId
            rootTypeTag
            generated
          }
        }
      }
    `),
    { projectVersionId },
    {}
  );
  // enable/disable subscription when projectVersionId changes
  watch(
    projectVersionId,
    () => {
      if (projectVersionId.value != null) {
        start();
      } else {
        stop();
      }
    },
    { immediate: true }
  );

  // cache last runtime
  const runtime: Ref<typeof fetchedRuntime.value | null> = ref(null);
  watch(fetchedRuntime, (newRuntime) => {
    if (newRuntime != null) {
      runtime.value = newRuntime;
    }
  });

  const connected = computed(
    () => WS_CONNECTED.value && fetchedRuntime.value && !error.value && !loading.value && projectVersionId.value != null
  );
  const lastUpdated: Ref<string | null> = ref(null);
  runtimeUpdated(() => (lastUpdated.value = runtime.value?.moduleRuntimeChanged.updatedAt));

  const module = computed(() => useFragment(InterpModuleContentType, runtime.value?.moduleRuntimeChanged.module));
  const dependencies = computed(() =>
    runtime.value?.moduleRuntimeChanged.dependencies.map((m) => useFragment(InterpModuleContentType, m))
  );
  const errors = computed(() =>
    runtime.value?.moduleRuntimeChanged.errors.map((e) => useFragment(InterpErrorContentType, e))
  );
  const jobs = computed(() =>
    runtime.value?.moduleRuntimeChanged.jobs.map((j) => useFragment(InterpJobContentType, j))
  );
  const staleSymbols = computed(() => runtime.value?.moduleRuntimeChanged.staleSymbols);

  const moduleIndex: Ref<ModuleIndex | null> = computed(() => {
    if (module.value) {
      return indexModule(module.value as InterpModule);
    }
    return null;
  });
  const dependenciesIndex: Ref<ModuleIndex[]> = computed(() => {
    return dependencies.value?.map((idx) => indexModule(idx as InterpModule)) ?? [];
  });

  return {
    connected,
    lastUpdated,
    module,
    dependencies,
    errors,
    jobs,
    staleSymbols,
    moduleIndex,
    dependenciesIndex,
  };
}

function _useCurrentModuleRuntime(projectVersionId?: Ref<string | null>) {
  const editor = useEditorState();
  const activeVersionId = computed(() =>
    projectVersionId?.value != null ? projectVersionId.value : editor.currentProjectVersionId
  );
  return _useModuleRuntime(activeVersionId);
}

export const useCurrentModuleRuntime = createSharedComposable(_useCurrentModuleRuntime);

export function fileOf(symbol: Pick<InterpSymbol, "id">, projectVersionId?: Ref<string | null>) {
  const { moduleIndex } = useCurrentModuleRuntime(projectVersionId);
  return moduleIndex.value?.fileByStatementId[symbol.id];
}

export function contextOf(symbol: Pick<InterpSymbol, "id">, projectVersionId?: Ref<string | null>) {
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime(projectVersionId);
  for (const idx of [moduleIndex.value, ...dependenciesIndex.value]) {
    if (idx && symbol.id in idx.fileByStatementId) {
      return {
        module: idx.module,
        file: idx.fileByStatementId[symbol.id],
        symbol: idx.symbolsById[symbol.id],
      };
    }
  }
  return undefined;
}

export function symbolOf(id: string, projectVersionId?: Ref<string | null>) {
  // TODO @Performance: symbol lookup by id is awfully inefficient (iterates dependencies)
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime(projectVersionId);
  for (const idx of [moduleIndex.value, ...dependenciesIndex.value]) {
    if (idx && id in idx.symbolsById) {
      return idx.symbolsById[id];
    }
  }
  return undefined;
}

export function relativePath(from_: InterpSymbol, to_: InterpSymbol, projectVersionId?: Ref<string | null>) {
  const from = contextOf(from_, projectVersionId);
  const to = contextOf(to_, projectVersionId);
  if (!from || !to) {
    return undefined;
  } else if (from.module.id == to.module.id) {
    return `.${to.file.path}`;
  } else {
    return `${to.module.name}.${to.file.path}`;
  }
}

export function localErrorsOf(symbol: Ref<{ id: string }>, projectVersionId?: Ref<string | null>) {
  const { errors } = useCurrentModuleRuntime(projectVersionId);
  return computed(() => errors.value?.filter((e) => e.symbol?.id == symbol.value.id));
}

export function isSymbolStale(symbol: Ref<{ id: string } | undefined>, projectVersionId?: Ref<string | null>) {
  const { staleSymbols } = useCurrentModuleRuntime(projectVersionId);
  return computed(() => (symbol.value == null ? undefined : staleSymbols.value?.some((s) => s.id == symbol.value.id)));
}

export type SymbolFilter = {
  types?: StatementType[];
  symbolTypes?: SymbolType[];
  includeAnonymous?: boolean;
  includeGenerated?: boolean;
  includeDependencies?: boolean;
};
export function symbolsLike(filter: Ref<SymbolFilter> | SymbolFilter, projectVersionId?: Ref<string | null>) {
  const filterRef = isRef(filter) ? filter : ref(filter);
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime(projectVersionId);
  const symbols = computed(() => {
    if (!moduleIndex.value) {
      return [];
    }
    const allSymbols = Object.values(moduleIndex.value.symbolsById);
    if (filterRef.value.includeDependencies) {
      for (const dependencyIndex of dependenciesIndex.value) {
        allSymbols.push(...Object.values(dependencyIndex.symbolsById));
      }
    }
    return allSymbols.filter((s) => {
      if (filterRef.value.includeAnonymous || (s.name?.length ?? 0) == 0) {
        return false;
      }
      if (!filterRef.value.includeGenerated && s.generated) {
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

export function useVisibleErrors() {
  const runtime = useCurrentModuleRuntime();
  const editor = useEditorState();
  return computed(() =>
    (runtime.errors.value ?? [])
      .map((e) => e as InterpError)
      .filter((e: InterpError) => e.symbol == null || editor.showGenerated || !e.symbol.generated)
  );
}

export function useSymbolNavigation() {
  const editor = useEditorState();

  function focusSymbol(symbol: { id: string }) {
    const context = contextOf(symbol);
    if (!context?.file) return;
    // can't focus external modules yet
    if (context.module.id != editor.currentProjectVersionId) return;

    editor.focusFile(context.file as any);
    editor.editElement(symbol as any);
  }

  return { focusSymbol };
}

export function useSymbolOps() {
  const operations = useOperations();
  const notifications = useNotifications();
  const editor = useEditorState();

  async function build(symbol: { id: string; name?: string | null }) {
    const ret = await operations.runtime.build(symbol.id);
    if (ret?.data?.build.__typename != "BuildState" || !ret.data.build.success) {
      notifications.show({
        type: "build.fail",
        kind: "error",
        message: "Build failed",
        description: `Build failed for ${symbol.name}`,
      });
    }
  }

  async function openRun(symbol: { id: string; name?: string | null; symbolType: SymbolType }) {
    const runEditor = editor.openRun(symbol as any);
    editor.focusEditor(runEditor);
  }

  async function evaluate(symbol: { id: string; name?: string | null }) {
    const ret = await operations.runtime.evaluate(symbol.id);
    if (ret?.data?.evaluate.__typename != "EvaluateState" || !ret.data.evaluate.success) {
      notifications.show({
        type: "evaluate.fail",
        kind: "error",
        message: "Evaluation failed",
        description: `Evaluation failed for ${symbol.name}`,
      });
    }
  }

  return { build, openRun, evaluate };
}
