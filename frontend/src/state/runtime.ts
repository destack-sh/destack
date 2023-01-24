import { graphql, useFragment } from "@/gql";
import type { InterpFile, InterpModule, InterpStatement, StatementType, SymbolType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, toRef, type Ref } from "vue";

export const TypeNodeContentInnerType = graphql(/* GraphQL */ `
  fragment TypeNodeContentInner on TypeNode {
    name
    type
    description
    required
    reference
  }
`);

// Nested TypeNodeContent with up to 3 levels of children
export const TypeNodeContent = graphql(/* GraphQL */ `
  fragment TypeNodeContent on TypeNode {
    ...TypeNodeContentInner
    children {
      ...TypeNodeContentInner
      children {
        ...TypeNodeContentInner
        children {
          ...TypeNodeContentInner
        }
      }
    }
  }
`);

export const InterpStatementContentType = graphql(/* GraphQL */ `
  fragment InterpStatementContent on InterpStatement {
    id
    globalId
    name
    type
    modifier
    symbolType
    typeNode {
      ...TypeNodeContent
    }
  }
`);

export const InterpModuleContentType = graphql(/* GraphQL */ `
  fragment InterpModuleContent on InterpModule {
    id
    globalId
    name
    files {
      id
      globalId
      path
      statements {
        ...InterpStatementContent
      }
    }
  }
`);

export const InterpErrorContentType = graphql(/* GraphQL */ `
  fragment InterpErrorContent on InterpError {
    type
    message
    statement {
      ...InterpStatementContent
    }
  }
`);

type ModuleIndex = {
  id: string;
  statementsByGlobalId: Record<string, InterpStatement>;
  fileByStatementId: Record<string, InterpFile>;
};

function indexModule(module: InterpModule): ModuleIndex {
  const statementsByGlobalId: Record<string, InterpStatement> = {};
  const fileByStatementId: Record<string, InterpFile> = {};
  for (const file of module.files) {
    for (const statement of file.statements) {
      statementsByGlobalId[statement.globalId] = statement;
      fileByStatementId[statement.id] = file;
    }
  }
  return { id: module.id, statementsByGlobalId, fileByStatementId };
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
    return dependencies.value?.map(indexModule) ?? [];
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

export function fileOf(statement: InterpStatement) {
  const { moduleIndex } = useCurrentModuleRuntime();
  return moduleIndex.value?.fileByStatementId[statement.id];
}

export function localErrorsOf(statement: Ref<{ id: string }>) {
  const { errors } = useCurrentModuleRuntime();
  return computed(() => errors.value?.filter((e) => e.statement?.globalId == statement.value.id));
}

export function statementsLike(filter: { types?: StatementType[]; symbolTypes?: SymbolType[] }) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const statements = computed(() => {
    if (!moduleIndex.value) {
      return [];
    }
    return Object.values(moduleIndex.value.statementsByGlobalId).filter((s) => {
      if (filter.types != null && !filter.types.includes(s.type)) {
        return false;
      }
      if (filter.symbolTypes != null && (s.symbolType == null || !filter.symbolTypes.includes(s.symbolType))) {
        return false;
      }

      return true;
    });
  });
  return statements;
}

export function useRuntimeTypeOf(statement: Ref<{ id: string }>) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const typeNode = computed(() => {
    if (moduleIndex.value) {
      return moduleIndex.value.statementsByGlobalId[statement.value.id]?.typeNode;
    }
    return null;
  });

  return typeNode;
}
