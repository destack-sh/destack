import { graphql, useFragment } from "@/gql";
import { StatementType, TypeTag, type InterpFileFragment, type InterpStatementFragment } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { FileEditor, useBenchState } from "@/state/bench";
import { FieldType, InterpFileType, InterpStatementType, IssueContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import type { Field } from "@/state/statement";
import { DEFAULT_EMBEDDING_DIMENSION, getStorageFormat } from "@/state/type";
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
  IsOptional = 1 << 2,
  IsUnionWith = 1 << 3,
  IsSecret = 1 << 4,
  IsStoreOnly = 1 << 5,
  IsArrayable = 1 << 6,
}

export type ModuleIndex = {
  id: string;
  name: string;
  path: string;
  statementsById: Record<string, InterpStatement>;
  statementsByFileId: Record<string, InterpStatement[]>;
  statementsByParentId: Record<string, InterpStatement[]>;
  filesById: Record<string, InterpFile>;
};

function _useModuleFlat(projectVersionId: Ref<string | null>) {
  const { result: module, loading } = useQuery(
    graphql(/* GraphQL */ `
      query module($projectVersionId: GlobalID!) {
        projectVersion(id: $projectVersionId) {
          id
          committed
          project {
            path
            name
          }
          files(filters: { isVisible: true }) {
            edges {
              node {
                ...InterpFile
                statements(filters: { isVisible: true }) {
                  ...InterpStatement
                  issues(filters: { scope: STATEMENT }) {
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

  const idx: Ref<ModuleIndex | null> = computed(() => {
    if (module.value?.projectVersion == null) return null;
    const statementsById: Record<string, InterpStatement> = {};
    const statementsByFileId: Record<string, InterpStatement[]> = {};
    const statementsByParentId: Record<string, InterpStatement[]> = {};
    const filesById: Record<string, InterpFile> = {};

    for (const fileEdge of module.value.projectVersion.files.edges) {
      const file = useFragment(InterpFileType, fileEdge.node);
      if (file.deletedAt != null) continue;
      filesById[fileEdge.node.id] = file;
      for (const statement of fileEdge.node.statements.map((s) => useFragment(InterpStatementType, s))) {
        if (statement.deletedAt != null) continue;
        statementsById[statement.id] = statement;
        if (statementsByParentId[statement.parent?.id] == null) {
          statementsByParentId[statement.parent?.id] = [];
        }
        statementsByParentId[statement.parent?.id].push(statement);
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
      statementsByParentId: statementsByParentId,
      filesById: filesById,
    } as ModuleIndex;
  });

  const issues = computed(() => {
    const issues: any[] = [];
    for (const fileEdge of module.value?.projectVersion?.files?.edges ?? []) {
      const file = useFragment(InterpFileType, fileEdge.node);
      if (file.deletedAt != null) continue;
      file.issues.forEach((i) => issues.push(i));
      for (const statementEdge of fileEdge.node.statements) {
        const statement = useFragment(InterpStatementType, statementEdge);
        if (statement.deletedAt != null) continue;
        statementEdge.issues?.forEach((i) => issues.push(i));
      }
    }
    return issues.map((i) => useFragment(IssueContentType, i));
  });

  return {
    loading: computed(() => loading.value || projectVersionId.value == null),
    module,
    idx,
    issues,
  };
}

function _useModule(projectVersionId: Ref<string | null>) {
  projectVersionId = toValueRef(projectVersionId);

  const { loading, module, idx, issues } = _useModuleFlat(projectVersionId);

  // TODO @Performance: cache symbolx lib (and other default module dependencies)
  // TODO @Broken: don't hardcode symbolx.lib id
  // (this is not that terrible since the project version id is static for now, see :LibImplementation)
  const symbolxLib = _useModuleFlat(ref("UHJvamVjdFZlcnNpb246ZjRmZjUxMWYtNzg4MS01NzUwLTgxMjEtODY1YTk1MGE5MDAz"));
  const dependenciesIndex: Ref<ModuleIndex[]> = computed(() =>
    [symbolxLib.idx.value].filter((v) => v != null).map((v) => v as ModuleIndex)
  );

  // run metadata fields are hardcoded for now
  const runMetadataFields = computed(() => {
    return (
      Object.values(symbolxLib.idx.value?.statementsById ?? {})
        .find((s) => s.name == "RunMetadata")
        ?.fields.map((f) => useFragment(FieldType, f)) ?? []
    );
  });
  function runMetadataKey(name: string): string | null {
    const field = runMetadataFields.value.find((f) => f.name == name);
    if (field == null) return null;
    return getTypedKey(field);
  }

  // utils

  function fileOf(statement: { id: string }) {
    return idx.value?.filesById[idx.value?.statementsById[statement.id]?.file?.id];
  }

  function pathOf(fileOrStatement: { id: string }) {
    // traverse parents
    const statement = idx.value?.statementsById[fileOrStatement.id];
    if (statement != null) {
      const filePath = pathOfFile(statement.file);
      return filePath + "." + statement.name;
    } else {
      return pathOfFile(fileOrStatement);
    }
  }

  function pathOfFile(file: { id: string }) {
    let f = idx.value?.filesById[file.id];
    const path: string[] = [];
    while (f != null) {
      path.push(f.name);
      f = idx.value?.filesById[f.parent?.id ?? ""];
    }
    return path.reverse().join(".");
  }

  function contextOf(symbol: { id: string }) {
    for (const i of [idx.value, ...dependenciesIndex.value]) {
      if (i && symbol.id in i.statementsById) {
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
      return `.${to.file.name}`;
    } else {
      return `${to.path}.${to.file.name}`;
    }
  }

  function localIssuesOf(statement: Ref<{ id: string }>) {
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

  const tags = statementsLike({
    types: [StatementType.Tag],
    includeDependencies: true,
  });
  const tagsByKey = computed(() => {
    const tagsByKey: Record<string, InterpStatement> = {};
    for (const tag of tags.value) {
      tagsByKey[tag.key as string] = tag;
    }
    return tagsByKey;
  });

  function getDescendantsOf(statementId: { id: string }) {
    const statement = idx.value?.statementsById[statementId.id];
    if (statement == null) return [];
    const descendants: InterpStatement[] = [];
    const walkDfs = (statement: InterpStatement) => {
      descendants.push(statement);
      for (const child of idx.value?.statementsByParentId[statement.id] ?? []) {
        walkDfs(child);
      }
    };
    walkDfs(statement);
    return descendants;
  }

  function getTypedKey(field: Pick<Field, "key" | "tag" | "hint" | "flags" | "reference" | "metadata">) {
    // TODO @Performance: cache getTypedKey (esp. when without references & metadata)
    let tag = field.tag;
    if (field.tag == TypeTag.TypeReference) {
      const reference = statementOf(field.reference?.id);
      if (reference == null) return null;
      tag = reference.rootTypeTag as TypeTag;
    }
    const storageFormat = getStorageFormat(tag, field.hint ?? undefined, field.flags);
    if (tag == TypeTag.Vector) {
      const dimension = field.metadata?.dimension ?? DEFAULT_EMBEDDING_DIMENSION;
      return `${field.key}-${storageFormat}${dimension}`;
    } else {
      return `${field.key}-${storageFormat}`;
    }
  }

  function effectiveTypeOf(field: Field): Field {
    if (field.tag == TypeTag.TypeReference) {
      const reference = statementOf(field.reference?.id);
      if (reference == null) return field;
      return { ...field, tag: reference.rootTypeTag as TypeTag };
    } else {
      return field;
    }
  }

  // wake langserver as needed & possible
  const wokeLangserver = ref(false);
  const ops = useOperations();
  const auth = useAuth();
  watch([module, WS_CONNECTED, () => auth.loggedIn.value], async () => {
    if (!WS_CONNECTED.value) {
      wokeLangserver.value = false; // reset woken state
    }
    if (
      WS_CONNECTED.value &&
      !wokeLangserver.value &&
      projectVersionId.value != null &&
      module.value != null &&
      !module.value?.projectVersion?.committed &&
      auth.loggedIn.value
    ) {
      wokeLangserver.value = true;
      await ops.runtime.wake(projectVersionId.value);
    }
  });

  return {
    loading: computed(() => loading.value || projectVersionId.value == null),
    module,
    id: computed(() => module.value?.projectVersion?.id),
    name: computed(() => module.value?.projectVersion?.project.name),
    path: computed(() => module.value?.projectVersion?.project.path),
    issues,
    idx,
    symbolxLib,
    dependenciesIndex,
    // utils
    runMetadataFields,
    runMetadataKey,
    fileOf,
    pathOf,
    contextOf,
    statementOf,
    relativePath,
    getTypedKey,
    effectiveTypeOf,
    localIssuesOf,
    statementsLike,
    tags,
    tagsByKey,
    getDescendantsOf,
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
  if (statements.length == 0) return [];
  const ordered: OrderedStatement<T>[] = [];
  const statementsByParentId: Record<string, T[]> = {};
  // group by parent
  statements.forEach((statement) => {
    if (statementsByParentId[statement.parent.id] != null) {
      statementsByParentId[statement.parent.id].push(statement);
    } else {
      statementsByParentId[statement.parent.id] = [statement];
    }
  });
  // walk from root
  function walkDfs(parentId: string, depth: number, ancestors: string[]) {
    const children = statementsByParentId[parentId];
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
  const fileId = statements.find((s) => s.parent.__typename == "File")?.parent.id; // assumes all statements are from the same file
  walkDfs(fileId, 0, []);
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

  function focusFile(file: { id: string }) {
    const file_ = module.fileOf({ id: file.id });
    if (file_ == null) return;
    bench.focusFile(file_);
  }

  return { focusStatement: focusSymbol, focusFile };
}

export function newRunId(): string {
  const nodeId = uuidv4();
  return btoa(`Run:${nodeId}`);
}

export function newSessionId(): string {
  const nodeId = uuidv4();
  return btoa(`Session:${nodeId}`);
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
  }
  return null;
}
