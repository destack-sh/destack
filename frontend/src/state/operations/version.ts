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

  const { mutate: snapshotMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation snapshot($projectVersionId: GlobalID!, $name: String, $tag: String, $description: String) {
        snapshot(input: { projectVersionId: $projectVersionId, name: $name, tag: $tag, description: $description }) {
          ... on SnapshotPayload {
            project {
              ...ProjectHeader
              head {
                ...ProjectVersionHeader
              }
            }
          }
          ...OperationInfoContent
        }
      }
    `),
    { refetchQueries: ["projectVersions", "projectBySlug"] }
  );

  async function snapshot(c: { projectVersionId: string; name?: string; tag?: string; description?: string }) {
    return await operations.perform({
      type: "version.snapshot",
      stateless: true,
      do: async () => {
        return await snapshotMut(c);
      },
    });
  }

  async function restore(projectVersionId: string) {
    throw new Error("not implemented");
  }

  return { update, snapshot, restore };
}
