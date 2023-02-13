import { graphql, useFragment } from "@/gql";
import type { InterpFile, InterpModule, InterpSymbol, StatementType, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, toRef, type Ref } from "vue";

export const InterpSymbolContentType = graphql(/* GraphQL */ `
  fragment InterpSymbolContent on InterpSymbol {
    id
    name
    type
    modifier
    symbolType
    rootTypeTag
    typeNodes {
      ...SimpleTypeNodeContent
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
  const { result: runtime } = useSubscription(
    graphql(/* GraphQL */ `
      subscription moduleRuntimeChanged($projectVersionId: GlobalID!) {
        moduleRuntimeChanged(projectVersionId: $projectVersionId) {
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
    { projectVersionId }
  );

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

export function relativePath(fromStmt: InterpSymbol, toStmt: InterpSymbol) {
  const from = contextOf(fromStmt);
  const to = contextOf(toStmt);
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

export function symbolsLike(filter: { types?: StatementType[]; symbolTypes?: SymbolType[] }) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const symbols = computed(() => {
    if (!moduleIndex.value) {
      return [];
    }
    return Object.values(moduleIndex.value.symbolsById).filter((s) => {
      if (filter.types != null && !filter.types.includes(s.type)) {
        return false;
      }
      if (filter.symbolTypes != null && (s.symbolType == null || !filter.symbolTypes.includes(s.symbolType))) {
        return false;
      }

      return true;
    });
  });
  return symbols;
}

export function useRuntimeTypeOf(symbol: Ref<{ id: string }>) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const typeNode = computed(() => {
    if (moduleIndex.value) {
      return moduleIndex.value.symbolsById[symbol.value.id]?.typeNode;
    }
    return null;
  });

  return typeNode;
}
