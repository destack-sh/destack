import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useStatementOps() {
  const operations = useOperationsStore();

  const { mutate: moveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int!) {
        moveStatement(input: { id: $id, fileId: $fileId, parentId: $parentId, index: $index }) {
          statement {
            id
            index
            file {
              id
              path
            }
            parent {
              id
            }
          }
          oldFile {
            id
            path
            statements {
              id
              index
            }
          }
          newFile {
            id
            path
            statements {
              id
              index
            }
          }
        }
      }
    `)
  );

  async function move(
    id: string,
    oldLoc: { fileId: string; parentId?: string; index: number },
    newLoc: { fileId: string; parentId?: string; index: number }
  ) {
    await operations.perform({
      type: "statement.move",
      do: async () => {
        await moveStatementMut({
          id: id,
          fileId: newLoc.fileId,
          parentId: newLoc.parentId,
          index: newLoc.index,
        });
      },
      undo: async () => {
        await moveStatementMut({
          id: id,
          fileId: oldLoc.fileId,
          parentId: oldLoc.parentId,
          index: oldLoc.index,
        });
      },
    });
  }

  return { move };
}
