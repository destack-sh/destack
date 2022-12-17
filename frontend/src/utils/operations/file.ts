import { graphql, useFragment } from "@/gql";
import { FileHeaderType } from "@/utils/fragments";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useFileOps() {
  const operations = useOperationsStore();

  const { mutate: createFileMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createFile($projectVersionId: GlobalID!, $name: String!) {
        createFile(input: { projectVersion: { id: $projectVersionId }, name: $name }) {
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
          id
          ...FileHeader
        }
      }
    `),
    { refetchQueries: ["projectVersionContent"] }
  );

  async function create(projectVersionId: string, name: string) {
    return await operations.perform({
      type: "file.create",
      do: async () => {
        const create = await createFileMut({ projectVersionId: projectVersionId, name });
        if (create?.data?.createFile == null || create?.data?.createFile.__typename !== "File") {
          throw new Error("invalid response");
        }
        return useFragment(FileHeaderType, create.data.createFile);
      },
      undo: async (file) => {
        await deleteFileMut({ id: file.id });
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
    });
  }

  return { create, rename, delete: delete_ };
}
