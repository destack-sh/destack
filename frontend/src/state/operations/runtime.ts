import { graphql } from "@/gql";
import type { BuildScope } from "@/gql/graphql";
import { useEditorState } from "@/state/editor";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useRuntimeOps() {
  const editor = useEditorState();
  const ops = useOperationsStore();

  const { mutate: buildMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation build($projectVersionId: GlobalID!, $scope: BuildScope!, $buildableId: GlobalID) {
        build(input: { projectVersionId: $projectVersionId, scope: $scope, buildableId: $buildableId }) {
          ... on BuildState {
            projectVersionId
            success
          }
        }
      }
    `)
  );

  async function build(scope: BuildScope, buildableId?: string) {
    return await ops.perform({
      type: "runtime.build",
      stateless: true,
      do: async () => {
        return await buildMut({
          projectVersionId: editor.currentProjectVersionId,
          scope,
          buildableId,
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
        $arguments: JSON
        $block: Boolean
        $timeoutSeconds: Int
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runnableId: $runnableId
            buildId: $buildId
            arguments: $arguments
            block: $block
            timeoutSeconds: $timeoutSeconds
          }
        ) {
          ... on RunState {
            projectVersionId
            runnableId
            buildId
            output
            success
            error
            errorDetails
          }
        }
      }
    `)
  );

  async function run(
    runnableId?: string,
    buildId?: string,
    arguments_?: Record<string, any>,
    options?: { block?: boolean; timeoutSeconds?: number }
  ) {
    return await ops.perform({
      type: "runtime.run",
      stateless: true,
      do: async () => {
        return await runMut({
          projectVersionId: editor.currentProjectVersionId,
          runnableId,
          buildId,
          arguments: arguments_,
          block: options?.block,
          timeoutSeconds: options?.timeoutSeconds,
        });
      },
    });
  }

  return {
    build,
    run,
  };
}
