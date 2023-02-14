import { graphql } from "@/gql";
import { useEditorState } from "@/state/editor";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useRuntimeOps() {
  const editor = useEditorState();
  const operations = useOperationsStore();

  const { mutate: buildMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation build($projectVersionId: GlobalID!, $buildId: GlobalID, $buildableId: GlobalID) {
        build(input: { projectVersionId: $projectVersionId, buildId: $buildId, buildableId: $buildableId }) {
          ... on BuildState {
            projectVersionId
            success
            buildIds
          }
        }
      }
    `)
  );

  async function build(buildId?: string, buildableId?: string) {
    return await operations.perform({
      type: "runtime.build",
      do: async () => {
        await buildMut({
          projectVersionId: editor.currentProjectVersionId,
          buildId,
          buildableId,
        });
      },
    });
  }

  const { mutate: runMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation run(
        $projectVersionId: GlobalID!
        $runconfigId: GlobalID
        $runnableId: GlobalID
        $buildId: GlobalID
        $arguments: JSON!
      ) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runconfigId: $runconfigId
            runnableId: $runnableId
            buildId: $buildId
            arguments: $arguments
          }
        ) {
          ... on RunState {
            projectVersionId
            runconfigId
            runnableId
            buildId
            output
            success
          }
        }
      }
    `)
  );

  async function run(runconfigId?: string, runnableId?: string, buildId?: string, arguments_?: Record<string, any>) {
    return await operations.perform({
      type: "runtime.run",
      do: async () => {
        return await runMut({
          projectVersionId: editor.currentProjectVersionId,
          runconfigId,
          runnableId,
          buildId,
          arguments: arguments_,
        });
      },
    });
  }

  return {
    build,
    run,
  };
}
