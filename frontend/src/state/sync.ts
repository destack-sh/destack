import { graphql } from "@/gql";
import type { ModuleMutation, ModuleMutationType } from "@/gql/graphql";
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
  useMutation,
  useSubscription,
  type UseMutationOptions,
  type UseMutationReturn,
  useApolloClient,
} from "@vue/apollo-composable";
import { print, parse } from "graphql";
import { computed, watch, type Ref } from "vue";

export const PENDING_REVISION = -1;

type MutationOp = {
  type: ModuleMutationType; // later other change types will be supported
  name: string;
  fragment: DocumentNode | TypedDocumentNode;
  optimisticResponse: (vars: any) => any;
  updateCache?: MutationUpdaterFunction<any, any, any, any>;
};

function applyOpLocally(client: ApolloClient<any>, op: MutationOp, vars: any, revision: number | null) {
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

export class OpRegistry {
  public ops: Partial<Record<ModuleMutationType, MutationOp>> = {};

  public merge(registry: OpRegistry) {
    for (const [type, op] of Object.entries(registry.ops)) {
      // error if already registered
      if (this.ops[type as ModuleMutationType] != null) {
        throw new Error(`type ${type} already registered`);
      }
      this.ops[type as ModuleMutationType] = op;
    }
  }

  static mergeAll(registries: OpRegistry[]) {
    const registry = new OpRegistry();
    for (const r of registries) {
      registry.merge(r);
    }
    return registry;
  }

  public useMutation<TResult = any, TVariables extends OperationVariables = OperationVariables>(
    type: ModuleMutationType,
    document: DocumentParameter<TResult, TVariables>,
    options?: OptionsParameter<TResult, TVariables>
  ): UseMutationReturn<TResult, TVariables> {
    /* Registers a mutation for multiplayer  */

    const operationName = document.definitions[0].name.value; // fails if op could not be parsed, but this is usually obvious
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
    const op: MutationOp = {
      type,
      name: operationName,
      fragment: parse(fragment),
      optimisticResponse: options?.optimisticResponse as (vars: any) => any,
      updateCache: options?.update,
    };
    this.ops[type] = op;

    return useMutation(document, options);
  }
}

export function useModuleSync(projectVersionId: Ref<string | null>) {
  projectVersionId = toValueRef(projectVersionId);
  const {
    onResult: onModuleChanged,
    start,
    stop,
  } = useSubscription(
    graphql(/* GraphQL */ `
      subscription moduleChanged($projectVersionId: GlobalID!) {
        moduleChanged(projectVersionId: $projectVersionId) {
          id
          clientId
          mutations {
            type
            fileId
            statementId
            revision
            input
          }
        }
      }
    `),
    {
      projectVersionId,
    }
  );
  // enable/disable subscription when projectVersionId changes
  startStopIf(
    computed(() => projectVersionId.value != null),
    start,
    stop,
    { immediate: true }
  );

  const syncedOps = useSyncedOps();
  onModuleChanged((result) => {
    if (result.data?.moduleChanged != null) {
      console.debug("accept sync change", result.data?.moduleChanged.clientId);
      // apply all mutations
      for (const mutation of result.data.moduleChanged.mutations) {
        syncedOps.applyMutation(mutation);
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
  onProjectChanged((result) => {
    // just reload versions query for now?
    client.client.refetchQueries({
      include: ["projectVersions"],
    });
  });
}

function useSyncedOps() {
  const ops = useOperations();
  const { client } = useApolloClient();
  const opRegistry = OpRegistry.mergeAll([ops.statement.registry, ops.file.registry, ops.symbol.registry]);

  function applyMutation(mutation: Pick<ModuleMutation, "type" | "fileId" | "statementId" | "revision" | "input">) {
    const registeredOp = opRegistry.ops[mutation.type];
    if (registeredOp == null) {
      throw new Error(`cannt apply unknown: ${mutation.type}`);
    }
    console.debug("apply sync mutation", mutation);
    applyOpLocally(client, registeredOp, mutation.input, mutation.revision as number | null);
  }

  return { applyMutation };
}
