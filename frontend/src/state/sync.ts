import { graphql } from "@/gql";
import { EditType, type Edit, type ResolvedField } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { dedent, startStopIf, toValueRef } from "@/utils/functools";
import type {
  ApolloClient,
  DocumentNode,
  MutationUpdaterFunction,
  OperationVariables,
  TypedDocumentNode,
} from "@apollo/client";
import {
  useApolloClient,
  useMutation,
  useSubscription,
  type UseMutationOptions,
  type UseMutationReturn,
} from "@vue/apollo-composable";
import { parse, print } from "graphql";
import { computed, onUnmounted, type Ref } from "vue";

export const PENDING_REVISION = -1;

type GqlMutation = {
  type: EditType; // later other change types will be supported
  name: string;
  fragment: DocumentNode | TypedDocumentNode;
  optimisticResponse: (vars: any) => any;
  updateCache?: MutationUpdaterFunction<any, any, any, any>;
};

function applyGqlMutation(client: ApolloClient<any>, op: GqlMutation, vars: any, revision: number | null) {
  /* Apply the mutation operation */

  // first, get the expected response for the input vars
  const expectedResponse = op.optimisticResponse(vars);
  const mutatedThing = expectedResponse[op.name];
  if (mutatedThing == null) {
    throw new Error(`expected response is missing ${op.name}: ${JSON.stringify(expectedResponse)}`);
  }
  if (revision != null) {
    // set revision (since optimistic updates set it to pending)
    mutatedThing.revision = revision;
  }
  // write the response fragment
  client.writeFragment({
    fragment: op.fragment,
    data: mutatedThing,
  });
  // update cache if needed
  if (op.updateCache != null) {
    op.updateCache(client.cache, { data: { [op.name]: mutatedThing, synced: true } }, {});
  }
}

// borrowed and trimmed from non-exported vue/apollo-composable file
declare type DocumentParameter<TResult, TVariables> = DocumentNode | TypedDocumentNode<TResult, TVariables>;
declare type OptionsParameter<TResult, TVariables> = UseMutationOptions<TResult, TVariables>;

export class EditRegistry {
  public edits: Partial<Record<EditType, GqlMutation>> = {};

  public merge(registry: EditRegistry) {
    for (const [type, op] of Object.entries(registry.edits)) {
      // error if already registered
      if (this.edits[type as EditType] != null) {
        throw new Error(`type ${type} already registered`);
      }
      this.edits[type as EditType] = op;
    }
  }

  static mergeAll(registries: EditRegistry[]) {
    const registry = new EditRegistry();
    for (const r of registries) {
      registry.merge(r);
    }
    return registry;
  }

  // TODO! @Cleanup @Architecture: module edits have substantial, opaque redundancy :BE-114
  //  For one, optimistic responses and even cache updates could be auto-generated with some relatively simple rules.
  //  Also, because the underline edits are currently 'opaque' to the operations system ('ops.perform(...)'), we can't simply
  //  collect and batch multiple edits without sending them off. This further prevents any reasonable offline support,
  //  and, coincidentally, makes it hard to walk a module node (e.g. to bump up on change, delete down).
  //
  //  We probably want to keep GQL edits (many reasons; they're nicely typed, debuggable and optimistic, auto-multiplayer, etc.),
  //  but we can simplify multiplayer module edits - some thoughts on a potential refactor:
  //   1. useEdit passes in only the graphql edit/fragment (no optimistic response or cache update)
  //   2. update operation store Operation to also accept a set of native GQL edit objects (somehow)
  //   3. have a global way to collect edits instead of sending them immediately (for offline, also for client-side 'transactions')
  public defineEdit<TResult = any, TVariables extends OperationVariables = OperationVariables>(
    type: EditType,
    document: DocumentParameter<TResult, TVariables>,
    options?: OptionsParameter<TResult, TVariables>
  ): UseMutationReturn<TResult, TVariables> {
    /* Registers a GQL mutation as a multiplayer edit  */

    const operationName = (document.definitions[0] as any)?.name?.value; // fails if op could not be parsed, but this is usually obvious
    if (operationName == null) {
      throw new Error(`could not parse operation name: ${type}`);
    }
    const mutationString = print(document);

    // get the string between '... on' and '...OperationInfoContent'
    const mutationFragment = mutationString.match(/(?<=\.\.\. on )(.*)(?=\.\.\.OperationInfoContent)/s)?.[0];
    if (mutationFragment == null) {
      throw new Error(`could not parse mutation fragment: ${type}`);
    }
    const fragment = `fragment __${operationName} on ${dedent(mutationFragment, 2)}`;

    if (options?.optimisticResponse == null || typeof options?.optimisticResponse != "function") {
      throw new Error(`optimisticResponse function is required: ${type}`);
    }
    const op: GqlMutation = {
      type,
      name: operationName,
      fragment: parse(fragment),
      optimisticResponse: options?.optimisticResponse as (vars: any) => any,
      updateCache: options?.update,
    };
    this.edits[type] = op;

    return useMutation(document, options);
  }
}

const editListeners: Partial<Record<string, ((edit: Edit) => void)[]>> = {};

function getEditKey(mutation: Pick<Edit, "type" | "statementId">) {
  return `${mutation.type}:${mutation.statementId}`;
}

export function useEditListener(types: EditType[], statementId: string, listener: (edit: Edit) => void) {
  for (const type of types) {
    const key = getEditKey({ type, statementId });
    if (editListeners[key] == null) {
      editListeners[key] = [];
    }
    editListeners[key]?.push(listener);
  }
  onUnmounted(() => {
    for (const type of types) {
      const key = getEditKey({ type, statementId });
      editListeners[key] = editListeners[key]?.filter((l) => l != listener);
    }
  });
}

export function useModuleSync(projectId: Ref<string | null>, projectVersionId: Ref<string | null>) {
  projectVersionId = toValueRef(projectVersionId);
  const {
    onResult: onModuleChanged,
    start,
    stop,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription moduleChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {
        moduleChanged(projectId: $projectId, projectVersionId: $projectVersionId) {
          id
          clientId
          edits {
            type
            fileId
            statementId
            revision
            input
            data {
              ... on ResolvedField {
                id
                ck
                fieldCk
              }
              ... on Issue {
                ...IssueContent
              }
            }
          }
        }
      }
    `),
    {
      projectId,
      projectVersionId,
    }
  );
  // TODO @Performance: module change objects should not be cached
  // enable/disable subscription when projectVersionId changes
  startStopIf(
    computed(() => projectId.value != null && projectVersionId.value != null),
    start,
    stop,
    { immediate: true }
  );

  const syncedOps = useSyncedOps();
  onModuleChanged((result) => {
    if (result.data?.moduleChanged != null) {
      console.debug("accept sync change", result.data?.moduleChanged.clientId);
      // apply all edits
      for (const edit of result.data.moduleChanged.edits) {
        if (edit.input != null) {
          // apply like regular input op
          syncedOps.applyApiEdit(edit);
        } else {
          // apply manually
          syncedOps.applyRawEdit(edit as Edit);
        }
        // TODO @Broken: cascade edits into relevant bumps
        const key = getEditKey(edit);
        for (const listener of editListeners[key] ?? []) {
          listener(edit as Edit);
        }
      }
    }
  });
}

export function useProjectSync(projectId: Ref<string | null>) {
  projectId = toValueRef(projectId);
  const {
    onResult: onProjectChanged,
    start,
    stop,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription projectChanged($projectId: GlobalID!) {
        projectChanged(projectId: $projectId) {
          id
          clientId
        }
      }
    `),
    {
      projectId,
    }
  );
  // enable/disable subscription when projectId changes
  startStopIf(
    computed(() => projectId.value != null),
    start,
    stop,
    { immediate: true }
  );

  const client = useApolloClient();
  onProjectChanged(() => {
    // just reload versions query for now?
    client.client.refetchQueries({
      include: ["projectVersions"],
    });
  });
}

function useSyncedOps() {
  const ops = useOperations();
  const { client } = useApolloClient();
  const opRegistry = EditRegistry.mergeAll([ops.statement.registry, ops.file.registry, ops.symbol.registry]);

  function applyApiEdit(edit: Pick<Edit, "type" | "fileId" | "statementId" | "revision" | "input">) {
    // edits that we just pass through to the regular op with the original input
    const registeredOp = opRegistry.edits[edit.type];
    if (registeredOp == null) {
      console.warn(`cannot apply unknown input edit: ${edit.type}`);
      return;
    }
    console.debug("apply api sync edit", edit);
    applyGqlMutation(client, registeredOp, edit.input, edit.revision as number | null);
  }

  function applyRawEdit(edit: Pick<Edit, "type" | "fileId" | "statementId" | "data">) {
    console.debug("apply raw sync edit", edit);
    // manual edits (when we don't have a registered op from a standard GQL mutation), will be overhauled soon with edits 2.0
    // map database edits to bumps
    if (edit.type == EditType.TruncateResolvedFields) {
      if (edit.statementId != null) {
        client.cache.modify({
          id: `Statement:${edit.statementId}`,
          fields: {
            resolvedFields(existingResolvedFields = []) {
              return [];
            },
          },
        });
      } else {
        const allStatements = client.cache.extract(true);
        Object.keys(allStatements).forEach((key) => {
          if (key.startsWith("Statement")) {
            client.cache.evict({ id: key, fieldName: "resolvedFields" });
          }
        });
        client.cache.gc();
      }
    } else if (edit.type == EditType.CreateResolvedField && edit.statementId != null) {
      client.cache.modify({
        id: `Statement:${edit.statementId}`,
        fields: {
          resolvedFields(existingResolvedFields = []) {
            const resolvedField = edit.data as ResolvedField;
            return [
              ...existingResolvedFields.filter((r: any) => r.id != resolvedField.id),
              {
                __typename: "ResolvedField",
                id: resolvedField.id,
                ck: resolvedField.ck,
                statement: { __ref: `Statement:${edit.statementId}` },
                fieldCk: resolvedField.fieldCk,
              },
            ];
          },
        },
      });
    } else if (edit.type == EditType.DeleteResolvedField && edit.statementId != null) {
      client.cache.modify({
        id: `Statement:${edit.statementId}`,
        fields: {
          resolvedFields(existingResolvedFields = []) {
            const resolvedField = edit.data;
            return existingResolvedFields.filter(
              (r: any) => r.id != resolvedField?.id && r.__ref != `ResolvedField:${resolvedField?.id}`
            );
          },
        },
      });
    } else if (edit.type == EditType.TruncateIssues) {
      if (edit.statementId != null) {
        client.cache.modify({
          id: `Statement:${edit.statementId}`,
          fields: {
            issues(existingIssues = []) {
              return [];
            },
          },
        });
      } else if (edit.fileId != null) {
        client.cache.modify({
          id: `File:${edit.fileId}`,
          fields: {
            issues(existingIssues = []) {
              return [];
            },
          },
        });
      } else {
        const allStatements = client.cache.extract(true);
        Object.keys(allStatements).forEach((key) => {
          if (key.startsWith("Statement") || key.startsWith("File") || key.startsWith("module")) {
            client.cache.evict({ id: key, fieldName: "issues" });
          }
        });
        client.cache.gc();
      }
    } else if (edit.type == EditType.CreateIssue && edit.statementId != null) {
      client.cache.modify({
        id: `Statement:${edit.statementId}`,
        fields: {
          issues(existingIssues = []) {
            return [...existingIssues.filter((i: any) => i.id != edit.data?.id), edit.data];
          },
        },
      });
    } else if (edit.type == EditType.CreateIssue && edit.fileId != null) {
      client.cache.modify({
        id: `File:${edit.fileId}`,
        fields: {
          issues(existingIssues = []) {
            return [...existingIssues.filter((i: any) => i.id != edit.data?.id), edit.data];
          },
        },
      });
    } else if (edit.type == EditType.DeleteIssue && edit.statementId != null) {
      client.cache.modify({
        id: `Statement:${edit.statementId}`,
        fields: {
          issues(existingIssues = []) {
            const issue = edit.data;
            return existingIssues.filter((i: any) => i.id != issue?.id && i.__ref != `Issue:${issue?.id}`);
          },
        },
      });
    } else if (edit.type == EditType.DeleteIssue && edit.fileId != null) {
      client.cache.modify({
        id: `File:${edit.fileId}`,
        fields: {
          issues(existingIssues = []) {
            const issue = edit.data;
            return existingIssues.filter((i: any) => i.id != issue?.id && i.__ref != `Issue:${issue?.id}`);
          },
        },
      });
    } else {
      console.warn(`cannot apply unknown data edit: ${edit.type} ${edit.data}`);
    }
  }

  return { applyApiEdit, applyRawEdit };
}
