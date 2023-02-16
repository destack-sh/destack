import { graphql, useFragment } from "@/gql";
import type { InterpFile, InterpModule, InterpSymbol, StatementType, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { WS_CONNECTED } from "@/utils/globals";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, isRef, ref, toRef, watch, type Ref } from "vue";

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

// TODO @Performance: moduleRuntimeChanged should be partial updates
function _useModuleRuntime(projectVersionId: Ref<string | null>) {
  const {
    result: runtime,
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

  const connected = computed(
    () => WS_CONNECTED.value && !!runtime.value && !error.value && !loading.value && projectVersionId.value != null
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
    moduleIndex,
    dependenciesIndex,
  };
}

export const useModuleRuntime = createSharedComposable(_useModuleRuntime);

export function useCurrentModuleRuntime() {
  const editor = useEditorState();
  return useModuleRuntime(toRef(editor, "currentProjectVersionId"));
}

export function fileOf(symbol: Pick<InterpSymbol, "id">) {
  const { moduleIndex } = useCurrentModuleRuntime();
  return moduleIndex.value?.fileByStatementId[symbol.id];
}

export function contextOf(symbol: Pick<InterpSymbol, "id">) {
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime();
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

export function symbolOf(id: string) {
  // TODO @Performance: symbol lookup by id is awfully inefficient (iterates dependencies)
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime();
  for (const idx of [moduleIndex.value, ...dependenciesIndex.value]) {
    if (idx && id in idx.symbolsById) {
      return idx.symbolsById[id];
    }
  }
  return undefined;
}

export function relativePath(from_: InterpSymbol, to_: InterpSymbol) {
  const from = contextOf(from_);
  const to = contextOf(to_);
  if (!from || !to) {
    return undefined;
  } else if (from.module.id == to.module.id) {
    return `.${to.file.path}`;
  } else {
    return `${to.module.name}.${to.file.path}`;
  }
}

export function localErrorsOf(symbol: Ref<{ id: string }>) {
  const { errors } = useCurrentModuleRuntime();
  return computed(() => errors.value?.filter((e) => e.symbol?.id == symbol.value.id));
}

export type SymbolFilter = {
  types?: StatementType[];
  symbolTypes?: SymbolType[];
  includeGenerated?: boolean;
  includeDependencies?: boolean;
};
export function symbolsLike(filter: Ref<SymbolFilter> | SymbolFilter) {
  const filterRef = isRef(filter) ? filter : ref(filter);
  const { moduleIndex, dependenciesIndex } = useCurrentModuleRuntime();
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
