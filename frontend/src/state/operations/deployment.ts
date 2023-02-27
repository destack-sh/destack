import { graphql } from "@/gql";
import type { DeploymentStatus } from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useDeploymentOps() {
  const operations = useOperationsStore();

  const { mutate: updateMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateDeployment($id: GlobalID!, $status: DeploymentStatus!) {
        updateDeployment(input: { id: $id, status: $status }) {
          ... on Deployment {
            id
            type
            status
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function update(id: string, status: DeploymentStatus) {
    return await operations.perform({
      type: "version.deploy",
      do: async () => {
        return await updateMut({ id, status });
      },
    });
  }

  return { update };
}
