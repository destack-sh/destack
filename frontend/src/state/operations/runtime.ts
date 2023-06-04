import { graphql } from "@/gql";
import { useBenchState } from "@/state/bench";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useRuntimeOps() {
  const bench = useBenchState();
  const ops = useOperationsStore();

  const { mutate: wakeLangserver } = useMutation(
    graphql(/* GraphQL */ `
      mutation wakeLangserver($projectVersionId: GlobalID!) {
        langserverWake(input: { projectVersionId: $projectVersionId }) {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function wake() {
    return await ops.perform({
      type: "runtime.wake",
      stateless: true,
      do: async () => {
        return await wakeLangserver({
          projectVersionId: bench.currentProjectVersionId,
        });
      },
    });
  }

  const { mutate: runMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation run(
        $projectVersionId: GlobalID!
        $runnableId: GlobalID
        $buildId: GlobalID
        $executionId: GlobalID
        $arguments: JSON
        $block: Boolean
        $timeoutSeconds: Int
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runnableId: $runnableId
            buildId: $buildId
            executionId: $executionId
            arguments: $arguments
            block: $block
            timeoutSeconds: $timeoutSeconds
          }
        ) {
          ... on RunState {
            projectVersionId
            runnableId
            success
            execution {
              id
              status
              startedAt
              terminatedAt
              createdAt
              updatedAt
              duration
              cachedGeneratedAt
              cachedDuration
              inputs
              outputs
              errorNice {
                type
                message
                traceback {
                  line
                  filename
                  lineno
                  name
                  locals
                }
              }
            }
          }
        }
      }
    `)
  );

  async function run(
    runnableId: string,
    buildId?: string,
    executionId?: string,
    arguments_?: Record<string, any>,
    options?: { block?: boolean; timeoutSeconds?: number }
  ) {
    return await ops.perform({
      type: "runtime.run",
      key: runnableId,
      stateless: true,
      do: async () => {
        return await runMut({
          projectVersionId: bench.currentProjectVersionId,
          runnableId,
          buildId,
          executionId,
          arguments: arguments_,
          block: options?.block,
          timeoutSeconds: options?.timeoutSeconds,
        });
      },
    });
  }

  const { mutate: cancelMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation cancel($projectVersionId: GlobalID!, $executionId: GlobalID!) {
        cancelRun(input: { projectVersionId: $projectVersionId, executionId: $executionId }) {
          ... on CancelRunPayload {
            success
            execution {
              id
              status
              startedAt
              terminatedAt
              createdAt
              updatedAt
              duration
              cachedGeneratedAt
              cachedDuration
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function cancel(executionId: string) {
    return await ops.perform({
      type: "runtime.cancel",
      key: executionId,
      stateless: true,
      do: async () => {
        return await cancelMut({
          projectVersionId: bench.currentProjectVersionId,
          executionId,
        });
      },
    });
  }

  return {
    wake,
    run,
    cancel,
  };
}
