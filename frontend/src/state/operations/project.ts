import { graphql } from "@/gql";
import type { ProjectType, ProjectVisibility } from "@/gql/graphql";
import { useOperationsStore } from "@/state/operations";
import { useMutation } from "@vue/apollo-composable";

export function useProjectOps() {
  const operations = useOperationsStore();

  const { mutate: createProjectMut } = useMutation(
    graphql(/* GraphQL */ `
      mutation createProject($input: ProjectCreateInput!) {
        createProject(input: $input) {
          ... on Project {
            ...ProjectHeader
          }
          ...OperationInfoContent
        }
      }
    `)
  );

  async function create(ownerId: string, name: string, slug: string, type: ProjectType, visibility: ProjectVisibility) {
    return await operations.perform({
      type: "project.create",
      do: async () => {
        return await createProjectMut({ input: { ownerId, name, slug, type, visibility } });
      },
    });
  }

  return { create };
}
