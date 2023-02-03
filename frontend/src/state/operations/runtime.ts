import { graphql } from "@/gql";
import { useEditorState } from "@/state/editor";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useRuntimeOps() {
  const editor = useEditorState();
  const operations = useOperationsStore();

  const { mutate: compileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation compile($projectVersionId: GlobalID!, $compilationId: GlobalID!) {
        compile(input: { projectVersionId: $projectVersionId, compilationId: $compilationId }) {
          ... on CompileState {
            success
          }
        }
      }
    `)
  );

  async function compile(compilationId: string) {
    return await operations.perform({
      type: "compile",
      do: async () => {
        await compileMut({
          projectVersionId: editor.currentProjectVersionId,
          compilationId,
        });
      },
    });
  }

  // const { mutate: runMut } = useMutation(
  //   graphql(/* GraphQL */ `
  //     # mutation run($projectVersionId: GlobalID!, $runconfigId: GlobalID!, arguments: JSON!) {
  //     #   run(input: { projectVersionId: $projectVersionId, runconfigId: $runconfigId, arguments: $arguments }) {
  //     #     ... on RunState {
  //     #       success
  //     #       output
  //     #     }
  //     #   }
  //     # }
  //   `)
  // );

  return {
    compile,
  };
}
