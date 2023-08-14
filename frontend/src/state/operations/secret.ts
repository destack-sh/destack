import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useSecretOps() {
  const ops = useOperationsStore();

  const { mutate: createSecretMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createSecret($projectId: GlobalID!, $name: String, $value: JSON!) {
        createSecret(input: { projectId: $projectId, name: $name, value: $value }) {
          ... on Secret {
            id
            createdAt
            updatedAt
            sha512
            name
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function create(projectId: string, name: string | null, value: string) {
    return await ops.perform({
      type: "secret.create",
      do: async () => {
        return await createSecretMut({ projectId, name, value });
      },
    });
  }

  const { mutate: updateSecretMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateSecret($id: GlobalID!, $name: String, $value: JSON!) {
        updateSecret(input: { id: $id, name: $name, value: $value }) {
          ... on Secret {
            id
            createdAt
            updatedAt
            sha512
            name
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function update(id: string, name: string | null, value: string) {
    return await ops.perform({
      type: "secret.update",
      do: async () => {
        return await updateSecretMut({ id, name, value });
      },
    });
  }

  const { mutate: deleteSecretMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation deleteSecret($id: GlobalID!) {
        deleteSecret(input: { id: $id }) {
          ...OperationInfoContent
        }
      }
    `)
  );

  async function delete_(id: string) {
    return await ops.perform({
      type: "secret.delete",
      do: async () => {
        return await deleteSecretMut({ id });
      },
    });
  }

  return {
    create,
    update,
    delete: delete_,
  };
}
