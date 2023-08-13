import { graphql } from "@/gql";
import { useBenchState } from "@/state/bench";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSessionOps() {
  const bench = useBenchState();
  const ops = useOperationsStore();

  const { mutate: wakeRuntimeMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation wakeRuntime($projectVersionId: GlobalID!) {
        wakeRuntime(input: { projectVersionId: $projectVersionId }) {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function wakeRuntime(moduleId: string) {
    return await ops.perform({
      type: "runtime.wake",
      stateless: true,
      do: async () => {
        return await wakeRuntimeMut({
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
          ... on WakeWorkerSetPayload {
            success
          }
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

  const { mutate: restartWorkerSetMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restartWorkerSet($projectId: GlobalID!) {
        restartWorkerSet(input: { projectId: $projectId }) {
          ...OperationInfoContent
          ... on RestartWorkerSetPayload {
            success
          }
        }
      }
    `)
  );

  async function restartWorkerSet(projectId: string) {
    return await ops.perform({
      type: "runtime.restart",
      stateless: true,
      do: async () => {
        return await restartWorkerSetMut({
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
        $inputs: JSON
        $keyed: Boolean
        $block: Float
        $timeoutSeconds: Int
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runnableId: $runnableId
            runId: $runId
            sessionId: $sessionId
            inputs: $inputs
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
    inputs?: Record<string, any>,
    options?: { block?: number; keyed?: boolean; timeoutSeconds?: number }
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
          inputs: inputs,
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
    wakeRuntime,
    wakeWorkerSet,
    restartWorkerSet,
    run,
    cancel,
  };
}
