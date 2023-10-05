import { graphql } from "@/gql";
import { useBenchState } from "@/state/bench";
import { useOperationsStore } from "@/state/operations";
import { SessionAccessLevel } from "@/state/session";
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
        $statementId: GlobalID
        $scopeCk: UUID
        $code: String
        $runId: GlobalID
        $sessionId: GlobalID
        $inputs: JSON
        $keyed: Boolean
        $block: Float
        $timeoutSeconds: Int
        $rootValue: JSON
        $globalValue: JSON
        $accessLevel: Int!
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            statementId: $statementId
            scopeCk: $scopeCk
            code: $code
            runId: $runId
            sessionId: $sessionId
            inputs: $inputs
            keyed: $keyed
            block: $block
            timeoutSeconds: $timeoutSeconds
            rootValue: $rootValue
            globalValue: $globalValue
            accessLevel: $accessLevel
          }
        ) {
          ... on RunState {
            projectVersionId
            statementId
            success
            error
            run {
              ...RunContent
            }
            logs {
              ...LogEntryContent
            }
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function run(
    statementId?: string,
    scopeCk?: string,
    code?: string,
    runId?: string,
    sessionId?: string,
    inputs?: Record<string, any>,
    options?: {
      block?: number;
      keyed?: boolean;
      timeoutSeconds?: number;
      rootValue?: any;
      globalValue?: any;
      accessLevel?: number;
    }
  ) {
    return await ops.perform({
      type: "runtime.run",
      key: statementId,
      stateless: true,
      do: async () => {
        return await startRunMut({
          projectVersionId: bench.projectVersionId,
          statementId,
          scopeCk,
          code,
          runId,
          sessionId,
          inputs: inputs,
          rootValue: options?.rootValue,
          globalValue: options?.globalValue,
          keyed: options?.keyed,
          block: options?.block,
          timeoutSeconds: options?.timeoutSeconds,
          accessLevel: options?.accessLevel ?? SessionAccessLevel.Read,
        });
      },
    });
  }

  const { mutate: killMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation kill($projectVersionId: GlobalID!, $runId: GlobalID!, $restartIfUnresponsive: Boolean!) {
        killRun(
          input: { projectVersionId: $projectVersionId, runId: $runId, restartIfUnresponsive: $restartIfUnresponsive }
        ) {
          ... on KillRunPayload {
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

  async function kill(runId: string, restartIfUnresponsive: boolean) {
    return await ops.perform({
      type: "runtime.kill",
      key: runId,
      stateless: true,
      do: async () => {
        return await killMut({
          projectVersionId: bench.projectVersionId,
          restartIfUnresponsive,
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
    kill,
  };
}
