import { graphql, useFragment } from "@/gql";
import { FileHeaderType } from "@/state/fragments";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";
import { v4 as uuidv4 } from "uuid";

export function newFileId(): string {
  /* Generates a new statement global id (as in relay) with a new uuid4 */
  const nodeId = uuidv4();
  return btoa(`File:${nodeId}`);
}

export function useFileOps() {
  const operations = useOperationsStore();

  const { mutate: createFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createFile($id: GlobalID, $projectVersionId: GlobalID!, $name: String!) {
        createFile(input: { id: $id, projectVersionId: $projectVersionId, name: $name }) {
          ... on File {
            id
            ...FileHeader
            statements {
              ...StatementHeader
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  const { mutate: renameFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation renameFile($id: GlobalID!, $name: String!) {
        renameFile(input: { id: $id, name: $name }) {
          ... on File {
            id
            ...FileHeader
          }
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  const { mutate: deleteFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteFile($id: GlobalID!) {
        softDeleteFile(input: { id: $id }) {
          ... on File {
            ...FileHeader
            statements {
              ...StatementHeader
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  const { mutate: restoreFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restoreFile($id: GlobalID!) {
        restoreFile(input: { id: $id }) {
          ... on File {
            id
            ...FileHeader
            statements {
              ...StatementHeader
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  async function create(id: string, projectVersionId: string, name: string) {
    return await operations.perform({
      type: "file.create",
      do: async () => {
        const create = await createFileMut({ id, projectVersionId, name });
        if (create?.data?.createFile == null || create?.data?.createFile.__typename !== "File") {
          throw new Error("invalid response");
        }
        return useFragment(FileHeaderType, create.data.createFile);
      },
      undo: async () => {
        await deleteFileMut({ id });
      },
    });
  }

  async function rename(id: string, oldName: string, newName: string) {
    return await operations.perform({
      type: "file.rename",
      do: async () => {
        await renameFileMut({ id: id, name: newName });
      },
      undo: async () => {
        await renameFileMut({ id: id, name: oldName });
      },
    });
  }

  async function delete_(id: string) {
    return await operations.perform({
      type: "file.delete",
      do: async () => {
        await deleteFileMut({ id: id });
      },
      undo: async () => {
        await restoreFileMut({ id: id });
      },
    });
  }

  async function restore(id: string) {
    return await operations.perform({
      type: "file.restore",
      do: async () => {
        await restoreFileMut({ id: id });
      },
      undo: async () => {
        await deleteFileMut({ id: id });
      },
    });
  }

  return { create, rename, delete: delete_, restore };
}
