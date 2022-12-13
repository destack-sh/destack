import { graphql } from "@/gql";
import { useOperationsStore } from "@/utils/operations";
import { useMutation } from "@vue/apollo-composable";

export function useProjectVersionOps() {
  const operations = useOperationsStore();

  const { mutate: commitMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation commit($projectVersionId: GlobalID!, $name: String!, $description: String) {
        commit(input: { projectVersionId: $projectVersionId, name: $name, description: $description }) {
          project {
            ...ProjectHeader
          }
          committedVersion {
            ...ProjectVersionHeader
          }
          newWorkingVersion {
            ...ProjectVersionHeader
          }
        }
      }
    `),
    { refetchQueries: ["projectVersions", "projectBySlug"] }
  );

  async function commit(projectVersionId: string, name: string, description?: string) {
    await operations.perform({
      type: "version.commit",
      do: async () => {
        return await commitMut({ projectVersionId, name, description });
      },
    });
  }

  return { commit };
}
