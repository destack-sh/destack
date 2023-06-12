import { graphql, useFragment } from "@/gql";
import { StatementType, TypeTag, type InterpFileFragment, type InterpStatementFragment } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { FileEditor, useBenchState } from "@/state/bench";
import { InterpFileType, InterpStatementType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import type { SimpleType } from "@/state/statement";
import { toValueRef } from "@/utils/functools";
import { WS_CONNECTED } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { v4 as uuidv4 } from "uuid";
import { computed, isRef, ref, watch, type Ref } from "vue";

// TODO @Cleanup: type InterpFile/InterpStatement more properly
//  apollo fragment typing is annoying..
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

export type ModuleIndex = {
  id: string;
  name: string;
  path: string;
  statementsById: Record<string, InterpStatement>;
  statementsByFileId: Record<string, InterpStatement[]>;
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
          committed
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

  // wake langserver if allowed
  const woken = ref(false);
  const ops = useOperations();
  const auth = useAuth();
  watch([module, WS_CONNECTED, () => auth.loggedIn.value], async () => {
    if (!WS_CONNECTED.value) {
      woken.value = false; // reset woken state
    }
    if (
      WS_CONNECTED.value &&
      !woken.value &&
      projectVersionId.value != null &&
      module.value != null &&
      !module.value?.projectVersion?.committed &&
      auth.loggedIn.value
    ) {
      woken.value = true;
      await ops.runtime.wake(projectVersionId.value);
    }
  });

  const idx: Ref<ModuleIndex | null> = computed(() => {
    if (module.value?.projectVersion == null) return null;
    const statementsById: Record<string, InterpStatement> = {};
    const statementsByFileId: Record<string, InterpStatement[]> = {};
    const filesById: Record<string, InterpFile> = {};

    for (const fileEdge of module.value.projectVersion.files.edges) {
      const file = useFragment(InterpFileType, fileEdge.node);
      if (file.deletedAt != null) continue;
      filesById[fileEdge.node.id] = file;
      for (const statement of fileEdge.node.statements.map((s) => useFragment(InterpStatementType, s))) {
        if (statement.deletedAt != null) continue;
        statementsById[statement.id] = statement;
      }
      statementsByFileId[fileEdge.node.id] = fileEdge.node.statements
        .map((s) => useFragment(InterpStatementType, s))
        .filter((s) => s.deletedAt == null);
    }
    return {
      id: module.value.projectVersion.id,
      name: module.value.projectVersion.project.name,
      path: module.value.projectVersion.project.path,
      statementsById: statementsById,
      statementsByFileId: statementsByFileId,
      filesById: filesById,
    } as ModuleIndex;
  });

  const issues = computed(() => {
    const issues = [];
    for (const statement of Object.values(idx.value?.statementsById ?? {})) {
      if (statement.issues == null) continue;
      issues.push(...statement.issues);
    }
    return issues;
  });

  // TODO @Broken: get dependencies
  const dependencies = computed(() => []);
  const dependenciesIndex: Ref<ModuleIndex[]> = computed(() => []);

  // utils

  function runtimeTypeOf(field: SimpleType): SimpleType {
    if (field.tag != TypeTag.TypeReference) {
      return field;
    } else {
      // impute reference type (not sure if this is a good place to do this)
      const reference = statementOf(field.reference?.id);
      if (reference == null) return field;
      return { ...field, tag: reference.rootTypeTag as TypeTag };
    }
  }

  function fileOf(statement: { id: string }) {
    return idx.value?.filesById[idx.value?.statementsById[statement.id]?.file?.id];
  }

  function contextOf(symbol: { id: string }) {
    for (const i of [idx.value, ...dependenciesIndex.value]) {
      if (i && symbol.id in i.filesById) {
        const statement = i.statementsById[symbol.id];
        return {
          id: i.id,
          name: i.name,
          path: i.path,
          file: i.filesById[statement.file?.id],
          statement,
        };
      }
    }
    return undefined;
  }

  function statementOf(id: string) {
    if (id == undefined) {
      return undefined;
    }
    // TODO @Performance: symbol lookup by id is awfully inefficient (iterates dependencies)
    for (const i of [idx.value, ...dependenciesIndex.value]) {
      if (i && id in i.statementsById) {
        return i.statementsById[id];
      }
    }
    return undefined;
  }

  function relativePath(from_: InterpStatement, to_: InterpStatement) {
    const from = contextOf(from_);
    const to = contextOf(to_);
    if (!from || !to) {
      return undefined;
    } else if (from.path == to.path) {
      return `.${to.file.path}`;
    } else {
      return `${to.path}.${to.file.path}`;
    }
  }

  function localErrorsOf(statement: Ref<{ id: string }>) {
    return computed(() => issues.value?.filter((e) => e.statement?.id == statement.value.id));
  }

  function statementsLike(filter: Ref<StatementFilter> | StatementFilter) {
    const filterRef = isRef(filter) ? filter : ref(filter);
    const statements = computed(() => {
      if (!idx.value) {
        return [];
      }
      const allStatements = Object.values(idx.value.statementsById);
      if (filterRef.value.includeDependencies) {
        for (const dependencyIndex of dependenciesIndex.value) {
          allStatements.push(...Object.values(dependencyIndex.statementsById));
        }
      }
      return allStatements.filter((s) => {
        if (filterRef.value.includeAnonymous || (s.name?.length ?? 0) == 0) {
          return false;
        }
        if (filterRef.value.types != null && !filterRef.value.types.includes(s.type)) {
          return false;
        }
        return true;
      });
    });
    return statements;
  }

  return {
    module,
    id: computed(() => module.value?.projectVersion?.id),
    name: computed(() => module.value?.projectVersion?.project.name),
    path: computed(() => module.value?.projectVersion?.project.path),
    issues,
    idx,
    dependencies,
    dependenciesIndex,
    // utils
    runtimeTypeOf,
    fileOf,
    contextOf,
    statementOf,
    relativePath,
    localErrorsOf,
    statementsLike,
  };
}

export const useModule = createSharedComposable(_useModule);

export function useCurrentModule(projectVersionId?: Ref<string | null>) {
  const bench = useBenchState();
  const activeVersionId = computed(() =>
    projectVersionId?.value != null ? projectVersionId.value : bench.projectVersionId
  );
  return useModule(activeVersionId);
}

type OrderableStatement = Pick<InterpStatement, "id" | "orderKey" | "parent">;
export type OrderedStatement<T extends OrderableStatement> = {
  id: string;
  depth: number;
  ancestors: string[];
  statement: T;
};

export function orderStatements<T extends OrderableStatement>(statements: T[]): OrderedStatement<T>[] {
  const ordered: OrderedStatement<T>[] = [];
  const statementsByParentId: Record<string, T[]> = {};
  // group by parent
  statements.forEach((statement) => {
    if (statementsByParentId[statement.parent?.id ?? ""] != null) {
      statementsByParentId[statement.parent?.id ?? ""].push(statement);
    } else {
      statementsByParentId[statement.parent?.id ?? ""] = [statement];
    }
  });
  // walk from root
  function walkDfs(parentId: string | undefined, depth: number, ancestors: string[]) {
    const children = statementsByParentId[parentId ?? ""];
    if (children) {
      children.sort((a, b) => (a.orderKey > b.orderKey ? 1 : -1));
      for (const child of children) {
        ordered.push({
          id: child.id,
          depth: depth,
          ancestors: ancestors,
          statement: child,
        });
        walkDfs(child.id, depth + 1, [...ancestors, child.id]);
      }
    }
  }
  walkDfs(undefined, 0, []);
  return ordered;
}

export type StatementFilter = {
  types?: StatementType[];
  includeAnonymous?: boolean;
  includeDependencies?: boolean;
};

export function useNavigation() {
  const bench = useBenchState();
  const module = useCurrentModule();

  function focusSymbol(symbol: { id: string }) {
    const context = module.contextOf(symbol);
    if (!context?.file) return;
    // can't focus external modules yet
    if (context.id != bench.projectVersionId) return;

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

export function getSymbolSubtype(statement: {
  type?: StatementType | null;
  rootTypeTag?: TypeTag | null;
  rootTypeFlags?: number | null;
}) {
  if (statement.type == StatementType.Type) {
    if (statement.rootTypeTag == TypeTag.Enum) {
      return "choice";
    } else {
      return "type";
    }
  } else if (statement.type == StatementType.Dataset) {
    return "table";
  } else if (statement.type == StatementType.Value) {
    return "record";
  }

  return null;
}
