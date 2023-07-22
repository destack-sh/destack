import { graphql } from "@/gql";
import { useBenchState } from "@/state/bench";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSessionOps() {
  const bench = useBenchState();
  const ops = useOperationsStore();

  const { mutate: wakeLangserverMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation wakeLangserver($projectVersionId: GlobalID!) {
        wakeLangserver(input: { projectVersionId: $projectVersionId }) {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function wakeLangserver(moduleId: string) {
    return await ops.perform({
      type: "runtime.wake",
      stateless: true,
      do: async () => {
        return await wakeLangserverMut({
          projectVersionId: moduleId,
        });
      },
    });
  }

  const { mutate: wakeWorkerSetMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation wakeWorkerSet($projectId: GlobalID!) {
        wakeWorkerSet(input: { projectId: $projectId }) {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function wakeWorkerSet(projectId: string) {
    return await ops.perform({
      type: "runtime.wake",
      stateless: true,
      do: async () => {
        return await wakeWorkerSetMut({
          projectId,
        });
      },
    });
  }

  const { mutate: startRunMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation startRun(
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
            error
            run {
              ...RunContent
            }
            logs {
              ...LogEntryContent
            }
          }
        }
      }
    `)
  );

  async function run(
    runnableId: string,
    runId?: string,
    sessionId?: string,
    arguments_?: Record<string, any>,
    options?: { block?: boolean; keyed?: boolean; timeoutSeconds?: number }
  ) {
    return await ops.perform({
      type: "runtime.run",
      key: runnableId,
      stateless: true,
      do: async () => {
        return await startRunMut({
          projectVersionId: bench.projectVersionId,
          runnableId,
          runId,
          sessionId,
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
    wakeLangserver,
    wakeWorkerSet,
    run,
    cancel,
  };
}
