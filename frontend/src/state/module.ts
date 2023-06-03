import { graphql, useFragment } from "@/gql";
import type { StatementType, SymbolType, InterpError, InterpFile, InterpModule, InterpSymbol } from "@/gql/graphql";
import { FileEditor, useBenchState } from "@/state/bench";
import { useNotifications } from "@/state/notifications";
import { useOperations } from "@/state/operations";
import { toValueRef } from "@/utils/functools";
import { WS_CONNECTED } from "@/utils/globals";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { DateTime } from "luxon";
import { v4 as uuidv4 } from "uuid";
import { computed, isRef, ref, watch, type Ref } from "vue";

export enum TypeFlag { // :TypeFlags
  Zero = 0,
  IsOutput = 1 << 0,
  IsArray = 1 << 1,
  IsNullable = 1 << 2,
  IsUnionWith = 1 << 3,
  IsSecret = 1 << 4,
}

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
    fields {
      # not using FieldContent fragment because it's for the editable node
      # and using a shared fragment seems overkill
      id
      name
      key
      tag
      hint
      description
      value
      orderKey
      reference {
        id
      }
      flags
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
    dependencies {
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
    errors {
      ...InterpErrorContent
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
function _useInterpModule(projectVersionId: Ref<string | null>) {
  projectVersionId = toValueRef(projectVersionId);
  const {
    result: fetchedRuntime,
    loading,
    error,
    start,
    stop,
    onResult: runtimeUpdated,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription interpChanged($projectVersionId: GlobalID!) {
        interpChanged(projectVersionId: $projectVersionId) {
          ...InterpModuleContent
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
  runtimeUpdated(() => (lastUpdated.value = DateTime.now().toISO()));

  const module = computed(() => useFragment(InterpModuleContentType, runtime.value?.interpChanged));
  const path = computed(() => module.value?.name);
  const name = computed(() => module.value?.name.split(".").slice(-1)[0]);
  const dependencies = computed(() => module.value?.dependencies.map((m) => useFragment(InterpModuleContentType, m)));
  const errors = computed(() => module.value?.errors.map((e) => useFragment(InterpErrorContentType, e)));

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
    path,
    name,
    dependencies,
    errors,
    moduleIndex,
    dependenciesIndex,
  };
}

function _useCurrentInterpModule(projectVersionId?: Ref<string | null>) {
  const bench = useBenchState();
  const activeVersionId = computed(() =>
    projectVersionId?.value != null ? projectVersionId.value : bench.currentProjectVersionId
  );
  return _useInterpModule(activeVersionId);
}

export const useCurrentInterpModule = createSharedComposable(_useCurrentInterpModule);

export function fileOf(symbol: Pick<InterpSymbol, "id">, projectVersionId?: Ref<string | null>) {
  const { moduleIndex } = useCurrentInterpModule(projectVersionId);
  return moduleIndex.value?.fileByStatementId[symbol.id];
}

export function contextOf(symbol: Pick<InterpSymbol, "id">, projectVersionId?: Ref<string | null>) {
  const { moduleIndex, dependenciesIndex } = useCurrentInterpModule(projectVersionId);
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
  if (id == undefined) {
    return undefined;
  }
  // TODO @Performance: symbol lookup by id is awfully inefficient (iterates dependencies)
  const { moduleIndex, dependenciesIndex } = useCurrentInterpModule(projectVersionId);
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
  const { errors } = useCurrentInterpModule(projectVersionId);
  return computed(() => errors.value?.filter((e) => e.symbol?.id == symbol.value.id));
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
  const { moduleIndex, dependenciesIndex } = useCurrentInterpModule(projectVersionId);
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
  const runtime = useCurrentInterpModule();
  const bench = useBenchState();
  return computed(() =>
    (runtime.errors.value ?? [])
      .map((e) => e as InterpError)
      .filter((e: InterpError) => e.symbol == null || bench.showGenerated || !e.symbol.generated)
  );
}

export function useSymbolNavigation() {
  const bench = useBenchState();

  function focusSymbol(symbol: { id: string }) {
    const context = contextOf(symbol);
    if (!context?.file) return;
    // can't focus external modules yet
    if (context.module.id != bench.currentProjectVersionId) return;

    const editor = bench.focusFile(context.file as any) as FileEditor;
    editor.editElement(symbol as any);
  }

  return { focusSymbol };
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
