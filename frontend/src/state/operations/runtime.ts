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

  async function wake(moduleId: string) {
    return await ops.perform({
      type: "runtime.wake",
      stateless: true,
      do: async () => {
        return await wakeLangserver({
          projectVersionId: moduleId,
        });
      },
    });
  }

  const { mutate: runMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation run(
        $projectVersionId: GlobalID!
        $runnableId: GlobalID
        $runId: GlobalID
        $sessionId: GlobalID
        $arguments: JSON
        $keyed: Boolean
        $block: Boolean
        $timeoutSeconds: Int
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runnableId: $runnableId
            runId: $runId
            sessionId: $sessionId
            arguments: $arguments
            keyed: $keyed
            block: $block
            timeoutSeconds: $timeoutSeconds
          }
        ) {
          ... on RunState {
            projectVersionId
            runnableId
            success
            run {
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
    runId?: string,
    arguments_?: Record<string, any>,
    options?: { block?: boolean; keyed?: boolean; timeoutSeconds?: number }
  ) {
    return await ops.perform({
      type: "runtime.run",
      key: runnableId,
      stateless: true,
      do: async () => {
        return await runMut({
          projectVersionId: bench.projectVersionId,
          runnableId,
          runId,
          arguments: arguments_,
          keyed: options?.keyed,
          block: options?.block,
          timeoutSeconds: options?.timeoutSeconds,
        });
      },
    });
  }

  const { mutate: cancelMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation cancel($projectVersionId: GlobalID!, $runId: GlobalID!) {
        cancelRun(input: { projectVersionId: $projectVersionId, runId: $runId }) {
          ... on CancelRunPayload {
            success
            run {
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

  async function cancel(runId: string) {
    return await ops.perform({
      type: "runtime.cancel",
      key: runId,
      stateless: true,
      do: async () => {
        return await cancelMut({
          projectVersionId: bench.projectVersionId,
          runId,
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
