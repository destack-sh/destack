import { graphql, useFragment } from "@/gql";
import type { InterpModule, InterpStatement, StatementType } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, type Ref, toRef, watch, reactive } from "vue";

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
    sourceId
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
    sourceId
    name
    files {
      id
      sourceId
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
  statementsBySourceId: Record<string, InterpStatement>;
};

function indexModule(module: InterpModule): ModuleIndex {
  const statementsBySourceId: Record<string, InterpStatement> = {};
  for (const file of module.files) {
    for (const statement of file.statements) {
      statementsBySourceId[statement.sourceId] = statement;
    }
  }
  return { id: module.id, statementsBySourceId };
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

export function useCurrentModuleRuntime() {
  const editor = useEditorState();
  return _useModuleRuntime(toRef(editor, "currentProjectVersionId"));
}

export const useModuleRuntime = createSharedComposable(_useModuleRuntime);

export function statementsOfType(type: StatementType) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const statements = computed(() => {
    if (!moduleIndex.value) {
      return [];
    }
    return Object.values(moduleIndex.value.statementsBySourceId).filter((s) => s.type === type);
  });
  return statements;
}

export function useRuntimeTypeOf(statement: Ref<{ id: string }>) {
  const { moduleIndex } = useCurrentModuleRuntime();
  const typeNode = computed(() => {
    if (moduleIndex.value) {
      return moduleIndex.value.statementsBySourceId[statement.value.id]?.typeNode;
    }
    return null;
  });

  return typeNode;
}
