import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useFileOps() {
  const operations = useOperationsStore();

  const { mutate: renameFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameFile($id: GlobalID!, $name: String!) {
        renameFile(input: { id: $id, name: $name }) {
          ... on File {
            id
            name
          }
        }
      }
    `)
  );

  async function rename(id: string, oldName: string, newName: string) {
    await operations.perform({
      type: "rename-file",
      apply: async () => {
        await renameFileMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameFileMut({ id: id, name: oldName });
      },
    });
  }

  return { rename };
}
