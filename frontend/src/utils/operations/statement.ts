import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useStatementOps() {
  const operations = useOperationsStore();

  const { mutate: moveStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation moveStatement($id: GlobalID!, $fileId: GlobalID!, $parentId: GlobalID, $index: Int) {
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
    oldLoc: { fileId: string; parentId?: string; index?: number },
    newLoc: { fileId: string; parentId?: string; index?: number }
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

  const { mutate: renameStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameStatement($id: GlobalID!, $name: String!) {
        renameStatement(input: { id: $id, name: $name }) {
          ... on Statement {
            id
            name
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  const { mutate: deleteStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteStatement($id: GlobalID!) {
        softDeleteStatement(input: { id: $id }) {
          statement {
            id
            deletedAt
          }
        }
      }
    `),
    {
      // refetchQueries: ["fileContentById"],
      awaitRefetchQueries: false,
    }
  );

  const { mutate: restoreStatementMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restoreStatement($id: GlobalID!) {
        restoreStatement(input: { id: $id }) {
          statement {
            id
            deletedAt
          }
        }
      }
    `),
    {
      // refetchQueries: ["fileContentById"],
      awaitRefetchQueries: false,
    }
  );

  async function rename(id: string, oldName: string, newName: string) {
    await operations.perform({
      type: "statement.rename",
      do: async () => {
        await renameStatementMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameStatementMut({ id: id, name: oldName });
      },
    });
  }

  async function delete_(id: string) {
    await operations.perform({
      type: "statement.delete",
      do: async () => {
        await deleteStatementMut({ id: id });
      },
      undo: async () => {
        await restoreStatementMut({ id: id });
      },
    });
  }

  return { move, rename, delete: delete_ };
}
