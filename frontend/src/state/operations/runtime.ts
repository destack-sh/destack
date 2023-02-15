import { graphql } from "@/gql";
import { useEditorState } from "@/state/editor";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useRuntimeOps() {
  const editor = useEditorState();
  const operations = useOperationsStore();

  const { mutate: buildMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation build($projectVersionId: GlobalID!, $buildableId: GlobalID) {
        build(input: { projectVersionId: $projectVersionId, buildableId: $buildableId }) {
          ... on BuildState {
            projectVersionId
            success
            buildIds
          }
        }
      }
    `)
  );

  async function build(buildableId?: string) {
    return await operations.perform({
      type: "runtime.build",
      stateless: true,
      do: async () => {
        await buildMut({
          projectVersionId: editor.currentProjectVersionId,
          buildableId,
        });
      },
    });
  }

  const { mutate: runMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation run($projectVersionId: GlobalID!, $runnableId: GlobalID, $buildId: GlobalID, $arguments: JSON!) {
        run(
          input: {
            projectVersionId: $projectVersionId
            runnableId: $runnableId
            buildId: $buildId
            arguments: $arguments
          }
        ) {
          ... on RunState {
            projectVersionId
            runnableId
            buildId
            output
            success
          }
        }
      }
    `)
  );

  async function run(runnableId?: string, buildId?: string, arguments_?: Record<string, any>) {
    return await operations.perform({
      type: "runtime.run",
      stateless: true,
      do: async () => {
        return await runMut({
          projectVersionId: editor.currentProjectVersionId,
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
