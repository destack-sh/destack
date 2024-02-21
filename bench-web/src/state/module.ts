import { graphql, useFragment } from "@/gql";
import {
  StatementType,
  TypeTag,
  type InterpFileFragment,
  type InterpStatementFragment,
  IssueKind,
  type HasCrud as HasCrudGql,
  type ProjectVersion as ProjectVersionGql,
  type Statement as StatementGql,
  type File as FileGql,
  type Field as FieldGql,
  type ResolvedField as ResolvedFieldGql,
  type Record as RecordGql,
  type Issue as IssueGql,
  type Tagging as TaggingGql,
  type Trigger as TriggerGql,
} from "@/gql/graphql";
import { v4 as uuidv4, v5 as uuidv5, validate } from "uuid";
import { useAuth } from "@/state/auth";
import { EditFilePanel, useBenchState } from "@/state/bench";
import { InterpFileType, IssueContentType } from "@/state/fragments";
import { useOperations } from "@/state/operations";
import { DEFAULT_EMBEDDING_DIMENSION, getStorageFormat } from "@/state/type";
import { getUUIDFromGlobalID, toValueRef } from "@/utils/functools";
import { VERSION, WS_CONNECTED } from "@/utils/globals";
import { useQuery } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, isRef, ref, watch, type Ref, nextTick } from "vue";
import { STATEMENT_TYPE_TAGS } from "@/state/statement";
import { AggregationOp } from "@/proto/wire";

export type NodeBase = { __typename: string; id: string; ck: string; name?: string | null };
// TODO :Cleanup :Robustness: type module objects more correctly
// full objects
export type HasCrud = Omit<HasCrudGql, "__typename" | "id">;
export type HasCrudKey = keyof HasCrud;
export type Module = ProjectVersionGql;
export type File = FileGql;
export type Statement = StatementGql;
export type Field = FieldGql;
export type ResolvedField = ResolvedFieldGql;
export type Record = RecordGql;
export type Tagging = TaggingGql;
export type Trigger = TriggerGql;
export type Issue = IssueGql;
// interpreter state
export type InterpFile = InterpFileFragment;
export type InterpStatement = Omit<InterpStatementFragment, "fields" | "tags" | "triggers" | "issues"> & {
  // proper typing for sub-fields
  fields: Field[];
  tags: Tagging[];
  triggers: Trigger[];
  issues: Issue[];
};

export type ModuleObjectTypename =
  | "ProjectVersion"
  | "File"
  | "Statement"
  | "Field"
  | "ResolvedField"
  | "Record"
  | "Issue"
  | "Tagging"
  | "Trigger";

export function newNodeIdentity(moduleId: string, type: ModuleObjectTypename): { id: string; ck: string } {
  const ck = uuidv4();
  const id = getNodeIdFromCk(moduleId, ck, type);
  return { id, ck };
}

export function getNodeIdFromCk(moduleId: string, ck: string, type: ModuleObjectTypename): string {
  if (!validate(moduleId)) {
    // looks like a global id
    moduleId = getUUIDFromGlobalID(moduleId);
  }
  const id = uuidv5(ck, moduleId);
  return btoa(`${type}:${id}`);
}

export function getNodeIdFromCkMaybe(
  moduleId: string | null | undefined,
  ck: string | null | undefined,
  type: ModuleObjectTypename
): string | null {
  if (moduleId == null || ck == null) return null;
  return getNodeIdFromCk(moduleId, ck, type);
}

export function newDetachedNodeIdentity(type: ModuleObjectTypename): { id: string; ck: string } {
  const ck = uuidv4();
  return { id: btoa(`${type}:${ck}`), ck };
}

export enum TypeFlag { // :TypeFlags
  ZERO = 0,
  IS_OUTPUT = 1 << 0,
  IS_ARRAY = 1 << 1,
  IS_OPTIONAL = 1 << 2,
  IS_UNION_WITH = 1 << 3,
  IS_SECRET = 1 << 4,
  IS_STORE_ONLY = 1 << 5,
  IS_ARRAYABLE = 1 << 6,
  IS_META = 1 << 7,
  IS_CONFIG = 1 << 8,
  IS_HIDDEN = 1 << 9,
}

type GRecord<K extends keyof any, V> = globalThis.Record<K, V>;
export type ModuleIndex = {
  id: string;
  name: string;
  path: string;
  statementsById: GRecord<string, InterpStatement>;
  statementsByFileId: GRecord<string, InterpStatement[]>;
  statementsByParentId: GRecord<string, InterpStatement[]>;
  filesById: GRecord<string, InterpFile>;
  fieldsById: GRecord<string, Field>;
  idByCk: GRecord<string, string>;
};

function _useModuleFlat(moduleOrProjectId: Ref<string | null>, options?: { cache?: boolean; required?: string }) {
  const {
    result: module,
    loading,
    onResult,
  } = useQuery(
    graphql(/* GraphQL */ `
      query moduleContentById($moduleOrProjectId: GlobalID!) {
        module(id: $moduleOrProjectId) {
          id
          committed
          project {
            path
            name
          }
          files(filters: { isVisible: true }) {
            ...InterpFile
            statements(filters: { isVisible: true }) {
              ...InterpStatement
            }
          }
        }
      }
    `),
    () => ({ moduleOrProjectId: moduleOrProjectId.value }),
    () => ({
      enabled: !!moduleOrProjectId.value,
      fetchPolicy: !(options?.cache ?? false) ? "cache-and-network" : "network-only",
    })
  );

  // check result if module is required
  onResult((result) => {
    if (options?.required && result.data.module == null) {
      throw new Error(`required module ${options?.required} (id=${moduleOrProjectId.value}) does not exist`);
    }
  });

  const idx: Ref<ModuleIndex | null> = computed(() => {
    if (module.value?.module == null) return null;
    // compile the primary index
    const statementsById: GRecord<string, InterpStatement> = {};
    const statementsByFileId: GRecord<string, InterpStatement[]> = {};
    const statementsByParentId: GRecord<string, InterpStatement[]> = {};
    const filesById: GRecord<string, InterpFile> = {};
    const fieldsById: GRecord<string, Field> = {};
    const idByCk: GRecord<string, string> = {}; // not comprehensive yet (does not include all module object types)

    // TODO :Cleanup: type module objects more correctly (file/statements/issues)
    for (const file of module.value.module.files.map((f) => useFragment(InterpFileType, f))) {
      if (file.deletedAt != null) continue;
      filesById[file.id] = file;
      idByCk[file.ck] = file.id;

      for (const statement of (file as unknown as { statements: InterpStatement[] }).statements) {
        if (statement.deletedAt != null) continue;
        statementsById[statement.id] = statement;
        idByCk[statement.ck] = statement.id;

        if (statementsByParentId[statement.parent?.id] == null) {
          statementsByParentId[statement.parent?.id] = [];
        }
        statementsByParentId[statement.parent?.id].push(statement);
        for (const field of statement.fields) {
          fieldsById[field.id] = field;
          idByCk[field.ck] = field.id;
        }
      }
      statementsByFileId[file.id] = (file as unknown as { statements: InterpStatement[] }).statements.filter(
        (s) => s.deletedAt == null
      );
    }
    return {
      id: module.value.module.id,
      name: module.value.module.project.name,
      path: module.value.module.project.path,
      statementsById: statementsById,
      statementsByFileId: statementsByFileId,
      statementsByParentId: statementsByParentId,
      filesById: filesById,
      fieldsById: fieldsById,
      idByCk,
    } as ModuleIndex;
  });

  const issues = computed(() => {
    if (module.value?.module == null) return [];
    const issues: any[] = [];
    for (const file of module.value.module.files.map((f) => useFragment(InterpFileType, f))) {
      if (file.deletedAt != null) continue;
      file.issues.forEach((i) => issues.push(i));
      for (const statement of (file as unknown as { statements: InterpStatement[] }).statements) {
        if (statement.deletedAt != null) continue;
        statement.issues?.forEach((i) => issues.push(i));
      }
    }
    return issues.map((i) => useFragment(IssueContentType, i) as Issue);
  });

  return {
    loading: computed(() => loading.value || moduleOrProjectId.value == null),
    module,
    idx,
    issues,
  };
}

function _defaultLibId(name: string): string {
  /* Derive the id of a default library project */
  return uuidv5(`builtin:${name}`, BENCH_UUID_NAMESPACE);
}

// TODO :Robustness: exclude own library if it's a dependency
const BENCH_UUID_NAMESPACE = "d822dab7-41ad-4706-a9c8-4379e15b2ed0"; // :BenchUuidNamespace
const DEFAULT_LIBRARIES: GRecord<string, string> = {
  // default libs
  "symbolx.lib": _defaultLibId("symbolx.lib"),
  "openai.lib": _defaultLibId("openai.lib"),
  "anthropic.lib": _defaultLibId("anthropic.lib"),
  "deepgram.lib": _defaultLibId("deepgram.lib"),
  "huggingface.lib": _defaultLibId("huggingface.lib"),
  // templates
  "symbolx.templates": "bba83b4f-04c2-40e1-8597-8263aa4c5fb6", // hard-coded since it's not deterministic
};

function _useModule(moduleOrProjectId: Ref<string | null>) {
  moduleOrProjectId = toValueRef(moduleOrProjectId);

  const { loading, module, idx, issues } = _useModuleFlat(moduleOrProjectId);

  const errors = computed(() => issues.value?.filter((e) => e.kind == IssueKind.Error));
  const warnings = computed(() => issues.value?.filter((e) => e.kind == IssueKind.Warning));
  const notices = computed(() => issues.value?.filter((e) => e.kind == IssueKind.Notice));

  // TODO :Performance: cache default libs (and any other static module dependencies)
  // load default libraries, derive their ids deterministically from their names and current version :BuiltinLibs
  const defaultLibs: GRecord<string, Ref<ModuleIndex | null>> = {};
  for (const name of Object.keys(DEFAULT_LIBRARIES)) {
    const gid = btoa(`Project:${DEFAULT_LIBRARIES[name]}`);
    defaultLibs[name] = _useModuleFlat(ref(gid), { cache: true, required: `${name}@${VERSION}` }).idx;
  }

  const dependenciesIndex: Ref<ModuleIndex[]> = computed(() =>
    Object.values(defaultLibs)
      .map((v) => v.value)
      .filter((v) => v != null)
      .map((v) => v as ModuleIndex)
  );

  function _getFields(name: string): Field[] {
    return (
      Object.values(defaultLibs["symbolx.lib"]?.value?.statementsById ?? {})
        .find((s) => s.name == name)
        ?.fields.map((f) => f as Field) ?? []
    );
  }
  function _getFieldKey(fields: Field[], name: string): string | null {
    const field = fields.find((f) => f.name == name);
    if (field == null) return null;
    return getTypedKey(field);
  }
  // run value fields are hardcoded for now
  const runMetadataFields = computed(() => _getFields("RunMetadata"));
  function runMetadataKey(name: string): string | null {
    return _getFieldKey(runMetadataFields.value, name);
  }
  const taskRunConfigFields = computed(() => _getFields("TaskRunConfig"));
  function taskRunConfigKey(name: string): string | null {
    return _getFieldKey(taskRunConfigFields.value, name);
  }

  // utils

  function fileOf(idOrCk: string) {
    const id = idx.value?.idByCk[idOrCk] ?? idOrCk;
    let statement = idx.value?.statementsById[id];
    if (statement == null) {
      const field = idx.value?.fieldsById[id];
      if (field == null) return undefined;
      statement = idx.value?.statementsById[field.parent?.id];
    }
    if (statement == null) return undefined;
    return idx.value?.filesById[statement.file?.id];
  }

  function pathOf(idOrCk: string, options?: { loffset?: number; roffset?: number }): string | undefined {
    let path = nodePathOf(idOrCk);
    if (path == null) return undefined;
    const loffset = options?.loffset ?? 0;
    const roffset = options?.roffset ?? 0;
    path = path.slice(loffset, path.length - roffset);
    return path?.map((e) => e.name).join(".");
  }

  function nodePathOf(idOrCk?: string): NodeBase[] | undefined {
    // get all ancestors of file or statement
    if (idOrCk == null) return undefined;
    const id = idx.value?.idByCk[idOrCk] ?? idOrCk;
    const statement = idx.value?.statementsById[id];
    if (statement != null) {
      const parentPath = nodePathOf(statement.parent?.id);
      if (parentPath == null) return undefined;
      return [...parentPath, statement as NodeBase];
    }
    const file = idx.value?.filesById[id];
    if (file != null) {
      const parentPath = nodePathOf(file.parent?.id);
      return [...(parentPath ?? []), file as NodeBase];
    }
    const field = idx.value?.fieldsById[id];
    if (field != null) {
      const parentPath = nodePathOf(field.parent?.id);
      if (parentPath == null) return undefined;
      return [...parentPath, field as NodeBase];
    }
    return undefined;
  }

  function nodeOf(idOrCk: string): NodeBase | undefined {
    const id = idx.value?.idByCk[idOrCk] ?? idOrCk;
    const statement = idx.value?.statementsById[id];
    if (statement != null) return statement as NodeBase;
    const file = idx.value?.filesById[id];
    if (file != null) return file as NodeBase;
    const field = idx.value?.fieldsById[id];
    if (field != null) return field as NodeBase;
    return undefined;
  }

  function contextOf(idOrCk: string) {
    for (const someIdx of [idx.value, ...dependenciesIndex.value]) {
      if (someIdx && (idOrCk in someIdx.statementsById || idOrCk in someIdx.filesById)) {
        const id = someIdx.idByCk[idOrCk] ?? idOrCk;
        const statementRef = someIdx.statementsById[id];
        return {
          id: someIdx.id,
          name: someIdx.name,
          path: someIdx.path,
          file: someIdx.filesById[statementRef.file?.id],
          statement: statementRef,
        };
      }
    }
    return undefined;
  }

  function statementOf(idOrCk: string) {
    if (idOrCk == undefined) return;
    for (const someIdx of [idx.value, ...dependenciesIndex.value]) {
      if (someIdx && (idOrCk in someIdx.statementsById || idOrCk in someIdx.idByCk)) {
        return someIdx.statementsById[someIdx.idByCk[idOrCk] ?? idOrCk];
      }
    }
    return undefined;
  }

  function fieldOf(idOrCk: string) {
    if (idOrCk == undefined) return;
    for (const someIdx of [idx.value, ...dependenciesIndex.value]) {
      if (someIdx && (idOrCk in someIdx.fieldsById || idOrCk in someIdx.idByCk)) {
        return someIdx.fieldsById[someIdx.idByCk[idOrCk] ?? idOrCk];
      }
    }
    return undefined;
  }

  function relativePath(from_: InterpStatement, to_: InterpStatement) {
    const from = contextOf(from_?.id);
    const to = contextOf(to_?.id);
    if (!from || !to) {
      return undefined;
    } else if (from.path == to.path) {
      return `.${to.file.name}`;
    } else {
      return `${to.path}.${to.file.name}`;
    }
  }

  function issuesOfRef(node: Ref<{ id: string }>) {
    return computed(() => issues.value?.filter((e) => e.parent?.id == node.value.id));
  }

  function issuesOf(node: { id: string }, filter?: { kind: IssueKind }): Issue[] {
    return issues.value?.filter((e) => e.parent?.id == node.id && (filter == null || e.kind == filter.kind)) ?? [];
  }

  function issuesIn(node: { id: string }, filter?: { kind: IssueKind }): Issue[] {
    const descendants = descendantsOf(node.id);
    return issues.value?.filter(
      (e) => descendants.find((d) => d.id == e.parent?.id) != null && (filter == null || e.kind == filter.kind)
    );
  }

  const allStatements = computed(() => {
    const allStatements = Object.values(idx.value?.statementsById ?? {});
    for (const dependencyIndex of dependenciesIndex.value) {
      allStatements.push(...Object.values(dependencyIndex.statementsById));
    }
    return allStatements;
  });
  const namedStatements = computed(() => {
    return Object.values(idx.value?.statementsById ?? {}).filter((s) => s.name != null && s.name.length > 0);
  });
  const allFiles = computed(() => {
    const allFiles = Object.values(idx.value?.filesById ?? {});
    for (const dependencyIndex of dependenciesIndex.value) {
      allFiles.push(...Object.values(dependencyIndex.filesById));
    }
    return allFiles;
  });
  const namedFiles = computed(() => {
    return Object.values(idx.value?.filesById ?? {}).filter((f) => f.name != null && f.name.length > 0);
  });

  function statementsLike(filter: Ref<StatementFilter> | StatementFilter) {
    const filterRef = isRef(filter) ? filter : ref(filter);
    const statements = computed(() => {
      if (!idx.value) {
        return [];
      }
      const statements = filterRef.value?.includeDependencies
        ? allStatements.value
        : Object.values(idx.value.statementsById);
      return statements.filter((s) => {
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
    const tagsByKey: GRecord<string, InterpStatement> = {};
    for (const tag of tags.value) {
      tagsByKey[tag.key as string] = tag;
    }
    return tagsByKey;
  });

  function descendantsOf(idOrCk: string): Array<InterpStatement | InterpFile> {
    const id = idx.value?.idByCk[idOrCk] ?? idOrCk;
    const statement = idx.value?.statementsById[id];
    if (statement == null) {
      const file = idx.value?.filesById[id];
      if (file == null) return [];
      return idx.value?.statementsByFileId[file.id].flatMap((s) => descendantsOf(s.id)) ?? [];
    }
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

  function getTypedKey(field: Pick<Field, "key" | "tag" | "hint" | "flags" | "referenceCk" | "value">) {
    // TODO :Performance: cache getTypedKey (esp. when without references & value)
    let tag = field.tag;
    if (field.tag == TypeTag.TypeReference) {
      const reference = statementOf(field.referenceCk);
      if (reference == null) return null;
      tag = STATEMENT_TYPE_TAGS[reference.type] as TypeTag;
    }
    const storageFormat = getStorageFormat(tag, field.hint ?? undefined, field.flags);
    let typedKey: string;
    if (tag == TypeTag.Vector) {
      const dimension = field.value?.dimension ?? DEFAULT_EMBEDDING_DIMENSION;
      typedKey = `${field.key}-${storageFormat}${dimension}`;
    } else {
      typedKey = `${field.key}-${storageFormat}`;
    }
    if (field.flags & TypeFlag.IS_ARRAY || field.flags & TypeFlag.IS_ARRAYABLE) {
      typedKey += "-arr";
    }
    return typedKey;
  }

  function effectiveTypeOf(field: Field): Field {
    if (field.tag == TypeTag.TypeReference) {
      const reference = statementOf(field.referenceCk);
      if (reference == null) return field;
      return { ...field, tag: STATEMENT_TYPE_TAGS[reference.type] as TypeTag };
    } else {
      return field;
    }
  }

  // wake runtime as needed if possible
  const wokeRuntime = ref(false);
  const ops = useOperations();
  const auth = useAuth();
  watch([module, WS_CONNECTED, () => auth.loggedIn.value], async () => {
    if (!WS_CONNECTED.value) {
      wokeRuntime.value = false; // reset woken state
    }
    if (
      WS_CONNECTED.value &&
      !wokeRuntime.value &&
      moduleOrProjectId.value != null &&
      module.value != null &&
      !module.value?.module?.committed &&
      auth.loggedIn.value
    ) {
      wokeRuntime.value = true;
      await ops.session.wakeRuntime(moduleOrProjectId.value);
    }
  });

  return {
    loading: computed(() => loading.value || moduleOrProjectId.value == null),
    module,
    modules: computed(() => [idx.value, ...dependenciesIndex.value]),
    id: computed(() => module.value?.module?.id),
    name: computed(() => module.value?.module?.project.name),
    path: computed(() => module.value?.module?.project.path),
    issues,
    errors,
    warnings,
    notices,
    idx,
    dependenciesIndex,
    // module
    fileOf,
    pathOf,
    contextOf,
    fieldOf,
    statementOf,
    nodeOf,
    nodePathOf,
    relativePath,
    getTypedKey,
    effectiveTypeOf,
    issuesOfRef,
    issuesOf,
    issuesIn,
    statementsLike,
    allStatements,
    allFiles,
    namedStatements,
    namedFiles,
    tags,
    tagsByKey,
    getDescendantsOf: descendantsOf,
    // utils
    defaultLibs,
    runMetadataFields,
    runMetadataKey,
    taskRunConfigFields,
    taskRunConfigKey,
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

type OrderableStatement = Pick<InterpStatement, "id" | "ck" | "orderKey" | "type" | "parent">;
export type OrderedStatement<T extends OrderableStatement> = {
  id: string;
  ck: string;
  depth: number;
  renderedDepth: number;
  ancestors: string[];
  statement: T;
  // TODO @UX :Architecture: statement grouping/nesting should happen on component level
  //  but that would require somehow managing component instances manually (because of the tree nesting).
  //  This is also why nested grouping is currently broken. :NestedStatementRendering
  isGroupStart?: boolean;
  isGroupMiddle?: boolean;
  isGroupEnd?: boolean;
};

export function orderStatements<T extends OrderableStatement>(statements: T[]) {
  if (statements.length == 0) return { ordered: [], statementsByParentId: {} };
  const ordered: OrderedStatement<T>[] = [];
  const statementsById: GRecord<string, T> = {};
  const statementsByParentId: GRecord<string, T[]> = {};
  // group by parent
  statements.forEach((statement) => {
    statementsById[statement.id] = statement;
    if (statementsByParentId[statement.parent?.id] != null) {
      statementsByParentId[statement.parent?.id].push(statement);
    } else {
      statementsByParentId[statement.parent?.id] = [statement];
    }
  });
  // walk from root
  function walkDfs(parentId: string, depth: number, renderedDepth: number, ancestors: string[]) {
    const isParentGroup = statementsById[parentId]?.type == StatementType.Group;
    const children = statementsByParentId[parentId];
    if (children) {
      if (isParentGroup) {
        ordered[ordered.length - 1].isGroupStart = true;
      }
      children.sort((a, b) => (a.orderKey > b.orderKey ? 1 : -1));
      for (const child of children) {
        ordered.push({
          id: child.id,
          ck: child.ck,
          depth: depth,
          renderedDepth: renderedDepth,
          ancestors: ancestors,
          statement: child,
          isGroupMiddle: renderedDepth < depth,
        });
        const isChildGroup = statementsById[child.id]?.type == StatementType.Group;
        walkDfs(child.id, depth + 1, renderedDepth + (isChildGroup ? 0 : 1), [...ancestors, child.id]);
      }
      if (isParentGroup) {
        ordered[ordered.length - 1].isGroupEnd = true;
      }
    }
  }
  const fileId = statements.find((s) => s.parent?.__typename == "File")?.parent?.id; // assumes all statements are from the same file
  walkDfs(fileId, 0, 0, []);
  return { ordered, statementsByParentId };
}

export type StatementFilter = {
  types?: StatementType[];
  includeAnonymous?: boolean;
  includeDependencies?: boolean;
};

export function useNavigation() {
  const bench = useBenchState();
  const module = useCurrentModule();

  function focusStatement(statement: { id: string }) {
    const context = module.contextOf(statement.id);
    if (!context?.file) {
      console.warn(`no context found for statement ${statement.id}`, statement);
      return;
    }
    // can't focus external modules yet
    if (context.id != bench.projectVersionId) return;
    const panel = bench.focusFile(context.file as any) as EditFilePanel;
    panel.editElement(statement as any);
  }

  function focusFile(file: { id: string }) {
    const file_ = module.fileOf(file.id);
    if (file_ == null) return;
    bench.focusFile(file_ as NodeBase);
  }

  return { focusStatement, focusFile };
}

export function newRunId(): string {
  const nodeId = uuidv4();
  return btoa(`Run:${nodeId}`);
}

export function newSessionId(): string {
  const nodeId = uuidv4();
  return btoa(`Session:${nodeId}`);
}

export function mergeNodePaths(a: NodeBase[], b: NodeBase[]): NodeBase[] {
  // merge b into (just find first common ancestor)
  const commonAncestor = a.findIndex((e) => b.find((e2) => e2.id == e.id) != null);
  if (commonAncestor == -1) return a;
  return [...a.slice(0, commonAncestor), ...b];
}

// :ModuleLimits
export const MODULE_RECORD_LIMIT = 25_000;
export const DATABASE_VERSIONED_RECORD_LIMIT = 2_500;
export const DATABASE_GENERAL_RECORD_LIMIT = 10_000_000;
