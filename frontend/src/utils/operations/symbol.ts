import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSymbolOps() {
  const operations = useOperationsStore();

  const { mutate: renameSymbolMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameSymbol($id: GlobalID!, $name: String!) {
        renameSymbol(input: { id: $id, name: $name }) {
          ... on Symbol {
            id
            name
            typeNameDeclaration
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function rename(id: string, oldName: string, newName: string) {
    await operations.perform({
      type: "rename-symbol",
      do: async () => {
        await renameSymbolMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameSymbolMut({ id: id, name: oldName });
      },
    });
  }

  return { rename };
}
