import { graphql } from "@/gql";
import type { ModuleMutation, ModuleMutationType } from "@/gql/graphql";
import { useOperations } from "@/state/operations";
import { dedent } from "@/utils/functools";
import type { DocumentNode, MutationUpdaterFunction, OperationVariables, TypedDocumentNode } from "@apollo/client";
import { useMutation, useSubscription, type UseMutationOptions, type UseMutationReturn } from "@vue/apollo-composable";
import { print, parse } from "graphql";
import { watch, type Ref } from "vue";

export const PENDING_REVISION = -1;

type RegisteredOp = {
  type: ModuleMutationType;
  fragment: DocumentNode | TypedDocumentNode;
  optimisticResponse: (vars: any) => any;
  updateCache?: MutationUpdaterFunction<any, any, any, any>;
};

// borrowed and trimmed from non-exported vue/apollo-composable file
declare type DocumentParameter<TResult, TVariables> = DocumentNode | TypedDocumentNode<TResult, TVariables>;
declare type OptionsParameter<TResult, TVariables> = UseMutationOptions<TResult, TVariables>;

export class OpRegistry {
  public ops: Partial<Record<ModuleMutationType, RegisteredOp>> = {};

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
    /* Registers a mutation for synced application  */
    const operationName = document.definitions[0].name?.value;
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
    const op: RegisteredOp = {
      type,
      fragment: parse(fragment),
      optimisticResponse: options?.optimisticResponse as (vars: any) => any,
      updateCache: options?.update,
    };
    this.ops[type] = op;

    return useMutation(document, options);
  }
}

export function useModuleSync(projectVersionId: Ref<string | null>) {
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

  const syncedOps = useSyncedOps();
  onModuleChanged((result) => {
    if (result.data?.moduleChanged != null) {
      console.log("accept sync change", result.data?.moduleChanged.clientId);
      // apply all mutations
      for (const mutation of result.data.moduleChanged.mutations) {
        syncedOps.applyMutation(mutation);
      }
    }
  });
}

function useSyncedOps() {
  const ops = useOperations();
  const opRegistry = OpRegistry.mergeAll([ops.statement.registry, ops.file.registry, ops.symbol.registry]);

  function applyMutation(mutation: Pick<ModuleMutation, "type" | "fileId" | "statementId" | "revision" | "input">) {
    const registeredOp = opRegistry.ops[mutation.type];
    if (registeredOp == null) {
      throw new Error(`cannt apply unknown: ${mutation.type}`);
    }
    console.log("apply mutation", mutation);
  }

  return { applyMutation };
}
