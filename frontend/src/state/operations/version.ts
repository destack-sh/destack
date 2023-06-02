import { graphql } from "@/gql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useProjectVersionOps() {
  const operations = useOperationsStore();

  const { mutate: updateMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation updateVersion($id: GlobalID!, $name: String!, $tag: String, $description: String) {
        updateProjectVersion(input: { id: $id, name: $name, tag: $tag, description: $description }) {
          ... on ProjectVersion {
            ...ProjectVersionHeader
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function update(id: string, name: string, tag?: string, description?: string) {
    return await operations.perform({
      type: "version.update",
      do: async () => {
        return await updateMut({ id, name, tag, description });
      },
    });
  }

  const { mutate: commitMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation commit($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {
        commit(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {
          ... on CommitPayload {
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
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersions", "projectBySlug"] }
  );

  async function commit(c: { projectVersionId: string; name?: string; tag?: string; description?: string }) {
    return await operations.perform({
      type: "version.commit",
      stateless: true,
      do: async () => {
        return await commitMut(c);
      },
    });
  }

  const { mutate: restoreMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation restore($projectVersionId: GlobalID!) {
        restore(input: { projectVersionId: $projectVersionId }) {
          ... on CommitPayload {
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
          ...OperationInfoContent
        }
      }
    `)
  );

  async function restore(projectVersionId: string) {
    return await operations.perform({
      type: "version.restore",
      stateless: true,
      do: async () => {
        return await restoreMut({ projectVersionId });
      },
    });
  }

  return { update, commit, restore };
}
